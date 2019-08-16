use crate::gui_painter::GUIPainter;

pub const SCOPE_SAMPLES : usize = 128;

#[derive(Debug, PartialEq, Clone)]
pub struct Scope {
    pub samples: Vec<f32>,
    pub points: Vec<[f32; 2]>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SampleRow {
    pub sample_row: Vec<f32>,
    pub pos:        usize,
}

impl SampleRow {
    pub fn new() -> Self {
        SampleRow {
            sample_row: Vec::new(),
            pos: 0,
        }
    }

    pub fn read_from_regs(&mut self, regs: &[f32], pos: usize) {
        if self.sample_row.len() < regs.len() {
            self.sample_row.resize(regs.len(), 0.0);
        }

        self.sample_row.copy_from_slice(regs);
        self.pos = pos;
    }
}

impl Scope {
    fn new(sample_len: usize) -> Self {
        let mut v = Vec::new();
        v.resize(sample_len, 0.0);
        let mut p = Vec::new();
        p.resize(sample_len, [0.0; 2]);
        Scope { samples: v, points: p }
    }

    fn draw<P>(&mut self, painter: &mut P, pos: [f32; 2], size: [f32; 2]) where P: GUIPainter {
        let x_offs : f32 = size[0] / self.samples.len() as f32;
        let mut min : f32 = 999999.0;
        let mut max : f32 = -999999.0;
        for v in self.samples.iter() {
            if min > *v { min = *v; }
            if max < *v { max = *v; }
        }
        let delta = max - min;
        for (v, (i, p)) in self.samples.iter().zip(self.points.iter_mut().enumerate()) {
            p[0] = x_offs * (i as f32);
            p[1] = ((v - min) / delta) * size[1];
        }
        painter.draw_lines(
            [1.0, 1.0, 1.0, 1.0],
            pos,
            &self.points,
            false,
            1.0);
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
        use std::ops::DerefMut;

        std::mem::swap(
            &mut *self.sample_row.lock().unwrap(),
            &mut self.my_sample_row);

        let pos = self.my_sample_row.pos;
        if self.scopes.len() < self.my_sample_row.sample_row.len() {
            self.scopes.resize(self.my_sample_row.sample_row.len(), Scope::new(SCOPE_SAMPLES));
        }

        for (i, s) in self.my_sample_row.sample_row.iter().enumerate() {
            self.scopes[i].samples[pos] = *s;
        }
    }

    pub fn draw_scopes<P>(&mut self, painter: &mut P, pos: [f32; 2]) where P: GUIPainter {
        let scope_width     = 20.0;
        let scope_height    = 10.0;
        let per_row : usize = 6;

        for (i, s) in self.scopes.iter_mut().enumerate() {
            let row_idx = i % per_row;
            let y = scope_height * ((i / per_row) as f32);
            s.draw(
                painter,
                [pos[0] + (row_idx as f32) * scope_width,
                 pos[1] + y],
                [scope_width, scope_height]);
        }
    }
}
