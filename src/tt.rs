#[derive(Debug, PartialEq, Clone)]
pub enum Turtle {
    Commands(Vec<Turtle>),
    LookDir(OpIn, OpIn),
    WithState(Box<Turtle>),
    Area((OpIn, OpIn), Box<Turtle>),
    Rect(OpIn, OpIn, ColorIn),
    Line(OpIn, OpIn, ColorIn),
    CtxInit,
    CtxMove(OpIn, OpIn),
    CtxRot(OpIn),
}

#[derive(Debug, PartialEq, Clone)]
pub struct TurtleState {
    w:          f64,
    h:          f64,
    pos:        [f64; 2],
    dir:        [f64; 2],
    trans:      [[f64; 3]; 2],
    init_trans: [[f64; 3]; 2],
//    color:      [f32; 4],
}

trait TurtleDrawing {
    fn draw_line(color: [f32; 4], transformation: matrix, from: [f32; 2], to: [f32; 2], thickness: f32);
    fn draw_rect_fill(color: [f32; 4], transformation: matrix, pos: [f32; 2], size: [f32; 2]);
}

impl Turtle {
    pub fn exec<T>(&self,
               ts: &mut TurtleState,
               regs: &[f32],
               ctx: T)
        where T: piston_window::Graphics {
        match self {
            Turtle::Commands(v) => {
                for c in v.iter() {
                    c.exec(ts, regs, context, graphics);
                }
            },
            Turtle::WithState(cmds) => {
                let mut sub_ts = ts.clone();
                cmds.exec(&mut sub_ts, regs, context, graphics);
            },
            Turtle::Area((_taw, _tah), _bt) => {
                //
                // turtle:
                //      look_dir x y
                //      rot_dir rad
                //      walk_dir n
                //      line_dir n thickness color
                //
            },
            Turtle::CtxInit => {
                ts.trans = ts.init_trans;
            },
            Turtle::CtxMove(xo, yo) => {
                let x = xo.calc(regs) as f64 * ts.w;
                let y = yo.calc(regs) as f64 * ts.h;
                ts.trans = (ts.trans).trans(x, y);
            },
            Turtle::CtxRot(rot) => {
                let rot = rot.calc(regs) as f64;
                ts.trans = (ts.trans).rot_rad(rot);
            },
            Turtle::LookDir(x, y) => {
                let x = x.calc(regs);
                let y = y.calc(regs);
                ts.dir = [x as f64, y as f64];
                ts.dir = vecmath::vec2_normalized(ts.dir);
            },
            Turtle::Line(n, thick, color) => {
                let n     = n.calc(regs);
                let t     = thick.calc(regs);
                let color = color.calc(regs);
                let mut new_pos = vecmath::vec2_scale(ts.dir, n as f64);
                new_pos[0] = ts.pos[0] + new_pos[0] * ts.w;
                new_pos[1] = ts.pos[1] + new_pos[1] * ts.h;
                let o = Ellipse::new(color);
                o.draw(
                    ellipse::circle(ts.pos[0], ts.pos[1], t.into()),
                    &context.draw_state, ts.trans, graphics);
                o.draw(
                    ellipse::circle(new_pos[0], new_pos[1], t.into()),
                    &context.draw_state, ts.trans, graphics);
                line_from_to(
                    color, t.into(), ts.pos, new_pos, ts.trans, graphics);
                ts.pos = new_pos;
            },
            Turtle::Rect(rw, rh, clr) => {
                let wh = ((rw.calc(regs) * ts.w as f32) / 2.0) as f64;
                let hh = ((rh.calc(regs) * ts.h as f32) / 2.0) as f64;
                let c = clr.calc(regs);
                rectangle(
                    c,
                    [-wh, -hh, wh * 2.0, hh * 2.0],
                    ts.trans,
                    graphics);
                ()
            },
        }
    }
}
