use serde::Serialize;
use serde::Deserialize;
use std::io::Write;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Interpolation {
    Empty,
    Step,
    Lerp,
    SStep,
    Exp,
}

impl std::default::Default for Interpolation {
    fn default() -> Self { Interpolation::Empty }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum PlayPos {
    Desync,
    End,
    At(usize),
}

impl std::default::Default for PlayPos {
    fn default() -> Self { PlayPos::Desync }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct InterpolationState {
    line_a: usize,
    line_b: usize,
    val_a:  f32,
    val_b:  f32,
    int:    Interpolation,
    desync: bool,
}

impl std::default::Default for InterpolationState {
    fn default() -> Self { InterpolationState::new() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub value: Option<(f32, Interpolation)>,
    pub a: u8,
    pub b: u8,
    pub note: u8,
}

impl Row {
    fn new() -> Self {
        Row {
            value: None,
            a: 0,
            b: 0,
            note: 0,
        }
    }
}

impl InterpolationState {
    fn new() -> Self {
        InterpolationState {
            line_a: 0,
            line_b: 0,
            val_a:  0.0,
            val_b:  0.0,
            int:    Interpolation::Empty,
            desync: true,
        }
    }

    fn clear(&mut self) {
        self.int = Interpolation::Empty;
    }

    fn to_end(&mut self, l: usize, d: &Row, end_line: usize) {
        self.line_a = l;
        self.val_a  = d.value.unwrap_or((0.0, Interpolation::Step)).0;
        self.int    = d.value.unwrap_or((0.0, Interpolation::Step)).1;
        self.line_b = end_line;
        self.val_b  = 0.0;
    }

    fn to_next(&mut self, l: usize, d: &Row, lb: usize, db: &Row) {
        self.line_a = l;
        self.val_a  = d.value.unwrap_or((0.0, Interpolation::Step)).0;
        self.int    = d.value.unwrap_or((0.0, Interpolation::Step)).1;
        self.line_b = lb;
        self.val_b  = db.value.unwrap_or((0.0, Interpolation::Step)).0;
    }

    fn desync(&mut self) {
        self.clear();
        self.desync = true;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    #[serde(skip)]
    play_pos: PlayPos,
    #[serde(skip)]
    interpol: InterpolationState,
    // if index is at or above desired key, interpolate
    // else set index = 0 and restart search for right key
    pub lpp:         usize,
    pub patterns:    Vec<Vec<Row>>,
    pub arrangement: Vec<usize>, // arrangement of the patterns
}

impl Track {
    pub fn new(name: &str, lpp: usize) -> Self {
        let mut fp = Vec::new();
        fp.resize(lpp, Row::new());

        Track {
            name:        String::from(name),
            play_pos:    PlayPos::Desync,
            interpol:    InterpolationState::new(),
            patterns:    vec![fp],
            arrangement: vec![0],
            lpp,
        }
    }

    pub fn read_from_file(filename: &str) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(filename)?;
        Ok(serde_json::from_str(&s).unwrap_or(Track {
            name: String::from("parseerr"),
            play_pos: PlayPos::Desync,
            interpol: InterpolationState::new(),
            patterns: vec![],
            arrangement: vec![],
            lpp: 0,
        }))
    }

    pub fn write_to_file(&self, prefix: &str) -> Result<(), String> {
        let mut f =
            std::fs::File::create(String::from(prefix) + &self.name + "~")
            .map_err(|e| format!("write track to file io create error: {}", e))?;

        match serde_json::to_string(&self) {
            Ok(s) => {
                f.write_all(s.as_bytes())
                 .map_err(|e| format!("write track to file io error: {}", e))
            },
            Err(e) => {
                Err(format!("write track to file serialize error: {}", e))
            }
        }
    }

    pub fn set_arrangement_pattern(&mut self, line: usize, pat_idx: usize) {
        if pat_idx < self.patterns.len() {
            self.arrangement[line / self.lpp] = pat_idx;
        }
    }

    pub fn touch_pattern_idx(&mut self, pat_idx: usize) {
        if pat_idx >= self.patterns.len() {
            let mut fp = Vec::new();
            fp.resize(self.lpp, Row::new());
            self.patterns.resize(pat_idx + 1, fp);
        }
    }

    pub fn line_count(&self) -> usize {
        self.arrangement.len() * self.lpp
    }

    pub fn row(&mut self, line: usize) -> &mut Row {
        &mut self.patterns[self.arrangement[line / self.lpp]][line % self.lpp]
    }

    pub fn prev_row_with_value(&mut self, line: usize) -> Option<(usize, Row)> {
        let mut ll = line;
        while ll > 0 {
            let row = &self.patterns[self.arrangement[(ll - 1) / self.lpp]][(ll - 1) % self.lpp];
            if (*row).value.is_some() {
                return Some(((ll - 1), *row));
            }
            ll -= 1;
        }

        None
    }

