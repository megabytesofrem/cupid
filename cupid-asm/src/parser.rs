//! Assembler for Cupids bytecode
//!
//! Parser for the Assembly syntax
//!
//! The Assembly dialect is pretty simple, and mostly consists of instructions with a few directives.
//! There are only three data types, integers, byte sequences and NUL-terminated strings. Byte sequences are
//! denoted between `[0x1 0x2 0x3]`.
use std::iter::Peekable;

use super::Instr;
use super::lexer::{self, Token, TokenKind};

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

#[derive(Debug, Clone)]
pub enum Directive {
    Define(String, Box<Node>), // %define VERY_IMPORTANT 42
    Include(String),           // %include "file"
    Stringz(String),           // %string "value"
    ByteSeq(Vec<u8>),          // %byteseq 0x01 0x02 0x03
    Rep(u32, Vec<Node>),       // %rep <count> ... %endrep
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

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: Option<TokenKind>,
        pos: usize,
    },

    UnexpectedEof,

    InvalidArity {
        expected: usize,
        found: usize,
        pos: usize,
    },

    InvalidInstruction {
        name: String,
        pos: usize,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken {
                expected,
                found,
                pos,
            } => {
                if let Some(found) = found {
                    write!(
                        f,
                        "Parse error at position {}: expected {}, found {:?}",
                        pos, expected, found
                    )
                } else {
                    write!(
                        f,
                        "Parse error at position {}: expected {}, found end of file",
                        pos, expected
                    )
                }
            }
            ParseError::UnexpectedEof => write!(f, "Parse error: unexpected end of file"),
            ParseError::InvalidArity {
                expected,
                found,
                pos,
            } => write!(
                f,
                "Parse error at position {}: invalid arity, expected {}, found {}",
                pos, expected, found
            ),
            ParseError::InvalidInstruction { name, pos } => write!(
                f,
                "Parse error at position {}: invalid instruction '{}'",
                pos, name
            ),
        }
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;

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

    /// Expect a specific token kind, erroring if the token is not found
    pub fn expect(&mut self, expected: TokenKind) -> ParseResult<Token> {
        match self.tokens.next() {
            Some(token) if token.kind == expected => Ok(token),
            Some(token) => Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: Some(token.kind),
                pos: self.pos,
            }),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Optionally expect a specific token kind, but we do not care if it is not found
    ///
    /// This is shorthand for manually checking `peek`
    pub fn maybe_expect(&mut self, expected: TokenKind) -> ParseResult<Token> {
        match self.tokens.peek() {
            Some(token) if token.kind == expected => Ok(token.clone()),
            Some(token) => Ok(token.clone()),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn parse_args(&mut self) -> ParseResult<Vec<Node>> {
        let mut args = Vec::new();
        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Ident(ref name) => {
                    let name = name.clone();
                    self.tokens.next(); // consume
                    args.push(Node::Ident(name));
                }
                TokenKind::Int(value) => {
                    self.tokens.next(); // consume
                    args.push(Node::Int(value));
                }
                TokenKind::String => {
                    let token = self.tokens.next().ok_or(ParseError::UnexpectedEof)?;
                    args.push(Node::Str(token.literal));
                }
                _ => break,
            }
        }

        Ok(args)
    }

    fn parse_instruction(&mut self) -> ParseResult<Node> {
        let token = self.tokens.next().ok_or(ParseError::UnexpectedEof)?;
        match token.kind {
            TokenKind::Instruction => {
                let instr = token.literal;
                let args = self.parse_args()?;

                match instr.as_str() {
                    "nop" => Ok(Node::Instruction(Instr::NOP, args)),
                    "pushi" => Ok(Node::Instruction(Instr::PUSH_I, args)),
                    "pushsz" => Ok(Node::Instruction(Instr::PUSHSZ, args)),
                    "pushac" => Ok(Node::Instruction(Instr::PUSHAC, args)),
                    "popi" => Ok(Node::Instruction(Instr::POP_I, args)),
                    "popsz" => Ok(Node::Instruction(Instr::POP_SZ, args)),
                    "j" => Ok(Node::Instruction(Instr::JMP_ABS, args)),
                    "jne" => Ok(Node::Instruction(Instr::JNE, args)),
                    "jeq" => Ok(Node::Instruction(Instr::JEQ, args)),
                    "add" => Ok(Node::Instruction(Instr::ADD, args)),
                    "sub" => Ok(Node::Instruction(Instr::SUB, args)),
                    "mul" => Ok(Node::Instruction(Instr::MUL, args)),
                    "div" => Ok(Node::Instruction(Instr::DIV, args)),
                    "call" => Ok(Node::Instruction(Instr::CALL, args)),
                    "callnat" => Ok(Node::Instruction(Instr::CALL_NAT, args)),
                    "ret" => Ok(Node::Instruction(Instr::RET, args)),
                    "halt" => Ok(Node::Instruction(Instr::HALT, args)),

                    _ => {
                        println!("parse_instruction: {}", instr);

                        Err(ParseError::InvalidInstruction {
                            name: instr,
                            pos: self.pos,
                        })
                    }
                }
            }
            _ => Err(ParseError::InvalidInstruction {
                name: token.literal,
                pos: self.pos,
            }),
        }
    }

    // Assembler Directives
    // --------------------
    // %include "file":                             include file by cut and paste
    // %define <name> <value>:                      define constant
    // %bytes(0x1 0x2 0x3) or %bytes 0x1 0x2 0x3:   define byte array
    // %string "hello":                             define string
    // %ip:                                         replace data with ip
    // %rep(n) ... %endrep:                         repeat block n times
    // --------------------------------------

    fn parse_define_directive(&mut self, args: Vec<Node>) -> ParseResult<Directive> {
        // %define VERY_IMPORTANT 42

        if args.len() != 2 {
            return Err(ParseError::InvalidArity {
                expected: 2,
                found: args.len(),
                pos: self.pos,
            });
        }

        let name = match &args[0] {
            Node::Ident(s) => s.clone(),
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "identifier for a define".to_string(),
                    found: None,
                    pos: self.pos,
                });
            }
        };

        let value = Box::new(args[1].clone());

        Ok(Directive::Define(name, value))
    }

    fn parse_rep_directive(&mut self, args: Vec<Node>) -> ParseResult<Directive> {
        // %rep(3)
        //   <body>
        // %endrep

        // %rep 3
        //  <body>
        // %endrep

        if args.len() != 1 {
            return Err(ParseError::InvalidArity {
                expected: 1,
                found: args.len(),
                pos: self.pos,
            });
        }

        let count = match &args[0] {
            Node::Int(n) => *n,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "integer argument".to_string(),
                    found: None,
                    pos: self.pos,
                });
            }
        };

        let body = self.parse_rep_body()?;

        Ok(Directive::Rep(count, body))
    }

    fn parse_endrep_directive(&mut self, args: Vec<Node>) -> ParseResult<Directive> {
        // %endrep should have no arguments
        if !args.is_empty() {
            return Err(ParseError::InvalidArity {
                expected: 0,
                found: args.len(),
                pos: self.pos,
            });
        }

        // %endrep should not appear outside of a %rep block
        Err(ParseError::UnexpectedToken {
            expected: "%rep block".to_string(),
            found: None,
            pos: self.pos,
        })
    }

    fn parse_rep_body(&mut self) -> ParseResult<Vec<Node>> {
        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek() {
            match &token.kind {
                TokenKind::Directive(name) if name == "endrep" => {
                    self.tokens.next();
                    break;
                }
                TokenKind::Directive(name) if name == "rep" => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "non-nested directive".to_string(),
                        found: Some(TokenKind::Directive(name.clone())),
                        pos: self.pos,
                    });
                }
                TokenKind::Directive(_) => {
                    let directive_node = self.parse_directive()?;
                    body.push(directive_node);
                }
                TokenKind::Instruction => {
                    let instr_node = self.parse_instruction()?;
                    body.push(instr_node);
                }
                TokenKind::Label(label) => {
                    // Labels inside a rep make absolutely no sense
                    return Err(ParseError::UnexpectedToken {
                        expected: "non-label token".to_string(),
                        found: Some(TokenKind::Label(label.clone())),
                        pos: self.pos,
                    });
                }
                _ => break,
            }
        }

        Ok(body)
    }

    fn parse_include_directive(&mut self, args: Vec<Node>) -> ParseResult<Directive> {
        // %include "file"

        if args.len() != 1 {
            return Err(ParseError::InvalidArity {
                expected: 1,
                found: args.len(),
                pos: self.pos,
            });
        }

        let path = match &args[0] {
            Node::Str(s) => s.clone(),
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "string argument".to_string(),
                    found: None,
                    pos: self.pos,
                });
            }
        };

        Ok(Directive::Include(path))
    }

    fn parse_bytes_directive(&mut self, args: Vec<Node>) -> ParseResult<Directive> {
        // %bytes(0x01 0x02 0x03)
        // %bytes 0x01 0x02 0x02

        if args.is_empty() {
            // Expected at least one argument
            return Err(ParseError::InvalidArity {
                expected: 1,
                found: 0,
                pos: self.pos,
            });
        }

        let mut bytes = Vec::new();
        for arg in args {
            match arg {
                Node::Int(b) if b <= 0xFF => bytes.push(b as u8),
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "byte (0-255)".to_string(),
                        found: None,
                        pos: self.pos,
                    });
                }
            }
        }

        Ok(Directive::ByteSeq(bytes))
    }

    fn parse_string_directive(&mut self, args: Vec<Node>) -> ParseResult<Directive> {
        if args.len() != 1 {
            return Err(ParseError::InvalidArity {
                expected: 1,
                found: args.len(),
                pos: self.pos,
            });
        }

        let s = match &args[0] {
            Node::Str(s) => s.clone(),
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "string argument".to_string(),
                    found: None,
                    pos: self.pos,
                });
            }
        };

        Ok(Directive::Stringz(s))
    }

    // --------------------------------------

    fn parse_directive(&mut self) -> ParseResult<Node> {
        let token = self.tokens.next().ok_or(ParseError::UnexpectedEof)?;
        match token.kind {
            TokenKind::Directive(ref name) => {
                println!("parser: Parsing directive {}", name);

                let has_parens =
                    matches!(self.tokens.peek(), Some(t) if t.kind == TokenKind::LParen);

                if has_parens {
                    self.tokens.next(); // consume '('
                }

                let args = self.parse_args()?;
                println!("parser: Parsed directive arguments {:?}", args);

                if has_parens {
                    self.expect(TokenKind::RParen)?; // expect ')'
                }

                match name.as_str() {
                    "define" => Ok(Node::Directive(self.parse_define_directive(args)?)),
                    "include" => Ok(Node::Directive(self.parse_include_directive(args)?)),
                    "bytes" => Ok(Node::Directive(self.parse_bytes_directive(args)?)),
                    "string" => Ok(Node::Directive(self.parse_string_directive(args)?)),
                    "rep" => Ok(Node::Directive(self.parse_rep_directive(args)?)),
                    "endrep" => Ok(Node::Directive(self.parse_endrep_directive(args)?)),
                    "ip" => Ok(Node::Directive(Directive::Stringz("ip".to_string()))), // Placeholder for `ip` directive

                    _ => Err(ParseError::UnexpectedToken {
                        expected: "a directive".to_string(),
                        found: Some(token.kind),
                        pos: self.pos,
                    }),
                }
            }
            _ => Err(ParseError::InvalidInstruction {
                name: token.literal,
                pos: self.pos,
            }),
        }
    }

    // --------------------------------------

    pub fn parse(&mut self) -> ParseResult<Ast> {
        while let Some(token) = self.tokens.peek() {
            match &token.kind {
                TokenKind::Instruction => {
                    let instr_node = self.parse_instruction()?;
                    self.ast.push(instr_node);
                }
                TokenKind::Directive(_) => {
                    let directive_node = self.parse_directive()?;
                    self.ast.push(directive_node);
                }
                TokenKind::Label(label) => {
                    let label_name = label.clone();
                    self.tokens.next(); // consume
                    self.ast.push(Node::Label(label_name));
                }
                TokenKind::ByteSeq(bytes) => {
                    let bytes = bytes.clone();
                    self.tokens.next(); // consume
                    self.ast.push(Node::ByteSeq(bytes));
                }
                _ => {
                    // Unexpected token
                    let token = self.tokens.next().unwrap();
                    return Err(ParseError::UnexpectedToken {
                        expected: "instruction, directive, label, or byte sequence".to_string(),
                        found: Some(token.kind),
                        pos: self.pos,
                    });
                }
            }
        }

        Ok(self.ast.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::lexer::Lexer;

    use super::*;

    fn parse(src: &str) -> Result<Ast, ParseError> {
        let mut lex = Lexer::new(src);
        let tokens_vec = lex.lex().collect::<Vec<_>>();
        println!("tokens: {:?}", tokens_vec);

        let mut parser = Parser::new(tokens_vec.clone().into_iter(), src);
        parser.parse()
    }

    #[test]
    fn parses_instruction() {
        let result = parse("pushi 42");
        assert!(result.is_ok());
    }

    #[test]
    fn parses_byte_sequence() {
        let result = parse("%bytes 0x01, 0x02, 0x03");
        assert!(result.is_ok());
    }

    #[test]
    fn parses_directive() {
        let result = parse("%include \"file.as\"");
        assert!(result.is_ok());
    }

    #[test]
    fn parses_label() {
        let result = parse("my_label:\n  nop");

        println!("parsed result: {:#?}", result);

        assert!(result.is_ok());
    }

    #[test]
    fn parse_file() {
        // FIXME: The lexer seems to be skipping comments, yet the parser still bugs out

        let src = "
%include 'file.as'
%define VERY_IMPORTANT 42

lottery_numbers: %bytes(12 14 34 52 37)

pick_winning_numbers: 
  %rep(5)
   nop
  %endrep
  j done

start:
  j pick_winning_numbers

done:
";

        let result = parse(src);

        println!("parsed result: {:#?}", result);

        assert!(result.is_ok());
    }
}
