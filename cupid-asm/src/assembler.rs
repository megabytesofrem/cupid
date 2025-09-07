//! Assembler for Cupids bytecode
//!
//! Bytecode assembler.
//!
//! Takes the generated AST from the parser, resolves labels, inlines data declarations
//! and outputs a form of bytecode for the VM.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser::Directive;

use super::Instr;
use super::parser::{Ast, Node};

#[macro_export]
macro_rules! def_opcode {
    ($self:ident, $mme:ident, $opcode:expr, $operand_count:expr) => {
        $self
            .operand_table
            .push((stringify!($mme).to_string(), ($opcode, $operand_count)));
    };
    ($self:ident, $mme:ident, $opcode:expr, $operand_count:expr, $desc:expr) => {
        $self
            .operand_table
            .push((stringify!($mme).to_string(), ($opcode, $operand_count)));
    };
}

macro_rules! find_opcode {
    ($self:ident, $mme:expr) => {
        $self
            .operand_table
            .iter()
            .find(|(name, _)| name == $mme)
            .map(|(_, (opcode, operand_count))| (*opcode, *operand_count))
    };

    ($self:ident, $opcode:expr, opcode) => {
        $self
            .operand_table
            .iter()
            .find(|(_, (code, _))| *code == $opcode)
            .map(|(name, (_, operand_count))| (name.clone(), *operand_count))
    };
}

macro_rules! find_opcode_size {
    ($self:ident, $mme:expr) => {
        $self
            .operand_table
            .iter()
            .find(|(name, _)| name == $mme)
            .map(|(_, (_, size))| *size)
            .unwrap_or(0)
    };
}

macro_rules! visit_push_int {
    ($self:ident, $instr:expr, $args:expr, $size:expr) => {
        if let Some(Node::Int(value)) = $args.first() {
            $self.push_sized_int(*value as u32, $size);
        } else if let Some(Node::Ident(label)) = $args.first() {
            let const_value = $self
                .consts
                .get(label)
                .cloned()
                .ok_or(format!("Undefined constant: {}", label))?;

            if let Node::Int(value) = const_value {
                $self.push_sized_int(value as u32, $size);
            } else {
                return Err(format!("{:?} expects an integer argument", $instr));
            }
        } else {
            return Err(format!("{:?} expects an integer argument", $instr));
        }
    };
}

#[derive(Debug, Clone)]
pub struct Assembler {
    pub ast: Ast,   // maybe make this a Rc type, so we dont need to '.clone' as much?
    pub ptr: usize, // current position, translates to ip

    operand_table: Vec<(String, (u8, usize))>, // opcode, operand count
    buffer: Vec<u8>,
    root_path: PathBuf,

    // Labels to resolve to addresses
    pub labels: HashMap<String, usize>,
    pub consts: HashMap<String, Node>,

    // Output bytecode
    pub output_bc: Vec<u8>,
}

impl Assembler {
    pub fn new(root_path: &Path) -> Self {
        Self {
            ast: Vec::new(),
            ptr: 0,
            labels: HashMap::new(),
            consts: HashMap::new(),
            output_bc: Vec::new(),
            buffer: Vec::new(),
            root_path: root_path.into(),
            operand_table: Vec::new(),
        }
    }

