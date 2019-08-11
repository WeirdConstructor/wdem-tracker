use crate::tracker::*;
use crate::track::*;
use std::rc::Rc;
use std::cell::RefCell;

pub struct TrackerEditor<SYNC> where SYNC: TrackerSync {
    pub tracker:    Rc<RefCell<Tracker<SYNC>>>,
    cur_track_idx:  usize,
    cur_input_nr:   String,
    cur_line_idx:   usize,
    scroll_line_offs: usize,
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
    PlayHead(PlayHeadAction),
}

pub trait TrackerEditorView<C> {
    fn start_drawing(&mut self, ctx: &mut C);
    fn start_track(&mut self, ctx: &mut C, track_idx: usize, name: &str, cursor: bool);
    fn draw_track_cell(
        &mut self, ctx: &mut C,
        line_pos: usize,
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
            scroll_line_offs:   0,
            cur_track_idx:      0,
            cur_input_nr:       String::from(""),
            cur_line_idx:        0,
            redraw_flag:        true,
        }
    }

    fn calc_cursor_scroll(&mut self, max_rows: usize) {
        if self.cur_line_idx >= self.tracker.borrow().lines {
            self.cur_line_idx = self.tracker.borrow().lines - 1;
        }
        if self.cur_line_idx < self.scroll_line_offs {
            self.scroll_line_offs = self.cur_line_idx;
        }
        if self.cur_line_idx > (self.scroll_line_offs + max_rows) {
            self.scroll_line_offs = self.cur_line_idx - (max_rows / 2);
        }
    }

    pub fn need_redraw(&self) -> bool { self.redraw_flag }

    pub fn show_state<T, C>(&mut self, max_rows: usize, view: &mut T, ctx: &mut C) where T: TrackerEditorView<C> {
        self.calc_cursor_scroll(max_rows);

        view.start_drawing(ctx);
        for (track_idx, track) in self.tracker.borrow().tracks.iter().enumerate() {
            view.start_track(ctx, track_idx, &track.name, self.cur_track_idx == track_idx);

            let first_data_cell = track.data.iter().enumerate().find(|v| (v.1).0 >= self.scroll_line_offs);
            let mut track_line_pointer =
                if let Some((i, _v)) = first_data_cell {
                    i
                } else {
                    0
                };

            let mut max_line = self.tracker.borrow().lines;
            if max_line > self.scroll_line_offs + max_rows {
                max_line = self.scroll_line_offs + max_rows;
            }

            let mut rows_shown_count = 0;
            for line_idx in self.scroll_line_offs..max_line {
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
                        line_idx - self.scroll_line_offs,
                        line_idx, track_idx, cursor_is_here, beat,
                        Some(track.data[track_line_pointer].1),
                        track.data[track_line_pointer].2);

                    track_line_pointer += 1;
                } else {
                    view.draw_track_cell(
                        ctx,
                        line_idx - self.scroll_line_offs,
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
            TrackerInput::PlayHead(a) => {
                self.tracker.borrow_mut().play_head(a);
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
                    self.cur_input_nr.parse::<f32>().unwrap_or(0.0));
        } else {
            self.cur_input_nr = String::from("");
        }
    }
}
