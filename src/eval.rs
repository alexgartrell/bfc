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

pub trait IO {
    fn putchar(&mut self, val: i8);
    fn getchar(&mut self) -> i8;
}

pub struct CIO {}
impl IO for CIO {
    fn putchar(&mut self, val: ir::Value) {
        unsafe {
            libc::putchar(val as libc::c_int);
        }
    }
    fn getchar(&mut self) -> ir::Value {
        unsafe { libc::getchar() as ir::Value }
    }
}

pub fn eval(prog: &ir::IRProgram) {
    let mut io = CIO {};
    eval_with_io(prog, &mut io)
}

pub fn eval_with_io(prog: &ir::IRProgram, io: &mut impl IO) {
    let mut state = State::default();
    fn run_series(irs: &[ir::IR], state: &mut State, io: &mut impl IO) {
        for ir in irs {
            match ir {
                IR::Loop(inner) => {
                    while state.read(0) != 0 {
                        run_series(inner, state, io);
                    }
                }
                IR::PtrChange(amt) => {
                    state.ptr_change(*amt);
                }
                IR::Add(add_off, amt) => {
                    state.write(*add_off, state.read(*add_off) + amt);
                }
                IR::Putch(off) => io.putchar(state.read(*off)),
                IR::Getch(off) => state.write(*off, io.getchar()),
                IR::SimpleLoop(delta, inner) => {
                    while state.read(0) != 0 {
                        run_series(inner, state, io);
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
    run_series(&prog.0, &mut state, io);
}