    #[rustfmt::skip]
    pub fn make_operand_table(&mut self) {
        //                mme       opcode operand_count description
        def_opcode!(self, nop,      0x00,   0,           "no-op");
        def_opcode!(self, push8,    0x01,   1,           "push immediate value (byte)");
        def_opcode!(self, push16,   0x02,   1,           "push immediate value (word)");
        def_opcode!(self, push32,   0x03,   1,           "push immediate value (dword)");
        def_opcode!(self, pushsz,   0x04,   1,           "push NUL terminated string");
        def_opcode!(self, pushac,   0x05,   0,           "push accumulator");
        def_opcode!(self, pop8,     0x06,   0,           "pop value from stack (byte)"); 
        def_opcode!(self, pop16,    0x07,   0,           "pop value from stack (word)");
        def_opcode!(self, pop32,    0x08,   0,           "pop value from stack (dword)");
        def_opcode!(self, popsz,    0x09,   0,           "pop value from stack (string)");

        def_opcode!(self, cmp,      0x0C,   0,           "pop two values, set f");
        def_opcode!(self, jabs,     0x0D,   1,           "jump unconditionally (absolute)");
        def_opcode!(self, jrel,     0x0E,   1,           "jump unconditionally (relative)");
        def_opcode!(self, jeq,      0x0F,   1,           "jump if equal (f == 0)");
        def_opcode!(self, jne,      0x10,   1,           "jump if not equal (f != 0)");

        def_opcode!(self, add,      0x11,   0,           "pop two values, push sum");
        def_opcode!(self, sub,      0x12,   0,           "pop two values, push difference");
        def_opcode!(self, mul,      0x13,   0,           "pop two values, push product");
        def_opcode!(self, div,      0x14,   0,           "pop two values, push quotient");

        def_opcode!(self, call,     0x15,   1,           "call function at address");
        def_opcode!(self, callnat,  0x16,   1,           "call native function");
        def_opcode!(self, ret,      0x17,   0,           "return from function");
        def_opcode!(self, halt,     0xFF,   0,           "halt execution");
    }

    fn count_size(&self, value: &Node) -> usize {
        match value {
            Node::Str(s) => s.len() + 1, // String bytes + null terminator
            Node::Int(value) => {
                if *value <= 0xFF {
                    1 // u8
                } else if *value <= 0xFFFF {
                    2 // u16
                } else {
                    4 // u32
                }
            }
            Node::ByteSeq(bytes) => bytes.len(),
            Node::Ident(ident) => ident.len(),

            _ => 0,
        }
    }

    fn calculate_instruction_size(&self, instr: &Instr, args: &[Node]) -> Result<usize, String> {
        let mut size = 1;
        let jumps = matches!(
            *instr,
            Instr::JABS | Instr::JREL | Instr::JEQ | Instr::JNE | Instr::CALL
        );

        size += match *instr {
            _instr if jumps => 4, // 4 byte address

            Instr::PUSHSZ => {
                if let Some(Node::Str(s)) = args.first() {
                    s.len() + 1 // String bytes + null terminator
                } else if let Some(arg) = args.first() {
                    self.count_size(arg)
                } else {
                    return Err("PUSHSZ expects a string argument".into());
                }
            }

            Instr::PUSH8 => find_opcode!(self, "push8")
                .map(|(_, size)| size)
                .unwrap_or(1), // Default to 1 if not found

            Instr::PUSH16 => find_opcode!(self, "push16")
                .map(|(_, size)| size)
                .unwrap_or(2), // Default to 2 if not found

            Instr::PUSH32 => find_opcode!(self, "push32")
                .map(|(_, size)| size)
                .unwrap_or(4), // Default to 4 if not found

            _ => self.operand_count(*instr as u8),
        };

        Ok(size)
    }

    // --------------------------------------
    // Instruction encoding
    // --------------------------------------
    #[rustfmt::skip]
    fn encode_int_operand(&mut self, value: u32, size: usize) {
        match size {
            1 => { self.buffer.push(value as u8); self.ptr += 1; } // u8
            2 => { self.buffer.extend_from_slice(&(value as u16).to_le_bytes()); self.ptr += 2; } // u16
            4 => { self.buffer.extend_from_slice(&value.to_le_bytes()); self.ptr += 4; } // u32
            _ => panic!("Invalid integer size"),
        }
    }

    fn encode_string_operand(&mut self, value: &str) {
        self.buffer.extend_from_slice(value.as_bytes());
        self.buffer.push(0); // Null terminator
        self.ptr += value.len() + 1;
    }

    fn encode_byte_seq_operand(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
        self.ptr += bytes.len();
    }

