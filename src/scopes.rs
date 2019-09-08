use wctr_signal_ops::sample_row::SampleRow;
use crate::gui_painter::*;

pub const SCOPE_SAMPLES : usize = 128;
pub const SCOPE_WIDTH   : f32   = 128.0;
pub const SCOPE_HEIGHT  : f32   = 48.0;
const SCOPE_FONT_HEIGHT : f32 = 13.0;

#[derive(Debug, PartialEq, Clone)]
pub struct Scope {
    pub samples: Vec<f32>,
        recent_value: f32,
    pub points: Vec<[f32; 2]>,
        min:    f32,
        max:    f32,
}

impl Scope {
    fn new(sample_count: usize) -> Self {
        let mut v = Vec::new();
        v.resize(sample_count, 0.0);
        let mut p = Vec::new();
        p.resize(sample_count, [0.0; 2]);
        Scope {
            samples:      v,
            points:       p,
            min:          99999.0,
            max:          -99999.0,
            recent_value: 0.0,
        }
    }

    fn draw<P>(&mut self, painter: &mut P, idx: usize, pos: [f32; 2], size: [f32; 2]) where P: GUIPainter {
        let x_offs : f32 = size[0] / self.samples.len() as f32;

        let mut diff = self.max - self.min;
        if diff <= std::f32::EPSILON {
            diff = 1.0;
        }

        for (v, (i, p)) in self.samples.iter().zip(self.points.iter_mut().enumerate()) {
            p[0] = x_offs * (i as f32);
            p[1] = size[1] - (((v - self.min) / diff) * size[1]);

            if self.min > *v { self.min = *v; }
            if self.max < *v { self.max = *v; }
        }

        painter.draw_lines(
            [0.0, 1.0, 0.0, 1.0],
            pos,
            &[[0.0, 0.0],[size[0], 0.0]],
            false,
            0.5);
        painter.draw_lines(
            [0.0, 1.0, 0.0, 1.0],
            pos,
            &[[0.0, size[1]],[size[0], size[1]]],
            false,
            0.5);
        painter.draw_lines(
            [0.0, 1.0, 0.0, 1.0],
            pos,
            &[[size[0], 0.0],[size[0], size[1]]],
            false,
            0.5);
        if !self.points.is_empty() {
            painter.draw_lines(
                [1.0, 1.0, 1.0, 1.0],
                pos,
                &self.points,
                false,
                0.5);
        }
        painter.draw_text(
            [1.0, 0.0, 1.0, 1.0],
            [pos[0], pos[1] + size[1]],
            SCOPE_FONT_HEIGHT,
            format!("r{} {:0.2}", idx, self.recent_value));
    }
}

pub struct Scopes {
    pub sample_count: usize,
    pub scopes:     Vec<Scope>,
    pub sample_row: std::sync::Arc<std::sync::Mutex<SampleRow>>,
        my_sample_row: SampleRow,
}

impl Scopes {
    pub fn new(sample_count: usize) -> Self {
        use std::sync::Arc;
        use std::sync::Mutex;

        Scopes {
            sample_count,
            scopes: Vec::new(),
            sample_row: Arc::new(Mutex::new(SampleRow::new())),
            my_sample_row: SampleRow::new(),
        }
    }

    pub fn update_from_audio_bufs(&mut self, bufs: &Vec<Vec<f32>>) {
        if bufs.len() != self.scopes.len() {
            self.scopes.resize(
                bufs.len() * 2,
                Scope::new(self.sample_count));
        }

        for (i, ab) in bufs.iter().enumerate() {
            for channel in 0..2 {
                let s : &mut Scope = &mut self.scopes[(i * 2) + channel];
                if s.samples.len() != ab.len() {
                    self.sample_count = ab.len();
                    s.samples.resize(ab.len() / 2, 0.0);
                    s.points.resize(ab.len() / 2, [0.0; 2]);
                }

                for (j, s) in s.samples.iter_mut().enumerate() {
                    *s = ab[(j * 2) + channel];
                }
            }
        }
    }

    pub fn update_from_sample_row(&mut self) {
//        use std::ops::DerefMut;

        if !self.sample_row.lock().unwrap().updated {
            return;
        }

        let old_pos = self.my_sample_row.pos;

        std::mem::swap(
            &mut *self.sample_row.lock().unwrap(),
            &mut self.my_sample_row);

        self.my_sample_row.updated = false;

        let len = self.my_sample_row.sample_row.len();

        let pos = self.my_sample_row.pos;
        if self.scopes.len() < len {
            self.scopes.resize(len, Scope::new(self.sample_count));
        }

        for (i, s) in self.my_sample_row.sample_row.iter().enumerate() {
            self.scopes[i].recent_value = *s;
//            println!("RECENT {}", *s);

            let mut j = old_pos;
            while j != pos {
                j = (j + 1) % self.sample_count;
                self.scopes[i].samples[j]   = *s;
            }
        }
    }

    pub fn draw_scopes<P>(&mut self, p: &mut P) where P: GUIPainter {
        let scope_width  = SCOPE_WIDTH;
        let scope_height = SCOPE_HEIGHT;
        let font_height  = SCOPE_FONT_HEIGHT;
        let elem_height  = scope_height + SCOPE_FONT_HEIGHT;
        let per_row      = (p.get_area_size().0 / scope_width).ceil() as usize;
        let max_rows     = (p.get_area_size().1 / elem_height).ceil() as usize;

        if per_row <= 0 { return; }

        let s = p.get_area_size();

        p.draw_rect(
            [0.3, 0.1, 0.1, 1.0],
            [0.0, 0.0],
            [s.0, s.1],
            true, 1.0);

        for (i, s) in self.scopes.iter_mut().enumerate() {
            let row_idx = i % per_row;
            if (1 + (i / per_row)) >= max_rows { break; }
            let y = (scope_height + font_height) * ((i / per_row) as f32);
            s.draw(
                p,
                i,
                [(row_idx as f32) * scope_width, y],
                [scope_width, scope_height]);
        }
    }
}
