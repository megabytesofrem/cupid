#![allow(non_camel_case_types)]

//! Core of the Cupid language VM
//! Our VM is a trivial stack machine which executes its bytecode

#[macro_export]
macro_rules! construct_dword {
    ($address_bytes:expr) => {{
        let mut address = 0u32;
        for (i, &byte) in $address_bytes.iter().enumerate() {
            address |= (byte as u32) << (i * 8); // Little Endian
        }
        address
    }};
}

macro_rules! decode {
    ($self:expr, $ip:expr, no_ops) => {
        (vec![], $ip + 1)
    };

    ($self:expr, $ip:expr, byte) => {{
        let val = $self.program[($ip as usize) + 1] as u32;
        (vec![val], $ip + 2)
    }};

    ($self:expr, $ip:expr, word) => {{
        let bytes = [
            $self.program[($ip as usize) + 1],
            $self.program[($ip as usize) + 2],
        ];
        let val = (bytes[0] as u32) | ((bytes[1] as u32) << 8);
        (vec![val], $ip + 3)
    }};

    ($self:expr, $ip:expr, until_null) => {{
        let mut operands = vec![];
        let mut i = 1;
        while $self.program[($ip as usize) + i] != 0 {
            operands.push($self.program[($ip as usize) + i] as u32);
            i += 1;
        }
        (operands, $ip + i as u32 + 1)
    }};

    ($self:expr, $ip:expr, reg_byte) => {{
        let reg = $self.program[$ip + 1] as u32;
        let val = $self.program[$ip + 2] as u32;
        (vec![reg, val], $ip + 3)
    }};

    ($self:expr, $ip:expr, dword) => {{
        let bytes = [
            $self.program[($ip as usize) + 1],
            $self.program[($ip as usize) + 2],
            $self.program[($ip as usize) + 3],
            $self.program[($ip as usize) + 4],
        ];
        (vec![construct_dword!(bytes)], $ip + 5)
    }};

    ($self:expr, $ip:expr, addr) => {{
        let bytes = [
            $self.program[($ip as usize) + 1],
            $self.program[($ip as usize) + 2],
            $self.program[($ip as usize) + 3],
            $self.program[($ip as usize) + 4],
        ];
        (vec![construct_dword!(bytes)], $ip + 5)
    }};
}

use crate::runtime::disasm;
use std::collections::HashSet;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    NOP = 0x00,

    PUSH8 = 0x01,
    PUSH16 = 0x02,
    PUSH32 = 0x03,
    PUSHSZ = 0x04,
    PUSHAC = 0x05,
    POP8 = 0x06,
    POP16 = 0x07,
    POP32 = 0x08,
    POPSZ = 0x09,

    // Jumping
    CMP = 0x0C,
    JABS = 0x0D,
    JREL = 0x0E,
    JEQ = 0x0F,
    JNE = 0x10,

    // Math
    ADD = 0x11,
    SUB = 0x12,
    MUL = 0x13,
    DIV = 0x14,

    // Other
    CALL = 0x15,
    CALL_NAT = 0x16,
    RET = 0x17,

    HALT = 0xFF,
}

/// A VM operation
#[derive(Debug, Clone)]
pub struct VMOp {
    opcode: Op,
    operands: Vec<u32>,
}

/// A managed value in the VM
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VMValue {
    Int(u32),
    String(Vec<u8>),
    Data(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub address: u32,
    pub arity: u8,
    pub local_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallFrame {
    pub function_id: u32,
    pub return_address: u32,
    pub base_ptr: u32,
}

#[derive(Debug, Clone)]
pub struct VM {
    next_ip: u32,
    pub ip: u32, // instruction pointer
    pub sp: u32, // stack pointer
    pub ac: u32, // accumulator
    pub bp: u32, // base pointer for current call frame

    pub program: Vec<u8>,
    pub function_table: Vec<Function>,
    pub call_stack: Vec<CallFrame>,

    pub strings: HashSet<(u32, Vec<u8>)>,
    pub heap: Vec<Option<VMValue>>,
    pub free_list: Vec<u32>,
    pub stack: Vec<VMValue>,
}

impl TryFrom<u8> for Op {
    type Error = &'static str;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Op::NOP),
            0x01 => Ok(Op::PUSH8),
            0x02 => Ok(Op::PUSH16),
            0x03 => Ok(Op::PUSH32),
            0x04 => Ok(Op::PUSHSZ),
            0x05 => Ok(Op::PUSHAC),
            0x06 => Ok(Op::POP8),
            0x07 => Ok(Op::POP16),
            0x08 => Ok(Op::POP32),
            0x09 => Ok(Op::POPSZ),
            0x0C => Ok(Op::CMP),
            0x0D => Ok(Op::JABS),
            0x0E => Ok(Op::JREL),
            0x0F => Ok(Op::JEQ),
            0x10 => Ok(Op::JNE),
            0x11 => Ok(Op::ADD),
            0x12 => Ok(Op::SUB),
            0x13 => Ok(Op::MUL),
            0x14 => Ok(Op::DIV),
            0x15 => Ok(Op::CALL),
            0x16 => Ok(Op::CALL_NAT),
            0x17 => Ok(Op::RET),
            0xFF => Ok(Op::HALT),
            _ => Err("Unknown opcode"),
        }
    }
}

