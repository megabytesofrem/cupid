use crate::runtime::machine::VM;

/// Print the disassembly of an instruction
pub fn disasm_instruction(vm: &VM, addr: usize) -> (String, usize) {
    let opcode = vm.program[addr];
    match opcode {
        0x01 => (format!("pushi {}", vm.program[addr + 1]), 2),
        0x02 => (format!("pushsz \"{}\"", vm.program[addr + 1]), 2),
        0x08 => (format!("jmp ${:04X}", vm.program[addr + 1]), 2),
        0x09 => (format!("jmp +{:04X}", vm.program[addr + 1]), 2),
        0x0A => (format!("jeq ${:04X}", vm.program[addr + 1]), 2),
        0x0B => (format!("jne ${:04X}", vm.program[addr + 1]), 2),
        0x0C => (format!("add"), 1),
        0xFF => (format!("halt"), 1),
        _ => (format!("${:02X}", opcode), 1),
    }
}

pub fn dump_memory(vm: &VM, start: usize, end: usize) {
    let end = end.min(vm.program.len());
    let mut addr = start;
    while addr < end {
        print!("{:04X}: ", addr);

        let mut line_bytes = 0;
        while line_bytes < 8 && addr + line_bytes < end {
            let (disasm, size) = disasm_instruction(vm, addr + line_bytes);
            // Print the bytes for this instruction
            for i in 0..size {
                if addr + line_bytes + i < end {
                    print!("{:02X} ", vm.program[addr + line_bytes + i]);
                }
            }
            print!(": {}    ", disasm);

            line_bytes += size;
        }

        // for offset in 0..8 {
        //     if addr + offset < end {
        //         let ins = vm.program[addr + offset];
        //         print!(
        //             "{:02X} : {}    ",
        //             ins,
        //             disasm_instruction(vm, addr + offset)
        //         );
        //     } else {
        //         print!("   ");
        //     }
        // }
        println!();
        addr += line_bytes;
    }
}
