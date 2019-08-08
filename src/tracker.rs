use std::rc::Rc;
use std::cell::RefCell;

pub struct Tracker {
    /// beats per minute
    bpm:            usize,
    /// rows per beat
    rpb:            usize,
    /// number of rows in all tracks
    rows:           usize,
    tracks:         Vec<Track>,
}

impl Tracker {
    pub fn new() -> Self {
        Tracker {
            bpm:    144,
            rpb:    1, // => 4 beats are 1 `Tackt`(de)
            rows:   64,
            tracks: Vec::new(),
        }
    }

    pub fn add_track(&mut self, name: &str, data: Vec<(usize, f32, Interpolation)>) {
        self.tracks.push(Track {
            name: String::from(name),
            last_idx: 0,
            data,
        });
    }

    pub fn set_value(&mut self, track_idx: usize, pos: usize,
                     value: f32, int: Option<Interpolation>) {
        self.tracks[track_idx].set_value(pos, value, int);
    }
}

pub struct TrackerEditor {
    tracker:        Rc<RefCell<Tracker>>,
    cur_track_idx:  usize,
    cur_input_nr:   String,
    cur_row_idx:    usize,
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
        row_idx: usize,
        track_idx: usize,
        cursor: bool,
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
            cur_row_idx:   0,
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

            let mut track_row_pointer = 0;

            let mut rows_shown_count = 0;
            for row_idx in 0..self.tracker.borrow().rows {
                if rows_shown_count > max_rows {
                    break;
                }

                let cursor_is_here =
                        self.cur_row_idx   == row_idx
                     && self.cur_track_idx == track_idx;

                if    track_row_pointer < track.data.len()
                   && track.data[track_row_pointer].0 == row_idx {

                    cc += 1;
                    view.draw_track_cell(
                        row_idx, track_idx, cursor_is_here,
                        Some(track.data[track_row_pointer].1),
                        track.data[track_row_pointer].2);

                    track_row_pointer += 1;
                } else {
                    cc += 1;
                    view.draw_track_cell(
                        row_idx, track_idx, cursor_is_here,
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
                self.cur_row_idx += 1;
            },
            TrackerInput::RowUp => {
                if self.cur_row_idx > 0 {
                    self.cur_row_idx -= 1;
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

        if self.cur_row_idx >= self.tracker.borrow().rows {
            self.cur_row_idx = self.tracker.borrow().rows;
        }

        if was_input {
            self.tracker.borrow_mut()
                .set_value(
                    self.cur_track_idx,
                    self.cur_row_idx,
                    10.0,
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
    last_idx: usize,
    // if index is at or above desired key, interpolate
    // else set index = 0 and restart search for right key
    data: Vec<(usize, f32, Interpolation)>,
}

impl Track {
    fn set_value(&mut self, pos: usize, value: f32, int: Option<Interpolation>) {
        let entry = self.data.iter().enumerate().find(|v| (v.1).0 >= pos);
        if let Some((idx, val)) = entry {
            if val.0 == pos {
                if int.is_none() {
                    self.data[idx] = (pos, value, val.2);
                } else {
                    self.data[idx] = (pos, value, int.unwrap());
                }
            } else {
                self.data.insert(
                    idx, (pos, value, int.unwrap_or(Interpolation::Empty)));
            }
        } else {
            self.data.push((pos, value, int.unwrap_or(Interpolation::Empty)));
        }
    }

    fn get_value(&mut self, _pos: usize) -> f32 {
        0.0
    }
    fn serialize(&self) -> Vec<u8> {
        Vec::new()
    }
    fn deserialize(_data: &[u8]) -> Track {
        Track {
            name:     String::from(""),
            last_idx: 0,
            data:     Vec::new(),
        }
    }
}
