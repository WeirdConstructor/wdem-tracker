use std::rc::Rc;
use std::cell::RefCell;

pub struct Tracker {
//    /// beats per minute
//    bpm:            usize,
    /// lines per beat
    lpb:            usize,
    /// ticks per row/line
    tpl:            usize,
    /// number of rows in all tracks
    rows:           usize,
    /// current play head, if -1 it will start with line 0
    play_line:      i32,
    /// number of played ticks
    tick_count:     usize,
    tracks:         Vec<Track>,
}

impl Tracker {
    pub fn new() -> Self {
        Tracker {
            lpb:    4, // => 4 beats are 1 `Tackt`(de)
            tpl:    10,
            rows:   64,
            tracks: Vec::new(),
            play_line: -1,
            tick_count: 0,
        }
    }

    pub fn add_track(&mut self, name: &str, data: Vec<(usize, f32, Interpolation)>) {
        self.tracks.push(Track {
            name: String::from(name),
            next_idx: 0,
            data,
        });
    }

    pub fn tick<T>(&mut self, output: &mut T) {
        self.tick_count += 1;
        if self.play_line == -1 {
            self.play_line = 0;
        }
        let new_play_line = self.tick_count / self.tpl;
        self.play_line = new_play_line as i32;

        if new_play_line > self.play_line as usize {
            for t in self.tracks.iter_mut() {
                // t.check_advance_pos(...)
            }
        }

        for (idx, t) in self.tracks.iter_mut().enumerate() {
            // output.values()[idx] = t.get_value(self.play_line as usize);
        }
    }

    pub fn set_value(&mut self, track_idx: usize, line: usize,
                     value: f32, int: Option<Interpolation>) {
        self.tracks[track_idx].set_value(line, value, int);
    }
}

pub struct TrackerEditor {
    tracker:        Rc<RefCell<Tracker>>,
    cur_track_idx:  usize,
    cur_input_nr:   String,
    cur_line_idx:    usize,
    redraw_flag:    bool,
}

pub enum TrackerInput {
    Escape,
    Enter,
    Character(char),
    SetInterpStep,
    SetInterpLerp,
    SetInterpSStep,
    SetInterpExp,
    RowDown,
    RowUp,
    TrackLeft,
    TrackRight,
}

pub trait TrackerEditorView {
    fn start_drawing(&mut self);
    fn start_track(&mut self, track_idx: usize, name: &str, cursor: bool);
    fn draw_track_cell(
        &mut self,
        line_idx: usize,
        track_idx: usize,
        cursor: bool,
        beat: bool,
        value: Option<f32>,
        interp: Interpolation);
    fn end_track(&mut self);
    fn end_drawing(&mut self);
}

impl TrackerEditor {
    pub fn new(tracker: Rc<RefCell<Tracker>>) -> Self {
        TrackerEditor {
            tracker,
            cur_track_idx: 0,
            cur_input_nr:  String::from(""),
            cur_line_idx:   0,
            redraw_flag:   true,
        }
    }

    pub fn need_redraw(&self) -> bool { self.redraw_flag }

    pub fn show_state<T>(&mut self, max_rows: usize, view: &mut T) where T: TrackerEditorView {
//        if !self.redraw_flag { return; }
//        self.redraw_flag = false;

        let mut cc = 0;
        view.start_drawing();
        for (track_idx, track) in self.tracker.borrow().tracks.iter().enumerate() {
            view.start_track(track_idx, &track.name, self.cur_track_idx == track_idx);

            let mut track_line_pointer = 0;

            let mut rows_shown_count = 0;
            for line_idx in 0..self.tracker.borrow().rows {
                if rows_shown_count > max_rows {
                    break;
                }

                let cursor_is_here =
                        self.cur_line_idx   == line_idx
                     && self.cur_track_idx == track_idx;
                let beat = (line_idx % self.tracker.borrow().lpb) == 0;

                if    track_line_pointer < track.data.len()
                   && track.data[track_line_pointer].0 == line_idx {

                    cc += 1;
                    view.draw_track_cell(
                        line_idx, track_idx, cursor_is_here, beat,
                        Some(track.data[track_line_pointer].1),
                        track.data[track_line_pointer].2);

                    track_line_pointer += 1;
                } else {
                    cc += 1;
                    view.draw_track_cell(
                        line_idx, track_idx, cursor_is_here, beat,
                        None, Interpolation::Empty);
                }

                rows_shown_count += 1;
            }

            view.end_track();
        }
        view.end_drawing();
        self.redraw_flag = false;
    }

