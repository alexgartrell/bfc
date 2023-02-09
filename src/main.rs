mod ast;
mod ir;
mod optimize;
mod parser;
mod x86_emitter;

use clap::Parser;
use std::io::Read;

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    nostdlib: bool,
}

fn main() {
    let args = Args::parse();
    let mut code = String::new();
    std::io::stdin().read_to_string(&mut code).unwrap();

    let ast_prog = parser::Parser::parse(&code);
    let ir_prog = ir::IRProgram::from_ast_program(&ast_prog);
    let ir_prog = optimize::optimize(&ir_prog);
    x86_emitter::X86Emitter::emit(&ir_prog, args.nostdlib);
}
