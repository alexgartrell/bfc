use crate::ir::{IRProgram, IR};

pub struct X86Emitter {
    label_count: usize,
}

impl X86Emitter {
    pub fn emit(prog: &IRProgram, nostdlib: bool) {
        let mut e = Self { label_count: 0 };
        println!(".section .bss");
        println!("arr: .skip 30000");
        println!(".text");
        if nostdlib {
            println!("putch:");
            println!("  mov $1, %rax"); // Write
            println!("  mov $1, %rdi"); // stdout
            println!("  movq %rbx, %rsi"); // ptr
            println!("  mov $1, %rdx"); // 1
            println!("  syscall");
            println!("  ret");

            println!("getch:");
            println!("  mov $0, %rax"); // Read
            println!("  mov $0, %rdi"); // stdin
            println!("  movq %rbx, %rsi"); // ptr
            println!("  mov $1, %rdx"); // 1
            println!("  syscall");
            println!("  ret");

            println!(".globl _start");
            println!("_start:");
        } else {
            println!(".globl main");
            println!("main:");
        }
        println!("  movq $arr, %rbx");

        for n in &prog.0 {
            e.emit_inner(&n, nostdlib);
        }

        println!("  mov $60, %rax"); // exit
        println!("  mov $0, %rdi"); // 0 success
        println!("  syscall");
    }

    fn emit_inner(&mut self, node: &IR, nostdlib: bool) {
        match node {
            IR::PtrChange(amt) => {
                println!("  add ${}, %rbx", amt);
            }
            IR::Add(amt) => {
                println!("  movb (%rbx), %dil");
                println!("  add ${}, %dil", amt);
                println!("  movb %dil, (%rbx)");
            }
            IR::Putch => {
                if nostdlib {
                    println!("  call putch"); // Read
                } else {
                    println!("  movb (%rbx), %dil");
                    println!("  call putchar");
                }
            }
            IR::Getch => {
                if nostdlib {
                    println!("  call gettch"); // Read
                } else {
                    println!("  call getchar");
                    println!("  movb %al, (%rbx)");
                }
            }
            IR::Loop(nodes) => {
                self.label_count += 1;
                let l = format!("label_{}", self.label_count);
                println!("{}:", l);
                println!("  movb (%rbx), %dil");
                println!("  cmp $0, %dil");
                println!("  je {}_done", l);

                for n in nodes {
                    self.emit_inner(n, nostdlib);
                }

                println!("  jmp {}", l);
                println!("{}_done:", l);
            }
        }
    }
}
