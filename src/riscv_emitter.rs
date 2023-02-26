use crate::ir::{IRProgram, IR};

pub struct RiscVEmitter {
    label_count: usize,
}

impl RiscVEmitter {
    pub fn emit(prog: &IRProgram, nostdlib: bool) {
        let mut e = Self { label_count: 0 };
        println!(".section .bss");
        println!("arr: .skip 30000");
        println!(".text");
        if nostdlib {
            println!(".globl _start");
            println!("_start:");
        } else {
            println!(".globl main");
            println!("main:");
        }
        println!("  la s1, arr");

        for n in &prog.0 {
            e.emit_inner(&n, nostdlib);
        }

        if nostdlib {
            println!("  li a0, 0");
            println!("  li a7, 93");
            println!("  ecall");
        } else {
            println!("  li a0, 0");
            println!("  call exit");
        }
    }

    fn emit_inner(&mut self, node: &IR, nostdlib: bool) {
        match node {
            IR::PtrChange(amt) => {
                println!("  addi s1, s1, {}", amt);
            }
            IR::Add(amt) => {
                println!("  lb t0, (s1)");
                println!("  addi t0, t0, {}", amt);
                println!("  sb t0, (s1)");
            }
            IR::Putch => {
                println!("  li a0, 1");
                println!("  mv a1, s1");
                println!("  li a2, 1");
                if nostdlib {
                    println!("  li a7, 64");
                    println!("  ecall");
                } else {
                    println!("  call write");
                }
            }
            IR::Getch => {
                println!("  li a0, 0");
                println!("  mv a1, s1");
                println!("  li a2, 1");
                if nostdlib {
                    println!("  li a7, 63");
                    println!("  ecall");
                } else {
                    println!("  call read");
                }
            }
            IR::Loop(nodes) => {
                self.label_count += 1;
                let l = format!("label_{}", self.label_count);
                println!("{}:", l);
                println!("  lb t0, (s1)");
                println!("  beqz t0, {}_done", l);

                for n in nodes {
                    self.emit_inner(n, nostdlib);
                }

                println!("  j {}", l);
                println!("{}_done:", l);
            }

            IR::SimpleLoop(delta, nodes) => {
                self.label_count += 1;
                let l = format!("label_{}", self.label_count);
                println!("{}:", l);
                println!("  lb t0, (s1)");
                println!("  beqz t0, {}_done", l);

                for n in nodes {
                    self.emit_inner(n, nostdlib);
                }

                self.emit_inner(&IR::Add(*delta), nostdlib);
                println!("  j {}", l);
                println!("{}_done:", l);
            }
            IR::AddMul(off, amt) => {
                println!("  lb t0, (s1)");
                println!("  muli t0, t0, {}", amt);
                println!("  lb t1, {}(s1)", off);
                println!("  add t0, t0, t1");
                println!("  sb t0, (s1)");
            }
            IR::MovImm(off, imm) => {
                println!("  li t0, {}", imm);
                println!("  sb t0, {}(s1)", off);
            }
        }
    }
}
