use crate::runtime::machine::VM;

/// Print the disassembly of an instruction
pub fn disasm_instruction(vm: &VM, addr: usize) -> String {
    let opcode = vm.program[addr];
    match opcode {
        0x01 => format!("pushi {}", vm.program[addr + 1]),
        0x02 => format!("pushsz \"{}\"", vm.program[addr + 1]),
        0x08 => format!("jmp ${:04X}", vm.program[addr + 1]),
        0x09 => format!("jmp +{:04X}", vm.program[addr + 1]),
        0x0A => format!("jeq ${:04X}", vm.program[addr + 1]),
        0x0B => format!("jne ${:04X}", vm.program[addr + 1]),
        _ => format!("${:02X}", opcode),
    }
}

pub fn dump_memory(vm: &VM, start: usize, end: usize) {
    let end = end.min(vm.program.len());
    let mut addr = start;
    while addr < end {
        print!("{:04X}: ", addr);
        for offset in 0..8 {
            if addr + offset < end {
                let ins = vm.program[addr + offset];
                print!(
                    "{:02X} : {}    ",
                    ins,
                    disasm_instruction(vm, addr + offset)
                );
            } else {
                print!("   ");
            }
        }
        println!();
        addr += 8;
    }
}
