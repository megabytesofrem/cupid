fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use cupid::runtime::machine::VM;

    #[test]
    fn test_machine_initialization() {
        let mach = VM::new();
        mach.dump_ctx();
    }

    #[test]
    fn test_machine_run() {
        let mut mach = VM::new();

        let code = [0x08, 0xDF]
            .iter()
            .cloned()
            .chain(std::iter::repeat(0xFF).take(16))
            .collect::<Vec<u8>>();

        mach.run_with(&code);
        mach.dump_ctx();
    }
}
