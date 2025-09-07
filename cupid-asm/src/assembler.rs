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

#[derive(Debug, Clone)]
pub struct Assembler {
    pub ast: Ast,   // maybe make this a Rc type, so we dont need to '.clone' as much?
    pub ptr: usize, // current position, translates to ip

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
        }
    }

    // Visitors
    // --------------------------------------

    // Copy and paste from src/runtime/machine.rs
    fn operand_count(&self, instr: u8) -> usize {
        match instr {
            0x00 => 0, // nop
            0x01 => 0, // pushi <value>, handled specially
            0x02 => 0, // pushsz <value>, handled specially (strings)
            0x03 => 0, // pushac
            0x04 => 0, // popi
            0x05 => 0, // popsz
            0x07 => 0, // cmp
            0x08 => 1, // jmp <address>, always 4 bytes
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

    // Helper function to push either an u8, u16 or u32.
    // We need to check and increment ptr by the appropriate amount
    fn push_sized_int(&mut self, value: usize) {
        if value <= 0xFF {
            self.buffer.push(value as u8);
            self.ptr += 1;
        } else if value <= 0xFFFF {
            // u16
            let bytes = (value as u16).to_le_bytes();
            self.buffer.extend_from_slice(&bytes);
            self.ptr += 2;
        } else {
            // u32
            let bytes = (value as u32).to_le_bytes();
            self.buffer.extend_from_slice(&bytes);
            self.ptr += 4;
        }
    }

    fn encode_arg(&mut self, arg: &Node) -> Result<usize, String> {
        match arg {
            Node::Ident(label) => {
                if let Some(_address) = self.labels.get(label) {
                    let addr = self.labels.get(label).ok_or("Undefined label")?;
                    self.buffer.extend_from_slice(&addr.to_le_bytes());
                    self.ptr += 4;

                    Ok(*addr)
                } else if let Some(const_value) = self.consts.get(label) {
                    // Inline constant value
                    let const_value = const_value.clone();
                    self.encode_arg(&const_value)
                } else {
                    Err(format!("Undefined label or constant: {}", label))
                }
            }
            Node::Int(value) => {
                if *value <= 0xFF {
                    self.buffer.push(*value as u8);
                    self.ptr += 1;
                } else if *value <= 0xFFFF {
                    // u16
                    let bytes = (*value as u16).to_le_bytes();
                    self.buffer.extend_from_slice(&bytes);
                    self.ptr += 2;
                } else {
                    // u32
                    let bytes = (*value as u32).to_le_bytes();
                    self.buffer.extend_from_slice(&bytes);
                    self.ptr += 4;
                }

                Ok(*value as usize)
            }
            Node::Str(value) => {
                self.buffer.extend_from_slice(value.as_bytes());
                Ok(0)
            }
            Node::ByteSeq(bytes) => {
                self.buffer.extend_from_slice(bytes);
                Ok(0)
            }
            n => Err(format!("Unknown argument type {:?}", n)),
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
            Instr::PUSH_I => {
                if let Some(Node::Int(value)) = args.first() {
                    self.push_sized_int(*value as usize);
                } else if let Some(Node::Ident(label)) = args.first() {
                    let const_value = self
                        .consts
                        .get(label)
                        .cloned()
                        .ok_or(format!("Undefined constant: {}", label))?;

                    if let Node::Int(value) = const_value {
                        self.push_sized_int(value as usize);
                    } else {
                        return Err("PUSH_I expects an integer argument".into());
                    }
                } else {
                    return Err("PUSH_I expects an integer argument".into());
                }
            }

            Instr::PUSHSZ => {
                if let Some(Node::Str(s)) = args.first() {
                    self.buffer.extend_from_slice(s.as_bytes());
                    self.buffer.push(0);
                    self.ptr += s.len() + 1;
                } else if let Some(arg) = args.first() {
                    self.encode_arg(arg)?;
                }
            }

            Instr::JMP_ABS | Instr::JMP_REL | Instr::JEQ | Instr::JNE | Instr::CALL => {
                self.encode_arg(&args[0])?;
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
                    self.encode_arg(&args[i])?;
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
                            self.ptr += 1;

                            // Special case for 4 byte opcodes
                            let four_bytes = matches!(
                                *instr,
                                Instr::JMP_ABS
                                    | Instr::JMP_REL
                                    | Instr::JEQ
                                    | Instr::JNE
                                    | Instr::CALL
                            );

                            match *instr {
                                _instr if four_bytes => {
                                    // Jumps need full addresses, which are variable length
                                    self.ptr += 4; // Assume 4 bytes for address
                                }

                                Instr::PUSHSZ => {
                                    if let Some(Node::Str(s)) = args.first() {
                                        self.ptr += s.len() + 1; // String bytes + null terminator
                                    } else if let Some(Node::Ident(label)) = args.first() {
                                        let const_value = self
                                            .consts
                                            .get(label)
                                            .cloned()
                                            .ok_or(format!("Undefined constant: {}", label))?;

                                        self.ptr += self.count_size(&const_value);
                                    } else {
                                        return Err("PUSHSZ expects a string argument".into());
                                    }
                                }

                                Instr::PUSH_I => {
                                    if let Some(Node::Int(value)) = args.first() {
                                        if *value <= 0xFF {
                                            self.ptr += 1; // u8
                                        } else if *value <= 0xFFFF {
                                            self.ptr += 2; // u16
                                        } else {
                                            self.ptr += 4; // u32
                                        }
                                    } else if let Some(Node::Ident(label)) = args.first() {
                                        let const_value = self
                                            .consts
                                            .get(label)
                                            .cloned()
                                            .ok_or(format!("Undefined constant: {}", label))?;

                                        self.ptr += self.count_size(&const_value);
                                    } else {
                                        return Err("PUSH_I expects an integer argument".into());
                                    }
                                }

                                _ => {
                                    // Count the operand size
                                    let operand_count = self.operand_count(*instr as u8);
                                    self.ptr += operand_count;
                                }
                            }
                        }
                    }
                }
            }

            // Count instruction size, and append that much to the ptr
            if let Node::Instruction(instr, args) = node {
                self.ptr += 1; // Count the instruction itself

                // Special case for 4 byte opcodes
                let four_bytes = matches!(
                    *instr,
                    Instr::JMP_ABS | Instr::JMP_REL | Instr::JEQ | Instr::JNE | Instr::CALL
                );

                match *instr {
                    _instr if four_bytes => {
                        // Jumps need full addresses
                        self.ptr += 4; // Assume 4 bytes for address
                    }

                    Instr::PUSHSZ => {
                        if let Some(Node::Str(s)) = args.first() {
                            self.ptr += s.len() + 1; // String bytes + null terminator
                        } else if let Some(arg) = args.first() {
                            self.ptr += self.count_size(arg);
                        }
                    }

                    Instr::PUSH_I => {
                        if let Some(Node::Int(value)) = args.first() {
                            if *value <= 0xFF {
                                self.ptr += 1; // u8
                            } else if *value <= 0xFFFF {
                                self.ptr += 2; // u16
                            } else {
                                self.ptr += 4; // u32
                            }
                        } else if let Some(arg) = args.first() {
                            self.ptr += self.count_size(arg);
                        } else {
                            return Err("PUSH_I expects an integer argument".into());
                        }
                    }

                    _ => {
                        // Count the operand size
                        let operand_count = self.operand_count(*instr as u8);
                        self.ptr += operand_count;
                    }
                }
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

        println!("assembler: Buffer data: {:?}", self.buffer);

        Ok(&self.output_bc)
    }
}
