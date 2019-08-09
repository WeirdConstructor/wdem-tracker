use std::rc::Rc;
use std::cell::RefCell;
mod tracker;
use tracker::*;

/*

modules:

Frontend
    number input
    navigation
    cursor management
    rendering
    undo/redo

Piece struct
    provides file save/loading
    track parameter setting (lpb, ticks, ...)
    song length

Track struct
    track name
    track type (note vs. automation)
    provides saving/loading
    provides undo/redo

Backend trait
    feedback of play position
    signal data output


//Track Signal Path GUI
//    basic component drawing
//    component selection/addition/removal
//    bind components to names
//    draw parameter collections
//    parameters implicitly named: <track name>/<component name>/<parameter name>
//
//Component trait
//    name set/get
//    parameter list set/get





*/

use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler, quit};
use ggez::graphics;
use ggez::input::keyboard::{KeyCode, KeyMods};

struct TrackerViewContext {
    
}

struct Painter {
    reg_view_font: graphics::Font,
    cur_reg_line: usize,
    tvc: TrackerViewContext,
    play_pos_row: i32,
    text_cache: std::collections::HashMap<String, graphics::Text>,
}

impl Painter {
    fn draw_rect(&mut self, ctx: &mut Context, color: [f32; 4], pos: [f32; 2], size: [f32; 2], filled: bool, thickness: f32) {
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
            ctx,
            &r,
            ([pos[0], pos[1]],
             0.0,
             [0.0, 0.0],
             graphics::WHITE)).unwrap();
    }

    fn draw_text(&mut self, ctx: &mut Context, color: [f32; 4], pos: [f32; 2], size: f32, text: String) {
        let txt = self.text_cache.get(&text);
        let txt_elem = if let Some(t) = txt {
            t
        } else {
            //d// println!("NEW TEXT {}", text);
            let t = graphics::Text::new((text.clone(), self.reg_view_font, size));
            self.text_cache.insert(text.clone(), t);
            self.text_cache.get(&text).unwrap()
        };

        graphics::queue_text(
            ctx, txt_elem, pos, Some(color.into()));

//        graphics::draw(
//            ctx, txt_elem,
//            (pos, 0.0, [0.0, 0.0], color.into())).unwrap();
    }

    fn finish_draw_text(&mut self, ctx: &mut Context) {
        graphics::draw_queued_text(
            ctx,
            graphics::DrawParam::default(),
            None,
            graphics::FilterMode::Linear).unwrap();
    }
}

const TPOS_PAD      : f32 = 50.0;
const TRACK_WIDTH   : f32 = 46.0;
const TRACK_PAD     : f32 =  0.0;
const TRACK_VAL_PAD : f32 =  4.0;
const ROW_HEIGHT    : f32 = 18.0;

impl TrackerEditorView<Context> for Painter {
    fn start_drawing(&mut self, _ctx: &mut Context) {
    }

    fn end_track(&mut self, _ctx: &mut Context) {
    }

    fn start_track(&mut self, ctx: &mut Context, track_idx: usize, _name: &str, cursor: bool) {
        let mut clr = [0.8, 0.8, 0.8, 1.0];
        if cursor {
            clr = [1.0, 0.7, 0.7, 1.0];
        }

        self.draw_rect(
            ctx,
            clr,
            [TPOS_PAD + track_idx as f32 * (TRACK_WIDTH + TRACK_PAD), 0.0],
            [TRACK_WIDTH, (10.0 + 1.0) * ROW_HEIGHT],
            false,
            0.5);
    }

    fn draw_track_cell(&mut self, ctx: &mut Context,
        scroll_offs: usize,
        line_idx: usize,
        track_idx: usize,
        cursor: bool,
        beat: bool,
        value: Option<f32>,
        _interp: Interpolation) {

        let s = if let Some(v) = value {
            format!("{:>03.2}", v)
        } else {
            String::from(" ---")
        };

        let txt_x =
            TRACK_VAL_PAD
            + TPOS_PAD
            + track_idx as f32 * (TRACK_WIDTH + TRACK_PAD);

        let txt_y = (line_idx - scroll_offs) as f32 * ROW_HEIGHT;

        if track_idx == 0 {
            if line_idx as i32 == self.play_pos_row {
                self.draw_rect(
                    ctx,
                    [0.4, 0.0, 0.0, 1.0],
                    [0.0, txt_y],
                    [800.0, ROW_HEIGHT],
                    true,
                    0.0);
            }

            self.draw_text(
                ctx,
                if beat { [0.5, 8.0, 0.5, 1.0] }
                else { [0.6, 0.6, 0.6, 1.0] },
                [TRACK_PAD / 2.0, txt_y],
                ROW_HEIGHT * 0.6,
                format!("[{:0>4}]", line_idx));
        }

        if cursor {
            self.draw_rect(
                ctx,
                [0.4, 0.8, 0.4, 1.0],
                [txt_x - TRACK_VAL_PAD + 1.0, txt_y + 1.0],
                [TRACK_WIDTH - 2.0, ROW_HEIGHT - 2.0],
                true,
                0.5);

            self.draw_text(
                ctx,
                [0.0, 0.0, 0.0, 1.0],
                [txt_x, txt_y],
                ROW_HEIGHT * 0.9,
                s);
        } else {
            if beat {
                self.draw_text(
                    ctx,
                    [0.6, 1.0, 0.6, 1.0],
                    [txt_x, txt_y],
                    ROW_HEIGHT * 0.9,
                    s);
            } else {
                self.draw_text(
                    ctx,
                    [0.8, 0.8, 0.8, 1.0],
                    [txt_x, txt_y],
                    ROW_HEIGHT * 0.9,
                    s);
            }
        }
    }

