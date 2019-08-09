use std::rc::Rc;
use std::cell::RefCell;

pub trait OutputHandler {
    fn emit_event(&mut self, track_idx: usize, val: f32);
    fn emit_play_line(&mut self, play_line: i32);
    fn value_buffer(&mut self) -> &mut Vec<f32>;
}

pub trait TrackerSync {
    fn add_track(&mut self, t: Track);
    fn set_value(&mut self, track_idx: usize, line: usize,
                     value: f32, int: Option<Interpolation>);
    fn remove_value(&mut self, track_idx: usize, line: usize);
}

pub struct Tracker<SYNC> {
//    /// beats per minute
//    bpm:            usize,
    /// lines per beat
    lpb:            usize,
    /// ticks per row/line
    tpl:            usize,
    /// number of lines in all tracks
    lines:          usize,
    /// current play head, if -1 it will start with line 0
pub play_line:      i32,
    /// number of played ticks
    tick_count:     usize,
    tracks:         Vec<Track>,
    /// the synchronization class:
    sync:           SYNC,
}

// TODO: Make an interface between tracker editor and tracker to replicate and
//       synchronize modifications between GUI and rendering thread.

impl<SYNC> Tracker<SYNC> where SYNC: TrackerSync {
    pub fn new(sync: SYNC) -> Self {
        Tracker {
            lpb:    4, // => 4 beats are 1 `Tackt`(de)
            tpl:    10,
            lines:  64,
            tracks: Vec::new(),
            play_line: -1,
            tick_count: 0,
            sync,
        }
    }

    pub fn add_track(&mut self, name: &str, data: Vec<(usize, f32, Interpolation)>) {
        let t = Track::new(name, data);
        self.sync.add_track(t.clone());
        self.tracks.push(t);
    }

    pub fn tick<T>(&mut self, output: &mut T)
        where T: OutputHandler {

        self.tick_count += 1;
        let new_play_line = self.tick_count / self.tpl;

        if new_play_line >= self.lines {
            self.tick_count = 0;
            self.play_line = -1;
            return;
        }

        if new_play_line as i32 > self.play_line {
            output.emit_play_line(self.play_line);

            for (track_idx, t) in self.tracks.iter_mut().enumerate() {
                let e = t.play_line(new_play_line, self.lines);
                if let Some(v) = e {
                    output.emit_event(track_idx, v);
                }
            }
        }
        //d// println!("TC: {} {}/{}", self.tick_count, new_play_line, self.play_line);

        self.play_line = new_play_line as i32;

        let buf : &mut Vec<f32> = &mut output.value_buffer();

        if buf.len() < self.tracks.len() {
            buf.resize(self.tracks.len(), 0.0);
        }

        for (idx, t) in self.tracks.iter_mut().enumerate() {
            buf[idx] = t.get_value(new_play_line);
        }
    }

    pub fn set_value(&mut self, track_idx: usize, line: usize,
                     value: f32, int: Option<Interpolation>) {
        self.sync.set_value(track_idx, line, value, int);
        self.tracks[track_idx].set_value(line, value, int);
    }

    pub fn remove_value(&mut self, track_idx: usize, line: usize) {
        self.sync.remove_value(track_idx, line);
        self.tracks[track_idx].remove_value(line);
    }
}

