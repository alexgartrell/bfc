use crate::ir::{IRProgram, IR};

pub struct CEmitter {}

impl CEmitter {
    pub fn emit(prog: &IRProgram, nostdlib: bool) {
        let mut e = Self {};
        println!("#include <stdio.h>");
        println!("char arr[3000];");
        println!("int idx = 0;");
        println!("int main() {{");

        for n in &prog.0 {
            e.emit_inner(&n, nostdlib);
        }
        println!("return 0;");
        println!("}}");
    }

    fn emit_inner(&mut self, node: &IR, nostdlib: bool) {
        match node {
            IR::PtrChange(amt) => {
                println!("  idx += {};", amt);
            }
            IR::Add(amt) => {
                println!("  arr[idx] += {};", amt);
            }
            IR::Putch => {
                println!("  putchar(arr[idx]);");
            }
            IR::Getch => {
                println!("  arr[idx] = (char)getchar();");
            }
            IR::Loop(nodes) => {
                println!("  while (arr[idx]) {{");
                for n in nodes {
                    self.emit_inner(n, nostdlib);
                }
                println!("  }}");
            }
            IR::SimpleLoop(delta, nodes) => {
                println!("  for ( ; arr[idx]; arr[idx] += {}) {{", delta);
                for n in nodes {
                    self.emit_inner(n, nostdlib);
                }
                println!("  }}");
            }
            IR::AddMul(off, amt) => {
                println!("  arr[idx + {}] += (arr[idx] * {});", off, amt);
            }
            IR::MovImm(off, imm) => {
                println!("  arr[idx + {}] = {};", off, imm);
            }
        }
    }
}
