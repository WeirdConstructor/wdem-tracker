use std::rc::Rc;
use std::cell::RefCell;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::graphics;
use crate::gui_painter;

pub struct GGEZPainter {
    pub reg_view_font: graphics::Font,
    pub text_cache: std::collections::HashMap<(usize, String), graphics::Text>,
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

pub struct GGEZGUIPainter<'b> {
    pub p: Rc<RefCell<GGEZPainter>>,
    pub c: &'b mut ggez::Context,
    pub offs: (f32, f32),
    pub area: (f32, f32),
}

impl<'b> gui_painter::GUIPainter for GGEZGUIPainter<'b> {
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