pub struct TrackerEditor<SYNC> {
    pub tracker:    Rc<RefCell<Tracker<SYNC>>>,
    cur_track_idx:  usize,
    cur_input_nr:   String,
    cur_line_idx:   usize,
    redraw_flag:    bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TrackerInput {
    Enter,
    Delete,
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

pub trait TrackerEditorView<C> {
    fn start_drawing(&mut self, ctx: &mut C);
    fn start_track(&mut self, ctx: &mut C, track_idx: usize, name: &str, cursor: bool);
    fn draw_track_cell(
        &mut self, ctx: &mut C,
        scroll_offs: usize,
        line_idx: usize,
        track_idx: usize,
        cursor: bool,
        beat: bool,
        value: Option<f32>,
        interp: Interpolation);
    fn end_track(&mut self, ctx: &mut C);
    fn end_drawing(&mut self, ctx: &mut C);
}

impl<SYNC> TrackerEditor<SYNC> where SYNC: TrackerSync {
    pub fn new(tracker: Rc<RefCell<Tracker<SYNC>>>) -> Self {
        TrackerEditor {
            tracker,
            cur_track_idx: 0,
            cur_input_nr:  String::from(""),
            cur_line_idx:   0,
            redraw_flag:   true,
        }
    }

    pub fn need_redraw(&self) -> bool { self.redraw_flag }

    pub fn show_state<T, C>(&mut self, scroll_offs: usize, max_rows: usize, view: &mut T, ctx: &mut C) where T: TrackerEditorView<C> {
//        if !self.redraw_flag { return; }
//        self.redraw_flag = false;

        view.start_drawing(ctx);
        for (track_idx, track) in self.tracker.borrow().tracks.iter().enumerate() {
            view.start_track(ctx, track_idx, &track.name, self.cur_track_idx == track_idx);

            let first_data_cell = track.data.iter().enumerate().find(|v| (v.1).0 >= scroll_offs);
            let mut track_line_pointer =
                if let Some((i, _v)) = first_data_cell {
                    i
                } else {
                    0
                };

            let mut rows_shown_count = 0;
            for line_idx in scroll_offs..self.tracker.borrow().lines {
                if rows_shown_count > max_rows {
                    break;
                }

                let cursor_is_here =
                        self.cur_line_idx  == line_idx
                     && self.cur_track_idx == track_idx;
                let beat = (line_idx % self.tracker.borrow().lpb) == 0;

                if    track_line_pointer < track.data.len()
                   && track.data[track_line_pointer].0 == line_idx {

                    view.draw_track_cell(
                        ctx,
                        scroll_offs,
                        line_idx, track_idx, cursor_is_here, beat,
                        Some(track.data[track_line_pointer].1),
                        track.data[track_line_pointer].2);

                    track_line_pointer += 1;
                } else {
                    view.draw_track_cell(
                        ctx,
                        scroll_offs,
                        line_idx, track_idx, cursor_is_here, beat,
                        None, Interpolation::Empty);
                }

                rows_shown_count += 1;
            }

            view.end_track(ctx);
        }
        view.end_drawing(ctx);
        self.redraw_flag = false;
    }

    pub fn process_input(&mut self, input: TrackerInput) {
        let mut was_input = false;

        self.redraw_flag = true;

        match input {
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
            TrackerInput::Delete => {
                self.tracker.borrow_mut()
                    .remove_value(
                        self.cur_track_idx,
                        self.cur_line_idx);
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

        if self.cur_line_idx >= self.tracker.borrow().lines {
            self.cur_line_idx = self.tracker.borrow().lines;
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

#[derive(Debug, Copy, Clone, PartialEq)]
enum PlayPos {
    Desync,
    End,
    At(usize),
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

    fn desync(&mut self) {
        self.clear();
        self.desync = true;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub name: String,
    play_pos: PlayPos,
    interpol: InterpolationState,
    // if index is at or above desired key, interpolate
    // else set index = 0 and restart search for right key
    pub data: Vec<(usize, f32, Interpolation)>,
}

impl Track {
    fn new(name: &str, data: Vec<(usize, f32, Interpolation)>) -> Self {
        Track {
            name: String::from(name),
            play_pos: PlayPos::Desync,
            interpol: InterpolationState::new(),
            data,
        }
    }

    fn desync(&mut self) {
        self.play_pos = PlayPos::Desync;
        self.interpol.desync();
    }

    fn remove_value(&mut self, line: usize) {
        let entry = self.data.iter().enumerate().find(|v| (v.1).0 == line);
        if let Some((idx, _val)) = entry {
            self.data.remove(idx);
            self.desync();
        }
    }

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
                    idx, (line, value, int.unwrap_or(Interpolation::Lerp)));
            }
        } else {
            self.data.push((line, value, int.unwrap_or(Interpolation::Lerp)));
        }

        self.desync();
    }

    fn sync_interpol_to_play_line(&mut self, line: usize, end_line: usize) {
        // idx.0 is either == line or > line. If > line, then we need to find
        // the previous data index (if any) and set the interpolation from that.
        if !self.interpol.desync { return; }

        match self.play_pos {
            PlayPos::End     => (), // get prev. iterpol
            PlayPos::At(idx) => {
                ()
                // index.0 == current line => get next end point
                // index.0 > curent line => get prev interpol
            },
            _ => (),
        }

//        let d = self.data[idx];
//        if (idx + 1) >= self.data.len() {
//            self.interpol.line_a = d.0;
//            self.interpol.line_b = end_line;
//            self.interpol.val_a  = d.1;
//            self.interpol.val_b  = 0.0;
//            self.interpol.int    = d.2;
//            self.play_pos = PlayPos::End;
//        } else {
//            let j = self.data[idx + 1];
//            self.interpol.line_a = d.0;
//            self.interpol.line_b = j.0;
//            self.interpol.val_a  = d.1;
//            self.interpol.val_b  = j.1;
//            self.interpol.int    = d.2;
//            self.play_pos = PlayPos::At(idx + 1);
//        }
    }

    fn check_sync(&mut self, line: usize, end_line: usize) {
        if self.play_pos == PlayPos::Desync {
            let entry = self.data.iter().enumerate().find(|v| (v.1).0 >= line);
            if let Some((idx, _val)) = entry {
                self.play_pos = PlayPos::At(idx);
            } else {
                self.play_pos = PlayPos::End;
            }
        }
    }

    fn play_line(&mut self, line: usize, end_line: usize) -> Option<f32> {
        self.check_sync(line, end_line);

        match self.play_pos {
            PlayPos::Desync  => None,
            PlayPos::End     => None,
            PlayPos::At(idx) => {
                let d = self.data[idx];
                if d.0 == line {
                    if (idx + 1) >= self.data.len() {
                        self.interpol.line_a = d.0;
                        self.interpol.line_b = end_line;
                        self.interpol.val_a  = d.1;
                        self.interpol.val_b  = 0.0;
                        self.interpol.int    = d.2;
                        self.play_pos = PlayPos::End;
                    } else {
                        let j = self.data[idx + 1];
                        self.interpol.line_a = d.0;
                        self.interpol.line_b = j.0;
                        self.interpol.val_a  = d.1;
                        self.interpol.val_b  = j.1;
                        self.interpol.int    = d.2;
                        self.play_pos = PlayPos::At(idx + 1);
                    }
                    Some(d.1)
                } else {
                    None
                }
            }
        }
    }

    /// Only works if the interpolation was initialized with self.play_line()!
    fn get_value(&mut self, line: usize) -> f32 {
        let i = &mut self.interpol;

        if line < i.line_a {
            i.clear();
        }

        let mut diff = i.line_b - i.line_a;
        if diff == 0 { diff = 1; }

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
                let x = ((line - i.line_a) as f64) / diff as f64;
                (  i.val_a as f64 * (1.0 - x)
                 + i.val_b as f64 * x)
                as f32
            },
            Interpolation::SStep => {
                let x = ((line - i.line_a) as f64) / diff as f64;
                let x = if x < 0.0 { 0.0 } else { x };
                let x = if x > 1.0 { 1.0 } else { x };
                let x = x * x * (3.0 - 2.0 * x);

                (  i.val_a as f64 * (1.0 - x)
                 + i.val_b as f64 * x)
                as f32
            },
            Interpolation::Exp => {
                let x = ((line - i.line_a) as f64) / diff as f64;
                let x = x * x;

                (  i.val_a as f64 * (1.0 - x)
                 + i.val_b as f64 * x)
                as f32
            },
        }
    }
    fn serialize(&self) -> Vec<u8> {
        Vec::new()
    }
    fn deserialize(_data: &[u8]) -> Track {
        Track {
            name:     String::from(""),
            play_pos: PlayPos::Desync,
            interpol: InterpolationState::new(),
            data:     Vec::new(),
        }
    }
}
