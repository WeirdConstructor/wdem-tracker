extern crate serde_json;
extern crate ggez;

use std::io::prelude::*;
use wdem_tracker::track::*;
use wdem_tracker::tracker::*;
use wdem_tracker::tracker_editor::*;
use wdem_tracker::scopes::Scopes;
use wctr_signal_ops::signals::*;

use std::rc::Rc;
use std::cell::RefCell;

use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler, quit, MouseButton};
use ggez::graphics;
use ggez::input::keyboard::{KeyCode, KeyMods, is_mod_active};
use ggez::input::mouse::button_pressed;
use ggez::input::mouse::set_cursor_grabbed;
use ggez::input::mouse::set_cursor_hidden;
use ggez::input::mouse::set_position;

/* Synth

- Add DemOp I/O names
- Make a Track an DemOp
- Have 4 outputs: Note, Value, A, B. Note/A/B is 0-256, Value is any.
- Make an DemOpUI, which takes an op index and a name (from wlambda for instance)
  the DemOpUI queries the backend Simulator for details about the OP I/O count
  and names.
    - The UI communicates to the Signal thread via DemOpUIMessage enum.
    - The Simulator can send it's config out via a mpsc channel or some
      other kind of way. It's triggered by a DemOpUIMessage::GetConfig.


- Parameters are just one large array of f32
- Indexes are per device (each device has a index <-> name mapping for access)
- values are calc'ed from the signal regs and inserted at their index.
- also static values are calced that way
- make a configurable link of static values and a GUI element somehow
    - should also be able to set values of static registers?!
      (maybe some static input Op, that acts as device with inputs?)
    => have an array of OpIn's for the device, device communicates the mapping,
       have one global signal device with customizable mapping
- configuration by wlambda



*/

struct GGEZPainter {
    reg_view_font: graphics::Font,
    text_cache: std::collections::HashMap<(usize, String), graphics::Text>,
}

impl GGEZPainter {
    fn draw_lines(&mut self, ctx: &mut Context, color: [f32; 4], pos: [f32; 2],
                  points: &[[f32; 2]], filled: bool, thickness: f32) {
        let pl =
            graphics::Mesh::new_polyline(
                ctx,
                if filled {
                    graphics::DrawMode::fill()
                } else {
                    graphics::DrawMode::stroke(thickness)
                },
                points,
                graphics::Color::from(color)).unwrap();
        graphics::draw(
            ctx, &pl, ([pos[0], pos[1]], 0.0, [0.0, 0.0], graphics::WHITE)).unwrap();
    }

    fn draw_rect(&mut self, ctx: &mut Context, color: [f32; 4], pos: [f32; 2],
                 size: [f32; 2], filled: bool, thickness: f32) {
        let r =
            graphics::Mesh::new_rectangle(
                ctx,
                if filled {
                    graphics::DrawMode::fill()
                } else {
                    graphics::DrawMode::stroke(thickness)
                },
                graphics::Rect::new(0.0, 0.0, size[0], size[1]),
                graphics::Color::from(color)).unwrap();
        graphics::draw(
            ctx, &r, ([pos[0], pos[1]], 0.0, [0.0, 0.0], graphics::WHITE)).unwrap();
    }

    fn draw_text(&mut self, ctx: &mut Context, color: [f32; 4], pos: [f32; 2], size: f32, text: String) {
        let us = (size * 1000.0) as usize;
        let key = (us, text.clone());
        let txt = self.text_cache.get(&key);
        let txt_elem = if let Some(t) = txt {
            t
        } else {
            let t = graphics::Text::new((text, self.reg_view_font, size));
            self.text_cache.insert(key.clone(), t);
            self.text_cache.get(&key).unwrap()
        };

        graphics::queue_text(
            ctx, txt_elem, pos, Some(color.into()));
    }

    fn finish_draw_text(&mut self, ctx: &mut Context) {
        graphics::draw_queued_text(
            ctx,
            graphics::DrawParam::default(),
            None,
            graphics::FilterMode::Linear).unwrap();
    }
}

struct GGEZGUIPainter<'b> {
    p: Rc<RefCell<GGEZPainter>>,
    c: &'b mut ggez::Context,
    offs: (f32, f32),
    area: (f32, f32),
}

impl<'b> wdem_tracker::gui_painter::GUIPainter for GGEZGUIPainter<'b> {
    fn start(&mut self) { }
    fn draw_lines(&mut self, color: [f32; 4], mut pos: [f32; 2], points: &[[f32; 2]], filled: bool, thickness: f32) {
        pos[0] += self.offs.0;
        pos[1] += self.offs.1;
        self.p.borrow_mut().draw_lines(&mut self.c, color, pos, points, filled, thickness);
    }
    fn draw_rect(&mut self, color: [f32; 4], mut pos: [f32; 2], size: [f32; 2], filled: bool, thickness: f32) {
        pos[0] += self.offs.0;
        pos[1] += self.offs.1;
        self.p.borrow_mut().draw_rect(&mut self.c, color, pos, size, filled, thickness);
    }
    fn draw_text(&mut self, color: [f32; 4], mut pos: [f32; 2], size: f32, text: String) {
        pos[0] += self.offs.0 - 0.5;
        pos[1] += self.offs.1 - 0.5;
        self.p.borrow_mut().draw_text(&mut self.c, color, pos, size, text);
    }
    fn show(&mut self) {
        self.p.borrow_mut().finish_draw_text(&mut self.c);
    }

    fn set_offs(&mut self, offs: (f32, f32)) { self.offs = offs; }
    fn get_offs(&mut self) -> (f32, f32) { self.offs }
    fn set_area_size(&mut self, area: (f32, f32)) { self.area = area; }
    fn get_area_size(&mut self) -> (f32, f32) { self.area }
}

#[derive(Debug, PartialEq, Clone)]
pub enum OperatorInputMode {
    Value(f32),
    RegInfo(String),
}


#[derive(Debug)]
pub struct OperatorInputSettings {
        simcom:       SimulatorCommunicator,
    pub specs:        Vec<(DemOpIOSpec, OpInfo)>,
    pub groups:       Vec<(String, Vec<usize>)>,
                    //     x/y/w/h,  op_i,  in_idx
        active_zones: Vec<([f32; 4], usize, usize)>,
        highlight: Option<(usize, usize)>,
        selection: Option<(usize, usize)>,
        accu_mpos: [f32; 2],
        orig_val:  f32,
        scroll_offs: (usize, usize),
}

