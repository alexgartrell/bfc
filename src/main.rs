mod ast;
mod c_emitter;
mod eval;
mod ir;
mod optimize;
mod parser;
mod riscv_emitter;
mod test;
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

    #[arg(short = 'O', default_value = "1")]
    opt_level: i32,

    #[arg(short, long, default_value = "30000")]
    mem_size: usize,

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

    let ast_prog = match parser::Parser::parse(&code) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse program: {:?}", e);
            return ExitCode::from(2);
        }
    };

    let ir_prog = ir::IRProgram::from_ast_program(&ast_prog);

    let ir_prog = if args.opt_level == 0 {
        ir_prog
    } else {
        optimize::optimize(&ir_prog)
    };

    if args.eval {
        eval::eval(&ir_prog);
        return ExitCode::SUCCESS;
    }
    if args.explore {
        println!("{:#?}", ir_prog);
        return ExitCode::SUCCESS;
    }
    match args.arch {
        Arch::X86_64 => x86_emitter::X86Emitter::emit(&ir_prog, args.nostdlib, args.mem_size),
        Arch::RiscV => riscv_emitter::RiscVEmitter::emit(&ir_prog, args.nostdlib, args.mem_size),
        Arch::C => c_emitter::CEmitter::emit(&ir_prog, args.nostdlib, args.mem_size),
    }
    return ExitCode::SUCCESS;
}
