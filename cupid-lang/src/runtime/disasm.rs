use crate::{
    construct_dword,
    runtime::machine::{VM, VMValue},
};

/// Print the disassembly of an instruction
pub fn disasm_instruction(vm: &VM, addr: usize) -> (String, usize) {
    let opcode = vm.program[addr];
    match opcode {
        0x00 => (format!("nop"), 1),
        0x01 => (format!("push8 {}", vm.program[addr + 1]), 2),
        0x02 => (
            format!(
                "push16 {}",
                construct_dword!(vec![vm.program[addr + 1], vm.program[addr + 2]])
            ),
            3,
        ),
        0x03 => (
            format!(
                "push32 {}",
                construct_dword!(vec![
                    vm.program[addr + 1],
                    vm.program[addr + 2],
                    vm.program[addr + 3],
                    vm.program[addr + 4]
                ])
            ),
            5,
        ),
        0x04 => {
            let len = read_variable_length(addr + 1, &vm.program);
            let string_bytes = &vm.program[addr + 1..addr + len + 1];
            let string = String::from_utf8_lossy(string_bytes);
            (
                format!("pushsz \"{}\"", string),
                len + 1, // +1 for the null terminator
            )
        }
        0x05 => (format!("pushac"), 1),
        0x06 => (format!("pop8"), 1),
        0x07 => (format!("pop16"), 1),
        0x08 => (format!("pop32"), 1),
        0x09 => (format!("popsz"), 1),
        0x0C => (format!("cmp"), 1),
        0x0D | 0x0E => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("j ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x0F => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("jeq ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x10 => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("jne ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x11 => (format!("add"), 1),
        0x12 => (format!("sub"), 1),
        0x13 => (format!("mul"), 1),
        0x14 => (format!("div"), 1),
        0x15 => {
            let len = read_variable_length(addr + 1, &vm.program);
            let address_bytes = &vm.program[addr + 1..addr + len + 1];
            (
                format!("call ${:08X}", format_address(&address_bytes)),
                len + 1,
            )
        }
        0x16 => (format!("callnat {}", vm.program[addr + 1]), 2),
        0x17 => (format!("ret"), 1),
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
    construct_dword!(bytes.to_vec())
}

pub fn dump_memory(vm: &VM, start: usize, end: usize) {
    let end = end.min(vm.program.len());
    let mut addr = start;

    while addr < end {
        print!("{:04X}: ", addr);

        let mut line_content = String::new();
        let mut bytes_on_line = 0;
        const MAX_BYTES_PER_LINE: usize = 8;

        // Process instructions until we fill the line or reach the end
        while addr < end && bytes_on_line < MAX_BYTES_PER_LINE {
            let (disasm, size) = disasm_instruction(vm, addr);

            // Check if this instruction would exceed the line limit
            if bytes_on_line > 0 && bytes_on_line + size > MAX_BYTES_PER_LINE {
                break; // Start a new line
            }

            // Format the bytes for this instruction
            let mut byte_str = String::new();
            for i in 0..size {
                if addr + i < end {
                    byte_str.push_str(&format!("{:02X} ", vm.program[addr + i]));
                }
            }

            // Calculate padding to align the colon and mnemonic
            let byte_width = size * 3; // Each byte is "XX " (3 chars)
            let padding = if byte_width < 12 { 12 - byte_width } else { 1 };

            // Add this instruction to the line with proper formatting
            line_content.push_str(&format!(
                "{}:{:width$}{}    ",
                byte_str.trim_end(),
                "",
                disasm,
                width = padding
            ));

            addr += size;
            bytes_on_line += size;
        }

        println!("{}", line_content.trim_end());
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