fn draw_op<P>(p: &mut P, op: &(DemOpIOSpec, OpInfo), highlight: &Option<(usize, usize)>, selection: &Option<(usize, usize)>) -> (f32, f32, Vec<([f32; 4], usize, usize)>)
        where P: wdem_tracker::gui_painter::GUIPainter {
    let inp_col_w : f32 = 102.0;
    let inp_col_wr: f32 =  70.0;
    let padding   : f32 =   2.0;
    let text_h    : f32 =  12.0;

    let mut io_lens = op.0.input_values.len();
    if op.0.output_regs.len() > io_lens {
        io_lens = op.0.output_regs.len();
    }

    let op_h = (1 + io_lens) as f32 * text_h + padding * 2.0;
    let op_w = padding + inp_col_w + padding + inp_col_wr;

    p.draw_rect(
        [0.2, 0.2, 0.2, 1.0], [0.0, 0.0], [op_w, op_h], true, 0.1);
    p.draw_rect(
        [1.0, 0.0, 1.0, 1.0], [0.0, 0.0], [op_w, op_h], false, 0.5);

    p.add_offs(padding, padding);

    p.draw_text(
        [0.3, 1.0, 0.8, 1.0], [0.0, 0.0], text_h, format!("{}", op.1.name));
    p.draw_text(
        [1.0, 0.3, 0.3, 1.0], [inp_col_w - (text_h + padding), 0.0], text_h, "IN".to_string());
    p.draw_text(
        [0.3, 1.0, 0.3, 1.0], [inp_col_w + padding, 0.0], text_h, "OUT".to_string());

    let mut y = text_h;

    let mut active_zones : Vec<([f32; 4], usize, usize)> = Vec::new();

    for (idx, (i, is)) in op.0.input_values.iter().zip(op.0.inputs.iter()).enumerate() {
        let text = match i {
            OpIn::Constant(v) => {
                format!("{:>8.2}", *v)
            },
            OpIn::Reg(u) =>
                format!("r{}", *u),
            OpIn::RegMix2(u, u2, f) =>
                format!("r{}x{:0.2}[{:0.2}]", *u, *u2, *f),
            OpIn::RegAdd(u, f) =>
                format!("r{}+[{:0.2}]", *u, *f),
            OpIn::RegMul(u, f) =>
                format!("r{}*[{:0.2}]", *u, *f),
            OpIn::RegAddMul(u, f, f2) =>
                format!("(r{}+[{:0.2}])*[{:0.2}]", *u, *f, *f2),
            OpIn::RegMulAdd(u, f, f2) =>
                format!("(r{}*[{:0.2}])+[{:0.2}]", *u, *f, *f2),
            OpIn::RegLerp(u, f, f2) =>
                format!("r{}/[{:0.2}][{:0.2}]", *u, *f, *f2),
            OpIn::RegSStep(u, f, f2) =>
                format!("r{}~[{:0.2}][{:0.2}]", *u, *f, *f2),
            OpIn::RegMap(u, f, f2, g, g2) =>
                format!("r{}[{:0.2}-{:0.2}]->[{:0.2}-{:0.2}]", *u, *f, *f2, *g, *g2),
        };

        let o = p.get_offs();
        active_zones.push(([o.0, o.1 + y, inp_col_w, text_h], op.0.index, idx));

        let mut highlighted = if let Some((op_idx, i_idx)) = highlight {
            *op_idx == op.0.index && idx == *i_idx
        } else {
            false
        };

        // XXX: Because the mouse cursor is repositioned, we would
        //      get flickering on neighbour elements.
        if selection.is_some() {
            highlighted = false;
        }

        let selected = if let Some((op_idx, i_idx)) = selection {
            *op_idx == op.0.index && idx == *i_idx
        } else {
            false
        };

        p.draw_rect(
            if selected { [1.0, 0.2, 0.2, 1.0] }
            else        { [0.4, 0.4, 0.4, 1.0] },
            [0.0, y],
            [inp_col_w - 1.0, text_h - 1.0],
            !selected && highlighted,
            0.5);

        p.draw_text(
            [1.0, 0.3, 0.8, 1.0], [0.0, y], text_h,
            format!("{:<7} {}", is.name, text));

        y += text_h;
    }

    y = text_h;
    for (o, os) in op.0.output_regs.iter().zip(op.0.outputs.iter()) {
        p.draw_text(
            [1.0, 0.3, 0.8, 1.0], [inp_col_w + padding, y], text_h,
            format!("{:<7} r{}", os.name, o));
        y += text_h;
    }

    p.add_offs(0.0, -padding);
    p.draw_lines(
        [1.0, 0.0, 1.0, 1.0],
        [inp_col_w, 0.0],
        &vec![[0.0, 0.0], [0.0, op_h]],
        false,
        0.5);

    (op_w, op_h, active_zones)
}

impl OperatorInputSettings {
    fn new(simcom: SimulatorCommunicator) -> Self {
        OperatorInputSettings {
            simcom:        simcom,
            specs:         Vec::new(),
            groups:        Vec::new(),
            active_zones:  Vec::new(),
            highlight:     None,
            selection:     None,
            accu_mpos:     [0.0, 0.0],
            orig_val:      0.0,
            scroll_offs:   (0, 0),
        }
    }

    pub fn save_input_values(&mut self) -> Vec<(String, Vec<(String, OpIn)>)> {
        self.simcom.save_input_values()
    }

    pub fn load_input_values(&mut self, inputs: &Vec<(String, Vec<(String, OpIn)>)>) {
        self.simcom.load_input_values(inputs);
    }

    pub fn hit_zone(&mut self, x: f32, y: f32) -> Option<(usize, usize)> {
        for az in self.active_zones.iter() {
            if    x >= (az.0)[0]
               && y >= (az.0)[1]
               && x <= ((az.0)[0] + (az.0)[2])
               && y <= ((az.0)[1] + (az.0)[3]) {

               return Some((az.1, az.2));
            }
        }

        None
    }