    fn encode_address_operand(&mut self, address: usize) {
        self.buffer
            .extend_from_slice(&(address as u32).to_le_bytes());
        self.ptr += 4; // Addresses are 4 bytes
    }

    fn encode_operand(&mut self, arg: &Node) -> Result<usize, String> {
        match arg {
            Node::Ident(label) => {
                if let Some(&addr) = self.labels.get(label) {
                    self.encode_address_operand(addr);
                    Ok(addr)
                } else if let Some(const_value) = self.consts.get(label) {
                    // Inline constant value
                    let const_value = const_value.clone();
                    self.encode_operand(&const_value)
                } else {
                    Err(format!("Undefined label or constant: {}", label))
                }
            }
            Node::Int(value) => {
                // VM expects 4 byte addresses
                self.encode_int_operand(*value as u32, 4);
                Ok(*value as usize)
            }
            Node::Str(value) => {
                self.encode_string_operand(value);
                Ok(0)
            }
            Node::ByteSeq(bytes) => {
                self.encode_byte_seq_operand(bytes);
                Ok(0)
            }
            n => Err(format!("Unknown argument type {:?}", n)),
        }
    }

    // --------------------------------------

    // Visitors
    // --------------------------------------

    // Copy and paste from src/runtime/machine.rs
    fn operand_count(&self, instr: u8) -> usize {
        find_opcode!(self, instr, opcode)
            .map(|(_, count)| count)
            .unwrap_or(0)
    }

    // Helper function to push either an u8, u16 or u32.
    // We need to check and increment ptr by the appropriate amount
    fn push_sized_int(&mut self, value: u32, size: usize) {
        if size == 1 {
            // u8
            self.buffer.push(value as u8);
            self.ptr += 1;
        } else if size == 2 {
            // u16
            self.buffer.extend_from_slice(&(value as u16).to_le_bytes());
            self.ptr += 2;
        } else {
            // u32
            self.buffer.extend_from_slice(&value.to_le_bytes());
            self.ptr += 4;
        }
    }

