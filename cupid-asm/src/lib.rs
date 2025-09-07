pub mod assembler;
pub mod lexer;
pub mod parser;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Instr {
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
