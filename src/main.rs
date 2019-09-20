extern crate serde_json;
extern crate ggez;

use std::io::prelude::*;
use wdem_tracker::track::*;
use wdem_tracker::tracker::*;
use wdem_tracker::tracker_editor::*;
use wdem_tracker::scopes::{Scopes, SCOPE_SAMPLES, SCOPE_WIDTH};
use wctr_signal_ops::*;
use wdem_tracker::audio::*;
use wdem_tracker::key_shortcut_help::*;
use wdem_tracker::ggez_gui_painter::{GGEZPainter, GGEZGUIPainter};
use wdem_tracker::operator_gui::*;
use wdem_tracker::tracker_thread::*;

use wlambda;
use wlambda::{VVal, GlobalEnv, EvalContext, Env};

use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
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
    HelpScreen(usize),
}

struct WDemTrackerGUI {
    tracker:            Rc<RefCell<Tracker<ThreadTrackSync>>>,
    editor:             TrackerEditor<ThreadTrackSync>,
    painter:            Rc<RefCell<GGEZPainter>>,
    force_redraw:       bool,
    tracker_thread_out: std::sync::Arc<std::sync::Mutex<TrackerThreadOutput>>,
    i:                  i32,
    mode:               InputMode,
    step:               i32,
    scopes:             Scopes,
    audio_scopes:       Scopes,
    num_txt:            String,
    octave:             u8,
    status_line:        String,
    grabbed_mpos:       Option<[f32; 2]>,
    ref_mpos:           [f32; 2],
    op_inp_set:         OperatorInputSettings,
    evctx:              wlambda::compiler::EvalContext,
}

