extern crate serde_json;
extern crate ggez;
mod gui_painter;
mod track;
mod tracker;
mod tracker_editor;
mod scopes;
mod signals;

use wdem_tracker::track::*;
use wdem_tracker::tracker::*;
use wdem_tracker::tracker_editor::*;
use wdem_tracker::scopes::Scopes;

use std::rc::Rc;
use std::cell::RefCell;

use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler, quit};
use ggez::graphics;
use ggez::input::keyboard::{KeyCode, KeyMods, is_mod_active};

struct DummyParamSet {
}

impl ModuleParameterSet for DummyParamSet {
    fn count(&self) -> usize { 9 }
    fn name(&self, _idx: usize) -> String { format!("ModulA") }
    fn range(&self, _idx: usize) -> (f32, f32) { (0.0, 1.0) }
}

trait ModuleParameterSet {
    fn count(&self) -> usize;
    fn name(&self, idx: usize) -> String;
    fn range(&self, idx: usize) -> (f32, f32);
}

#[derive(Debug, Clone, PartialEq)]
struct Module<PS> where PS: ModuleParameterSet {
    name:          String,
    parameter_set: Box<PS>,
    values:        Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
struct ModuleChain<PS> where PS: ModuleParameterSet {
    name:    String,
    modules: Vec<Module<PS>>,
}

#[derive(Debug, Clone, PartialEq)]
struct ModuleParamSelection {
    chain_name: String,
    mod_name:   String,
    param_idx:  usize,
}

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
        pos[0] += self.offs.0;
        pos[1] += self.offs.1;
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

struct Output {
    values:         Vec<f32>,
    pos:            i32,
    song_pos_s:     f32,
}

impl OutputHandler for Output {
    fn emit_event(&mut self, track_idx: usize, val: f32, flags: u16) {
        println!("EMIT: {}: {}/{}", track_idx, val, flags);
    }

    fn emit_play_line(&mut self, play_line: i32) {
        //d// println!("EMIT PLAYLINE OUT {}", play_line);
        self.pos = play_line;
    }

    fn value_buffer(&mut self) -> &mut Vec<f32> { return &mut self.values; }

