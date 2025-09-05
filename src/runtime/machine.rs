//! Core of the Cupid language VM
//! Our VM is a trivial stack machine which executes its bytecode

use crate::runtime::disasm;
use std::collections::HashSet;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Nop = 0x00,

    // Memory
    Pushi = 0x01,
    Pushsz = 0x02,
    Popi = 0x03,
    Popsz = 0x04,

    // Jumping
    JmpAbs = 0x08,
    JmpRel = 0x09,
    Jeq = 0x0A,
    Jne = 0x0B,

    // Math
    Add = 0x0C,
    Sub = 0x0D,
    Mul = 0x0E,
    Div = 0x0F,
    Halt = 0xFF,
}

/// A VM operation
#[derive(Debug, Clone)]
pub struct VMOp {
    opcode: Op,
    operands: Vec<u16>,
}

/// A managed value in the VM
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VMValue {
    Int(u16),
    String(Vec<u8>),
    Data(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct VM {
    pub ip: u16, // instruction pointer
    pub sp: u16, // stack pointer
    pub ac: u16, // accumulator

    pub program: Vec<u8>,
    strings: HashSet<(u16, Vec<u8>)>,
    stack: Vec<VMValue>,
}

impl From<u8> for Op {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => Op::Nop,
            0x01 => Op::Pushi,
            0x02 => Op::Pushsz,
            0x03 => Op::Popi,
            0x04 => Op::Popsz,
            0x08 => Op::JmpAbs,
            0x09 => Op::JmpRel,
            0x0A => Op::Jeq,
            0x0B => Op::Jne,
            0xFF => Op::Halt,
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
            program: Vec::new(),
            strings: HashSet::new(),
            stack: Vec::new(),
        }
    }

    fn operand_count(&self, opcode: u8) -> usize {
        match opcode {
            0x00 => 0, // nop
            0x01 => 1, // pushi <value>
            0x02 => 1, // pushsz <value>
            0x03 => 0, // popi
            0x04 => 0, // popsz
            0x08 => 1, // jmp <address>
            0x09 => 1, // jmp <offset>
            0x0A => 1, // jeq <address>
            0x0B => 1, // jne <address>
            0xFF => 0, // halt
            _ => 0,
        }
    }

    fn fetch_decode(&mut self) -> VMOp {
        // Determine the number of operands and read that many
        let opcode = self.program[self.ip as usize];
        let operand_count = self.operand_count(opcode);
        let mut operands = Vec::new();
        while operands.len() < operand_count {
            self.step();
            operands.push(self.program[self.ip as usize] as u16);
        }
        VMOp {
            opcode: opcode.into(),
            operands: operands.into(),
        }
    }

    fn execute(&mut self, mach_op: VMOp) {
        self.dump_ctx();

        match mach_op.opcode {
            Op::Nop => {} // nop

            Op::Halt => {} // halt

            // Memory
            Op::Pushi => {
                // pushi <value> - push immediate value onto stack
                let value = mach_op.operands[0];
                self.stack.push(VMValue::Int(value));
            }
            Op::Pushsz => {
                // pushsz "<value>" - push string literal onto stack
                let value = mach_op.operands[0];
                self.stack.push(VMValue::String(vec![value as u8]));
            }
            Op::Popi => {
                // popi - pop immediate value from stack, store in ac
                if let Some(value) = self.stack.pop() {
                    self.ac = match value {
                        VMValue::Int(v) => v,
                        _ => unreachable!(),
                    };
                }
            }
            Op::Popsz => {
                // popsz - pop string literal from stack, store in string table
                if let Some(value) = self.stack.pop() {
                    if let VMValue::String(v) = value {
                        self.strings.insert((self.ac, v));
                    }
                }
            }

            // Jumping
            Op::JmpAbs => {
                // jmp $<address> - absolute jump
                let address = mach_op.operands[0];
                self.ip = address;
            }
            Op::JmpRel => {
                // jmp +<offset> - relative jump
                let offset = mach_op.operands[0];
                self.ip = self.ip.wrapping_add(offset);
            }
            Op::Jeq => {
                // jeq <address> - jump if ac == 0
                let address = mach_op.operands[0];
                if self.ac == 0 {
                    self.ip = address;
                }
            }
            Op::Jne => {
                // jne <address> - jump if ac != 0
                let address = mach_op.operands[0];
                if self.ac != 0 {
                    self.ip = address;
                }
            }

            // Math
            Op::Add => {
                // add - pop two values from stack, add, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        self.stack.push(VMValue::Int(i1.wrapping_add(i2)));
                    }
                }
            }
            Op::Sub => {
                // sub - pop two values from stack, subtract, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        self.stack.push(VMValue::Int(i1.wrapping_sub(i2)));
                    }
                }
            }
            Op::Mul => {
                // mul - pop two values from stack, multiply, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        self.stack.push(VMValue::Int(i1.wrapping_mul(i2)));
                    }
                }
            }
            Op::Div => {
                // div - pop two values from stack, divide, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        self.stack.push(VMValue::Int(i1.wrapping_div(i2)));
                    }
                }
            }
        }

        self.dump_ctx();
    }

    pub fn dump_ctx(&self) {
        println!("--------------------------");
        println!("ip: {:04X}", self.ip);
        println!("ac: {:04X}", self.ac);
        println!("--------------------------");
        disasm::dump_memory(self, 0, 64);
    }

    pub fn cycle(&mut self) {
        let mach_op = self.fetch_decode();
        self.execute(mach_op);
        self.step();
    }

    pub fn step(&mut self) {
        self.ip += 1;
    }

    pub fn run(&mut self) {
        while (self.ip as usize) < self.program.len() {
            let opcode = self.program[self.ip as usize];
            if opcode == 0xFF {
                // halt instruction
                break;
            }
            self.cycle();
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
