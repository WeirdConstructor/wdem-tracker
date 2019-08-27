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


