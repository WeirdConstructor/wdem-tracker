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

    fn to_end(&mut self, d: (usize, f32, Interpolation, u16), end_line: usize) {
        self.line_a = d.0;
        self.val_a  = d.1;
        self.int    = d.2;
        self.line_b = end_line;
        self.val_b  = 0.0;
    }

    fn to_next(&mut self, d: (usize, f32, Interpolation, u16), db: (usize, f32, Interpolation, u16)) {
        self.line_a = d.0;
        self.val_a  = d.1;
        self.int    = d.2;
        self.line_b = db.0;
        self.val_b  = db.1;
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
    pub data: Vec<(usize, f32, Interpolation, u16)>,
}

impl Track {
    pub fn new(name: &str, data: Vec<(usize, f32, Interpolation, u16)>) -> Self {
        Track {
            name: String::from(name),
            play_pos: PlayPos::Desync,
            interpol: InterpolationState::new(),
            data,
        }
    }

    pub fn read_from_file(filename: &str) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(filename)?;
        Ok(serde_json::from_str(&s).unwrap_or(Track {
            name: String::from("parseerr"),
            play_pos: PlayPos::Desync,
            interpol: InterpolationState::new(),
            data: Vec::new(),
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

    pub fn desync(&mut self) {
        self.play_pos = PlayPos::Desync;
        self.interpol.desync();
    }

    pub fn remove_value(&mut self, line: usize) {
        let entry = self.data.iter().enumerate().find(|v| (v.1).0 == line);
        if let Some((idx, _val)) = entry {
            self.data.remove(idx);
            self.desync();
        }
    }

    pub fn set_int(&mut self, line: usize, int: Interpolation) {
        let entry = self.data.iter_mut().find(|v| v.0 == line);
        if let Some(val) = entry {
            val.2 = int;
        }

        self.desync();
    }

    pub fn set_a(&mut self, line: usize, value: u8) {
        let entry = self.data.iter().enumerate().find(|v| (v.1).0 >= line);
        if let Some((idx, val)) = entry {
            if val.0 == line {
                self.data[idx] = (line, val.1, val.2, (val.3 & 0xFF00) | (value as u16));
            } else {
                self.data.insert(idx, (line, 0.0, Interpolation::Step, value as u16));
            }
        } else {
            self.data.push((line, 0.0, Interpolation::Step, value as u16));
        }

        self.desync();
    }

    pub fn set_b(&mut self, line: usize, value: u8) {
        let entry = self.data.iter().enumerate().find(|v| (v.1).0 >= line);
        if let Some((idx, val)) = entry {
            if val.0 == line {
                self.data[idx] = (line, val.1, val.2, (val.3 & 0x00FF) | ((value as u16) << 8));
            } else {
                self.data.insert(idx, (line, 0.0, Interpolation::Step, (value as u16) << 8));
            }
        } else {
            self.data.push((line, 0.0, Interpolation::Step, (value as u16) << 8));
        }

        self.desync();
    }

    pub fn set_value(&mut self, line: usize, value: f32) {
        let entry = self.data.iter().enumerate().find(|v| (v.1).0 >= line);
        if let Some((idx, val)) = entry {
            if val.0 == line {
                self.data[idx] = (line, value, val.2, val.3);
            } else {
                self.data.insert(idx, (line, value, Interpolation::Step, 0));
            }
        } else {
            self.data.push((line, value, Interpolation::Step, 0));
        }

        self.desync();
    }

    fn sync_interpol_to_play_line(&mut self, line: usize, end_line: usize) -> Option<(f32, u16)> {
        match self.play_pos {
            PlayPos::End     => {
                if self.data.is_empty() {
                    self.interpol.clear();
                } else {
                    let d = self.data[self.data.len() - 1];
                    self.interpol.to_end(d, end_line);
                }

                None
            },
            PlayPos::At(idx) => {
                let d = self.data[idx];
                if d.0 == line {
                    if (idx + 1) >= self.data.len() {
                        self.interpol.to_end(d, end_line);
                        self.play_pos = PlayPos::End;
                    } else {
                        self.interpol.to_next(d, self.data[idx + 1]);
                        self.play_pos = PlayPos::At(idx + 1);
                    }

                    Some((d.1, d.3))
                } else { // assuming here: d.0 > line
                    if idx == 0 {
                        self.interpol.to_next(
                            (0, 0.0, Interpolation::Step, 0), d);
                    } else {
                        self.interpol.to_next(
                            self.data[idx - 1], d);
                    }

                    None
                }
            },
            _ => None,
        }
    }

    fn check_sync(&mut self, line: usize, _end_line: usize) {
        if self.play_pos == PlayPos::Desync {
            let entry = self.data.iter().enumerate().find(|v| (v.1).0 >= line);
            if let Some((idx, _val)) = entry {
                self.play_pos = PlayPos::At(idx);
            } else {
                self.play_pos = PlayPos::End;
            }
        }
    }

    /// Advances the play head to the line. The last line has to be
    /// specified for setting up the interpolations.
    /// Should be called in order of the track events, othewise 
    /// desync() should be called first.
    pub fn play_line(&mut self, line: usize, end_line: usize) -> Option<(f32, u16)> {
        self.check_sync(line, end_line);
        self.sync_interpol_to_play_line(line, end_line)
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
