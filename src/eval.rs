use crate::ir::{self, IR};
use std::collections::HashMap;

#[derive(Default)]
struct State {
    mem: HashMap<i32, i8>,
    idx: i32,
}

impl State {
    fn read(&self, off: i32) -> i8 {
        *self.mem.get(&(self.idx + off)).unwrap_or(&0)
    }

    fn write(&mut self, off: i32, val: i8) {
        self.mem.insert(self.idx + off, val);
    }

    fn ptr_change(&mut self, amt: i32) {
        self.idx += amt;
    }
}

pub fn eval(prog: &ir::IRProgram) {
    let mut state = State::default();
    fn run_series(irs: &[ir::IR], state: &mut State) {
        for ir in irs {
            match ir {
                IR::Loop(inner) => {
                    while state.read(0) != 0 {
                        run_series(inner, state);
                    }
                }
                IR::PtrChange(amt) => {
                    state.ptr_change(*amt);
                }
                IR::Add(amt) => {
                    state.write(0, state.read(0) + amt);
                }
                IR::Putch => unsafe {
                    libc::putchar(state.read(0) as libc::c_int);
                },
                IR::Getch => unsafe {
                    state.write(0, libc::getchar() as i8);
                },
                IR::SimpleLoop(delta, inner) => {
                    while state.read(0) != 0 {
                        run_series(inner, state);
                        state.write(0, state.read(0) + delta);
                    }
                }
                IR::AddMul(off, amt) => {
                    state.write(*off, state.read(*off) + state.read(0) * amt);
                }
                IR::MovImm(off, val) => {
                    state.write(*off, *val);
                }
            }
        }
    }
    run_series(&prog.0, &mut state);
}
