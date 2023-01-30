use crate::ir::{IRProgram, IR};

pub fn kill_trivial_dead_loops(irs: &Vec<IR>) -> Vec<IR> {
    let mut ret = Vec::new();
    let mut changes = false;

    for ir in irs {
	match ir {
	    IR::Loop(_) if ! changes => {},
	    IR::Add(_) | IR::Getch | IR::Putch  => {
		changes = true;
		ret.push(ir.clone());
	    },
	    ir => ret.push(ir.clone()),
	}
    }
    ret
}

pub fn compress_adds(irs: &Vec<IR>) -> Vec<IR> {
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

pub fn compress_changes(irs: &Vec<IR>) -> Vec<IR> {
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

pub fn optimize(prog: &IRProgram) -> IRProgram {
    let irs = compress_adds(&prog.0);
    let irs = compress_changes(&irs);
    let irs = kill_trivial_dead_loops(&irs);
    IRProgram(irs)
}
