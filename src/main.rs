mod ast;
mod ir;
mod optimize;
mod parser;
mod riscv_emitter;
mod x86_emitter;

use clap::{Parser, ValueEnum};
use std::io::Read;

#[derive(Debug, Clone, ValueEnum)]
enum Arch {
    #[value(name = "x86_64")]
    X86_64,
    RiscV,
}

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    nostdlib: bool,
    #[arg(long, value_enum, default_value = "x86_64")]
    arch: Arch,
}

fn main() {
    let args = Args::parse();
    let mut code = String::new();
    std::io::stdin().read_to_string(&mut code).unwrap();

    let ast_prog = parser::Parser::parse(&code);
    let ir_prog = ir::IRProgram::from_ast_program(&ast_prog);
    let ir_prog = optimize::optimize(&ir_prog);
    match args.arch {
        Arch::X86_64 => x86_emitter::X86Emitter::emit(&ir_prog, args.nostdlib),
        Arch::RiscV => riscv_emitter::RiscVEmitter::emit(&ir_prog, args.nostdlib),
    }
}
