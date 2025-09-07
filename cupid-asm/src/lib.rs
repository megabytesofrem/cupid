pub mod assembler;
pub mod lexer;
pub mod parser;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Instr {
    NOP = 0x00,

    // Memory
    PUSH_I = 0x01,
    PUSHSZ = 0x02,
    PUSHAC = 0x03,
    POP_I = 0x04,
    POP_SZ = 0x05,

    // Jumping
    JMP_ABS = 0x08,
    JMP_REL = 0x09,
    JEQ = 0x0A,
    JNE = 0x0B,

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

    // Assembler specific operations
    PUSHBZ = 0xBA,
}