    pub fn process_input(&mut self, input: TrackerInput) {
        let mut was_input = false;

        self.redraw_flag = true;

        match input {
            TrackerInput::Escape => {
            },
            TrackerInput::Enter => {
                // parse self.cur_input_nr to float and enter into current row.
            },
            TrackerInput::Character(c) => {
                match c {
                    '0'..='9' => {
                        was_input = true;
                        self.cur_input_nr += &c.to_string();
                    },
                    '.' => {
                        was_input = true;
                        self.cur_input_nr += &c.to_string();
                    },
                    _ => (),
                }
            },
            TrackerInput::RowDown => {
                self.cur_line_idx += 1;
            },
            TrackerInput::RowUp => {
                if self.cur_line_idx > 0 {
                    self.cur_line_idx -= 1;
                }
            },
            TrackerInput::TrackLeft => {
                if self.cur_track_idx > 0 {
                    self.cur_track_idx -= 1;
                }
            },
            TrackerInput::TrackRight => {
                self.cur_track_idx += 1;
            },
            TrackerInput::SetInterpStep => {
                // set interp mode of cur row to *
            },
            TrackerInput::SetInterpLerp => {
                // set interp mode of cur row to *
            },
            TrackerInput::SetInterpSStep => {
                // set interp mode of cur row to *
            },
            TrackerInput::SetInterpExp => {
                // set interp mode of cur row to *
            },
        };

        if self.tracker.borrow().tracks.len() == 0 {
            return;
        }

        if self.cur_track_idx >= self.tracker.borrow().tracks.len() {
            self.cur_track_idx = self.tracker.borrow().tracks.len() - 1;
        }

        if self.cur_line_idx >= self.tracker.borrow().rows {
            self.cur_line_idx = self.tracker.borrow().rows;
        }

        if was_input {
            self.tracker.borrow_mut()
                .set_value(
                    self.cur_track_idx,
                    self.cur_line_idx,
                    self.cur_input_nr.parse::<f32>().unwrap_or(0.0),
                    None);
        } else {
            self.cur_input_nr = String::from("");
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Interpolation {
    Empty,
    Step,
    Lerp,
    SStep,
    Exp,
}

pub struct Track {
    name:     String,
    next_idx: usize,
    // if index is at or above desired key, interpolate
    // else set index = 0 and restart search for right key
    data: Vec<(usize, f32, Interpolation)>,
}

impl Track {
    fn set_value(&mut self, line: usize, value: f32, int: Option<Interpolation>) {
        let entry = self.data.iter().enumerate().find(|v| (v.1).0 >= line);
        if let Some((idx, val)) = entry {
            if val.0 == line {
                if int.is_none() {
                    self.data[idx] = (line, value, val.2);
                } else {
                    self.data[idx] = (line, value, int.unwrap());
                }
            } else {
                self.data.insert(
                    idx, (line, value, int.unwrap_or(Interpolation::Empty)));
            }
        } else {
            self.data.push((line, value, int.unwrap_or(Interpolation::Empty)));
        }
    }

    fn check_advance_pos(&mut self, line: usize) -> Option<(f32, Interpolation)> {
        if self.next_idx > 0 {
            if line == self.data[self.next_idx].0 {
                self.next_idx += 1;
                Some((self.data[0].1, self.data[0].2))
            } else {
                None
            }
        } else {
            if line == self.data[0].0 {
                self.next_idx = 1;
                Some((self.data[0].1, self.data[0].2))
            } else {
                None
            }
        }
    }

    fn get_value(&mut self, line: usize) -> f32 {
        if self.next_idx > 0 {
            let interp     = self.data[self.next_idx - 1].2;
            let last_pos   = self.data[self.next_idx - 1].0;
            let last_value = self.data[self.next_idx - 1].1;
            let next_pos   = self.data[self.next_idx].0;
            let next_value = self.data[self.next_idx].1;

            let diff = next_pos - last_pos;
            let x    = diff as f64 / ((line - last_pos) as f64);

            (  last_value as f64 * (1.0 - x)
             + next_value as f64 * x)
            as f32
        } else {
            0.0
        }
    }
    fn serialize(&self) -> Vec<u8> {
        Vec::new()
    }
    fn deserialize(_data: &[u8]) -> Track {
        Track {
            name:     String::from(""),
            next_idx: 0,
            data:     Vec::new(),
        }
    }
}
