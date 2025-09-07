//! Assembler for Cupids bytecode
//!
//! Lexer for the Assembly syntax

use std::iter::Peekable;

#[derive(Debug, Clone)]
pub struct Lexer<'l> {
    pub src: &'l str,
    pub pos: usize,

    reserved_words: Vec<&'l str>,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub literal: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Operators
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    LParen,  // (
    RParen,  // )
    LSquare, // [
    RSquare, // ]
    Comma,   // ,

    Instruction,       // pushi <value>, pushac
    Directive(String), // %include "file"
    Label(String),     // name:

    Int(u32),
    Ident(String),
    String,
    ByteSeq(Vec<u8>), // 0x01 0x02 ...
}

impl<'l> Lexer<'l> {
    pub fn new(src: &'l str) -> Self {
        Lexer {
            pos: 0,
            src,

            #[rustfmt::skip]
            reserved_words: vec![
                "nop", "push8", "push16", "push32", "pushsz", "pushac",
                "pop8", "pop16", "pop32", "popsz", "cmp", "j", "jeq", "jne",
                "add", "sub", "mul", "div", "call", "callnat", "ret", "halt",
            ],
        }
    }

    /// Peek at the next character without consuming it.
    fn peek(&self) -> char {
        self.src.chars().nth(self.pos).unwrap_or('\0')
    }

    fn peek_ahead(&self, offset: usize) -> char {
        self.src.chars().nth(self.pos + offset).unwrap_or('\0')
    }

    // Helper to push a token, without a specific literal value
    fn push(&mut self, token: TokenKind, tokens: &mut Vec<Token>) {
        tokens.push(Token {
            kind: token.clone(),
            literal: format!("{:?}", token),
        });
    }

    /// Peek, advance and return the peeked character.
    fn advance(&mut self) -> char {
        let peeked = self.peek();
        self.pos += 1;
        peeked
    }

    fn eat_whitespace(&mut self) {
        while self.peek().is_whitespace() {
            self.advance();
        }
    }

    fn lex_number(&mut self, start: char) -> Token {
        let mut value = String::new();
        let mut base = 10;
        let mut basech = '\0';

        if start == '0' {
            match self.peek() {
                'x' | 'X' => {
                    base = 16;
                    basech = 'x';
                    self.advance();
                }
                'b' | 'B' => {
                    base = 2;
                    basech = 'b';
                    self.advance();
                }
                'o' | 'O' => {
                    base = 8;
                    basech = 'o';
                    self.advance();
                }
                _ => value.push(start), // The literal '0'
            }
        } else {
            value.push(start);
        }

        if base != 10 {
            value.push('0');
            value.push(basech);
        }

        while match base {
            2 => matches!(self.peek(), '0' | '1'),
            8 => matches!(self.peek(), '0'..='7'),
            10 => self.peek().is_digit(10),
            16 => self.peek().is_digit(16),
            _ => false,
        } {
            value.push(self.advance());
        }

        let int_value = if base != 10 {
            u32::from_str_radix(value.trim_start_matches(&format!("0{}", basech)), base).unwrap()
        } else {
            value.parse().unwrap()
        };

        Token {
            kind: TokenKind::Int(int_value),
            literal: value,
        }
    }

    fn lex_byte_seq(&mut self) -> Token {
        let mut bytes = Vec::new();

        self.eat_whitespace();

        while self.peek() != ']' && self.peek() != '\0' {
            match self.peek() {
                ' ' | '\t' => {
                    self.advance();
                }
                '0'..'9' => {
                    let digit = self.advance();
                    let num_token = self.lex_number(digit);

                    if let TokenKind::Int(b) = num_token.kind {
                        if b > 0xFF {
                            panic!("Byte value out of range: {}", b);
                        }
                        bytes.push(b as u8);
                    }
                }

                _ => {
                    panic!("Unexpected character in byte sequence");
                }
            }
        }

        if self.peek() == ']' {
            self.advance(); // consume closing ']'
        } else {
            panic!("Unterminated byte sequence");
        }

        Token {
            kind: TokenKind::ByteSeq(bytes.clone()),
            literal: format!("{:?}", bytes),
        }
    }

    fn lex_string(&mut self) -> Token {
        let mut value = String::new();
        while self.peek() != '"' && self.peek() != '\'' && self.peek() != '\0' {
            let c = self.advance();
            match c {
                '\\' => {
                    let c = self.advance();
                    // Handle character escape sequences
                    match c {
                        '\\' => value.push('\\'),
                        '0' => value.push(0 as char),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        _ => value.push(c),
                    }
                }
                _ => {
                    value.push(c);
                }
            }
        }

        // Check for closing quote, consume and advance
        if self.peek() == '"' || self.peek() == '\'' {
            self.advance();
        } else {
            // Handle unterminated string
            panic!("Unterminated string literal");
        }

        Token {
            kind: TokenKind::String,
            literal: value,
        }
    }

    // Lex things that are *like* identifiers, but may not be
    // This includes reserved words and labels
    fn lex_ident_like(&mut self) -> Token {
        let mut ident = String::new();
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            ident.push(self.advance());
        }

        if self.reserved_words.contains(&ident.as_str()) {
            return Token {
                kind: TokenKind::Instruction,
                literal: ident,
            };
        }

        if self.peek() == ':' {
            self.advance(); // consume ':'
            return Token {
                kind: TokenKind::Label(ident.clone()),
                literal: ident,
            };
        }