    pub fn handle_mouse_move(&mut self, x: f32, y: f32, xr: f32, yr: f32, button_is_down: bool) -> bool {
        if !button_is_down { self.selection = None; }

        let old_highlight = self.highlight;

        self.highlight = None;

        for az in self.active_zones.iter() {
            if    x >= (az.0)[0]
               && y >= (az.0)[1]
               && x <= ((az.0)[0] + (az.0)[2])
               && y <= ((az.0)[1] + (az.0)[3]) {

                self.highlight = Some((az.1, az.2));

                if    button_is_down
                   && self.selection.is_none()
                   && old_highlight == self.highlight {

                    self.accu_mpos = [0.0, 0.0];
                    self.selection = self.highlight;
                    self.orig_val  = self.get_selection_val();
                    return true;
                }
                break;
            }
        }

        if self.selection.is_some() && button_is_down {
            self.accu_mpos[0] += xr;
            self.accu_mpos[1] += yr;

            let ampli = -(self.accu_mpos[1] as f32 / 100.0);
            let s = self.selection.unwrap();
            self.set_input_val(s.0, s.1, self.orig_val + ampli);
            return true;
        }

        false
    }

    pub fn set_input_default(&mut self, op_idx: usize, i_idx: usize) {
        let iname = self.specs[op_idx].0.inputs[i_idx].name.clone();
        let default = self.specs[op_idx].0.input_defaults[i_idx];
        self.specs[op_idx].0.input_values[i_idx] = default;
        self.simcom.set_op_input(op_idx, &iname, default, false);
    }

    pub fn set_input_val(&mut self, op_idx: usize, i_idx: usize, val: f32) {
        let iname = self.specs[op_idx].0.inputs[i_idx].name.clone();
        self.specs[op_idx].0.input_values[i_idx] = OpIn::Constant(val);
        self.simcom.set_op_input(op_idx, &iname, OpIn::Constant(val), false);
    }

    pub fn get_selection_val(&self) -> f32 {
        if self.selection.is_some() {
            let s = self.selection.unwrap();
            self.get_input_val(s.0, s.1)
        } else {
            0.0
        }
    }

    pub fn get_input_val(&self, op_idx: usize, i_idx: usize) -> f32 {
        if let OpIn::Constant(v) = self.specs[op_idx].0.input_values[i_idx] {
            v
        } else {
            0.0
        }
    }

    pub fn update(&mut self) {
        let r = self.simcom.update(|ev| {
            if let SimulatorUIEvent::OpSpecUpdate(up) = ev {
                Some(up)
            } else { None }
        });

        if r.is_some() {
            self.update_from_spec(r.unwrap().unwrap());
        }
    }

    fn update_from_spec(&mut self, specs: Vec<(DemOpIOSpec, OpInfo)>) {
        println!("Updated: {:?}", specs);
        self.specs = specs;
        self.groups = Vec::new();

        for iv in self.specs.iter() {
            let group = &iv.1.group;
            if group.index <= self.groups.len() {
                self.groups.resize(group.index + 1, ("".to_string(), Vec::new()));
            }
        }

        for i in 0..self.groups.len() {
            let ops : Vec<usize> =
                self.specs
                    .iter()
                    .filter(|o| o.1.group.index == i)
                    .map(|o| o.0.index)
                    .collect();

            if ops.is_empty() { continue; }

            let group = self.specs[ops[0]].1.group.clone();
            println!("OP: {:?} => {}", ops, group.name);

            self.groups[i] = (group.name.clone(), ops);
        }
    }

    pub fn draw<P>(&mut self, p: &mut P) where P: wdem_tracker::gui_painter::GUIPainter {
        let text_h = 10.0;

        self.active_zones = Vec::new();

        let oo = p.get_offs();

        let mut skip_groups_count = self.scroll_offs.1;
        for grp in self.groups.iter() {
            if skip_groups_count > 0 {
                skip_groups_count -= 1;
                continue;
            }

            p.draw_text([1.0, 1.0, 1.0, 1.0], [0.0, 0.0], text_h, grp.0.clone());

            let ooo = p.get_offs();

            let mut skip_ops_count = self.scroll_offs.0;

            let mut max_op_h = 0.0;
            for op_i in grp.1.iter() {
                if skip_ops_count > 0 {
                    skip_ops_count -= 1;
                    continue;
                }

                let op = &self.specs[*op_i];
                let o = p.get_offs();
                p.set_offs((o.0, o.1 + text_h));

                let (w, h, zones) = draw_op(p, op, &self.highlight, &self.selection);
                self.active_zones.extend_from_slice(&zones);
                if h > max_op_h { max_op_h = h; }
                p.set_offs((o.0 + w + 3.0, o.1));

                if (p.get_offs().0 - oo.0) > p.get_area_size().0 {
                    break;
                }

            }

            p.set_offs(ooo);

            p.add_offs(0.0, max_op_h + text_h);
        }

        p.set_offs(oo);
    }
}

struct Output {
    pos:            i32,
    song_pos_s:     f32,
    cpu:            (f64, f64, f64),
}

impl OutputHandler for Output {
    fn emit_event(&mut self, track_idx: usize, val: f32, flags: u16) {
        println!("EMIT: {}: {}/{}", track_idx, val, flags);
    }

    fn emit_play_line(&mut self, play_line: i32) {
        //d// println!("EMIT PLAYLINE OUT {}", play_line);
        self.pos = play_line;
    }

    fn song_pos(&mut self) -> &mut f32 { return &mut self.song_pos_s; }
}

fn calc_cpu_percentage(millis: u128, interval_ms: u128) -> f64 {
     ((millis * 100000)
      / ((interval_ms * 1000) as u128)) as f64 / 1000.0
}

use wlambda;
use wlambda::vval::VVal;
use wlambda::prelude::create_wlamba_prelude;
use wlambda::vval::{Env};

struct AudioThreadWLambdaContext {
    pub sim: Simulator,
    pub track_values: std::rc::Rc<std::cell::RefCell<Vec<f32>>>,
}

