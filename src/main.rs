/*

3 modules:

Frontend
    number input
    navigation
    cursor management
    rendering

Backend
    track data holding
    audio engine

Signal trait

Backend passes time and control signals
to a signal trait after each tick (or a number
of ticks - performance).





*/

use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::graphics;

struct Painter<'a> {
    ctx: &'a mut Context,
    reg_view_font: &'a graphics::Font,
    cur_reg_line: usize,
}

impl<'a> Painter<'a> {
    fn draw_rect(&mut self, color: [f32; 4], rot: ShapeRotation, pos: [f32; 2], size: [f32; 2], filled: bool, thickness: f32) {
        let rot = match rot {
            ShapeRotation::Center(a) => a,
            _ => 0.0,
        };
        let r =
            graphics::Mesh::new_rectangle(
                self.ctx,
                if filled {
                    graphics::DrawMode::fill()
                } else {
                    graphics::DrawMode::stroke(thickness)
                },
                graphics::Rect::new(-size[0] / 2.0, -size[1] / 2.0, size[0], size[1]),
                graphics::Color::from(color)).unwrap();
        graphics::draw(
            self.ctx,
            &r,
            ([pos[0], pos[1]],
             rot,
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
    font: graphics::Font,
}

impl WCtrDemEngine {
    pub fn new(ctx: &mut Context) -> WCtrDemEngine {
        wlctx.init();
        wlctx.load_script("in.wl");
        let font = graphics::Font::new(ctx, "/DejaVuSansMono.ttf").unwrap();
        WCtrDemEngine {
            wlctx,
            i: 0,
            debug_font: font,
        }
    }
}

impl EventHandler for WCtrDemEngine {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);

        self.i += 1;
        if self.i > 100 {
            println!("FPS: {}", ggez::timer::fps(ctx));
            self.i = 0;
        }

        let sz = graphics::drawable_size(ctx);
        let param =
            graphics::DrawParam::from(
                ([sz.0 / 2.0, sz.1 / 2.0],));
        graphics::push_transform(ctx, Some(param.to_matrix()));
        graphics::apply_transformations(ctx)?;

        let now_time = ggez::timer::time_since_start(ctx).as_millis();
        let scale_size = 300.0;
        {
            let mut p = Painter { ctx, cur_reg_line: 0, reg_view_font: &self.debug_font };
            self.wlctx.one_step(now_time as i64, scale_size, &mut p);
            self.wlctx.show_debug_registers(&mut p);
        }

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
       ContextBuilder::new("wctr_dem_engine", "Weird Constructor")
            .window_setup(ggez::conf::WindowSetup {
                title: "wctr_dem_engine".to_owned(),
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

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object
    // so it can load resources like images during setup.
    let mut engine = WCtrDemEngine::new(&mut ctx);

    // Run!
    match event::run(&mut ctx, &mut event_loop, &mut engine) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e)
    }
}

//pub fn main() -> Result<(), String> {
//    let opengl = OpenGL::V3_2;
//    let mut window: PistonWindow =
//        WindowSettings::new("Hello Piston!", [640, 480])
//        .graphics_api(opengl)
//        .resizable(true)
//        .vsync(true)
//        .exit_on_esc(true).build().unwrap();
//
//    let mut cnt = 0;
//    let mut avg = 0;
//
//    let mut wlctx = WLambdaCtx::new();
//    wlctx.init();
//    wlctx.load_script("in.wl");
//
//    let start_time = Instant::now();
//    while let Some(event) = window.next() {
//        let ws = window.draw_size();
//
//        window.draw_2d(&event, |mut context, graphics, _device| {
//            clear([0.1; 4], graphics);
//
//            let scale_size = 200.0 as f32;
//
//            context.transform =
//                context.transform.trans(
//                    ws.width / 2.0 as f64,
//                    ws.height / 2.0 as f64);
//
//            let mut p = Painter {
//                ctx: &context,
//                g: graphics,
//            };
//
//            let b = Instant::now();
//
//            let now_time = start_time.elapsed().as_millis();
//
//            wlctx.one_step(now_time as i64, scale_size, &mut p);
//
//            avg += b.elapsed().as_millis();
//            cnt += 1;
//            if cnt > 100 {
//                println!("exec took {}", avg / cnt);
//                cnt = 0;
//                avg = 0;
//            }
//
//        });
//    }
//
//    Ok(())
//}