        Token {
            kind: TokenKind::Ident(ident.clone()),
            literal: ident,
        }
    }

    fn lex_directive(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        self.advance();
        let name_token = self.lex_ident_like();

        tokens.push(Token {
            kind: TokenKind::Directive(name_token.literal.clone()),
            literal: name_token.literal,
        });

        self.eat_whitespace();

        // Lex directive arguments
        self.lex_token(&mut tokens);

        tokens
    }

    fn lex_token(&mut self, tokens: &mut Vec<Token>) {
        loop {
            let c = self.peek();
            match c {
                '\0' | '\r' | '\n' => break,
                ' ' | '\t' => {
                    self.advance();
                }

                '/' if self.peek_ahead(1) == '/' => {
                    // Skip line comment
                    while self.peek() != '\n' && self.peek() != '\0' {
                        self.advance();
                    }
                }

                '[' => {
                    self.advance();
                    tokens.push(self.lex_byte_seq());
                }

                '0'..='9' => {
                    let digit = self.advance();
                    tokens.push(self.lex_number(digit));
                }
                '"' | '\'' => {
                    self.advance(); // consume opening quote
                    tokens.push(self.lex_string());
                }
                '%' => {
                    println!("lex_value: reading a directive");
                    tokens.extend(self.lex_directive());
                }
                _ if c.is_alphabetic() || c == '_' => {
                    tokens.push(self.lex_ident_like());
                }
                _ => {
                    self.advance();
                }
            }

            self.eat_whitespace();
        }
    }

    pub fn lex(&mut self) -> Peekable<impl Iterator<Item = Token>> {
        let mut tokens = Vec::new();
        loop {
            let c = self.peek();
            match c {
                '\0' => break,
                '/' if self.peek_ahead(1) == '/' => {
                    // Skip line comment
                    while self.peek() != '\n' && self.peek() != '\0' {
                        self.advance();
                    }
                }
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                    self.eat_whitespace();
                }

                // Operators
                '+' => {
                    self.advance();
                    self.push(TokenKind::Plus, &mut tokens);
                }
                '-' => {
                    self.advance();
                    self.push(TokenKind::Minus, &mut tokens);
                }
                '*' => {
                    self.advance();
                    self.push(TokenKind::Star, &mut tokens);
                }
                '/' => {
                    self.advance();
                    self.push(TokenKind::Slash, &mut tokens);
                }
                '(' => {
                    self.advance();
                    self.push(TokenKind::LParen, &mut tokens);
                }
                ')' => {
                    self.advance();
                    self.push(TokenKind::RParen, &mut tokens);
                }
                '[' => {
                    self.advance();
                    tokens.push(self.lex_byte_seq());
                }
                // ']' => {
                //     self.advance();
                //     self.push(TokenKind::RSquare, &mut tokens);
                // }
                ',' => {
                    self.advance();
                    self.push(TokenKind::Comma, &mut tokens);
                }

                '0'..='9' => {
                    let digit = self.advance();
                    tokens.push(self.lex_number(digit));
                }
                '"' | '\'' => {
                    self.advance(); // consume opening quote
                    tokens.push(self.lex_string());
                }
                '%' => {
                    println!("lex_value: reading a directive");
                    tokens.extend(self.lex_directive());
                }

                // Identifiers/Keywords/Labels
                _ if c.is_alphabetic() || c == '_' => {
                    tokens.push(self.lex_ident_like());
                }
                _ => {
                    self.advance();
                }
            }

            self.eat_whitespace();
        }

        // Return an iterator over the collected tokens
        tokens.into_iter().peekable()
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn make_lexer<'a>(input: &'a str) -> Lexer<'a> {
        Lexer::new(input)
    }

    #[test]
    fn lex_skips_comments() {
        let mut lex = make_lexer("// this is a comment\n123");
        let mut tokens = lex.lex();

        assert_eq!(tokens.next().unwrap().kind, TokenKind::Int(123));
    }

    #[test]
    fn lex_number() {
        let mut lex = make_lexer("123 0xff 0b0010");
        let mut tokens = lex.lex();

        let decimal = &tokens.next().unwrap();
        let hex = &tokens.next().unwrap();
        let binary = &tokens.next().unwrap();

        assert_eq!(decimal.kind, TokenKind::Int(123));
        assert_eq!(hex.kind, TokenKind::Int(0xFF));
        assert_eq!(binary.kind, TokenKind::Int(0b0010));
    }

    #[test]
    fn lex_byte_sequence() {
        let mut lex = make_lexer("[0x01 0x02 0x03]");
        let mut tokens = lex.lex();

        let byte_seq = tokens.next().unwrap();
        assert!(matches!(byte_seq.kind, TokenKind::ByteSeq(_)));
    }

    #[test]
    fn lex_string() {
        let mut lex = make_lexer("\"hello\" 'world'");
        let mut tokens = lex.lex();

        let double_quoted = &tokens.next().unwrap();
        let single_quoted = &tokens.next().unwrap();

        assert_eq!(double_quoted.kind, TokenKind::String);
        assert_eq!(double_quoted.literal, "hello");
        assert_eq!(single_quoted.kind, TokenKind::String);
        assert_eq!(single_quoted.literal, "world");
    }

    #[test]
    fn lex_directive() {
        let mut lex = make_lexer("%include 'foobar'");
        let mut tokens = lex.lex();

        assert!(matches!(
            tokens.next().unwrap().kind,
            TokenKind::Directive(_)
        ));
    }

    #[test]
    fn lex_reserved_word() {
        let mut lex = make_lexer("nop pushi pushsz pushac popi foobar:");
        let mut tokens = lex.lex();

        assert!(tokens.any(|t| t.kind == TokenKind::Instruction));
    }
}
