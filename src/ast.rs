#[derive(Debug)]
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

#[derive(Debug)]
pub struct ASTProgram(pub Vec<AST>);
