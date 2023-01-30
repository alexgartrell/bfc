use crate::ast::{ASTProgram, AST};

pub struct Parser {
    code: Vec<char>,
    off: usize,
}

impl Parser {
    pub fn parse(code: &str) -> ASTProgram {
        let mut p = Self {
            code: code.chars().collect(),
            off: 0,
        };
        ASTProgram(p.parse_inner())
    }

    fn parse_inner(&mut self) -> Vec<AST> {
        let mut ret = Vec::new();
        while self.off < self.code.len() {
            let c = self.code[self.off];
            self.off += 1;

            match c {
                '>' => ret.push(AST::PtrAdvance),
                '<' => ret.push(AST::PtrRetreat),
                '+' => ret.push(AST::Incr),
                '-' => ret.push(AST::Decr),
                '.' => ret.push(AST::Putch),
                ',' => ret.push(AST::Getch),
                '[' => ret.push(AST::Loop(self.parse_inner())),
                ']' => break,
                _ => {}
            }
        }
        ret
    }
}
