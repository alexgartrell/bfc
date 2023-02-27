use crate::ir::{self, IR};
use std::collections::HashMap;

#[derive(Default)]
struct State {
    mem: HashMap<ir::Offset, ir::Value>,
    idx: ir::Offset,
}

impl State {
    fn read(&self, off: ir::Offset) -> ir::Value {
        *self.mem.get(&(self.idx + off)).unwrap_or(&0)
    }

    fn write(&mut self, off: ir::Offset, val: ir::Value) {
        self.mem.insert(self.idx + off, val);
    }

    fn ptr_change(&mut self, amt: ir::Offset) {
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
                IR::Putch(off) => unsafe {
                    libc::putchar(state.read(*off) as libc::c_int);
                },
                IR::Getch(off) => unsafe {
                    state.write(*off, libc::getchar() as ir::Value);
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
