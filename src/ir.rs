use crate::ast::{ASTProgram, AST};

pub type Offset = i32;
pub type Value = i8;

#[derive(Clone, Debug, PartialEq)]
pub enum IR {
    Loop(Vec<IR>),
    PtrChange(Offset),
    Add(Offset, Value),
    Putch(Offset),
    Getch(Offset),

    SimpleLoop(Value, Vec<IR>),
    AddMul(Offset, Value),
    MovImm(Offset, Value),
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
            AST::Incr => IR::Add(0, 1),
            AST::Decr => IR::Add(0, -1),
            AST::Putch => IR::Putch(0),
            AST::Getch => IR::Getch(0),
        }
    }
}