fn eval_audio_script(ctxref: std::rc::Rc<std::cell::RefCell<AudioThreadWLambdaContext>>) {
    let genv = create_wlamba_prelude();

    genv.borrow_mut().add_func(
        "p", |env: &mut Env, _argc: usize| {
            println!("{}", env.arg(0).s_raw());
            Ok(VVal::Bol(true))
        }, Some(1), Some(1));

    genv.borrow_mut().add_func(
        "signal_group", |env: &mut Env, _argc: usize| {
            let name = env.arg(0).s_raw();
            env.with_user_do(|ctx: &mut AudioThreadWLambdaContext| {
                Ok(VVal::Int(ctx.sim.add_group(&name) as i64))
            })
        }, Some(1), Some(1));

    genv.borrow_mut().add_func(
        "op", |env: &mut Env, _argc: usize| {
            let op_type     = env.arg(0).s_raw();
            let op_name     = env.arg(1).s_raw();
            let group_index = env.arg(2).i() as usize;
            env.with_user_do(|ctx: &mut AudioThreadWLambdaContext| {
                if let Some(outreg) =
                    ctx.sim.new_op(&op_type, &op_name, group_index) {

                    Ok(VVal::Int(outreg as i64))
                } else {
                    Ok(VVal::Nul)
                }
            })
        }, Some(3), Some(3));

    genv.borrow_mut().add_func(
        "track_proxy", |env: &mut Env, _argc: usize| {
            let track_count = env.arg(0).i() as usize;
            let group_index = env.arg(1).i() as usize;
            env.with_user_do(|ctx: &mut AudioThreadWLambdaContext| {
                let oprox = DoOutProxy::new(track_count);
                ctx.track_values = oprox.values.clone();
                ctx.sim.add_op(Box::new(oprox), String::from("T"), group_index);
                Ok(VVal::Bol(true))
            })
        }, Some(2), Some(2));

    let mut wl_eval_ctx =
        wlambda::compiler::EvalContext::new_with_user(genv, ctxref);

    match wl_eval_ctx.eval_file("tracker.wl") {
        Ok(_) => (),
        Err(e) => { panic!(format!("AUDIO SCRIPT ERROR: {}", e)); }
    }
}