impl WDemTrackerGUI {
    pub fn new(ctx: &mut Context) -> WDemTrackerGUI {
        let (sync_tx, sync_rx) = std::sync::mpsc::channel::<TrackerSyncMsg>();

        let mut simcom = SimulatorCommunicator::new();

        let sync = ThreadTrackSync::new(sync_tx);
        let out = std::sync::Arc::new(std::sync::Mutex::new(TrackerThreadOutput::new()));

        let genv = GlobalEnv::new_default();
        let mut wl_eval_ctx =
            wlambda::compiler::EvalContext::new(genv);

        let msgh = wlambda::threads::MsgHandle::new();
        let snd = msgh.sender();

        let scopes =
            start_tracker_thread(
                msgh,
                out.clone(),
                sync_rx,
                simcom.get_endpoint());

        let audio_scopes = Scopes::new(0);

        snd.register_on_as(&mut wl_eval_ctx, "audio");

        match wl_eval_ctx.eval_file("tracker.wl") {
            Ok(_) => (),
            Err(e) => { panic!(format!("SCRIPT ERROR: {}", e)); }
        }

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
            ref_mpos:           [0.0, 0.0],
            num_txt:            String::from(""),
            octave:             4,
            grabbed_mpos:       None,
            status_line:        String::from("(F1 - Help, q - Quit)"),
            op_inp_set:         OperatorInputSettings::new(simcom),
            evctx:              wl_eval_ctx,
            scopes,
            audio_scopes,
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
        for i in 0..6 {
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
        } else if keycode == KeyCode::F1 {
            self.mode = InputMode::HelpScreen(0);
        }

        println!("KEY {:?}", keycode);

        match self.mode {
            InputMode::HelpScreen(p) => {
                match keycode {
                    KeyCode::Space | KeyCode::PageDown => {
                        let mut p = p + 1;
                        if p > 2 { p = 0; }
                        self.mode = InputMode::HelpScreen(p);
                    },
                    KeyCode::Back | KeyCode::PageUp => {
                        if p > 0 {
                            let mut p = p - 1;
                            self.mode = InputMode::HelpScreen(p);
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
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
                          mut xr: f32, mut yr: f32) {

        let sz = graphics::drawable_size(ctx);
        // XXX: Workaround for bug in winit, where on windows WM_MOUSEMOTION
        //      is kept being sent to the application. And ggez does
        xr = x - self.ref_mpos[0];
        yr = y - self.ref_mpos[1];

        let mouse_is_grabbed =
            self.op_inp_set.handle_mouse_move(
                x, y, xr, yr, button_pressed(ctx, MouseButton::Left));

        if mouse_is_grabbed {
            if self.grabbed_mpos.is_none() {
                self.grabbed_mpos = Some([x, y]);
                set_position(ctx, [sz.0 / 2.0, sz.1 / 2.0]);
                self.ref_mpos = [sz.0 / 2.0, sz.1 / 2.0];
            }

        } else {
            if self.grabbed_mpos.is_some() {
                set_position(ctx, self.grabbed_mpos.unwrap());
            }
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
                self.set_status_text(String::from("(F1 - Help, q - Quit)"));
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
            InputMode::HelpScreen(_) => {
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

        self.force_redraw = true;
        if self.force_redraw || self.editor.need_redraw() {
            use wdem_tracker::gui_painter::GUIPainter;

            graphics::clear(ctx, graphics::BLACK);
            let mut play_line = 0;
            let mut cpu       = (0.0, 0.0, 0.0);

            if let Ok(mut out) = self.tracker_thread_out.lock() {
                play_line = out.pos;
                cpu       = out.cpu;
                if out.audio_scope_done {
                    self.audio_scopes.update_from_audio_bufs(
                        &out.audio_scope_samples);

                    for ass in out.audio_scope_samples.iter_mut() {
                        ass.clear();
                    }
                    out.audio_scope_done = false;
                }

            }


            self.force_redraw = false;
            let mut p : GGEZGUIPainter =
                GGEZGUIPainter { p: self.painter.clone(), c: ctx, offs: (0.0, 0.0), area: (0.0, 0.0) };

            match self.mode {
                InputMode::HelpScreen(page) => {
                    p.set_offs((10.0, 10.0));
                    p.draw_rect(
                        [0.2, 0.2, 0.2, 1.0], [0.0, 0.0],
                        [sz.0 - 20.0, sz.1 - 20.0], true, 0.0);
                    p.draw_rect(
                        [1.0, 1.0, 1.0, 1.0], [0.0, 0.0],
                        [sz.0 - 20.0, sz.1 - 20.0], false, 2.0);
                    p.add_offs(5.0, 5.0);

                    p.draw_text(
                        [1.0, 1.0, 1.0, 1.0], [0.0, 0.0],
                        15.0,
                        format!("[page {}/3] (navigation: Space/Backspace or PageUp/PageDown)\n", page + 1) +
                        &get_shortcut_help_page(page),
                    ) // p.draw_text
                },
                _ => {
                    p.set_offs(((sz.0 - 126.0).floor() + 0.5, 0.5));
                    p.draw_text(
                        [1.0, 1.0, 1.0, 1.0],
                        [0.0, 0.0],
                        10.0,
                        format!("CPU {:6.2}/{:6.2}/{:6.2}", cpu.0, cpu.1, cpu.2));

                    p.set_offs((0.5, 0.5));
                    p.draw_text([1.0, 0.0, 0.0, 1.0], [0.0, 0.0], 10.0, self.get_status_text());

                    p.set_offs((0.5, 20.5));
                    p.set_area_size((sz.0 - 2.0 * SCOPE_WIDTH, sz.1 / 2.0));
                    self.editor.draw(&mut p, play_line);

                    let y_below_tracker = 40.5 + (sz.1 / 2.0).floor();

                    let areas = p.get_area_size();
                    p.set_offs((areas.0 + 0.5, 0.5));
                    p.set_area_size((SCOPE_WIDTH, sz.1 / 2.0));
                    self.scopes.update_from_sample_row();
                    self.scopes.draw_scopes(&mut p);

                    p.add_offs(SCOPE_WIDTH, 0.0);
                    p.set_area_size((SCOPE_WIDTH, sz.1 / 2.0));
                    self.audio_scopes.draw_scopes(&mut p);

                    p.set_offs((0.5, y_below_tracker));
                    p.set_area_size((sz.0, sz.1 / 2.0));
                    self.op_inp_set.draw(&mut p);
                }
            }

            p.show();
        }

        graphics::present(ctx)
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        graphics::set_screen_coordinates(ctx,
            graphics::Rect::new(0.0, 0.0, width, height)).unwrap();
        self.force_redraw = true;
    }
}


fn main() {
    use wave_sickle::helpers;
    wave_sickle::helpers::init_cos_tab();

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
