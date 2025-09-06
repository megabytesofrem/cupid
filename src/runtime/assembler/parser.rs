//! Assembler for Cupids bytecode
//!
//! Parser for the Assembly syntax
use std::iter::Peekable;

use crate::runtime::assembler::lexer;

#[derive(Debug, Clone)]
pub struct Parser<'p, I>
where
    I: Iterator<Item = lexer::Token>,
{
    pub tokens: Peekable<I>,
    pub pos: usize,
    pub src: &'p str,

    pub ast: Ast,
}

// --------------------------------------------
// AST definition

// Instead of redefining, alias the one from the VM
pub type Instr = crate::runtime::machine::Op;

#[derive(Debug, Clone)]
pub enum Directive {
    Include(String),     // %include "file"
    Stringz(String),     // %string "value"
    ByteSeq(Vec<u8>),    // %byteseq 0x01 0x02 0x03
    Rep(u32, Vec<Node>), // %rep <count> ... %endrep
}

#[derive(Debug, Clone)]
pub enum Node {
    Instruction(Instr, Vec<Node>),
    Directive(Directive),
    Label(String),

    Ident(String),
    Int(u32),
    Str(String),
    ByteSeq(Vec<u8>),
}

pub type Ast = Vec<Node>;

// --------------------------------------------
// Parsing

impl<'p, I> Parser<'p, I>
where
    I: Iterator<Item = lexer::Token>,
{
    pub fn new(tokens: I, src: &'p str) -> Self {
        Self {
            tokens: tokens.peekable(),
            pos: 0,
            src,

            ast: Vec::new(),
        }
    }
}