    fn song_pos(&mut self) -> &mut f32 { return &mut self.song_pos_s; }
}

fn start_tracker_thread(
    ext_out: std::sync::Arc<std::sync::Mutex<Output>>,
    rcv: std::sync::mpsc::Receiver<TrackerSyncMsg>) -> Scopes {

    let sr = Scopes::new();

    let rr = sr.sample_row.clone();

    std::thread::spawn(move || {

        let mut sim = signals::Simulator::new(1);
        let sin1_out_reg = sim.new_op(0, "sin").unwrap();
        println!("SIN OUT REG : {}", sin1_out_reg);

        let mut o = Output { values: Vec::new(), pos: 0, song_pos_s: 0.0 };
        let mut t = Tracker::new(TrackerNopSync { });

        let mut is_playing = true;
        let mut out_updated = false;
        loop {
            let r = rcv.try_recv();
            match r {
                Ok(TrackerSyncMsg::AddTrack(track)) => {
                    t.add_track(Track::new(&track.name, track.data));
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
                Ok(TrackerSyncMsg::PlayHead(a)) => {
                    match a {
                        PlayHeadAction::TogglePause => {
                            is_playing = !is_playing;
                        },
                        PlayHeadAction::Pause    => { is_playing = false; },
                        PlayHeadAction::Play     => { is_playing = true; },
                        PlayHeadAction::NextLine => {
                            println!("NEXT LINE");
                            t.tick_to_next_line(&mut o);
                            out_updated = true;
                            is_playing = false;
                        },
                        PlayHeadAction::PrevLine => {
                            println!("PREV LINE");
                            t.tick_to_prev_line(&mut o);
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
                t.tick(&mut o);
                out_updated = true;
                //d// println!("THRD: TICK {}", o.pos);
            }

            if out_updated {
                out_updated = false;

                sim.copy_reserved_values(&o.values[..]);
                sim.exec(o.song_pos_s, rr.clone());

                if let Ok(ref mut m) = ext_out.try_lock() {
                    m.pos        = o.pos;
                    m.song_pos_s = o.song_pos_s;
                    o.values     = std::mem::replace(&mut m.values, o.values);
                }
            }

            //d// println!("OUT SIN: sp[{}] {}", t.tick2song_pos_in_s(), sim.get_reg(sin1_out_reg));

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
    SetA(usize, usize, u8),
    SetB(usize, usize, u8),
    SetInt(usize, usize, Interpolation),
    RemoveValue(usize, usize),
    PlayHead(PlayHeadAction),
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
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum InputMode {
    Normal,
    Interpolation,
    Step,
    Value,
    A,
    B,
}

struct WDemTrackerGUI {
    tracker: Rc<RefCell<Tracker<ThreadTrackSync>>>,
    editor:  TrackerEditor<ThreadTrackSync>,
    painter: Rc<RefCell<GGEZPainter>>,
    force_redraw: bool,
    tracker_thread_out: std::sync::Arc<std::sync::Mutex<Output>>,
    i: i32,
    mode: InputMode,
    step: i32,
    scopes: Scopes,
    num_txt: String,
    status_line: String,
}

impl WDemTrackerGUI {
    pub fn new(ctx: &mut Context) -> WDemTrackerGUI {
        let (sync_tx, sync_rx) = std::sync::mpsc::channel::<TrackerSyncMsg>();

        let sync = ThreadTrackSync::new(sync_tx);
        let out = std::sync::Arc::new(std::sync::Mutex::new(Output { values: Vec::new(), pos: 0, song_pos_s: 0.0 }));

        let scopes = start_tracker_thread(out.clone(), sync_rx);

        let font = graphics::Font::new(ctx, "/DejaVuSansMono.ttf").unwrap();
        let trk = Rc::new(RefCell::new(Tracker::new(sync)));
        WDemTrackerGUI {
            tracker: trk.clone(),
            editor: TrackerEditor::new(trk),
            tracker_thread_out: out,
            painter: Rc::new(RefCell::new(GGEZPainter {
                text_cache: std::collections::HashMap::new(),
                reg_view_font: font,
            })),
            scopes,
            force_redraw: true,
            mode: InputMode::Normal,
            step: 0,
            i: 0,
            num_txt: String::from(""),
            status_line: String::from(""),
        }
    }

    pub fn get_status_text(&self) -> String {
        format!("[{:?}] {}", self.mode, self.status_line)
    }

    pub fn set_status_text(&mut self, txt: String) {
        self.status_line = txt;
    }

    pub fn init(&mut self) {
        for i in 0..1 {
            self.tracker.borrow_mut().add_track(
                Track::new(
                    &format!("xxx{}", i),
                    vec![
                        (0, 1.0, Interpolation::Step, 0xFF88),
                        (4, 4.0, Interpolation::Step, 0x88FF),
                        (5, 0.2, Interpolation::Step, 0xBEEF),
                    ]));
        }
    }

    pub fn inp(&mut self, ti: TrackerInput) {
        self.editor.process_input(ti);
    }
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

    fn text_input_event(&mut self, ctx: &mut Context, character: char) {
        println!("CHR: {:?}", character);

        if character == '\u{1b}' { self.mode = InputMode::Normal; }

        match self.mode {
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
            }
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {

        self.i += 1;
        if self.i > 100 {
            println!("FPS: {}", ggez::timer::fps(ctx));
            self.i = 0;
        }

        let _sz = graphics::drawable_size(ctx);
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
            let play_pos_row = self.tracker_thread_out.lock().unwrap().pos;
            let val = self.tracker_thread_out.lock().unwrap().values.clone();
            self.force_redraw = false;
            let mut p : GGEZGUIPainter =
                GGEZGUIPainter { p: self.painter.clone(), c: ctx, offs: (0.0, 0.0), area: (0.0, 0.0) };

            p.set_offs((0.0, 0.0));
            p.draw_text([1.0, 0.0, 0.0, 1.0], [0.0, 0.0], 10.0, self.get_status_text());
            p.set_offs((0.0, 20.0));
            self.editor.show_state(32, &mut p, play_pos_row, &val);

            p.set_offs((0.0, 640.0));
            self.scopes.update_from_sample_row();
            self.scopes.draw_scopes(&mut p);
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
