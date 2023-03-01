use crate::ir::{self, IRProgram, IR};
use std::collections::HashMap;

fn compress_changes(irs: &Vec<IR>) -> Vec<IR> {
    let mut ret = Vec::new();

    let mut last_change = None;

    for ir in irs {
        match ir {
            IR::PtrChange(amt) => {
                last_change = Some(last_change.map_or(*amt, |a: ir::Offset| a + *amt));
            }
            IR::Getch(off) => {
                ret.push(IR::Getch(off + last_change.unwrap_or(0)));
            }
            IR::Putch(off) => {
                ret.push(IR::Putch(off + last_change.unwrap_or(0)));
            }
            _ => {
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
            IR::Putch(..) => {
                ret_inner.push(i.clone());
            }
            IR::Getch(..) => {
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
            IR::Add(add_off, amt) => {
                if ptr_change == *add_off {
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
                    IR::Add(add_off, amt) => {
                        *changes.entry(off + add_off).or_insert(0) += amt;
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
    #[derive(Clone, Debug)]
    enum Value {
        Const(i8),
        Add(i8),
    }

    fn recur(irs: &Vec<IR>, state: &mut HashMap<ir::Offset, Value>, idx: ir::Offset) -> Vec<IR> {
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
                IR::Add(add_off, amt) => {
                    let init = state
                        .get(&(idx + off + add_off))
                        .unwrap_or(&Value::Const(0));
                    match init {
                        Value::Add(cur) => {
                            state.insert(idx + off + add_off, Value::Add(amt + cur));
                        }
                        Value::Const(cur) => {
                            state.insert(idx + off + add_off, Value::Const(amt + cur));
                        }
                    }
                }
                IR::AddMul(dst_off, amt) => {
                    let init = match state
                        .get(&(idx + off + dst_off))
                        .unwrap_or(&Value::Const(0))
                    {
                        Value::Add(amt) => {
                            ret.push(IR::Add(*dst_off, *amt));
                            ret.push(i.clone());
                            continue;
                        }
                        Value::Const(amt) => amt,
                    };
                    let multiplier = match state.get(&(idx + off)).unwrap_or(&Value::Const(0)) {
                        Value::Add(amt) => {
                            ret.push(IR::Add(0, *amt));
                            ret.push(i.clone());
                            continue;
                        }
                        Value::Const(amt) => amt,
                    };
                    state.insert(*dst_off, Value::Const(init + multiplier * amt));
                }
                IR::SimpleLoop(delta, inner) => {
                    match state.get(&(idx + off)).unwrap_or(&Value::Const(0)) {
                        Value::Const(0) => {
                            // No looping
                        }
                        _ => {
                            for (loc_off, v) in &mut *state {
                                match v {
                                    Value::Add(amt) => ret.push(IR::Add(loc_off - off - idx, *amt)),
                                    Value::Const(amt) => {
                                        ret.push(IR::MovImm(loc_off - off - idx, *amt))
                                    }
                                }
                            }
                            state.clear();
                            ret.push(i.clone());
                            knowable = false;
                        }
                    }
                }
                IR::Loop(_inner) => match state.get(&(idx + off)).unwrap_or(&Value::Const(0)) {
                    Value::Const(0) => {}
                    _ => {
                        for (loc_off, v) in &mut *state {
                            match v {
                                Value::Add(amt) => ret.push(IR::Add(loc_off - off - idx, *amt)),
                                Value::Const(amt) => {
                                    ret.push(IR::MovImm(loc_off - off - idx, *amt))
                                }
                            }
                        }
                        state.clear();

                        ret.push(i.clone());
                        knowable = false;
                    }
                },
                IR::Putch(put_off) => {
                    match state.get(&(idx + off + put_off)) {
                        Some(Value::Add(amt)) => ret.push(IR::Add(*put_off, *amt)),
                        Some(Value::Const(amt)) => ret.push(IR::MovImm(*put_off, *amt)),
                        None => {}
                    }
                    ret.push(i.clone());
                }
                IR::Getch(get_off) => {
                    ret.push(i.clone());
                    state.insert(idx + off + get_off, Value::Add(0));
                }
                IR::MovImm(dst_off, val) => {
                    state.insert(idx + off + dst_off, Value::Const(*val));
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
                IR::Add(add_off, _amt) => {
                    if let Some(val) = writes.remove(&(idx + off + add_off)) {
                        ret.push(IR::MovImm(*add_off, val));
                    }
                    ret.push(ir.clone());
                }
                IR::Putch(put_off) => {
                    if let Some(val) = writes.remove(&(idx + off + put_off)) {
                        ret.push(IR::MovImm(*put_off, val));
                    }
                    ret.push(ir.clone());
                }
                IR::Getch(getch_off) => {
                    writes.remove(&(idx + off + getch_off));
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
    let irs = &prog.0;
    let irs = compress_changes(&irs);
    let irs = irs.iter().map(simplify_loop).collect();
    let irs = compress_changes(&irs);
    let irs = compress_muls(&irs);
    let irs = collapse_consts(&irs);
    let irs = remove_unread_stores(&irs);
    let irs = compress_changes(&irs);
    IRProgram(irs)
}
