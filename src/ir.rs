use crate::ast::{ASTProgram, AST};

#[derive(Clone, Debug)]
pub enum IR {
    Loop(Vec<IR>),
    PtrChange(i32),
    Add(i8),
    Putch,
    Getch,
}

#[derive(Debug)]
pub struct IRProgram(pub Vec<IR>);

impl IRProgram {
    pub fn from_ast_program(prog: &ASTProgram) -> Self {
        IRProgram(prog.0.iter().map(|a| Self::from_ast_node(a)).collect())
    }

    fn from_ast_node(ast: &AST) -> IR {
        match ast {
            AST::Loop(asts) => IR::Loop(asts.iter().map(|a| Self::from_ast_node(a)).collect()),
            AST::PtrAdvance => IR::PtrChange(1),
            AST::PtrRetreat => IR::PtrChange(-1),
            AST::Incr => IR::Add(1),
            AST::Decr => IR::Add(-1),
            AST::Putch => IR::Putch,
            AST::Getch => IR::Getch,
        }
    }
}
