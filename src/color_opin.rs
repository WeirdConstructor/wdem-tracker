use serde::Serialize;
use serde::Deserialize;
use wctr_signal_ops::signals::OpIn;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
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

