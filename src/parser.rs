use crate::ast::{ASTProgram, AST};

#[derive(Debug, PartialEq)]
pub enum Error {
    UnterminatedLoop,
    UnexpectedLoopTermination,
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Parser {
    code: Vec<char>,
    off: usize,
}

impl Parser {
    pub fn parse(code: &str) -> Result<ASTProgram> {
        let mut p = Self {
            code: code.chars().collect(),
            off: 0,
        };
        Ok(ASTProgram(p.parse_inner(false)?))
    }

    fn parse_inner(&mut self, is_loop: bool) -> Result<Vec<AST>> {
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
                '[' => ret.push(AST::Loop(self.parse_inner(true)?)),
                ']' => {
                    return if !is_loop {
                        Err(Error::UnexpectedLoopTermination)
                    } else {
                        Ok(ret)
                    }
                }
                _ => {}
            }
        }
        return if is_loop {
            Err(Error::UnterminatedLoop)
        } else {
            Ok(ret)
        };
    }
}

macro_rules! make_test {
    ($test_name:ident, $code:expr, $ast:expr) => {
        #[cfg(test)]
        mod $test_name {
            use super::*;
            #[test]
            fn test_eq() {
                assert_eq!(Parser::parse($code), $ast)
            }
        }
    };
}

make_test!(empty, "", Ok(ASTProgram(vec![])));
make_test!(
    simple,
    "+-><.,",
    Ok(ASTProgram(vec![
        AST::Incr,
        AST::Decr,
        AST::PtrAdvance,
        AST::PtrRetreat,
        AST::Putch,
        AST::Getch
    ]))
);
make_test!(
    simple_loop,
    "[++]",
    Ok(ASTProgram(vec![AST::Loop(vec![AST::Incr, AST::Incr])]))
);
make_test!(malformed_loop, "[", Err(Error::UnterminatedLoop));
make_test!(
    complex_malformed_loop,
    "[[[[[[[[]]]]]]]",
    Err(Error::UnterminatedLoop)
);
