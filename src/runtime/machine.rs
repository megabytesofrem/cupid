#![allow(non_camel_case_types)]

//! Core of the Cupid language VM
//! Our VM is a trivial stack machine which executes its bytecode

#[macro_export]
macro_rules! construct_vm_addr {
    ($address_bytes:expr) => {{
        // Convert to a u32
        let mut address = 0u32;
        for &byte in &$address_bytes {
            address = (address << 8) | (byte as u32);
        }

        address
    }};
}

use crate::runtime::disasm;
use std::collections::HashSet;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    NOP = 0x00,

    // Memory
    PUSH_I = 0x01,
    PUSH_SZ = 0x02,
    PUSHAC = 0x03,
    POP_I = 0x04,
    POP_SZ = 0x05,

    // Jumping
    JMP_ABS = 0x08,
    JMP_REL = 0x09,
    JMP_EQ = 0x0A,
    JMP_NE = 0x0B,

    // Math
    ADD = 0x0C,
    SUB = 0x0D,
    MUL = 0x0E,
    DIV = 0x0F,

    // Calls
    CALL = 0x10,
    CALL_NAT = 0x11,
    RET = 0x12,

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
    pub ip: u32, // instruction pointer
    pub sp: u32, // stack pointer
    pub ac: u32, // accumulator
    pub bp: u32, // base pointer for current call frame

    pub program: Vec<u8>,
    pub function_table: Vec<Function>,
    pub call_stack: Vec<CallFrame>,

    pub strings: HashSet<(u32, Vec<u8>)>,
    pub stack: Vec<VMValue>,
}

impl From<u8> for Op {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => Op::NOP,
            0x01 => Op::PUSH_I,
            0x02 => Op::PUSH_SZ,
            0x03 => Op::PUSHAC,
            0x04 => Op::POP_I,
            0x05 => Op::POP_SZ,
            // Jumping
            0x08 => Op::JMP_ABS,
            0x09 => Op::JMP_REL,
            0x0A => Op::JMP_EQ,
            0x0B => Op::JMP_NE,
            0xFF => Op::HALT,
            0x0C => Op::ADD,
            0x0D => Op::SUB,
            0x0E => Op::MUL,
            0x0F => Op::DIV,
            0x10 => Op::CALL,
            0x11 => Op::CALL_NAT,
            0x12 => Op::RET,
            _ => unreachable!(),
        }
    }
}

impl VM {
    pub fn new() -> Self {
        Self {
            ip: 0,
            sp: 0,
            ac: 0,
            bp: 0,
            program: Vec::new(),

            function_table: Vec::new(),
            call_stack: Vec::new(),
            strings: HashSet::new(),
            stack: Vec::new(),
        }
    }

    fn operand_count(&self, opcode: u8) -> usize {
        match opcode {
            0x00 => 0, // nop
            0x01 => 1, // pushi <value>
            0x02 => 0, // pushsz <value>, handled specially
            0x03 => 0, // pushac
            0x04 => 0, // popi
            0x05 => 0, // popsz
            0x08 => 0, // jmp <address>
            0x09 => 1, // jmp <offset>
            0x0A => 1, // jeq <address>
            0x0B => 1, // jne <address>
            0x0C => 0, // add
            0x0D => 0, // sub
            0x0E => 0, // mul
            0x0F => 0, // div
            0x10 => 1, // call <address>
            0x11 => 0, // callnat <name>, handled specially
            0x12 => 0, // ret
            0xFF => 0, // halt
            _ => 0,
        }
    }

    fn fetch_decode(&mut self) -> VMOp {
        // Determine the number of operands and read that many
        let opcode = self.program[self.ip as usize];
        let mut operands = Vec::new();

        println!("fetch_decode: called for {:?}", opcode);

        match opcode {
            0x02 => {
                // pushsz <value>: read until null byte
                let mut i = 1;
                while (self.ip as usize + i) < self.program.len()
                    && self.program[self.ip as usize + i] != 0
                {
                    let next = self.program[self.ip as usize + i] as u32;
                    operands.push(next);
                    i += 1;
                }
            }

            0x08 | 0x10 | 0x0A | 0x0B => {
                // jmp: read until null byte as address
                // jeq: read until null byte as address
                // jne: read until null byte as address
                // call: read until null byte as address
                println!("fetch_decode: reading operands for {:?}", opcode);

                // Read until NUL byte
                let mut i = 1;
                while (self.ip as usize + i) < self.program.len()
                    && self.program[self.ip as usize + i] != 0
                {
                    let next = self.program[self.ip as usize + i] as u32;
                    operands.push(next);
                    i += 1;
                }

                // Convert individual bytes back to address (big-endian)
                let address = construct_vm_addr!(operands);

                println!("fetch_decode: final address {:08X}", address);

                // Replace operands with single address value
                operands.clear();
                operands.push(address);
            }
            _ => {
                let operand_count = self.operand_count(opcode);
                for i in 1..=operand_count {
                    operands.push(self.program[self.ip as usize + i] as u32);
                }
            }
        }

        VMOp {
            opcode: opcode.into(),
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
        self.ip = self.function_table[func as usize].address;
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
            Op::PUSH_I => {
                // pushi <value> - push immediate value onto stack
                let value = mach_op.operands[0];
                self.stack.push(VMValue::Int(value));
            }
            Op::PUSH_SZ => {
                // pushsz "<value>" - push string literal onto stack
                let string_bytes: Vec<u8> = mach_op.operands.iter().map(|&x| x as u8).collect();
                self.stack.push(VMValue::String(string_bytes));
            }
            Op::PUSHAC => {
                // pushac - push ac onto stack
                self.stack.push(VMValue::Int(self.ac));
            }
            Op::POP_I => {
                // popi - pop immediate value from stack, store in ac
                if let Some(value) = self.stack.pop() {
                    self.ac = match value {
                        VMValue::Int(v) => v,
                        _ => unreachable!(),
                    };
                }
            }
            Op::POP_SZ => {
                // popsz - pop string literal from stack, store in string table
                if let Some(value) = self.stack.pop() {
                    if let VMValue::String(v) = value {
                        self.strings.insert((self.ac, v));
                    }
                }
            }

            // Jumping
            Op::JMP_ABS => {
                // jmp $<address> - absolute jump
                let address = mach_op.operands[0];
                self.ip = address;
            }
            Op::JMP_REL => {
                // jmp +<offset> - relative jump
                let offset = mach_op.operands[0];
                self.ip = self.ip.wrapping_add(offset);
            }
            Op::JMP_EQ => {
                // jeq <address> - jump if ac == 0
                let address = mach_op.operands[0];

                if self.ac == 0 {
                    self.ip = address;
                }
            }
            Op::JMP_NE => {
                // jne <address> - jump if ac != 0
                let address = mach_op.operands[0];

                if self.ac != 0 {
                    self.ip = address;
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
        println!("--------------------------");
        println!("ip: {:08X}", self.ip);
        println!("sp: {:08X}", self.sp);
        println!("ac: {:08X}", self.ac);
        println!("bp: {:08X}", self.bp);
        println!("--------------------------");
        disasm::dump_memory(self, 0, 64);

        disasm::dump_stack(self);
    }

    pub fn cycle(&mut self) {
        let opcode = self.program[self.ip as usize];

        // Step by the operand count
        let operand_count = self.operand_count(opcode);
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
                self.step(operand_count as u32);
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