    fn end_drawing(&mut self, ctx: &mut Context) {
    }
}

//impl<'a> signals::RegisterView for Painter<'a> {
//    fn start_print_registers(&mut self) {
//        self.cur_reg_line = 0;
//    }
//
//    fn print_register(&mut self, name: &str, value: f32) {
//        let sz = graphics::drawable_size(self.ctx);
//        let font_size = 20.0;
//        self.draw_text(
//            [-(sz.0 / 2.0),
//             -(sz.1 / 2.0)
//             + self.cur_reg_line as f32 * (font_size + 1.0)],
//            font_size,
//            format!("{:<10} = {}", name, value));
//        self.cur_reg_line += 1;
//    }
//
//    fn end_print_registers(&mut self) {
//    }
//}

struct ThreadTrackSync {
}

impl ThreadTrackSync {
    fn new() -> Self {
        ThreadTrackSync { }
    }
}

impl tracker::TrackerSync for ThreadTrackSync {
    fn add_track(&mut self, t: Track) {
    }
    fn set_value(&mut self, track_idx: usize, line: usize,
                     value: f32, int: Option<Interpolation>) {
    }
    fn remove_value(&mut self, track_idx: usize, line: usize) {
    }
}

struct OutputValues {
    values: Vec<f32>,
}

struct WDemTrackerGUI {
    tracker: Rc<RefCell<Tracker<ThreadTrackSync>>>,
    editor:  TrackerEditor<ThreadTrackSync>,
    painter: Rc<RefCell<Painter>>,
    force_redraw: bool,
    i: i32,
}

impl WDemTrackerGUI {
    pub fn new(ctx: &mut Context) -> WDemTrackerGUI {
        let sync = ThreadTrackSync::new();
        let font = graphics::Font::new(ctx, "/DejaVuSansMono.ttf").unwrap();
        let trk = Rc::new(RefCell::new(Tracker::new(sync)));
        WDemTrackerGUI {
            tracker: trk.clone(),
            editor: TrackerEditor::new(trk),
            painter: Rc::new(RefCell::new(Painter {
                text_cache: std::collections::HashMap::new(),
                reg_view_font: font,
                cur_reg_line: 0,
                tvc: TrackerViewContext { },
                play_pos_row: 0,
            })),
            force_redraw: true,
            i: 0,
        }
    }

    pub fn init(&mut self) {
        for i in 0..20 {
            self.tracker.borrow_mut().add_track(
                &format!("xxx{}", i),
                vec![
                    (0, 1.0, Interpolation::Lerp),
                    (4, 4.0, Interpolation::Lerp),
                    (5, 0.2, Interpolation::Lerp),
                ]);
        }
    }
}

impl tracker::OutputHandler for OutputValues {
    fn emit_event(&mut self, track_idx: usize, val: f32) {
        println!("EMIT: {}: {}", track_idx, val);
    }

    fn emit_play_line(&mut self, play_line: i32) {
        println!("EMIT PP: {}", play_line);
    }

    fn value_buffer(&mut self) -> &mut Vec<f32> {
        return &mut self.values;
    }
}

impl EventHandler for WDemTrackerGUI {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
        if keycode == KeyCode::Q {
            quit(ctx);
        } else {
            match keycode {
                KeyCode::X => {
                    self.editor.process_input(TrackerInput::Delete);
                },
                KeyCode::H => {
                    self.editor.process_input(TrackerInput::TrackLeft);
                },
                KeyCode::J => {
                    self.editor.process_input(TrackerInput::RowDown);
                },
                KeyCode::K => {
                    self.editor.process_input(TrackerInput::RowUp);
                },
                KeyCode::L => {
                    self.editor.process_input(TrackerInput::TrackRight);
                },
                _ => {
                    //d// println!("KEY: {:?}", keycode);
                }
            }
        }
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) {
        //d// println!("CHR: {:?}", character);
        self.editor.process_input(TrackerInput::Character(character));
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

        let mut ov = OutputValues { values: Vec::new() };

        self.editor.tracker.borrow_mut().tick(&mut ov);
//        if !ov.values.is_empty() {
//            println!("OUT: {:?}", ov.values[0]);
//        }

        self.force_redraw = true;
        if self.force_redraw || self.editor.need_redraw() {
            graphics::clear(ctx, graphics::BLACK);
            self.painter.borrow_mut().play_pos_row = self.editor.tracker.borrow().play_line;
            self.force_redraw = false;
            self.editor.show_state(2, 40, &mut *self.painter.borrow_mut(), ctx);
            self.painter.borrow_mut().finish_draw_text(ctx);
        }
//        let scale_size = 300.0;
//        {
//            let mut p = Painter { ctx, cur_reg_line: 0, reg_view_font: &self.debug_font };
//            self.wlctx.one_step(now_time as i64, scale_size, &mut p);
//            self.wlctx.show_debug_registers(&mut p);
//        }

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
