#![allow(non_camel_case_types)]

//! Core of the Cupid language VM
//! Our VM is a trivial stack machine which executes its bytecode

use crate::runtime::disasm;
use std::collections::HashSet;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    NOP = 0x00,

    // Memory
    PUSH_I = 0x01,
    PUSH_SZ = 0x02,
    POP_I = 0x03,
    POP_SZ = 0x04,

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
    HALT = 0xFF,
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
            0x00 => Op::NOP,
            0x01 => Op::PUSH_I,
            0x02 => Op::PUSH_SZ,
            0x03 => Op::POP_I,
            0x04 => Op::POP_SZ,
            0x08 => Op::JMP_ABS,
            0x09 => Op::JMP_REL,
            0x0A => Op::JMP_EQ,
            0x0B => Op::JMP_NE,
            0xFF => Op::HALT,
            0x0C => Op::ADD,
            0x0D => Op::SUB,
            0x0E => Op::MUL,
            0x0F => Op::DIV,
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
            0x0C => 0, // add
            0x0D => 0, // sub
            0x0E => 0, // mul
            0x0F => 0, // div
            0xFF => 0, // halt
            _ => 0,
        }
    }

    fn fetch_decode(&mut self) -> VMOp {
        // Determine the number of operands and read that many
        let opcode = self.program[self.ip as usize];
        let operand_count = self.operand_count(opcode);
        let mut operands = Vec::new();

        for i in 1..=operand_count {
            operands.push(self.program[self.ip as usize + i] as u16);
        }

        VMOp {
            opcode: opcode.into(),
            operands: operands.into(),
        }
    }

    fn execute(&mut self, mach_op: VMOp) {
        self.dump_ctx();

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
                let value = mach_op.operands[0];
                self.stack.push(VMValue::String(vec![value as u8]));
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
                        self.stack.push(VMValue::Int(i1.wrapping_add(i2)));
                    }
                }
            }
            Op::SUB => {
                // sub - pop two values from stack, subtract, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        self.stack.push(VMValue::Int(i1.wrapping_sub(i2)));
                    }
                }
            }
            Op::MUL => {
                // mul - pop two values from stack, multiply, push result
                if let (Some(v1), Some(v2)) = (self.stack.pop(), self.stack.pop()) {
                    if let (VMValue::Int(i1), VMValue::Int(i2)) = (v1, v2) {
                        self.stack.push(VMValue::Int(i1.wrapping_mul(i2)));
                    }
                }
            }
            Op::DIV => {
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
        let opcode = self.program[self.ip as usize];

        // Step by the operand count
        let operand_count = self.operand_count(opcode);
        let mach_op = self.fetch_decode();

        self.execute(mach_op);
        self.step(operand_count as u16);
    }

    pub fn step(&mut self, size: u16) {
        self.ip += 1 + size;
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
