use crate::gui_painter::*;

pub const SCOPE_SAMPLES : usize = 128;
const SCOPE_FONT_HEIGHT : f32 = 13.0;

#[derive(Debug, PartialEq, Clone)]
pub struct Scope {
    pub samples: Vec<f32>,
        recent_value: f32,
    pub points: Vec<[f32; 2]>,
        min:    f32,
        max:    f32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SampleRow {
    pub sample_row: Vec<f32>,
    pub pos:        usize,
    pub updated:    bool,
}

impl SampleRow {
    pub fn new() -> Self {
        SampleRow {
            sample_row: Vec::new(),
            pos: 0,
            updated: false,
        }
    }

    pub fn read_from_regs(&mut self, regs: &[f32], pos: usize) {
        if self.sample_row.len() < regs.len() {
            self.sample_row.resize(regs.len(), 0.0);
        }

        self.sample_row.copy_from_slice(regs);
        self.pos = pos;
        self.updated = true;
    }
}

impl Scope {
    fn new(sample_len: usize) -> Self {
        let mut v = Vec::new();
        v.resize(sample_len, 0.0);
        let mut p = Vec::new();
        p.resize(sample_len, [0.0; 2]);
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
        painter.draw_lines(
            [1.0, 1.0, 1.0, 1.0],
            pos,
            &self.points,
            false,
            0.5);
        painter.draw_text(
            [1.0, 0.0, 1.0, 1.0],
            [pos[0], pos[1] + size[1]],
            SCOPE_FONT_HEIGHT,
            format!("r{} {:0.2}", idx, self.recent_value));

    }
}

pub struct Scopes {
    pub scopes:     Vec<Scope>,
    pub sample_row: std::sync::Arc<std::sync::Mutex<SampleRow>>,
        my_sample_row: SampleRow,
}

impl Scopes {
    pub fn new() -> Self {
        use std::sync::Arc;
        use std::sync::Mutex;

        Scopes {
            scopes: Vec::new(),
            sample_row: Arc::new(Mutex::new(SampleRow::new())),
            my_sample_row: SampleRow::new(),
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
            self.scopes.resize(len, Scope::new(SCOPE_SAMPLES));
        }

        for (i, s) in self.my_sample_row.sample_row.iter().enumerate() {
            self.scopes[i].recent_value = *s;
//            println!("RECENT {}", *s);

            let mut j = old_pos;
            while j != pos {
                j = (j + 1) % SCOPE_SAMPLES;
                self.scopes[i].samples[j]   = *s;
            }
        }
    }

    pub fn draw_scopes<P>(&mut self, p: &mut P) where P: GUIPainter {
        let scope_width  = 128.0;
        let scope_height = 48.0;
        let font_height  = SCOPE_FONT_HEIGHT;
        let elem_height  = scope_height + SCOPE_FONT_HEIGHT;
        let per_row      = (p.get_area_size().0 / scope_width) as usize;
        let max_rows     = (p.get_area_size().1 / elem_height) as usize;

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
