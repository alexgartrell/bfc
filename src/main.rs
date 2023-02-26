mod ast;
mod c_emitter;
mod eval;
mod ir;
mod optimize;
mod parser;
mod riscv_emitter;
mod x86_emitter;

use clap::{Parser, ValueEnum};
use std::io::Read;
use std::process::ExitCode;

#[derive(Debug, Clone, ValueEnum)]
enum Arch {
    #[value(name = "x86_64")]
    X86_64,
    RiscV,
    C,
}

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    nostdlib: bool,
    #[arg(long)]
    explore: bool,
    #[arg(long)]
    eval: bool,
    #[arg(long, value_enum, default_value = "x86_64")]
    arch: Arch,

    path: Option<std::path::PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.eval && args.path.is_none() {
        eprintln!("Error: cannot eval when reading program from stdin");
        return ExitCode::from(2);
    }

    let code = if let Some(ref path) = args.path {
        std::fs::read_to_string(path).unwrap()
    } else {
        let mut code = String::new();
        std::io::stdin().read_to_string(&mut code).unwrap();
        code
    };

    let ast_prog = parser::Parser::parse(&code);
    let ir_prog = ir::IRProgram::from_ast_program(&ast_prog);
    if args.eval {
        eval::eval(&ir_prog);
        return ExitCode::SUCCESS;
    }

    let ir_prog = optimize::optimize(&ir_prog);
    if args.explore {
        println!("{:#?}", ir_prog);
        return ExitCode::SUCCESS;
    }
    match args.arch {
        Arch::X86_64 => x86_emitter::X86Emitter::emit(&ir_prog, args.nostdlib),
        Arch::RiscV => riscv_emitter::RiscVEmitter::emit(&ir_prog, args.nostdlib),
        Arch::C => c_emitter::CEmitter::emit(&ir_prog, args.nostdlib),
    }
    return ExitCode::SUCCESS;
}
