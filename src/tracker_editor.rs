use crate::tracker::*;
use crate::track::*;
use crate::gui_painter::*;
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

const TPOS_PAD      : f32 = 50.0;
const TRACK_PAD     : f32 =  0.0;
const TRACK_VAL_PAD : f32 =  4.0;
const TRACK_WIDTH   : f32 = 70.0 + TRACK_VAL_PAD;
const ROW_HEIGHT    : f32 = 18.0;

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
        if self.cur_line_idx >= (self.scroll_line_offs + max_rows) {
            self.scroll_line_offs = self.cur_line_idx - (max_rows / 2);
        }
    }

    pub fn need_redraw(&self) -> bool { self.redraw_flag }

    fn draw_track<P>(&self, painter: &mut P, track_idx: usize, cursor: bool, val: f32) where P: GUIPainter {
        let mut clr = [0.8, 0.8, 0.8, 1.0];
        if cursor {
            clr = [1.0, 0.7, 0.7, 1.0];
        }

        painter.draw_rect(
            clr,
            [TPOS_PAD + track_idx as f32 * (TRACK_WIDTH + TRACK_PAD), 0.0],
            [TRACK_WIDTH, 10.0 * ROW_HEIGHT],
            false,
            0.5);
        painter.draw_text(
            clr,
            [TPOS_PAD + track_idx as f32 * (TRACK_WIDTH + TRACK_PAD) + 2.0,
             10.0 * ROW_HEIGHT + 2.0],
            0.5 * ROW_HEIGHT,
            format!("{:<6.2}", val));
    }

    fn draw_track_cell<P>(&self, painter: &mut P,
        line_pos: usize,
        line_idx: usize,
        track_idx: usize,
        cursor: bool,
        beat: bool,
        play_pos_row: i32,
        value: Option<f32>,
        interp: Interpolation) where P: GUIPainter {

        let int_s = match interp {
            Interpolation::Empty => "e",
            Interpolation::Step  => "_",
            Interpolation::Lerp  => "/",
            Interpolation::SStep => "~",
            Interpolation::Exp   => "^",
        };

        let s = if let Some(v) = value {
            format!("{} {:>6.2}", int_s, v)
        } else {
            String::from("- ------")
        };

        let txt_x =
            TRACK_VAL_PAD
            + TPOS_PAD
            + track_idx as f32 * (TRACK_WIDTH + TRACK_PAD);

        let txt_y = line_pos as f32 * ROW_HEIGHT;

        if track_idx == 0 {
            if line_idx as i32 == play_pos_row {
                painter.draw_rect(
                    [0.4, 0.0, 0.0, 1.0],
                    [0.0, txt_y],
                    [800.0, ROW_HEIGHT],
                    true,
                    0.0);
            }

            painter.draw_text(
                if beat { [0.5, 8.0, 0.5, 1.0] }
                else { [0.6, 0.6, 0.6, 1.0] },
                [TRACK_PAD / 2.0, txt_y],
                ROW_HEIGHT * 0.6,
                format!("[{:0>4}]", line_idx));
        }

        if cursor {
            painter.draw_rect(
                [0.4, 0.8, 0.4, 1.0],
                [txt_x - TRACK_VAL_PAD + 1.0, txt_y + 1.0],
                [TRACK_WIDTH - 2.0, ROW_HEIGHT - 2.0],
                true,
                0.5);

            painter.draw_text(
                [0.0, 0.0, 0.0, 1.0],
                [txt_x, txt_y],
                ROW_HEIGHT * 0.9,
                s);
        } else {
            if beat {
                painter.draw_text(
                    [0.6, 1.0, 0.6, 1.0],
                    [txt_x, txt_y],
                    ROW_HEIGHT * 0.9,
                    s);
            } else {
                painter.draw_text(
                    [0.8, 0.8, 0.8, 1.0],
                    [txt_x, txt_y],
                    ROW_HEIGHT * 0.9,
                    s);
            }
        }

    }

    pub fn show_state<P>(&mut self, max_rows: usize, painter: &mut P, play_pos_row: i32, values: &Vec<f32>) where P: GUIPainter {
        self.calc_cursor_scroll(max_rows);

        painter.start();

        for (track_idx, track) in self.tracker.borrow().tracks.iter().enumerate() {
            let val = if values.len() > track_idx { values[track_idx] } else { 0.0 };
            self.draw_track(painter, track_idx, self.cur_track_idx == track_idx, val);

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

                    self.draw_track_cell(
                        painter,
                        line_idx - self.scroll_line_offs,
                        line_idx, track_idx, cursor_is_here, beat,
                        play_pos_row,
                        Some(track.data[track_line_pointer].1),
                        track.data[track_line_pointer].2);

                    track_line_pointer += 1;
                } else {
                    self.draw_track_cell(
                        painter,
                        line_idx - self.scroll_line_offs,
                        line_idx, track_idx, cursor_is_here, beat,
                        play_pos_row,
                        None, Interpolation::Empty);
                }

                rows_shown_count += 1;
            }

        }

        self.redraw_flag = false;

        painter.show();
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
