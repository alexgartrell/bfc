#[derive(Debug, PartialEq)]
pub enum AST {
    // The actual language constructs
    Loop(Vec<AST>),
    PtrAdvance,
    PtrRetreat,
    Incr,
    Decr,
    Putch,
    Getch,
}

#[derive(Debug, PartialEq)]
pub struct ASTProgram(pub Vec<AST>);