    pub fn next_row_with_value(&mut self, line: usize) -> Option<(usize, Row)> {
        let mut ll = line;
        let lc = self.line_count();
        while ll <= lc {
            let row = &self.patterns[self.arrangement[ll / self.lpp]][ll % self.lpp];
            if (*row).value.is_some() {
                return Some((ll, *row));
            }
            ll += 1;
        }

        None
    }

    pub fn touch_row(&mut self, line: usize) -> &mut Row {
        let a = line / self.lpp;
        while a >= self.arrangement.len() {
            self.patterns.push(Vec::new());
            self.patterns[self.patterns.len() - 1].resize(self.lpp, Row::new());
            self.arrangement.push(self.patterns.len() - 1);
        }

        &mut self.patterns[self.arrangement[a]][line % self.lpp]
    }

    pub fn desync(&mut self) {
        self.play_pos = PlayPos::Desync;
        self.interpol.desync();
    }

    pub fn remove_value(&mut self, line: usize) {
        *self.touch_row(line) = Row::new();
        self.desync();
    }

    pub fn set_int(&mut self, line: usize, int: Interpolation) {
        if let Some((v, _i)) = (*self.touch_row(line)).value {
            (*self.touch_row(line)).value = Some((v, int));
        } else {
            (*self.touch_row(line)).value = Some((0.0, int));
        }
        self.desync();
    }

    pub fn set_note(&mut self, line: usize, value: u8) {
        (*self.touch_row(line)).note = value;
        self.desync();
    }

    pub fn set_a(&mut self, line: usize, value: u8) {
        (*self.touch_row(line)).a = value;
        self.desync();
    }

    pub fn set_b(&mut self, line: usize, value: u8) {
        (*self.touch_row(line)).b = value;
        self.desync();
    }

    pub fn set_value(&mut self, line: usize, value: f32) {
        if let Some((_v, i)) = (*self.touch_row(line)).value {
            (*self.touch_row(line)).value = Some((value, i));
        } else {
            (*self.touch_row(line)).value = Some((value, Interpolation::Step));
        }
        self.desync();
    }

    fn sync_interpol_to_play_line(&mut self, line: usize) -> Option<Row> {
        if let Some((l_a, row_a)) = self.next_row_with_value(line) {
            if let Some((l_b, row_b)) = self.prev_row_with_value(line) {
                self.interpol.to_next(l_a, &row_a, l_b, &row_b);
            } else {
                self.interpol.to_end(l_a, &row_a, self.line_count() - 1);
            }

            Some(row_a)
        } else {
            if let Some((l_b, row_b)) = self.prev_row_with_value(line) {
                self.interpol.to_end(l_b, &row_b, self.line_count() - 1);
            } else {
                self.interpol.clear();
            }

            None
        }
    }

    /// Advances the play head to the line. The last line has to be
    /// specified for setting up the interpolations.
    /// Should be called in order of the track events, othewise 
    /// desync() should be called first.
    pub fn play_line(&mut self, line: usize) -> Option<Row> {
        self.sync_interpol_to_play_line(line)
    }

    /// Returns the interpolated value of this track at the specified line.
    /// Only works if the interpolation was
    /// initialized with self.sync_interpol_to_play_line() in self.play_line()!
    pub fn get_value(&mut self, line: usize, fract_next_line: f64) -> f32 {
        let i = &mut self.interpol;

        if line < i.line_a {
            i.clear();
        }

        let mut diff = i.line_b - i.line_a;
        if diff == 0 { diff = 1; }
        let diff = diff as f64;
        let line_f = line as f64 + fract_next_line;

        match i.int {
            Interpolation::Empty => 0.0,
            Interpolation::Step => {
                if line == i.line_b {
                    i.val_b
                } else {
                    i.val_a
                }
            },
            Interpolation::Lerp => {
                let x = (line_f - (i.line_a as f64)) / diff;
                (  i.val_a as f64 * (1.0 - x)
                 + i.val_b as f64 * x)
                as f32
            },
            Interpolation::SStep => {
                let x = (line_f - (i.line_a as f64)) / diff;
                let x = if x < 0.0 { 0.0 } else { x };
                let x = if x > 1.0 { 1.0 } else { x };
                let x = x * x * (3.0 - 2.0 * x);

                (  i.val_a as f64 * (1.0 - x)
                 + i.val_b as f64 * x)
                as f32
            },
            Interpolation::Exp => {
                let x = (line_f - (i.line_a as f64)) / diff;
                let x = x * x;

                (  i.val_a as f64 * (1.0 - x)
                 + i.val_b as f64 * x)
                as f32
            },
        }
    }
}
