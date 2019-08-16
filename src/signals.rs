use wlambda::vval::VVal;
use crate::scopes::SampleRow;
use crate::scopes::SCOPE_SAMPLES;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OpIn {
    Constant(f32),
    Reg(usize),
    RegMix2(usize, usize, f32),
    RegAdd(usize,f32),
    RegMul(usize,f32),
    RegAddMul(usize,f32,f32),
    RegMulAdd(usize,f32,f32),
    RegLerp(usize,f32,f32),
    RegSStep(usize,f32,f32),
    RegMap(usize,f32,f32,f32,f32),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ColorIn {
    pub h:  OpIn,
    pub s:  OpIn,
    pub v:  OpIn,
    pub a:  OpIn,
}

impl ColorIn {
    pub fn calc(&self, regs: &[f32]) -> [f32; 4] {
        extern crate palette;
        use palette::{Rgb};
        let hue : palette::Hsv =
            palette::Hsv::new(
                self.h.calc(regs).into(),
                self.s.calc(regs),
                self.v.calc(regs));
        let rc : Rgb = hue.into();

        [
            rc.red,
            rc.green,
            rc.blue,
            self.a.calc(regs)
        ]
    }
}

impl OpIn {
    pub fn calc(&self, regs: &[f32]) -> f32 {
        match self {
            OpIn::Constant(v)            => *v,
            OpIn::Reg(i)                 => regs[*i],
            OpIn::RegMix2(ia, ib, am)    => regs[*ia] * am + regs[*ib] * (1.0 - am),
            OpIn::RegAdd(i, v)           => v + regs[*i],
            OpIn::RegMul(i, v)           => v * regs[*i],
            OpIn::RegAddMul(i, a, v)     => v * (regs[*i] + a),
            OpIn::RegMulAdd(i, v, a)     => (v * regs[*i]) + a,
            OpIn::RegLerp(i, a, b)       => (a * regs[*i]) + (b * (1.0 - regs[*i])),
            OpIn::RegSStep(i, a, b)      => {
                let x = (regs[*i] - a) / (b - a);
                let x = if x < 0.0 { 0.0 } else { x };
                let x = if x > 1.0 { 1.0 } else { x };
                x * x * (3.0 - 2.0 * x)
            },
            OpIn::RegMap(i, a_frm, b_frm, a_to, b_to) => {
                let x = (regs[*i] - a_frm) / (b_frm - a_frm);
                (a_to * x) + (b_to * (1.0 - x))
            },
        }
    }

    pub fn vv2opin(v: VVal) -> Option<Self> {
        let t = if v.is_vec() {
            v.at(0).unwrap_or(VVal::Nul)
        } else {
            v.clone()
        };

        if t.is_sym() || t.is_str() {
            let s = t.s_raw();
            match &s[..] {
                "reg"  => Some(OpIn::Reg(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize)),
                "mix2" => Some(OpIn::RegMix2(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).i() as usize,
                            v.at(3).unwrap_or(VVal::Nul).f() as f32)),
                "add"  => Some(OpIn::RegAdd(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).f() as f32)),
                "mul"  => Some(OpIn::RegMul(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).f() as f32)),
                "addmul"  => Some(OpIn::RegAddMul(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).f() as f32,
                            v.at(3).unwrap_or(VVal::Nul).f() as f32)),
                "muladd"  => Some(OpIn::RegMulAdd(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).f() as f32,
                            v.at(3).unwrap_or(VVal::Nul).f() as f32)),
                "lerp" => Some(OpIn::RegLerp(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).f() as f32,
                            v.at(3).unwrap_or(VVal::Nul).f() as f32)),
                "sstep" => Some(OpIn::RegSStep(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).f() as f32,
                            v.at(3).unwrap_or(VVal::Nul).f() as f32)),
                "map" => Some(OpIn::RegMap(
                            v.at(1).unwrap_or(VVal::Nul).i() as usize,
                            v.at(2).unwrap_or(VVal::Nul).f() as f32,
                            v.at(3).unwrap_or(VVal::Nul).f() as f32,
                            v.at(4).unwrap_or(VVal::Nul).f() as f32,
                            v.at(5).unwrap_or(VVal::Nul).f() as f32)),
                _ => None
            }
        } else {
            Some(OpIn::Constant(t.f() as f32))
        }
    }
}

pub trait DemOp {
    fn input_count(&self) -> usize;
    fn output_count(&self) -> usize;
    fn init_regs(&mut self, start_reg: usize, regs: &mut [f32]);
    fn get_output_reg(&mut self, name: &str) -> Option<usize>;
    fn set_input(&mut self, name: &str, to: OpIn) -> bool;
    fn exec(&mut self, t: f32, regs: &mut [f32]);
}

pub struct DoSin {
    amp:    OpIn,
    phase:  OpIn,
    vert:   OpIn,
    f:      OpIn,
    out:    usize,
}

