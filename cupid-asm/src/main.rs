use clap::Parser as CParser;
use cupid_asm::assembler::Assembler;
use cupid_asm::lexer::Lexer;
use cupid_asm::parser::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(CParser)]
#[command(name = "cas")]
#[command(about = "Cupid bytecode assembler")]
struct Args {
    input: PathBuf,

    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read input file
    let input = fs::read_to_string(&args.input)?;
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| args.input.with_extension("cupid"));

    // Assemble
    let mut lexer = Lexer::new(&input);
    let mut tokens = lexer.lex();
    let mut parser = Parser::new(&mut tokens, &input);
    let ast = parser
        .parse()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let root_path = args
        .input
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut assembler = Assembler::new(&root_path);
    let bc = assembler.assemble(&ast)?;

    // Write output file
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("bc");
        path
    });

    println!("Assembled bytecode written to {}", output_path.display());
    println!("Bytecode size: {} bytes", bc.len());

    fs::write(&output_path, bc)?;

    Ok(())
}
