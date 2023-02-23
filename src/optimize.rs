use crate::ir::{IRProgram, IR};
use std::collections::HashMap;

fn kill_trivial_dead_loops(irs: &Vec<IR>) -> Vec<IR> {
    let mut ret = Vec::new();
    let mut changes = false;

    for ir in irs {
        match ir {
            IR::Loop(_) if !changes => {}
            IR::Add(_) | IR::Getch | IR::Putch => {
                changes = true;
                ret.push(ir.clone());
            }
            ir => ret.push(ir.clone()),
        }
    }
    ret
}

fn compress_adds(irs: &Vec<IR>) -> Vec<IR> {
    let mut ret = Vec::new();

    let mut last_add = None;

    for ir in irs {
        if let IR::Add(amt) = ir {
            last_add = Some(last_add.map_or(*amt, |a: i8| a + *amt));
            continue;
        }

        if let Some(amt) = last_add {
            if amt != 0 {
                ret.push(IR::Add(amt));
            }
            last_add = None;
        }

        if let IR::Loop(inner) = ir {
            ret.push(IR::Loop(compress_adds(inner)));
        } else {
            ret.push(ir.clone());
        }
    }
    if let Some(amt) = last_add {
        if amt != 0 {
            ret.push(IR::Add(amt));
        }
    }

    ret
}

fn compress_changes(irs: &Vec<IR>) -> Vec<IR> {
    let mut ret = Vec::new();

    let mut last_change = None;

    for ir in irs {
        if let IR::PtrChange(amt) = ir {
            last_change = Some(last_change.map_or(*amt, |a: i32| a + *amt));
            continue;
        }

        if let Some(amt) = last_change {
            if amt != 0 {
                ret.push(IR::PtrChange(amt));
            }
            last_change = None;
        }

        if let IR::Loop(inner) = ir {
            ret.push(IR::Loop(compress_changes(inner)));
        } else {
            ret.push(ir.clone());
        }
    }
    if let Some(amt) = last_change {
        if amt != 0 {
            ret.push(IR::PtrChange(amt));
        }
    }

    ret
}

fn simplify_loop(ins: &IR) -> IR {
    let irs: Vec<_> = match ins {
        IR::Loop(i) => i.iter().map(simplify_loop).collect(),
        _ => return ins.clone(),
    };

    let mut ptr_change = 0;
    let mut delta = 0;

    let mut ret_inner = Vec::new();

    let mut simplifiable = true;
    for i in irs.iter() {
        assert!(!matches!(i, IR::MovImm(_)));
        match i {
            IR::Putch => {
                ret_inner.push(IR::Putch);
            }
            IR::Getch | IR::MovImm(_) => {
                if ptr_change == 0 {
                    simplifiable = false;
                    break;
                } else {
                    ret_inner.push(i.clone());
                }
            }
            IR::Add(amt) => {
                if ptr_change == 0 {
                    delta += amt;
                } else {
                    ret_inner.push(i.clone());
                }
            }
            IR::PtrChange(amt) => {
                ptr_change += amt;
                ret_inner.push(i.clone());
            }
            IR::Loop(_) => {
                simplifiable = false;
                break;
            }
            IR::SimpleLoop(d, inner) => {
                if ptr_change == 0 {
                    simplifiable = false;
                    break;
                }
                ret_inner.push(IR::SimpleLoop(*d, inner.clone()));
            }
            IR::AddMul(off, amt) => {
                assert_ne!(*off, -ptr_change); // Too lazy to think through implications
                ret_inner.push(IR::AddMul(*off, *amt));
            }
        }
    }
    if simplifiable && ptr_change == 0 {
        // Can simplify
        IR::SimpleLoop(delta, ret_inner)
    } else {
        IR::Loop(irs)
    }
}

fn compress_muls(irs: &Vec<IR>) -> Vec<IR> {
    let mut ret = Vec::new();
    'outer: for ir in irs {
        if let IR::SimpleLoop(delta, inner) = ir {
            if *delta != -1 {
                ret.push(IR::SimpleLoop(*delta, compress_muls(inner)));
                continue;
            }

            let mut changes = HashMap::new(); //<i32, i8>
            let mut off = 0;
            for iir in inner {
                match iir {
                    IR::PtrChange(amt) => {
                        off += amt;
                    }
                    IR::Add(amt) => {
                        *changes.entry(off).or_insert(0) += amt;
                    }
                    _ => {
                        ret.push(IR::SimpleLoop(*delta, compress_muls(inner)));
                        continue 'outer;
                    }
                }
            }
            for (off, amt) in changes {
                ret.push(IR::AddMul(off, amt));
            }
            ret.push(IR::MovImm(0));
            continue 'outer;
        }
        if let IR::Loop(inner) = ir {
            ret.push(IR::Loop(compress_muls(inner)));
        } else {
            ret.push(ir.clone());
        }
    }
    ret
}

pub fn optimize(prog: &IRProgram) -> IRProgram {
    let irs = compress_adds(&prog.0);
    let irs = compress_changes(&irs);
    let irs = kill_trivial_dead_loops(&irs);
    let irs = irs.iter().map(simplify_loop).collect();
    let irs = compress_changes(&irs);
    let irs = compress_muls(&irs);
    IRProgram(irs)
}
