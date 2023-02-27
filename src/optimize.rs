use crate::ir::{self, IRProgram, IR};
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
            last_add = Some(last_add.map_or(*amt, |a: ir::Value| a + *amt));
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
            last_change = Some(last_change.map_or(*amt, |a: ir::Offset| a + *amt));
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
        assert!(!matches!(i, IR::MovImm(..)));
        match i {
            IR::Putch => {
                ret_inner.push(IR::Putch);
            }
            IR::Getch => {
                if ptr_change == 0 {
                    simplifiable = false;
                    break;
                } else {
                    ret_inner.push(i.clone());
                }
            }
            IR::MovImm(dst_off, _) => {
                if ptr_change + dst_off == 0 {
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

            let mut changes = HashMap::new(); //<ir::Offset, ir::Value>
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
            ret.push(IR::MovImm(0, 0));
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

fn collapse_consts(irs: &Vec<IR>) -> Vec<IR> {
    fn recur(irs: &Vec<IR>, state: &mut HashMap<ir::Offset, Option<ir::Value>>, idx: ir::Offset) -> Vec<IR> {
        let mut knowable = true;
        let mut ret = Vec::new();
        let mut off = 0;
        for i in irs {
            if !knowable {
                ret.push(i.clone());
                continue;
            }

            match i {
                IR::PtrChange(amt) => {
                    ret.push(i.clone());
                    off += amt;
                }
                IR::Add(amt) => {
                    let init = match state.get(&(idx + off)) {
                        None => 0,
                        Some(None) => {
                            ret.push(i.clone());
                            continue;
                        }
                        Some(Some(v)) => *v,
                    };
                    let sum = init + amt;
                    state.insert(idx + off, Some(sum));
                    ret.push(IR::MovImm(0, sum));
                }
                IR::AddMul(dst_off, amt) => {
                    let init = match state.get(&(idx + off + dst_off)) {
                        None => 0,
                        Some(None) => {
                            ret.push(i.clone());
                            continue;
                        }
                        Some(Some(v)) => *v,
                    };
                    let multiplier = match state.get(&(idx + off)) {
                        None => 0,
                        Some(None) => {
                            ret.push(i.clone());
                            continue;
                        }
                        Some(Some(v)) => *v,
                    };
                    ret.push(IR::MovImm(*dst_off, init + multiplier * amt));
                    state.insert(*dst_off, Some(init + multiplier * amt));
                }
                IR::SimpleLoop(delta, inner) => {
                    match state.get(&(idx + off)) {
                        None | Some(Some(0)) => {
                            // No looping
                        }
                        _ => {
                            ret.push(i.clone());
                            knowable = false;
                        }
                    }
                }
                IR::Loop(_inner) => match state.get(&(idx + off)) {
                    None | Some(Some(0)) => {}
                    _ => {
                        ret.push(i.clone());
                        knowable = false;
                    }
                },
                IR::Putch => {
                    ret.push(i.clone());
                }
                IR::Getch => {
                    ret.push(i.clone());
                    state.insert(idx + off, None);
                }
                IR::MovImm(dst_off, val) => {
                    state.insert(idx + off + dst_off, Some(*val));
                    ret.push(i.clone());
                }
            }
        }
        ret
    }

    recur(irs, &mut HashMap::new(), 0)
}

fn remove_unread_stores(irs: &Vec<IR>) -> Vec<IR> {
    fn flush_writes(writes: &mut HashMap<ir::Offset, ir::Value>, cur: ir::Offset) -> Vec<IR> {
        let mut ret = Vec::new();
        for (glob_off, val) in writes.iter() {
            ret.push(IR::MovImm(glob_off - cur, *val));
        }
        writes.clear();
        ret
    }
    fn recur(irs: &Vec<IR>, idx: ir::Offset, flush: bool) -> Vec<IR> {
        let mut ret = Vec::new();
        let mut off = 0;
        let mut writes = HashMap::new();
        let mut knowable = true;
        for ir in irs {
            if !knowable {
                ret.push(ir.clone());
                continue;
            }

            match ir {
                IR::Loop(_) => {
                    ret.extend(flush_writes(&mut writes, idx + off));
                    ret.push(ir.clone());
                    knowable = false;
                }
                IR::SimpleLoop(delta, inner) => {
                    ret.extend(flush_writes(&mut writes, idx + off));
                    ret.push(IR::SimpleLoop(*delta, recur(inner, idx + off, true)));
                    writes.insert(idx + off, 0);
                }
                IR::AddMul(dst_off, _amt) => {
                    if let Some(val) = writes.remove(&(idx + off + dst_off)) {
                        ret.push(IR::MovImm(*dst_off, val));
                    }
                    if let Some(val) = writes.remove(&(idx + off)) {
                        ret.push(IR::MovImm(0, val));
                    }
                    ret.push(ir.clone());
                }
                IR::PtrChange(amt) => {
                    off += amt;
                    ret.push(ir.clone());
                }
                IR::Add(_amt) => {
                    if let Some(val) = writes.remove(&(idx + off)) {
                        ret.push(IR::MovImm(0, val));
                    }
                    ret.push(ir.clone());
                }
                IR::Putch => {
                    if let Some(val) = writes.remove(&(idx + off)) {
                        ret.push(IR::MovImm(0, val));
                    }
                    ret.push(ir.clone());
                }
                IR::Getch => {
                    writes.remove(&(idx + off));
                    ret.push(ir.clone());
                }
                IR::MovImm(dst_off, amt) => {
                    writes.insert(idx + off + dst_off, *amt);
                }
            }
        }
        if flush {
            ret.extend(flush_writes(&mut writes, idx + off));
        }
        ret
    }
    recur(irs, 0, false)
}

pub fn optimize(prog: &IRProgram) -> IRProgram {
    let irs = compress_adds(&prog.0);
    let irs = compress_changes(&irs);
    let irs = kill_trivial_dead_loops(&irs);
    let irs = irs.iter().map(simplify_loop).collect();
    let irs = compress_changes(&irs);
    let irs = compress_muls(&irs);
    let irs = collapse_consts(&irs);
    let irs = remove_unread_stores(&irs);
    let irs = compress_changes(&irs);
    IRProgram(irs)
}
