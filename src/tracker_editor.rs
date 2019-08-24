use crate::tracker::*;
use crate::track::*;
use crate::gui_painter::*;
use std::rc::Rc;
use std::cell::RefCell;

pub struct TrackerEditor<SYNC> where SYNC: TrackerSync {
    pub tracker:    Rc<RefCell<Tracker<SYNC>>>,
    cur_track_idx:  usize,
    cur_line_idx:   usize,
    scroll_offs:    usize,
    redraw_flag:    bool,
    step_size:      usize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TrackerInput {
    Delete,
    SetNote(u8),
    SetValue(f32),
    SetA(u8),
    SetB(u8),
    SetInterpStep,
    SetInterpLerp,
    SetInterpSStep,
    SetInterpExp,
    SetStep(usize),
    StepDown,
    StepUp,
    RowDown,
    RowUp,
    TrackLeft,
    TrackRight,
    PlayHead(PlayHeadAction),
}

impl<SYNC> TrackerEditor<SYNC> where SYNC: TrackerSync {
    pub fn new(tracker: Rc<RefCell<Tracker<SYNC>>>) -> Self {
        TrackerEditor {
            tracker,
            cur_track_idx:      0,
            cur_line_idx:       0,
            scroll_offs:        0,
            redraw_flag:        true,
            step_size:          1,
        }
    }

//    fn calc_cursor_scroll(&mut self, max_rows: usize) {
//        if self.cur_line_idx >= self.tracker.borrow().lpp {
//            self.cur_line_idx = self.tracker.borrow().lpp - 1;
//        }
//        if self.cur_line_idx < self.scroll_line_offs {
//            self.scroll_line_offs = self.cur_line_idx;
//        }
//        if self.cur_line_idx >= (self.scroll_line_offs + max_rows) {
//            self.scroll_line_offs = self.cur_line_idx - (max_rows / 2);
//        }
//    }

    pub fn need_redraw(&self) -> bool { self.redraw_flag }

    pub fn draw<P>(&mut self, p: &mut P, play_line: i32) where P: GUIPainter {
        let mut gs = GUIState {
            cursor_track_idx: self.cur_track_idx,
            track_index:      0,
            cursor_on_track:  false,
            cursor_on_line:   false,
            scroll_offs:      self.scroll_offs,
            play_on_line:     false,
            pattern_index:    0,
            on_beat:          false,
            cursor_line:      self.cur_line_idx,
            lpb:              0,
            play_line,
        };
        self.tracker.borrow_mut().draw(p, &mut gs);
        self.cur_track_idx = gs.cursor_track_idx;
        self.cur_line_idx  = gs.cursor_line;
        self.scroll_offs   = gs.scroll_offs;
    }

    pub fn process_input(&mut self, input: TrackerInput) {
        self.redraw_flag = true;

        match input {
            TrackerInput::SetNote(v) => {
                self.tracker.borrow_mut()
                    .set_note(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        v);
            },
            TrackerInput::SetA(v) => {
                self.tracker.borrow_mut()
                    .set_a(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        v);
            },
            TrackerInput::SetB(v) => {
                self.tracker.borrow_mut()
                    .set_b(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        v);
            },
            TrackerInput::SetValue(v) => {
                self.tracker.borrow_mut()
                    .set_value(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        v);
            },
            TrackerInput::Delete => {
                self.tracker.borrow_mut()
                    .remove_value(
                        self.cur_track_idx,
                        self.cur_line_idx);
            },
            TrackerInput::StepDown => {
                self.cur_line_idx += self.step_size;
            },
            TrackerInput::StepUp => {
                if self.cur_line_idx > 0 {
                    if self.step_size > self.cur_line_idx {
                        self.cur_line_idx = 0;
                    } else {
                        self.cur_line_idx -= self.step_size;
                    }
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
            TrackerInput::PlayHead(a) => {
                self.tracker.borrow_mut().play_head(a);
            },
            TrackerInput::SetInterpStep => {
                self.tracker.borrow_mut()
                    .set_int(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        Interpolation::Step);
            },
            TrackerInput::SetInterpLerp => {
                self.tracker.borrow_mut()
                    .set_int(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        Interpolation::Lerp);
            },
            TrackerInput::SetInterpSStep => {
                self.tracker.borrow_mut()
                    .set_int(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        Interpolation::SStep);
            },
            TrackerInput::SetInterpExp => {
                self.tracker.borrow_mut()
                    .set_int(
                        self.cur_track_idx,
                        self.cur_line_idx,
                        Interpolation::Exp);
            },
            TrackerInput::SetStep(s) => {
                self.step_size = s;
            },
        };

        if self.tracker.borrow().tracks.len() == 0 {
            return;
        }

        if self.cur_track_idx >= self.tracker.borrow().tracks.len() {
            self.cur_track_idx = self.tracker.borrow().tracks.len() - 1;
        }

        if self.cur_line_idx >= self.tracker.borrow().max_line_count() {
            self.cur_line_idx = self.tracker.borrow().max_line_count();
            if self.cur_line_idx > 0 { self.cur_line_idx -= 1; }
        }
    }
}