fn start_tracker_thread(
    ext_out: std::sync::Arc<std::sync::Mutex<Output>>,
    rcv: std::sync::mpsc::Receiver<TrackerSyncMsg>,
    mut ep: SimulatorCommunicatorEndpoint) -> Scopes {

    let sr = Scopes::new();
    let rr = sr.sample_row.clone();

    std::thread::spawn(move || {
        let ctxref =
            std::rc::Rc::new(std::cell::RefCell::new(AudioThreadWLambdaContext {
                sim:          Simulator::new(),
                track_values: std::rc::Rc::new(std::cell::RefCell::new(vec![])),
            }));

        eval_audio_script(ctxref.clone());

        // wlambda API:
        // - (audio thread) setup simulator groups
        // - (audio thread) setup simulator operators and their default vals
        // - (audio thread) setup audio buffers and routings between the audio
        //                  devices.
        // - (audio thread) specify which audio devices receive note events
        //                  from the tracks.
        // - (frontend thread) add tracks
        // - (frontend thread) configure tracker values (needs sync!)
        // - (frontend thread) specify project file name
        // - (frontend thread) turtle setup
        // - (frontend thread) frontend simulator setup (groups, operators, ...)
        //                     (insert backend values via DoOutProxy)

//        let mut sim = Simulator::new();
//        sim.add_group("globals");
//        let sin1_out_reg = sim.new_op(0, "sin", "Sin1", 0).unwrap();

//        let oprox = DoOutProxy::new(5);
//        let val_rc = oprox.values.clone();
//        sim.add_op(1, Box::new(oprox), "tracks".to_string(), 0);

//        println!("SIN OUT REG : {}", sin1_out_reg);

        let mut ctx = ctxref.borrow_mut();

        let mut o = Output { pos: 0, song_pos_s: 0.0, cpu: (0.0, 0.0, 0.0) };
        let mut t = Tracker::new(TrackerNopSync { });

        let mut is_playing = true;
        let mut out_updated = false;
        let mut micros_min : u128 = 9999999;
        let mut micros_max : u128 = 0;
        let mut micros_sum : u128 = 0;
        let mut micros_cnt : u128 = 0;
        loop {
            let now = std::time::Instant::now();

            ep.handle_ui_messages(&mut ctx.sim);

            let r = rcv.try_recv();
            match r {
                Ok(TrackerSyncMsg::AddTrack(track)) => {
                    t.add_track(track.clone());
                    println!("THRD: TRACK ADD TRACK");
                },
                Ok(TrackerSyncMsg::SetInt(track_idx, line, int)) => {
                    t.set_int(track_idx, line, int);
                    println!("THRD: SET VAL");
                },
                Ok(TrackerSyncMsg::SetValue(track_idx, line, v)) => {
                    t.set_value(track_idx, line, v);
                    println!("THRD: SET VAL");
                },
                Ok(TrackerSyncMsg::SetNote(track_idx, line, v)) => {
                    t.set_note(track_idx, line, v);
                    println!("THRD: SET NOTE {}", v);
                },
                Ok(TrackerSyncMsg::SetA(track_idx, line, v)) => {
                    t.set_a(track_idx, line, v);
                    println!("THRD: SET A");
                },
                Ok(TrackerSyncMsg::SetB(track_idx, line, v)) => {
                    t.set_b(track_idx, line, v);
                    println!("THRD: SET B");
                },
                Ok(TrackerSyncMsg::RemoveValue(track_idx, line)) => {
                    t.remove_value(track_idx, line);
                    println!("THRD: REMO VAL");
                },
                Ok(TrackerSyncMsg::DeserializeContents(track_idx, contents)) => {
                    t.deserialize_contents(track_idx, contents);
                },
                Ok(TrackerSyncMsg::PlayHead(a)) => {
                    match a {
                        PlayHeadAction::TogglePause => {
                            is_playing = !is_playing;
                        },
                        PlayHeadAction::Pause    => { is_playing = false; },
                        PlayHeadAction::Play     => { is_playing = true; },
                        PlayHeadAction::NextLine => {
                            println!("NEXT LINE");
                            t.tick_to_next_line(&mut o, &ctx.track_values);
                            out_updated = true;
                            is_playing = false;
                        },
                        PlayHeadAction::PrevLine => {
                            println!("PREV LINE");
                            t.tick_to_prev_line(&mut o, &ctx.track_values);
                            out_updated = true;
                            is_playing = false;
                        },
                        PlayHeadAction::Restart  => {
                            t.reset_pos();
                            is_playing = true;
                        },
                        // _ => (),
                    }
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => (),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return (),
            }

            if is_playing {
                t.tick(&mut o, &ctx.track_values);
                out_updated = true;
                //d// println!("THRD: TICK {}", o.pos);
            }

            if out_updated {
                out_updated = false;

                ctx.sim.exec(o.song_pos_s, rr.clone());

                if let Ok(ref mut m) = ext_out.try_lock() {
                    m.pos        = o.pos;
                    m.song_pos_s = o.song_pos_s;
                    m.cpu        = o.cpu;
                }
            }

//            std::thread::sleep(
//                std::time::Duration::from_millis(1));

            let elap = now.elapsed().as_micros();
            micros_sum += elap;
            micros_cnt += 1;
            if micros_min > elap { micros_min = elap; }
            if micros_max < elap { micros_max = elap; }


            if micros_cnt > 200 {
                o.cpu = (
                    calc_cpu_percentage(micros_sum / micros_cnt, t.tick_interval as u128),
                    calc_cpu_percentage(micros_min, t.tick_interval as u128),
                    calc_cpu_percentage(micros_max, t.tick_interval as u128));

                println!("audio thread %cpu: min={:<6}, max={:<6}, {:<6} {:<4} | {:<4} / {:6.2}/{:6.2}/{:6.2}",
                         micros_min,
                         micros_max,
                         micros_sum,
                         micros_cnt,
                         micros_sum / micros_cnt,
                         o.cpu.0,
                         o.cpu.1,
                         o.cpu.2);

                micros_cnt = 0;
                micros_sum = 0;
                micros_min = 9999999;
                micros_max = 0;
            }

            std::thread::sleep(
                std::time::Duration::from_millis(
                    t.tick_interval as u64));
        }
    });

    sr
}

#[derive(Debug, Clone)]
enum TrackerSyncMsg {
    AddTrack(Track),
    SetValue(usize, usize, f32),
    SetNote(usize, usize, u8),
    SetA(usize, usize, u8),
    SetB(usize, usize, u8),
    SetInt(usize, usize, Interpolation),
    RemoveValue(usize, usize),
    PlayHead(PlayHeadAction),
    DeserializeContents(usize, TrackSerialized),
}

struct ThreadTrackSync {
    send: std::sync::mpsc::Sender<TrackerSyncMsg>,
}

impl ThreadTrackSync {
    fn new(send: std::sync::mpsc::Sender<TrackerSyncMsg>) -> Self {
        ThreadTrackSync { send }
    }
}

impl TrackerSync for ThreadTrackSync {
    fn add_track(&mut self, t: Track) {
        self.send.send(TrackerSyncMsg::AddTrack(t));
    }
    fn set_int(&mut self, track_idx: usize, line: usize, int: Interpolation) {
        self.send.send(TrackerSyncMsg::SetInt(track_idx, line, int));
    }
    fn set_value(&mut self, track_idx: usize, line: usize, value: f32) {
        self.send.send(TrackerSyncMsg::SetValue(track_idx, line, value));
    }
    fn set_note(&mut self, track_idx: usize, line: usize, value: u8) {
        self.send.send(TrackerSyncMsg::SetNote(track_idx, line, value));
    }
    fn set_a(&mut self, track_idx: usize, line: usize, value: u8) {
        self.send.send(TrackerSyncMsg::SetA(track_idx, line, value));
    }
    fn set_b(&mut self, track_idx: usize, line: usize, value: u8) {
        self.send.send(TrackerSyncMsg::SetB(track_idx, line, value));
    }
    fn remove_value(&mut self, track_idx: usize, line: usize) {
        self.send.send(TrackerSyncMsg::RemoveValue(track_idx, line));
    }
    fn play_head(&mut self, act: PlayHeadAction) {
        self.send.send(TrackerSyncMsg::PlayHead(act));
    }
    fn deserialize_contents(&mut self, track_idx: usize, contents: TrackSerialized) {
        self.send.send(TrackerSyncMsg::DeserializeContents(track_idx, contents));
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum InputMode {
    Normal,
    Interpolation,
    Step,
    Value,
    A,
    B,
    Note,
    OpInValue(usize, usize),
    FileActions,
    ScrollOps,
}

struct WDemTrackerGUI {
    tracker:            Rc<RefCell<Tracker<ThreadTrackSync>>>,
    editor:             TrackerEditor<ThreadTrackSync>,
    painter:            Rc<RefCell<GGEZPainter>>,
    force_redraw:       bool,
    tracker_thread_out: std::sync::Arc<std::sync::Mutex<Output>>,
    i:                  i32,
    mode:               InputMode,
    step:               i32,
    scopes:             Scopes,
    num_txt:            String,
    octave:             u8,
    status_line:        String,
    grabbed_mpos:       Option<[f32; 2]>,
    op_inp_set:         OperatorInputSettings,
}

impl WDemTrackerGUI {
    pub fn new(ctx: &mut Context) -> WDemTrackerGUI {
        let (sync_tx, sync_rx) = std::sync::mpsc::channel::<TrackerSyncMsg>();

        let mut simcom = SimulatorCommunicator::new();

        let sync = ThreadTrackSync::new(sync_tx);
        let out = std::sync::Arc::new(std::sync::Mutex::new(Output { pos: 0, song_pos_s: 0.0, cpu: (0.0, 0.0, 0.0) }));

        let scopes =
            start_tracker_thread(
                out.clone(),
                sync_rx,
                simcom.get_endpoint());

        let font = graphics::Font::new(ctx, "/DejaVuSansMono.ttf").unwrap();
        let trk = Rc::new(RefCell::new(Tracker::new(sync)));
        let mut ctx = WDemTrackerGUI {
            tracker:            trk.clone(),
            editor:             TrackerEditor::new(trk),
            tracker_thread_out: out,
            force_redraw:       true,
            mode:               InputMode::Normal,
            step:               0,
            i:                  0,
            num_txt:            String::from(""),
            octave:             4,
            grabbed_mpos:       None,
            status_line:        String::from(""),
            op_inp_set:         OperatorInputSettings::new(simcom),
            scopes,
            painter: Rc::new(RefCell::new(GGEZPainter {
                text_cache: std::collections::HashMap::new(),
                reg_view_font: font,
            })),
        };

        ctx.op_inp_set.update();

        ctx
    }

    pub fn get_status_text(&self) -> String {
        format!("[{:?}] {}", self.mode, self.status_line)
    }

    pub fn set_status_text(&mut self, txt: String) {
        self.status_line = txt;
    }

    pub fn init(&mut self) {
        for i in 0..3 {
            let lpp = self.tracker.borrow().lpp;
            let mut t = Track::new(&format!("xxx{}", i), lpp);
            t.touch_pattern_idx(1);
            t.touch_pattern_idx(2);
            t.set_arrangement_pattern(lpp, 2);
            t.set_arrangement_pattern(lpp * 2, 1);
            t.set_arrangement_pattern(lpp * 3, 0);
            self.tracker.borrow_mut().add_track(t);
        }
    }

    pub fn inp(&mut self, ti: TrackerInput) {
        self.editor.process_input(ti);
    }
}

fn write_file_safely(filename: &str, s: &str) -> std::io::Result<()> {
    let tmpfile = format!("{}~", filename);
    let mut file = std::fs::File::create(tmpfile.clone())?;
    file.write_all(s.as_bytes())?;
    std::fs::rename(tmpfile, filename)?;
    Ok(())
}

impl EventHandler for WDemTrackerGUI {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
        if keycode == KeyCode::Q {
            quit(ctx);
        }
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Right {
            if let Some((op_idx, in_idx)) = self.op_inp_set.hit_zone(x, y) {
                self.mode = InputMode::OpInValue(op_idx, in_idx);
                self.num_txt = String::from("");
                self.set_status_text(format!("input value[]"));
            }

        } else if button == MouseButton::Middle {
            if let Some((op_idx, in_idx)) = self.op_inp_set.hit_zone(x, y) {
                self.op_inp_set.set_input_default(op_idx, in_idx);
            }
        }
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32,
                          xr: f32, yr: f32) {

        let mouse_is_grabbed =
            self.op_inp_set.handle_mouse_move(
                x, y, xr, yr, button_pressed(ctx, MouseButton::Left));

        if mouse_is_grabbed {
            if self.grabbed_mpos.is_none() {
                self.grabbed_mpos = Some([x, y]);
            }

            set_cursor_grabbed(ctx, true).expect("mouse ok");
            set_cursor_hidden(ctx, true);
            set_position(ctx, self.grabbed_mpos.unwrap()).expect("mouse ok");

        } else {
            set_cursor_grabbed(ctx, false).expect("mouse ok");
            set_cursor_hidden(ctx, false);
            self.grabbed_mpos = None;
        }
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) {
        println!("CHR: {:?}", character);

        if character == '\u{1b}' { self.mode = InputMode::Normal; }

        let mode =
            if is_mod_active(ctx, KeyMods::ALT) {
                InputMode::Note
            } else {
                self.mode
            };

        match mode {
            InputMode::Normal => {
                self.set_status_text(String::from(""));
                match character {
                    's' => {
                        self.mode = InputMode::Step;
                        self.step = 0;
                    },
                    'x' => {
                        self.editor.process_input(TrackerInput::Delete);
                    },
                    'h' => {
                        self.editor.process_input(TrackerInput::TrackLeft);
                    },
                    'f' => {
                        self.mode = InputMode::FileActions;
                        self.set_status_text(format!("'w' write, 'r' read"));
                    },
                    'y' => {
                        self.op_inp_set.update();
                    },
                    'j' | 'J' => {
                        if is_mod_active(ctx, KeyMods::SHIFT) {
                            self.editor.process_input(TrackerInput::RowDown);
                        } else {
                            self.editor.process_input(TrackerInput::StepDown);
                        }
                    },
                    'k' | 'K' => {
                        if is_mod_active(ctx, KeyMods::SHIFT) {
                            self.editor.process_input(TrackerInput::RowUp);
                        } else {
                            self.editor.process_input(TrackerInput::StepUp);
                        }
                    },
                    'l' => {
                        self.editor.process_input(TrackerInput::TrackRight);
                    },
                    'i' => {
                        self.mode = InputMode::Interpolation;
                    },
                    ' ' => {
                        self.editor.process_input(
                            TrackerInput::PlayHead(PlayHeadAction::TogglePause));
                    },
                    '#' => {
                        self.mode = InputMode::Note;
                    },
                    'o' => {
                        self.mode = InputMode::ScrollOps;

                    },
                    'n' => {
                        self.editor.process_input(
                            TrackerInput::PlayHead(PlayHeadAction::PrevLine));
                    },
                    'm' => {
                        self.editor.process_input(
                            TrackerInput::PlayHead(PlayHeadAction::NextLine));
                    },
                    'a' => {
                        self.num_txt = String::from("");
                        self.mode = InputMode::A;
                    },
                    'b' => {
                        self.num_txt = String::from("");
                        self.mode = InputMode::B;
                    },
                    '-' | '.' | '0'..='9' => {
                        self.num_txt = String::from("");
                        self.num_txt.push(character);
                        self.mode = InputMode::Value;
                        self.set_status_text(format!("value[{}]", self.num_txt));
                    },
                    _ => { },
                }
            },
            InputMode::Note => {
                let mut note = 0;
                // XXX: This is just german layout :-/
                match character {
                    '+' => { if self.octave < 9 { self.octave += 1; } },
                    '-' => { if self.octave > 0 { self.octave -= 1; } },
                    'y' => { note = (self.octave + 1) * 12 + 0;  }, // C
                    's' => { note = (self.octave + 1) * 12 + 1;  }, // C#
                    'x' => { note = (self.octave + 1) * 12 + 2;  }, // D
                    'd' => { note = (self.octave + 1) * 12 + 3;  }, // D#
                    'c' => { note = (self.octave + 1) * 12 + 4;  }, // E
                    'v' => { note = (self.octave + 1) * 12 + 5;  }, // F
                    'g' => { note = (self.octave + 1) * 12 + 6;  }, // F#
                    'b' => { note = (self.octave + 1) * 12 + 7;  }, // G
                    'h' => { note = (self.octave + 1) * 12 + 8;  }, // G#
                    'n' => { note = (self.octave + 1) * 12 + 9;  }, // A
                    'j' => { note = (self.octave + 1) * 12 + 10; }, // A#
                    'm' => { note = (self.octave + 1) * 12 + 11; }, // B

                    'q' => { note = (self.octave + 2) * 12 + 0;  }, // C
                    '2' => { note = (self.octave + 2) * 12 + 1;  }, // C#
                    'w' => { note = (self.octave + 2) * 12 + 2;  }, // D
                    '3' => { note = (self.octave + 2) * 12 + 3;  }, // D#
                    'e' => { note = (self.octave + 2) * 12 + 4;  }, // E
                    'r' => { note = (self.octave + 2) * 12 + 5;  }, // F
                    '5' => { note = (self.octave + 2) * 12 + 6;  }, // F#
                    't' => { note = (self.octave + 2) * 12 + 7;  }, // G
                    '6' => { note = (self.octave + 2) * 12 + 8;  }, // G#
                    'z' => { note = (self.octave + 2) * 12 + 9;  }, // A
                    '7' => { note = (self.octave + 2) * 12 + 10; }, // A#
                    'u' => { note = (self.octave + 2) * 12 + 11; }, // B

                    'i' => { note = (self.octave + 3) * 12 + 0;  }, // C
                    '9' => { note = (self.octave + 3) * 12 + 1;  }, // C#
                    'o' => { note = (self.octave + 3) * 12 + 2;  }, // D
                    '0' => { note = (self.octave + 3) * 12 + 3;  }, // D#
                    'p' => { note = (self.octave + 3) * 12 + 4;  }, // E
                    _ => { },
                }

                self.set_status_text(format!("octave[{}]", self.octave));

                if note > 0 {
                    self.inp(TrackerInput::SetNote(note));
                    self.editor.process_input(TrackerInput::StepDown);
                }
            },
            InputMode::A => {
                match character {
                    '0'..='9' | 'A'..='F' | 'a'..='f'  => {
                        self.num_txt.push(character);
                        self.set_status_text(format!("a[{}]", self.num_txt));
                    },
                    _ => { }
                }

                if self.num_txt.len() >= 2 {
                    self.inp(TrackerInput::SetA(
                        u8::from_str_radix(&self.num_txt, 16).unwrap_or(0)));
                    self.mode = InputMode::Normal;
                }
            },
            InputMode::B => {
                match character {
                    '0'..='9' | 'A'..='F' | 'a'..='f'  => {
                        self.num_txt.push(character);
                        self.set_status_text(format!("a[{}]", self.num_txt));
                    },
                    _ => { }
                }

                if self.num_txt.len() >= 2 {
                    self.inp(TrackerInput::SetB(
                        u8::from_str_radix(&self.num_txt, 16).unwrap_or(0)));
                    self.mode = InputMode::Normal;
                }
            },
            InputMode::OpInValue(op_idx, in_idx) => {
                match character {
                    '-' | '.' | '0'..='9' => {
                        self.num_txt.push(character);
                    },
                    '\r' => {
                        self.op_inp_set.set_input_val(
                            op_idx, in_idx,
                            self.num_txt.parse::<f32>().unwrap_or(0.0));
                        self.mode = InputMode::Normal;
                    },
                    _ => { }
                }

                self.set_status_text(format!("input value[{}]", self.num_txt));
            },
            InputMode::Value => {
                match character {
                    '-' | '.' | '0'..='9' => {
                        self.num_txt.push(character);
                    },
                    '\r' => {
                        self.inp(TrackerInput::SetValue(
                            self.num_txt.parse::<f32>().unwrap_or(0.0)));
                        self.mode = InputMode::Normal;
                    },
                    _ => { }
                }

                self.set_status_text(format!("value[{}]", self.num_txt));
            },
            InputMode::Interpolation => {
                match character {
                    'e' => { self.inp(TrackerInput::SetInterpExp); },
                    't' => { self.inp(TrackerInput::SetInterpSStep); },
                    's' => { self.inp(TrackerInput::SetInterpStep); },
                    'l' => { self.inp(TrackerInput::SetInterpLerp); },
                    _ => { },
                }

                self.mode = InputMode::Normal;
            },
            InputMode::Step => {
                match character {
                    '0' => { self.step *= 10; },
                    '1' => { self.step += 1; },
                    '2' => { self.step += 2; },
                    '3' => { self.step += 3; },
                    '4' => { self.step += 4; },
                    '5' => { self.step += 5; },
                    '6' => { self.step += 6; },
                    '7' => { self.step += 7; },
                    '8' => { self.step += 8; },
                    '9' => { self.step += 9; },
                    _ => { self.mode = InputMode::Normal; },
                }

                self.set_status_text(format!("step[{}]", self.step));

                self.editor.process_input(
                    TrackerInput::SetStep(self.step as usize));
            },
            InputMode::FileActions => {
                match character {
                    'w' => {
                        let s  = self.op_inp_set.save_input_values();
                        let st = self.editor.tracker.borrow().serialize_tracks();

                        match serde_json::to_string_pretty(&(s, st)) {
                            Ok(s) => {
                                match write_file_safely("tracker.json", &s) {
                                    Ok(()) => {
                                        self.set_status_text(
                                            format!("everything written ok"));
                                    },
                                    Err(e) => {
                                        self.set_status_text(
                                            format!("write error 'tracker.json': {}", e));
                                        println!("tracker.json WRITE ERROR: {}", e);
                                    }
                                }
                            },
                            Err(e) => {
                                self.set_status_text(format!("serialize error: {}", e));
                                println!("SERIALIZE ERROR: {}", e);
                            }
                        };
                    },
                    'r' => {
                        match std::fs::File::open("tracker.json") {
                            Ok(mut file) => {
                                let mut c = String::new();
                                match file.read_to_string(&mut c) {
                                    Ok(_) => {
                                        match serde_json::from_str(&c) {
                                            Ok(v) => {
                                                let v : (Vec<(String, Vec<(String, OpIn)>)>, Vec<TrackSerialized>) = v;
                                                self.op_inp_set.load_input_values(&v.0);
                                                self.editor.tracker.borrow_mut().deserialize_tracks(v.1);
                                                self.op_inp_set.update();
                                            },
                                            Err(e) => {
                                                self.set_status_text(
                                                    format!("deserialize error 'tracker.json': {}", e));
                                                println!("tracker.json DESERIALIZE ERROR: {}", e);
                                            },
                                        }
                                    },
                                    Err(e) => {
                                        self.set_status_text(
                                            format!("read error 'tracker.json': {}", e));
                                        println!("tracker.json READ ERROR: {}", e);
                                    }
                                }
                            },
                            Err(e) => {
                                self.set_status_text(
                                    format!("open error 'tracker.json': {}", e));
                                println!("tracker.json OPEN ERROR: {}", e);
                            }
                        }
//        valmap = serde_json::from_str(s).unwrap_or(valmap);
                    },
                    _ => (),
                }

                self.mode = InputMode::Normal;
            },
            InputMode::ScrollOps => {
                match character {
                    'h' => {
                        if self.op_inp_set.scroll_offs.0 > 0 {
                            self.op_inp_set.scroll_offs.0 -= 1;
                        }
                    },
                    'l' => {
                        self.op_inp_set.scroll_offs.0 += 1;
                    },
                    'j' => {
                        self.op_inp_set.scroll_offs.1 += 1;
                    },
                    'k' => {
                        if self.op_inp_set.scroll_offs.1 > 0 {
                            self.op_inp_set.scroll_offs.1 -= 1;
                        }
                    },
                    _ => { self.mode = InputMode::Normal; },
                }

                self.set_status_text(
                    format!("offset[{}, {}]",
                            self.op_inp_set.scroll_offs.0,
                            self.op_inp_set.scroll_offs.1));
            },
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {

        self.i += 1;
        if self.i > 100 {
            println!("FPS: {}", ggez::timer::fps(ctx));
            self.i = 0;
        }

        let sz = graphics::drawable_size(ctx);
//        let param =
//            graphics::DrawParam::from(
//                ([sz.0 / 2.0, sz.1 / 2.0],));
//        graphics::push_transform(ctx, Some(param.to_matrix()));
//        graphics::apply_transformations(ctx)?;

        let _now_time = ggez::timer::time_since_start(ctx).as_millis();

        //d// let mut ov = OutputValues { values: Vec::new() };

        //d// self.editor.tracker.borrow_mut().tick(&mut ov);
//        if !ov.values.is_empty() {
//            println!("OUT: {:?}", ov.values[0]);
//        }

        // println!("THREAD POS: {}", self.tracker_thread_out.lock().unwrap().pos);

        self.force_redraw = true;
        if self.force_redraw || self.editor.need_redraw() {
            use wdem_tracker::gui_painter::GUIPainter;

            graphics::clear(ctx, graphics::BLACK);
            let play_line = self.tracker_thread_out.lock().unwrap().pos;
            let cpu       = self.tracker_thread_out.lock().unwrap().cpu;
            self.force_redraw = false;
            let mut p : GGEZGUIPainter =
                GGEZGUIPainter { p: self.painter.clone(), c: ctx, offs: (0.0, 0.0), area: (0.0, 0.0) };

            p.set_offs(((sz.0 - 126.0).floor() + 0.5, 0.5));
            p.draw_text(
                [1.0, 1.0, 1.0, 1.0],
                [0.0, 0.0],
                10.0,
                format!("CPU {:6.2}/{:6.2}/{:6.2}", cpu.0, cpu.1, cpu.2));

            p.set_offs((0.5, 0.5));
            p.draw_text([1.0, 0.0, 0.0, 1.0], [0.0, 0.0], 10.0, self.get_status_text());

            p.set_offs((0.5, 20.5));
            p.set_area_size((sz.0, sz.1 / 2.0));
            self.editor.draw(&mut p, play_line);

            p.set_offs((0.5, 40.5 + (sz.1 / 2.0).floor()));
            p.set_area_size((sz.0 / 2.0, sz.1 / 2.0));
            self.scopes.update_from_sample_row();
            self.scopes.draw_scopes(&mut p);

            p.set_offs(((sz.0 / 2.0).floor() + 0.5, 40.5 + sz.1 / 2.0));
            self.op_inp_set.draw(&mut p);

            p.show();
//            p.draw_lines([1.0, 0.0, 1.0, 1.0], [200.0, 100.0], &vec![[1.0, 10.0], [5.0, 20.0]], false, 0.5);
//            self.painter.borrow_mut().finish_draw_text(ctx);
        }

        //d// println!("O: {:?}", self.tracker_thread_out.lock().unwrap().values);
        //d// println!("POS: {:?}", self.tracker_thread_out.lock().unwrap().pos);

        graphics::present(ctx)
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        graphics::set_screen_coordinates(ctx,
            graphics::Rect::new(0.0, 0.0, width, height)).unwrap();
        self.force_redraw = true;
    }
}


fn main() {
    // Make a Context and an EventLoop.
    let (mut ctx, mut event_loop) =
       ContextBuilder::new("wdem_tracker", "Weird Constructor")
            .window_setup(ggez::conf::WindowSetup {
                title: "wdem_tracker".to_owned(),
                samples: ggez::conf::NumSamples::Four,
                ..Default::default()
            })
            .window_mode(ggez::conf::WindowMode {
                width:           640.0,
                height:          480.0,
                maximized:       false,
                fullscreen_type: ggez::conf::FullscreenType::Windowed,
                borderless:      false,
                min_width:       0.0,
                max_width:       0.0,
                min_height:      0.0,
                max_height:      0.0,
                resizable:       true,
            })
           .build()
           .unwrap();

    let mut engine = WDemTrackerGUI::new(&mut ctx);
    engine.init();

    match event::run(&mut ctx, &mut event_loop, &mut engine) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e)
    }
}