    fn visit_directive(&mut self, directive: &Directive) -> Result<(), String> {
        // TODO: Implement later, for now just fake it
        // self.buffer.push(0xFF);
        // self.ptr += 1;

        match directive {
            Directive::Define(name, value) => {
                self.consts.insert(name.clone(), *value.clone());
            }

            Directive::Include(path) => {
                let full_path = self.root_path.join(path);

                // C style #include, very dumb
                let contents = std::fs::read_to_string(full_path).map_err(|e| e.to_string())?;
                let mut lexer = crate::lexer::Lexer::new(&contents);
                let mut tokens = lexer.lex();
                let mut parser = crate::parser::Parser::new(&mut tokens, &contents);

                let ast = parser.parse().map_err(|e| e.to_string())?;
                for node in ast {
                    self.visit_node(&node)?;
                }
            }

            Directive::Stringz(string) => {
                self.buffer.extend_from_slice(string.as_bytes());
                self.buffer.push(0); // Null terminator
                self.ptr += string.len() + 1;
            }

            Directive::ByteSeq(bytes) => {
                self.buffer.extend_from_slice(bytes);
                self.ptr += bytes.len();
            }

            Directive::Rep(count, body) => {
                for _ in 0..*count {
                    for node in body {
                        self.visit_node(node)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn visit_instruction(&mut self, instr: &Instr, args: Vec<Node>) -> Result<(), String> {
        self.buffer.push(*instr as u8);
        self.ptr += 1;

        // Push all arguments encoded
        match *instr {
            Instr::PUSH8 => {
                visit_push_int!(self, instr, args, 1);
            }

            Instr::PUSH16 => {
                visit_push_int!(self, instr, args, 2);
            }

            Instr::PUSH32 => {
                visit_push_int!(self, instr, args, 4);
            }

            Instr::PUSHSZ => {
                if let Some(Node::Str(s)) = args.first() {
                    self.buffer.extend_from_slice(s.as_bytes());
                    self.buffer.push(0);
                    self.ptr += s.len() + 1;
                } else if let Some(arg) = args.first() {
                    self.encode_operand(arg)?;
                }
            }

            Instr::JABS | Instr::JREL | Instr::JEQ | Instr::JNE | Instr::CALL => {
                self.encode_operand(&args[0])?;
            }

            _ => {
                if args.len() != self.operand_count(*instr as u8) {
                    return Err(format!(
                        "Instruction {:?} expects {} operands, got {}",
                        instr,
                        self.operand_count(*instr as u8),
                        args.len()
                    ));
                }

                for i in 0..self.operand_count(*instr as u8) {
                    self.encode_operand(&args[i])?;
                }
            }
        }

        Ok(())
    }

    fn visit_label(&mut self, _label: &str) -> Result<(), String> {
        // Don't do anything - we resolved at the first pass
        Ok(())
    }

    fn visit_node(&mut self, node: &Node) -> Result<(), String> {
        match node {
            Node::Directive(dir) => self.visit_directive(dir),
            Node::Instruction(instr, args) => self.visit_instruction(&instr, args.clone()),
            Node::Label(label) => self.visit_label(label),

            Node::ByteSeq(bytes) => {
                self.buffer.extend_from_slice(bytes);
                self.ptr += bytes.len();
                Ok(())
            }

            Node::Int(_) | Node::Str(_) | Node::Ident(_) => {
                Err("Unexpected standalone argument node".into())
            }
        }
    }

    fn visit_ast(&mut self) -> Result<(), String> {
        for node in &self.ast.clone() {
            self.visit_node(node)?;
        }
        Ok(())
    }

    fn resolve_const_pass(&mut self, ast: &Ast) -> Result<(), String> {
        for node in ast {
            if let Node::Directive(Directive::Define(name, value)) = node {
                self.consts.insert(name.clone(), *value.clone());
                println!("assembler: Defined constant {} as {:?}", name, value);
            }
        }
        Ok(())
    }

    // Pass to resolve labels to addresses
    fn resolve_label_pass(&mut self, ast: &Ast) -> Result<(), String> {
        for node in ast {
            if let Node::Label(label) = node {
                self.labels.insert(label.clone(), self.ptr);
                println!("assembler: Defined label {} at {:08X}", label, self.ptr);
            }

            if let Node::Directive(Directive::Rep(count, body)) = node {
                for _ in 0..*count {
                    for rep_node in body {
                        // Process each repeated instruction to calculate the size
                        // Cut and paste from below

                        if let Node::Instruction(instr, args) = rep_node {
                            let size = self.calculate_instruction_size(instr, args)?;
                            self.ptr += size;
                        }
                    }
                }
            }

            // Count instruction size, and append that much to the ptr
            if let Node::Instruction(instr, args) = node {
                let size = self.calculate_instruction_size(instr, args)?;
                self.ptr += size;
            }
        }
        Ok(())
    }

    // --------------------------------------

    pub fn assemble(&mut self, ast: &Ast) -> Result<&Vec<u8>, String> {
        self.ast = ast.clone();

        // First pass: collect and resolve labels (NO buffer writing)
        println!("assembler: Performing first pass");
        self.ptr = 0;
        self.labels.clear();
        self.buffer.clear();

        self.resolve_const_pass(&ast)?;

        self.resolve_label_pass(&ast)?;

        println!("assembler: Resolved labels: {:?}", self.labels);
        println!("assembler: Total bytecode size: {}", self.ptr);

        // Second pass: generate bytecode
        println!("assembler: Performing second pass");
        self.ptr = 0; // Reset ptr for second pass
        self.buffer.clear();

        self.visit_ast()?;

        // Copy buffer to output
        self.output_bc = self.buffer.clone();

        println!("assembler: Data written: {:02X?}", self.buffer);

        Ok(&self.output_bc)
    }
}
