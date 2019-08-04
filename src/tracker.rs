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
}

pub struct TrackerEditor<'a> {
    tracker:        &'a mut Tracker,
    cur_track_idx:  usize,
    cur_input_nr:   String,
    cur_row_idx:    usize,
    redraw_flag:    bool,
}

pub enum TrackerInput {
    KeyEsc,
    KeyEnter,
    KeyNum(u8),
    KeyDot,
    KeyStep,
    KeyLerp,
    KeySStep,
    KeyExp,
    KeyDown,
    KeyUp,
    KeyLeft,
    KeyRight,
}

pub trait TrackerEditorView {
    fn start_drawing(&mut self);
    fn draw_track_cell(
        &mut self,
        row_idx: usize,
        track_idx: usize,
        cursor: bool,
        value: Option<f32>,
        interp: Interpolation);
    fn end_drawing(&mut self);
}

impl<'a> TrackerEditor<'a> {
    pub fn new(trk: &'a mut Tracker) -> Self {
        TrackerEditor {
            tracker:       trk,
            cur_track_idx: 0,
            cur_input_nr:  String::from(""),
            cur_row_idx:   0,
            redraw_flag:   true,
        }
    }

    pub fn show_state<T>(&mut self, view: &mut T) where T: TrackerEditorView {
        if !self.redraw_flag { return; }
        self.redraw_flag = false;

        view.start_drawing();
        for (track_idx, track) in self.tracker.tracks.iter().enumerate() {

            let mut track_row_pointer = 0;

            for row_idx in 0..self.tracker.rows {

                let cursor_is_here =
                        self.cur_row_idx   == row_idx
                     && self.cur_track_idx == track_idx;

                if    track_row_pointer <= track.data.len()
                   && track.data[track_row_pointer].0 == row_idx {

                    view.draw_track_cell(
                        row_idx, track_idx, cursor_is_here,
                        Some(track.data[track_row_pointer].1),
                        track.data[track_row_pointer].2);
                } else {
                    view.draw_track_cell(
                        row_idx, track_idx, cursor_is_here,
                        None, Interpolation::Empty);
                }

                track_row_pointer += 1;
            }
        }
        view.end_drawing();
    }

    pub fn process_input(&mut self, input: TrackerInput) {
        match input {
            TrackerInput::KeyEsc => {
                self.cur_input_nr = String::from("");
            },
            TrackerInput::KeyEnter => {
                // parse self.cur_input_nr to float and enter into current row.
            },
            TrackerInput::KeyNum(n) => {
                self.cur_input_nr += &n.to_string();
            },
            TrackerInput::KeyDot => {
                // if dot in cur_input_nr ignore
                // else add dot
            },
            TrackerInput::KeyDown => {
                // row idx += 1
            },
            TrackerInput::KeyUp => {
                // row idx -= 1
            },
            TrackerInput::KeyRight => {
                // track idx += 1
            },
            TrackerInput::KeyLeft => {
                // track idx -= 1
            },
            TrackerInput::KeyStep => {
                // set interp mode of cur row to *
            },
            TrackerInput::KeyLerp => {
                // set interp mode of cur row to *
            },
            TrackerInput::KeySStep => {
                // set interp mode of cur row to *
            },
            TrackerInput::KeyExp => {
                // set interp mode of cur row to *
            },
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
