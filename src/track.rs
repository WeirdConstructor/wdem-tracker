use serde::Serialize;
use serde::Deserialize;
use std::io::Write;
use crate::gui_painter::GUIPainter;

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

    pub fn draw<P>(&self, p: &mut P, state: &mut GUIState, line: usize) where P: GUIPainter {
        let val_s =
            if let Some((val, int)) = self.value {
                format!("{:>6.2}{}",
                    val,
                    match int {
                        Interpolation::Empty => "e",
                        Interpolation::Step  => "_",
                        Interpolation::Lerp  => "/",
                        Interpolation::SStep => "~",
                        Interpolation::Exp   => "^",
                    })
            } else {
                String::from("------ ")
            };

        let note_s = match self.note {
            0 => String::from("---"),
            1 => String::from("off"),
            n => format!("{:<4}", note2name(n)),
        };

        let s =
            if state.track_index == 0 {
                format!("{:<05} |{:<02}|{:<4}{:>7}|{:02X} {:02X}|",
                        line,
                        state.pattern_index,
                        note_s, val_s, self.a, self.b)
            } else {
                format!("|{:<02}|{:<4}{:>7}|{:02X} {:02X}|",
                        state.pattern_index,
                        note_s, val_s, self.a, self.b)
            };

        let color =
            if state.cursor_on_line && state.play_on_line {
                [0.8, 0.8, 0.4, 1.0]
            } else if state.play_on_line {
                [0.8, 0.4, 0.4, 1.0]
            } else if state.cursor_on_line {
                [0.4, 0.8, 0.4, 1.0]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            };

        let txt_color =
            if state.cursor_on_line || state.play_on_line {
                if state.on_beat { [0.0, 0.4, 0.0, 1.0] }
                else             { [0.0, 0.0, 0.0, 1.0] }
            } else {
                if state.on_beat { [0.6, 1.0, 0.6, 1.0] }
                else             { [0.8, 0.8, 0.8, 1.0] }
            };

        let width =
            if state.track_index == 0 {
                FIRST_TRACK_WIDTH
            } else {
                TRACK_WIDTH
            };
        p.draw_rect(
            color,
            [0.0, 0.0],
            [width, ROW_HEIGHT], true, 0.5);
        p.draw_text(txt_color, [0.0, 0.0], ROW_HEIGHT * 0.9, s);
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

const NOTE_NAMES : &'static [&str] = &["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

fn note2name(note: u8) -> String {
    if note == 0 { return String::from(""); }

    let octave   : i32   = (note / 12) as i32 - 1;
    let name_idx : usize = (note % 12) as usize;
    format!("{}{}", NOTE_NAMES[name_idx], octave)
}

pub const TPOS_PAD      : f32 = 50.0;
pub const TRACK_PAD     : f32 =  0.0;
pub const TRACK_WIDTH   : f32 = 160.0;
pub const FIRST_TRACK_WIDTH : f32 = TRACK_WIDTH + 40.0;
pub const ROW_HEIGHT    : f32 = 15.0;
pub const ROW_COMPR_FACT : f32 = 0.8;
pub const CONTEXT_LINES : usize = 6;

pub struct GUIState {
    pub cursor_track_idx:   usize,
    pub cursor_line:        usize,
    pub play_line:          i32,
    pub cursor_on_track:    bool,
    pub track_index:        usize,
    pub pattern_index:      usize,
    pub play_on_line:       bool,
    pub on_beat:            bool,
    pub lpb:                usize,
    pub cursor_on_line:     bool,
    pub scroll_offs:        usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
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
            interpol:    InterpolationState::new(),
            patterns:    vec![fp],
            arrangement: vec![0],
            lpp,
        }
    }

    pub fn draw<P>(&self, p: &mut P, state: &mut GUIState) where P: GUIPainter {
        let rows = (p.get_area_size().1 / (ROW_HEIGHT * ROW_COMPR_FACT)) as usize;

        let lines = self.line_count();
        let offs =
            if rows > 2 * CONTEXT_LINES {
                if state.cursor_line < (state.scroll_offs + CONTEXT_LINES) {
                    let mut cl : i32 = state.cursor_line as i32;
                    cl -= CONTEXT_LINES as i32;
                    if cl < 0 { cl = 0; }
                    cl as usize
                } else if state.cursor_line > (state.scroll_offs + (rows - CONTEXT_LINES)) {
                    let mut cl = state.cursor_line as i32;
                    cl -= (rows - CONTEXT_LINES) as i32;
                    if cl < 0 { cl = 0; }
                    cl as usize
                } else {
                    state.scroll_offs
                }
            } else {
                if state.cursor_line >= (rows / 2) {
                    state.cursor_line - (rows / 2)
                } else {
                    0
                }
            };

        state.scroll_offs = offs;

        let from = offs;
        let to   = if from + rows > lines { lines } else { from + rows };

        // rows is the nums of displayable lines
        // the window of viewed rows should be stable and only scroll if
        // the cursor is close to the edge of an window.

        let o = p.get_offs();

        p.draw_text(
            [1.0, 1.0, 1.0, 1.0],
            [0.0, 0.2 * ROW_HEIGHT],
            0.8 * ROW_HEIGHT,
            self.name.clone());
        p.add_offs(0.0, ROW_HEIGHT);

        for l in from..to {
            state.play_on_line   = state.play_line == l as i32;
            state.cursor_on_line = state.cursor_on_track && state.cursor_line == l;
            state.on_beat        = (l % state.lpb) == 0;

            if let Some((pat_idx, row)) = self.row_checked(l) {
                state.pattern_index = pat_idx;
                row.draw(p, state, l);
            }

            p.add_offs(0.0, ROW_HEIGHT * ROW_COMPR_FACT);
        }

        p.set_offs(o);
    }

    pub fn read_from_file(filename: &str) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(filename)?;
        Ok(serde_json::from_str(&s).unwrap_or(Track {
            name: String::from("parseerr"),
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
            let arr_idx = line / self.lpp;
            while arr_idx >= self.arrangement.len() {
                self.arrangement.push(pat_idx)
            }
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

    pub fn row_checked(&self, line: usize) -> Option<(usize, Row)> {
        if line >= self.line_count() { return None; }
        Some((
            self.arrangement[line / self.lpp],
            self.patterns[self.arrangement[line / self.lpp]][line % self.lpp].clone()
        ))
    }

    pub fn row(&mut self, line: usize) -> &mut Row {
        &mut self.patterns[self.arrangement[line / self.lpp]][line % self.lpp]
    }

    pub fn prev_row_with_value(&mut self, line: usize) -> Option<(usize, Row)> {
        let mut ll = line;
        while ll > 0 {
            let row = &self.patterns[self.arrangement[(ll - 1) / self.lpp]][(ll - 1) % self.lpp];
            if (*row).value.is_some() {
                return Some(((ll - 1), row.clone()));
            }
            ll -= 1;
        }

        None
    }

    pub fn next_row_with_value(&mut self, line: usize) -> Option<(usize, Row)> {
        let mut ll = line;
        let lc = self.line_count();
        while ll < lc {
            let pat_idx = self.arrangement[ll / self.lpp];
            let row = &self.patterns[pat_idx][ll % self.lpp];
            if (*row).value.is_some() {
                return Some((ll, row.clone()));
            }
            ll += 1;
        }

        None
    }

    pub fn touch_row(&mut self, line: usize) -> &mut Row {
        let a = line / self.lpp;
        while a >= self.arrangement.len() {
            self.patterns.push(Vec::new());
            let last_idx = self.patterns.len() - 1;
            self.patterns[last_idx].resize(self.lpp, Row::new());
            self.arrangement.push(self.patterns.len() - 1);
        }

        &mut self.patterns[self.arrangement[a]][line % self.lpp]
    }

    pub fn desync(&mut self) {
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
                self.interpol.to_next(l_b, &row_b, l_a, &row_a);
            } else {
                self.interpol.to_end(l_a, &row_a, self.line_count() - 1);
            }

            if l_a == line { Some(row_a) }
            else { None }
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