impl VM {
    pub fn new() -> Self {
        Self {
            next_ip: 0,
            ip: 0,
            sp: 0,
            ac: 0,
            bp: 0,
            program: Vec::new(),

            function_table: Vec::new(),
            call_stack: Vec::new(),
            strings: HashSet::new(),

            heap: Vec::new(),
            free_list: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn fetch_decode(&mut self) -> VMOp {
        // Determine the number of operands and read that many
        let opcode = self.program[self.ip as usize];
        println!("fetch_decode: called for {:02X}", opcode);
        let (operands, next_ip) = match opcode.try_into().unwrap() {
            Op::NOP | Op::HALT => decode!(self, self.ip, no_ops),

            Op::PUSH8 => decode!(self, self.ip, byte),
            Op::PUSH16 => decode!(self, self.ip, word),
            Op::PUSH32 => decode!(self, self.ip, dword),
            Op::PUSHSZ => decode!(self, self.ip, until_null),
            Op::PUSHAC => decode!(self, self.ip, no_ops),
            Op::POP8 => decode!(self, self.ip, no_ops),
            Op::POP16 => decode!(self, self.ip, no_ops),
            Op::POP32 => decode!(self, self.ip, no_ops),
            Op::POPSZ => decode!(self, self.ip, no_ops),
            Op::CMP => decode!(self, self.ip, no_ops),
            Op::JABS => decode!(self, self.ip, addr),
            Op::JREL => decode!(self, self.ip, addr),
            Op::JEQ => decode!(self, self.ip, addr),
            Op::JNE => decode!(self, self.ip, addr),
            Op::ADD => decode!(self, self.ip, no_ops),
            Op::SUB => decode!(self, self.ip, no_ops),
            Op::MUL => decode!(self, self.ip, no_ops),
            Op::DIV => decode!(self, self.ip, no_ops),
            Op::CALL => decode!(self, self.ip, addr),
            Op::CALL_NAT => decode!(self, self.ip, addr),
            Op::RET => decode!(self, self.ip, no_ops),

            _ => panic!("Unimplemented opcode in fetch_decode: {:02X}", opcode),
        };

        self.next_ip = next_ip;

        VMOp {
            opcode: opcode.try_into().unwrap(),
            operands: operands.into(),
        }
    }

    fn call_function(&mut self, func: u32) {
        // Call a function by pushing a new call frame
        let frame = CallFrame {
            function_id: func,
            return_address: self.ip + 2,
            base_ptr: self.bp,
        };

        self.call_stack.push(frame);
        self.bp = self.stack.len() as u32;
        self.ip = func;
    }

    fn return_from_function(&mut self) {
        // Return from a function by popping the call frame
        if let Some(frame) = self.call_stack.pop() {
            // Restore stack to pre-call state
            self.ip = frame.return_address;

            let func = &self.function_table[frame.function_id as usize];
            let return_value = self.stack.pop();

            // Clean up local variables defined in func
            for _ in 0..func.local_count {
                self.stack.pop();
            }

            self.stack.truncate(frame.base_ptr as usize);

            if let Some(rv) = return_value {
                self.stack.push(rv);
            }

            self.bp = frame.base_ptr;
        }
    }

    fn execute(&mut self, mach_op: VMOp) {
        // self.dump_ctx();

        match mach_op.opcode {
            Op::NOP => {} // nop

            Op::HALT => {} // halt

            // Memory
            Op::PUSH8 => {
                // pushi <value> - push immediate value onto stack
                let value = mach_op.operands[0];
                self.stack.push(VMValue::Int(value));
            }
            Op::PUSH16 => {
                // pushi <value> - push immediate value onto stack
                let value = mach_op.operands[0];
                self.stack.push(VMValue::Int(value));
            }
            Op::PUSH32 => {
                // pushi <value> - push immediate value onto stack
                let value = mach_op.operands[0];
                self.stack.push(VMValue::Int(value));
            }

            Op::PUSHSZ => {
                // pushsz "<value>" - push string literal onto stack
                let string_bytes: Vec<u8> = mach_op.operands.iter().map(|&x| x as u8).collect();
                self.stack.push(VMValue::String(string_bytes));
            }
            Op::PUSHAC => {
                // pushac - push ac onto stack
                self.stack.push(VMValue::Int(self.ac));
            }
            Op::POP8 => {
                // popi - pop value from stack into ac
                if let Some(value) = self.stack.pop() {
                    if let VMValue::Int(v) = value {
                        self.ac = v & 0xFF; // Mask to 8 bits
                    }
                }
            }
            Op::POP16 => {
                // popi - pop value from stack into ac
                if let Some(value) = self.stack.pop() {
                    if let VMValue::Int(v) = value {
                        self.ac = v & 0xFFFF; // Mask to 16 bits
                    }
                }
            }
            Op::POP32 => {
                // popi - pop value from stack into ac
                if let Some(value) = self.stack.pop() {
                    if let VMValue::Int(v) = value {
                        self.ac = v; // Full 32 bits
                    }
                }
            }
            Op::POPSZ => {
                // popsz - pop string literal from stack, store in string table
                if let Some(value) = self.stack.pop() {
                    if let VMValue::String(v) = value {
                        self.strings.insert((self.ac, v));
                    }
                }
            }

            // Jumping
            Op::CMP => {
                // cmp - compare top two values on stack
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        let result = if i1 < i2 {
                            0
                        } else if i1 == i2 {
                            1
                        } else {
                            2
                        };
                        self.stack.push(VMValue::Int(result));
                    }
                }
            }

            Op::JABS => {
                // jmp $<address> - absolute jump
                let address = mach_op.operands[0];
                self.ip = address;
            }
            Op::JREL => {
                // jmp +<offset> - relative jump
                let offset = mach_op.operands[0];
                self.ip = self.ip.wrapping_add(offset);
            }
            Op::JEQ => {
                // jeq <address> - jump if ac == 0
                let address = mach_op.operands[0];

                if let Some(top) = self.stack.pop() {
                    if top == VMValue::Int(1) {
                        self.ip = address;
                    }
                }
            }
            Op::JNE => {
                // jne <address> - jump if ac != 0
                let address = mach_op.operands[0];

                if let Some(top) = self.stack.pop() {
                    if top != VMValue::Int(1) {
                        self.ip = address;
                    }
                }
            }

            // Math
            Op::ADD => {
                // add - pop two values from stack, add, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        let result = i1.wrapping_add(i2);

                        self.ac = result;
                        self.stack.push(VMValue::Int(result));
                    }
                }
            }
            Op::SUB => {
                // sub - pop two values from stack, subtract, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        let result = i1.wrapping_sub(i2);

                        self.ac = result;
                        self.stack.push(VMValue::Int(result));
                    }
                }
            }
            Op::MUL => {
                // mul - pop two values from stack, multiply, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        let result = i1.wrapping_mul(i2);

                        self.ac = result;
                        self.stack.push(VMValue::Int(result));
                    }
                }
            }
            Op::DIV => {
                // div - pop two values from stack, divide, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        let result = i1.wrapping_div(i2);

                        self.ac = result;
                        self.stack.push(VMValue::Int(result));
                    }
                }
            }

            // Call function
            Op::CALL => {
                let func = mach_op.operands[0];
                self.call_function(func);
            }

            Op::RET => {
                self.return_from_function();
            }

            _ => todo!("Unimplemented opcode: {:?}", mach_op.opcode),
        }

        self.dump_ctx();
    }

    pub fn dump_ctx(&self) {
        println!("------------------------------------------------------------");
        println!(
            "ip: {:08X}\tsp: {:08X}\tac: {:08X}\tbp: {:08X}",
            self.ip, self.sp, self.ac, self.bp
        );
        println!("------------------------------------------------------------");
        disasm::dump_memory(self, 0, 64);

        disasm::dump_stack(self);
    }

    pub fn cycle(&mut self) {
        let opcode = self.program[self.ip as usize];
        let mach_op = self.fetch_decode();

        self.execute(mach_op);

        match opcode {
            0x02 => {
                // Step past the entire instruction including null terminator
                let mut i = 1;
                while (self.ip as usize + i) < self.program.len()
                    && self.program[self.ip as usize + i] != 0
                {
                    i += 1;
                }
                self.ip += i as u32 + 1; // +1 for null terminator
            }

            0x08 | 0x09 | 0x0A | 0x0B | 0x10 | 0x12 => {
                // These opcodes modify ip directly, so we don't step here
            }

            _ => {
                self.ip = self.next_ip;
            }
        }
    }

    pub fn step(&mut self, size: u32) {
        self.ip += 1 + size;
    }

    pub fn run(&mut self) {
        loop {
            if self.ip as usize >= self.program.len() {
                break;
            }

            let opcode = self.program[self.ip as usize];
            if opcode == 0xFF {
                // halt instruction
                break;
            }

            self.cycle();
            self.dump_ctx();

            // Safety break to prevent infinite loops
            if self.ip > 1000 {
                println!("ERROR: ip exceeded 1000");
                break;
            }
        }
    }

    pub fn run_with(&mut self, code: &[u8]) {
        for i in 0..code.len() {
            self.program.push(code[i]);
        }

        self.run();
    }

    // Opcodes
}