impl DoSin {
    pub fn new() -> Self {
        DoSin {
            amp:   OpIn::Constant(1.0),
            phase: OpIn::Constant(0.0),
            vert:  OpIn::Constant(0.0),
            f:     OpIn::Constant(0.001),
            out:   0,
        }
    }
}

impl DemOp for DoSin {
    fn input_count(&self) -> usize { 4 }
    fn output_count(&self) -> usize { 1 }

    fn init_regs(&mut self, start_reg: usize, regs: &mut [f32]) {
        regs[start_reg] = 0.0;
        self.out   = start_reg;
    }

    fn get_output_reg(&mut self, name: &str) -> Option<usize> {
        match name {
            "out"   => Some(self.out),
            _       => None,
        }
    }

    fn set_input(&mut self, name: &str, to: OpIn) -> bool {
        match name {
            "amp"   => { self.amp   = to; true },
            "phase" => { self.phase = to; true },
            "vert"  => { self.vert  = to; true },
            "freq"  => { self.f     = to; true },
            _       => false,
        }
    }

    fn exec(&mut self, t: f32, regs: &mut [f32]) {
        let a = self.amp.calc(regs);
        let p = self.phase.calc(regs);
        let v = self.vert.calc(regs);
        let f = self.f.calc(regs);
        regs[self.out] = a * (((f * t) + p).sin() + v);
        //d// println!("OUT: {}, {}", regs[self.out], self.out);
    }
}

pub struct Simulator {
    pub regs:               Vec<f32>,
    pub ops:                Vec<Box<dyn DemOp>>,
    pub sample_row:         SampleRow,
    pub scope_sample_len:   usize,
    pub scope_sample_pos:   usize,
}

impl Simulator {
    pub fn new() -> Self {
        Simulator {
            regs:               Vec::new(),
            ops:                Vec::new(),
            sample_row:         SampleRow::new(),
            scope_sample_len:   SCOPE_SAMPLES,
            scope_sample_pos:   0,
        }
    }

    pub fn add_op(&mut self, idx: usize, mut op: Box<dyn DemOp>) -> Option<usize> {
        let new_start_reg = self.regs.len();
        let new_reg_count = self.regs.len() + op.output_count();
        self.regs.resize(new_reg_count, 0.0);
        op.init_regs(new_start_reg, &mut self.regs[..]);
        let out_reg = op.get_output_reg("out");

        self.ops.insert(idx, op);
        println!("INSRT {} , {} ", idx, self.ops.len());

        out_reg
    }

    pub fn new_op(&mut self, idx: usize, t: &str) -> Option<usize> {
        let o : Box<dyn DemOp> = match t {
            "sin" => { Box::new(DoSin::new()) },
            _     => { return None; },
        };

        self.add_op(idx, o)
    }

    pub fn set_reg(&mut self, idx: usize, v: f32) -> bool {
        if self.regs.len() > idx {
            self.regs[idx] = v;
            true
        } else {
            false
        }
    }

    pub fn get_reg(&self, idx: usize) -> f32 {
        if self.regs.len() > idx {
            self.regs[idx]
        } else {
            0.0
        }
    }

    pub fn set_op_input(&mut self, idx: usize, input_name: &str, to: OpIn) -> bool {
        if idx >= self.ops.len() {
            return false;
        }
        self.ops[idx].set_input(input_name, to)
    }

    pub fn exec(&mut self, t: f32, ext_scopes: std::sync::Arc<std::sync::Mutex<SampleRow>>) {
        for r in self.ops.iter_mut() {
            r.as_mut().exec(t, &mut self.regs[..]);
        }

        self.sample_row.read_from_regs(&self.regs[..], self.scope_sample_pos);
        self.scope_sample_pos =
            (self.scope_sample_pos + 1) % self.scope_sample_len;

        if let Ok(ref mut m) = ext_scopes.try_lock() {
            use std::ops::DerefMut;
            std::mem::swap(&mut self.sample_row, &mut *m);
        }
    }
}

pub struct DebugRegisters {
    pub debug_regs: Vec<(String, OpIn)>,
}

impl DebugRegisters {
    pub fn new() -> Self {
        DebugRegisters { debug_regs: Vec::new() }
    }

    pub fn add(&mut self, name: String, op_in: OpIn) {
        self.debug_regs.push((name, op_in));
    }

    pub fn show<T>(&self, regs: &[f32], view: &mut T) where T: RegisterView {
        view.start_print_registers();
        for r in self.debug_regs.iter() {
            view.print_register(&r.0, r.1.calc(regs));
        }
        view.end_print_registers();
    }
}

pub trait RegisterView {
    fn start_print_registers(&mut self);
    fn print_register(&mut self, name: &str, value: f32);
    fn end_print_registers(&mut self);
}

