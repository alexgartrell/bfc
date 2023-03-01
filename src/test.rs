#![allow(dead_code)]

use crate::eval;
use crate::ir;
use crate::optimize;
use crate::parser;

struct TestIO {
    input: Vec<ir::Value>,
    input_idx: usize,
    output: Vec<ir::Value>,
    output_idx: usize,
}

impl eval::IO for TestIO {
    fn putchar(&mut self, val: ir::Value) {
        assert!(
            self.output_idx < self.output.len(),
            "Produced too much output"
        );
        assert_eq!(val, self.output[self.output_idx]);
        self.output_idx += 1;
    }
    fn getchar(&mut self) -> ir::Value {
        assert!(self.input_idx < self.input.len(), "Consumed too much input");
        let ret = self.input[self.input_idx];
        self.input_idx += 1;
        ret
    }
}

impl TestIO {
    fn new(input: &str, output: &str) -> Self {
        Self {
            input: input.chars().map(|c| c as ir::Value).collect(),
            input_idx: 0,
            output: output.chars().map(|c| c as ir::Value).collect(),
            output_idx: 0,
        }
    }
    fn done(&self) {
        assert_eq!(
            self.input_idx,
            self.input.len(),
            "Did not consume full input"
        );
        assert_eq!(
            self.output_idx,
            self.output.len(),
            "Did not read full output"
        );
    }
}

fn test_unopt_program(code: &str, input: &str, output: &str) {
    let ast_prog = parser::Parser::parse(&code);
    let ir_prog = ir::IRProgram::from_ast_program(&ast_prog);
    let mut io = TestIO::new(input, output);
    eval::eval_with_io(&ir_prog, &mut io);
    io.done();
}

fn test_opt_program(code: &str, input: &str, output: &str) {
    let ast_prog = parser::Parser::parse(&code);
    let ir_prog = ir::IRProgram::from_ast_program(&ast_prog);
    dbg!(&ir_prog);
    let ir_prog = optimize::optimize(&ir_prog);
    dbg!(&ir_prog);
    let mut io = TestIO::new(input, output);
    eval::eval_with_io(&ir_prog, &mut io);
    io.done();
}

fn test_program(code: &str, input: &str, output: &str) {
    test_unopt_program(code, input, output);
    test_opt_program(code, input, output);
}

macro_rules! make_test {
    ($test_name:ident, $code:expr, $input:expr, $output:expr) => {
        #[cfg(test)]
        mod $test_name {
            use super::*;
            #[test]
            fn test_unoptimized() {
                test_unopt_program($code, $input, $output);
            }

            #[test]
            fn test_optimized() {
                test_opt_program($code, $input, $output);
            }
        }
    };
}

make_test!(get_put, ",.", "a", "a");
make_test!(put_zero, ".", "", "\0");
make_test!(put_newline, "++++++++++.", "", "\n");
make_test!(get_get_put, ",,.", "ab", "b");
make_test!(addmul, "++++++[->+++++<]>++.", "", " ");
// test_program("++++++++[->++++++++<]>+.", "", "A");
