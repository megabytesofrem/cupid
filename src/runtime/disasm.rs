use crate::runtime::machine::{VM, VMValue};

/// Print the disassembly of an instruction
pub fn disasm_instruction(vm: &VM, addr: usize) -> (String, usize) {
    let opcode = vm.program[addr];
    match opcode {
        0x00 => (format!("nop"), 1),
        0x01 => (format!("pushi {}", vm.program[addr + 1]), 2),
        0x02 => (format!("pushsz \"{}\"", vm.program[addr + 1]), 2),
        0x03 => (format!("popi"), 1),
        0x04 => (format!("popsz"), 1),
        0x08 => (format!("jmp ${:08X}", vm.program[addr + 1]), 2),
        0x09 => (format!("jmp +{:08X}", vm.program[addr + 1]), 2),
        0x0A => (format!("jeq ${:08X}", vm.program[addr + 1]), 2),
        0x0B => (format!("jne ${:08X}", vm.program[addr + 1]), 2),
        0x0C => (format!("add"), 1),
        0x0D => (format!("sub"), 1),
        0x0E => (format!("mul"), 1),
        0x0F => (format!("div"), 1),
        0x10 => (format!("call ${:08X}", vm.program[addr + 1]), 2),
        0x11 => (format!("callnat {}", vm.program[addr + 1]), 2),
        0x12 => (format!("ret"), 1),
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

        println!();
        addr += line_bytes;
    }
}

pub fn dump_stack_val(vm_value: &VMValue) -> String {
    match vm_value {
        VMValue::Int(i) => format!("{}", i),
        VMValue::String(s) => format!("'{}'", String::from_utf8_lossy(s)),
        VMValue::Data(d) => format!("<data: {:?}>", d),
    }
}

pub fn dump_stack(vm: &VM) {
    println!("Stack (sp={}):", vm.sp);
    for (i, val) in vm.stack.iter().enumerate() {
        println!("  [{}]: {}", i, dump_stack_val(val));
    }
}
