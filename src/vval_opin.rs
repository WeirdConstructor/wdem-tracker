use wlambda::vval::VVal;
use wctr_signal_ops::signals::OpIn;

pub fn vv2opin(v: VVal) -> Option<OpIn> {
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
