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

struct Painter<'a> {
    ctx: &'a mut Context,
    reg_view_font: &'a graphics::Font,
    cur_reg_line: usize,
    tvc: TrackerViewContext,
}

impl<'a> Painter<'a> {
    fn draw_rect(&mut self, color: [f32; 4], pos: [f32; 2], size: [f32; 2], filled: bool, thickness: f32) {
        let r =
            graphics::Mesh::new_rectangle(
                self.ctx,
                if filled {
                    graphics::DrawMode::fill()
                } else {
                    graphics::DrawMode::stroke(thickness)
                },
                graphics::Rect::new(0.0, 0.0, size[0], size[1]),
                graphics::Color::from(color)).unwrap();
        graphics::draw(
            self.ctx,
            &r,
            ([pos[0], pos[1]],
             0.0,
             [0.0, 0.0],
             graphics::WHITE)).unwrap();
    }

    fn draw_text(&mut self, pos: [f32; 2], size: f32, text: String) {
        let txt =
            graphics::Text::new((text, *self.reg_view_font, size));
        graphics::draw(
            self.ctx, &txt,
            (pos, 0.0, [0.0, 0.0], graphics::WHITE)).unwrap();
    }
}

const TRACK_WIDTH: f32 = 30.0;
const TRACK_PAD: f32 = 4.0;
const ROW_HEIGHT: f32 = 20.0;

impl<'a> TrackerEditorView for Painter<'a> {
    fn start_drawing(&mut self) {
    }

    fn end_track(&mut self) {
    }

    fn start_track(&mut self, track_idx: usize, name: &str, cursor: bool) {
        let mut clr = [0.8, 0.8, 0.8, 1.0];
        if cursor {
            clr = [1.0, 0.7, 0.7, 1.0];
        }

        self.draw_rect(
            clr,
            [track_idx as f32 * (TRACK_WIDTH + TRACK_PAD), 0.0],
            [TRACK_WIDTH, (10.0 + 1.0) * ROW_HEIGHT],
            false,
            0.5);
    }

    fn draw_track_cell(&mut self,
        row_idx: usize,
        track_idx: usize,
        cursor: bool,
        value: Option<f32>,
        interp: Interpolation) {

        let s = if let Some(v) = value {
            format!("{}", v)
        } else {
            String::from("---")
        };

        self.draw_text(
            [(track_idx as f32 * (TRACK_WIDTH + TRACK_PAD))
             + TRACK_PAD / 2.0,
             row_idx   as f32 * ROW_HEIGHT],
            ROW_HEIGHT * 0.7,
            s);
    }

    fn end_drawing(&mut self) {
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

struct WDemTrackerGUI {
    font:    graphics::Font,
    tracker: Rc<RefCell<Tracker>>,
    editor:  TrackerEditor,
    i: i32,
}

impl WDemTrackerGUI {
    pub fn new(ctx: &mut Context) -> WDemTrackerGUI {
        let font = graphics::Font::new(ctx, "/DejaVuSansMono.ttf").unwrap();
        let trk = Rc::new(RefCell::new(Tracker::new()));
        WDemTrackerGUI {
            font,
            tracker: trk.clone(),
            editor: TrackerEditor::new(trk),
            i: 0,
        }
    }

    pub fn init(&mut self) {
        for i in 0..20 {
            self.tracker.borrow_mut().add_track(
                &format!("xxx{}", i),
                vec![
                    (0, 1.0, Interpolation::Step),
                    (4, 4.0, Interpolation::Step),
                    (5, 0.2, Interpolation::Lerp),
                ]);
        }
    }
}

fn to_tracker_editor_input(keycode: KeyCode) -> Option<TrackerInput> {
    match keycode {
        KeyCode::Escape => Some(TrackerInput::Escape),
//        KeyCode::Key0   => Some(TrackerInput::Digit(0)),
//        KeyCode::Key1   => Some(TrackerInput::Digit(1)),
//        KeyCode::Key2   => Some(TrackerInput::Digit(2)),
//        KeyCode::Key3   => Some(TrackerInput::Digit(3)),
//        KeyCode::Key4   => Some(TrackerInput::Digit(4)),
//        KeyCode::Key5   => Some(TrackerInput::Digit(5)),
//        KeyCode::Key6   => Some(TrackerInput::Digit(6)),
//        KeyCode::Key7   => Some(TrackerInput::Digit(7)),
//        KeyCode::Key8   => Some(TrackerInput::Digit(8)),
//        KeyCode::Key9   => Some(TrackerInput::Digit(9)),
//        KeyCode::
        _ => None,
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
            println!("KEY: {:?}", keycode);
        }
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) {
        println!("CHR: {:?}", character);
        self.editor.process_input(TrackerInput::Character(character));
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);

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

        let now_time = ggez::timer::time_since_start(ctx).as_millis();

        let mut p = Painter {
            ctx,
            reg_view_font: &self.font,
            cur_reg_line: 0,
            tvc: TrackerViewContext {
            },
        };

        self.editor.show_state(10, &mut p);
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
