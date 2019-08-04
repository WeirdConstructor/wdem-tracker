use vecmath;
use crate::signals::OpIn;
use crate::signals::ColorIn;
use std::rc::Rc;
use std::cell::RefCell;

// Turtle TODO:
//      color   (3 arbitrary OpIn regs: hsv)
//      push        - state push (pos, direction, color)
//      pop         - state pop
//      move_to
//      rot_rad (direction from last 2 movements)
//      rot_deg (direction from last 2 movements)
//      line_to
//      line_walk
//      rect_walk
//      rect_to
//      rect
//      arc
//      ellipse_walk
//      ellipse_to
//      ellipse


#[derive(Debug, PartialEq, Clone)]
pub enum Turtle {
    Commands(Vec<Turtle>),
    LookDir(OpIn, OpIn),
    WithState(Box<Turtle>),
    Rect(OpIn, OpIn, ColorIn),
    RectLine(OpIn, OpIn, OpIn, ColorIn),
    Line(OpIn, OpIn, ColorIn),
    SeedRand(OpIn),
    SeedGRand(OpIn),
    NextRand(usize, usize),
    NextGRand(usize, usize),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ShapeRotation {
    LeftBottom(f32),
//    TopRight(f32),
    Center(f32),
}

#[derive(Clone)]
pub struct TurtleState {
    w:          f32,
    h:          f32,
    pos:        [f32; 2],
    dir:        [f32; 2],
    rand:       [[u64; 2]; 10],
    randg:      Rc<RefCell<[[u64; 2];100]>>,
}

// Taken from xoroshiro128 crate under MIT License
// Implemented by Matthew Scharley (Copyright 2016)
// https://github.com/mscharley/rust-xoroshiro128
pub fn next_xoroshiro128(state: &mut [u64; 2]) -> u64 {
    let s0: u64     = state[0];
    let mut s1: u64 = state[1];
    let result: u64 = s0.wrapping_add(s1);

    s1 ^= s0;
    state[0] = s0.rotate_left(55) ^ s1 ^ (s1 << 14); // a, b
    state[1] = s1.rotate_left(36); // c

    result
}

// Taken from rand::distributions
// Licensed under the Apache License, Version 2.0
// Copyright 2018 Developers of the Rand project.
pub fn u64_to_open01(u: u64) -> f64 {
    use core::f64::EPSILON;
    let float_size         = std::mem::size_of::<f64>() as u32 * 8;
    let fraction           = u >> (float_size - 52);
    let exponent_bits: u64 = (1023 as u64) << 52;
    f64::from_bits(fraction | exponent_bits) - (1.0 - EPSILON / 2.0)
}

impl TurtleState {
    pub fn new(w: f32, h: f32) -> Self {
        TurtleState {
            w,
            h,
            pos: [0.0, 0.0],
            dir: [0.0, 1.0],
            rand: [[0; 2]; 10],
            randg: Rc::new(RefCell::new([[0; 2]; 100])),
        }
    }

    pub fn go_dir_n(&mut self, n: f32) -> ([f32; 2], [f32; 2]) {
        let mut new_pos = vecmath::vec2_scale(self.dir, n);
        new_pos[0] = self.pos[0] + new_pos[0] * self.w;
        new_pos[1] = self.pos[1] + new_pos[1] * self.h;
        return (std::mem::replace(&mut self.pos, new_pos), new_pos);
    }

    pub fn get_direction_angle(&self) -> f32 {
        2.0 * std::f32::consts::PI
        - ((1.0 as f32).atan2(0.0)
           - self.dir[1].atan2(self.dir[0]))
    }
}

pub trait TurtleDrawing {
    fn draw_line(&mut self, color: [f32; 4], rot: ShapeRotation, from: [f32; 2], to: [f32; 2], thickness: f32);
    fn draw_rect_fill(&mut self, color: [f32; 4], rot: ShapeRotation, pos: [f32; 2], size: [f32; 2]);
    fn draw_rect_outline(&mut self, color: [f32; 4], rot: ShapeRotation, pos: [f32; 2], size: [f32; 2], thickness: f32);
}

impl Turtle {
    pub fn exec<T>(&self,
               ts: &mut TurtleState,
               regs: &mut [f32],
               ctx: &mut T)
        where T: TurtleDrawing {
        match self {
            Turtle::Commands(v) => {
                for c in v.iter() {
                    c.exec(ts, regs, ctx);
                }
            },
            Turtle::SeedRand(seed) => {
                let seed = seed.calc(regs) as u64;
                for i in 0..10 {
                    let a : u64 = i + seed + 0x193a6754a8a7d469;
                    let b : u64 = (i * 7) + seed + 0x97830e05113ba7bb;
                    ts.rand[i as usize] = [a, b];
                }
            },
            Turtle::SeedGRand(seed) => {
                let seed = seed.calc(regs) as u64;
                for i in 0..100 {
                    let a : u64 = i + seed + 0x193a6754a8a7d469;
                    let b : u64 = (i * 7) + seed + 0x97830e05113ba7bb;
                    ts.rand[i as usize] = [a, b];
                }
            },
            Turtle::NextRand(idx, reg_idx) => {
//                regs[reg_idx] = 
            },
            Turtle::NextGRand(idx, reg_idx) => {
            },
            Turtle::WithState(cmds) => {
                let mut sub_ts = ts.clone();
                cmds.exec(&mut sub_ts, regs, ctx);
            },
            Turtle::LookDir(x, y) => {
                let x = x.calc(regs);
                let y = y.calc(regs);
                ts.dir = [x as f32, y as f32];
                ts.dir = vecmath::vec2_normalized(ts.dir);
            },
            Turtle::Line(n, thick, color) => {
                let n     = n.calc(regs);
                let t     = thick.calc(regs);
                let color = color.calc(regs);
                let (pos_a, pos_b) = ts.go_dir_n(n as f32);
                ctx.draw_line(
                    color,
                    ShapeRotation::LeftBottom(0.0),
                    pos_a,
                    pos_b,
                    t.into());
            },
            Turtle::RectLine(rw, rh, thick, clr) => {
                let w = rw.calc(regs) * ts.w;
                let h = rh.calc(regs) * ts.h;
                let t = thick.calc(regs);
                let c = clr.calc(regs);
                let angle = ts.get_direction_angle();

                ctx.draw_rect_outline(
                    c,
                    ShapeRotation::Center(angle),
                    [ts.pos[0], ts.pos[1]],
                    [w, h],
                    t);
            },
            Turtle::Rect(rw, rh, clr) => {
                let w = rw.calc(regs) * ts.w;
                let h = rh.calc(regs) * ts.h;
                let c = clr.calc(regs);
                let angle = ts.get_direction_angle();

                ctx.draw_rect_fill(
                    c,
                    ShapeRotation::Center(angle),
                    [ts.pos[0], ts.pos[1]],
                    [w, h]);
            },
        }
    }
}
