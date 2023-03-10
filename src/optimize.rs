use crate::ir::{self, IRProgram, IR};
use std::collections::HashMap;

fn compress_changes(irs: &Vec<IR>) -> Vec<IR> {
    fn recur(irs: &Vec<IR>, preserve_change: bool) -> Vec<IR> {
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
                IR::Add(off, amt) => {
                    ret.push(IR::Add(off + last_change.unwrap_or(0), *amt));
                }
                _ => {
                    if let Some(amt) = last_change {
                        if amt != 0 {
                            ret.push(IR::PtrChange(amt));
                        }
                        last_change = None;
                    }

                    if let IR::Loop(inner) = ir {
                        ret.push(IR::Loop(recur(inner, true)));
                    } else {
                        ret.push(ir.clone());
                    }
                }
            }
        }
        if let Some(amt) = last_change {
            if preserve_change && amt != 0 {
                ret.push(IR::PtrChange(amt));
            }
        }

        ret
    }
    recur(irs, false)
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
            IR::AddMul(dst_off, amt) => {
                assert_ne!(*dst_off, -ptr_change); // Too lazy to think through implications
                ret_inner.push(IR::AddMul(*dst_off, *amt));
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
                    let init = state
                        .get(&(idx + off + dst_off))
                        .unwrap_or(&Value::Const(0));
                    let multiplier = state.get(&(idx + off)).unwrap_or(&Value::Const(0));

                    if let (Value::Const(i), Value::Const(m)) = (init, multiplier) {
                        state.insert(idx + off + *dst_off, Value::Const(i + m * amt));
                        continue;
                    }

                    match multiplier {
                        Value::Const(i) => {
                            ret.push(IR::MovImm(0, *i));
                        }
                        Value::Add(i) => {
                            if *i != 0 {
                                ret.push(IR::Add(0, *i));
                            }
                        }
                    }
                    match init {
                        Value::Const(i) => {
                            ret.push(IR::MovImm(*dst_off, *i));
                        }
                        Value::Add(i) => {
                            if *i != 0 {
                                ret.push(IR::Add(*dst_off, *i));
                            }
                        }
                    }
                    ret.push(i.clone());
                    state.insert(idx + off + *dst_off, Value::Add(0));
                }
                IR::SimpleLoop(..) => {
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
                IR::Loop(..) => match state.get(&(idx + off)).unwrap_or(&Value::Const(0)) {
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
                        Some(Value::Add(amt)) => {
                            if *amt != 0 {
                                ret.push(IR::Add(*put_off, *amt))
                            }
                        }
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

mod test {
    #![allow(dead_code)]
    use super::*;
    fn lp(inner: Vec<IR>) -> IR {
        IR::Loop(inner)
    }
    fn pc(amt: ir::Offset) -> IR {
        IR::PtrChange(amt)
    }
    fn a(off: ir::Offset, amt: ir::Value) -> IR {
        IR::Add(off, amt)
    }
    fn put(off: ir::Offset) -> IR {
        IR::Putch(off)
    }
    fn get(off: ir::Offset) -> IR {
        IR::Getch(off)
    }
    fn sl(delta: ir::Value, inner: Vec<IR>) -> IR {
        IR::SimpleLoop(delta, inner)
    }
    fn am(off: ir::Offset, delta: ir::Value) -> IR {
        IR::AddMul(off, delta)
    }
    fn mi(off: ir::Offset, imm: ir::Value) -> IR {
        IR::MovImm(off, imm)
    }

    #[test]
    fn test_compress_changes() {
        assert_eq!(
            compress_changes(&vec![lp(vec![pc(1), pc(1), pc(1)])]),
            vec![lp(vec![pc(3)])]
        );
        assert_eq!(compress_changes(&vec![pc(1), pc(-1), pc(1)]), vec![]);
        assert_eq!(compress_changes(&vec![pc(1), pc(-1)]), vec![]);
        assert_eq!(
            compress_changes(&vec![pc(7), a(0, 1), pc(-1)]),
            vec![a(7, 1)]
        );
        assert_eq!(compress_changes(&vec![pc(1), get(0), pc(-1)]), vec![get(1)]);
        assert_eq!(compress_changes(&vec![pc(1), put(0), pc(-1)]), vec![put(1)]);
    }

    #[test]
    fn test_simplify() {
        assert_eq!(simplify_loop(&lp(vec![])), sl(0, vec![]));
        assert_eq!(simplify_loop(&lp(vec![a(0, 1)])), sl(1, vec![]));
        assert_eq!(
            simplify_loop(&lp(vec![a(0, 1), pc(1)])),
            lp(vec![a(0, 1), pc(1)])
        );
        assert_eq!(
            simplify_loop(&lp(vec![a(0, 1), pc(1), a(0, 2), pc(-1)])),
            sl(1, vec![pc(1), a(0, 2), pc(-1)])
        );
    }

    #[test]
    fn test_remove_unread_stores() {
        // assert_eq!(remove_unread_stores(&vec![get(1), mi(0, 1), am(1, 5), put(1)]), vec![]);
    }

    #[test]
    fn test_collapse_consts() {
        assert_eq!(
            remove_unread_stores(&collapse_consts(&vec![
                mi(1, 5),
                mi(0, 1),
                am(1, 5),
                put(1)
            ])),
            vec![mi(1, 10), put(1)]
        );
    }

    fn optimize_code(code: &str) -> Vec<IR> {
        dbg!(code);
        let ap = crate::parser::Parser::parse(code).unwrap();
        let ip = crate::ir::IRProgram::from_ast_program(&ap);
        let ip = optimize(&ip);
        ip.0
    }

    #[test]
    fn test_e2e() {
        //assert_eq!(optimize_code("++>++<++>.>[]"), vec![mi(1, 2), put(1)]);
        //assert_eq!(optimize_code("+++++[->-----<]>."), vec![mi(1, -25), put(1)]);
        assert_eq!(
            optimize_code(">,<++[->+<]>."),
            vec![get(1), mi(0, 2), am(1, 1), put(1)]
        );
        assert_eq!(
            optimize_code(">,<+++++[->-----<]>."),
            vec![get(1), mi(0, 5), am(1, -5), put(1)]
        );
    }
}
