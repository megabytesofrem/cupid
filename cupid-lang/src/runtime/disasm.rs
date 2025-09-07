use crate::{
    construct_vm_addr,
    runtime::machine::{VM, VMValue},
};

/// Print the disassembly of an instruction
pub fn disasm_instruction(vm: &VM, addr: usize) -> (String, usize) {
    let opcode = vm.program[addr];
    match opcode {
        0x00 => (format!("nop"), 1),
        0x01 => (format!("pushi {}", vm.program[addr + 1]), 2),
        0x02 => {
            let len = read_variable_length(addr + 1, &vm.program);
            let string_bytes = &vm.program[addr + 1..addr + len + 1];
            let string = String::from_utf8_lossy(string_bytes);
            (
                format!("pushsz \"{}\"", string),
                len + 1, // +1 for the null terminator
            )
        }
        0x03 => (format!("pushac"), 1),
        0x04 => (format!("popi"), 1),
        0x05 => (format!("popsz"), 1),
        0x08 => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("jmp ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x09 => (
            format!("jmp +{:08X}", format_address(&vm.program[addr + 1..])),
            2,
        ),
        0x0A => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("jeq ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x0B => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("jne ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x0C => (format!("add"), 1),
        0x0D => (format!("sub"), 1),
        0x0E => (format!("mul"), 1),
        0x0F => (format!("div"), 1),
        0x10 => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("call ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x11 => (format!("callnat {}", vm.program[addr + 1]), 2),
        0x12 => (format!("ret"), 1),
        0xFF => (format!("halt"), 1),
        _ => (format!("${:02X}", opcode), 1),
    }
}

fn read_variable_length(start_addr: usize, bytes: &[u8]) -> usize {
    let mut i = 0;
    while start_addr + i < bytes.len() && bytes[start_addr + i] != 0 {
        i += 1;
    }

    // Return the length
    i as usize
}

fn format_address(bytes: &[u8]) -> u32 {
    construct_vm_addr!(bytes.to_vec())
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
