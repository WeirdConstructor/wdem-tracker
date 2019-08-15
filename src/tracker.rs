use crate::track::*;

extern crate serde_json;

/// This trait handles the output of a Tracker when being driven
/// by the tick() method. It generates events for starting notes
/// on a synthesizer and returns a vector of the interpolated values
/// on all tracks. The emit_play_line() function gives feedback of the
/// current song position in terms of the track line index.
pub trait OutputHandler {
    /// Called by Tracker::tick() when a new line started and
    /// a track has a new value defined. Useful for driving note on/off
    /// events on a synthesizer.
    fn emit_event(&mut self, track_idx: usize, val: f32);
    /// Called when the Tracker::tick() function advanced to a new line.
    fn emit_play_line(&mut self, play_line: i32);
    /// This should return a buffer for storing the interpolated values
    /// of all tracks. Useful for driving synthesizer automation.
    fn value_buffer(&mut self) -> &mut Vec<f32>;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlayHeadAction {
    TogglePause,
    Pause,
    Play,
    Restart,
    NextLine,
    PrevLine,
}

/// This trait provides an interface to synchronize the track data
/// between two Tracker instances. The main purpose is to connect a
/// frontend Tracker with an audio thread tracker in such a way, that
/// changes to the track data is transmitted to the backend thread.
/// How threading is done is up to the implementor of this trait.
/// You may even not use threads at all and use some network protocol
/// for synchronization.
pub trait TrackerSync {
    /// Called by Tracker when a new Track is added.
    fn add_track(&mut self, t: Track);
    /// Called by Tracker when a value in a specific track and line
    /// is added.
    fn set_value(&mut self, track_idx: usize, line: usize, value: f32);
    /// Called by Tracker when an interpolation for a value should be set.
    /// Does nothing if no value at that position exists.
    fn set_int(&mut self, track_idx: usize, line: usize, int: Interpolation);
    /// Called by Tracker when a value is removed from a track.
    fn remove_value(&mut self, track_idx: usize, line: usize);
    /// Called when the tracker should change the play head state:
    fn play_head(&mut self, _act: PlayHeadAction) { }
}

/// This is a Tracker synchronizer that does nothing.
/// Use it if you don't want to or not need to sync.
pub struct TrackerNopSync { }

impl TrackerSync for TrackerNopSync {
    fn add_track(&mut self, _t: Track) { }
    fn set_value(&mut self, _track_idx: usize, _line: usize, _value: f32) { }
    fn set_int(&mut self, _track_idx: usize, _line: usize, _int: Interpolation) { }
    fn remove_value(&mut self, _track_idx: usize, _line: usize) { }
    fn play_head(&mut self, _act: PlayHeadAction) { }
}

/// This structure stores the state of a tracker.
/// It stores the play state aswell as the actual track data.
/// The SYNC type must implement the TrackerSync trait.
/// It is responsible for connecting the tracker frontend
/// in a graphics thread with a tracker in the audio thread.
pub struct Tracker<SYNC> where SYNC: TrackerSync {
    /// lines per beat
pub lpb:            usize,
    /// ticks per row/line
pub tpl:            usize,
    /// number of lines in all tracks
pub lines:          usize,
    /// current play head, if -1 it will start with line 0
pub play_line:      i32,
    /// The actual track data.
pub tracks:         Vec<Track>,
    /// the synchronization class:
    sync:           SYNC,
    /// number of played ticks
    tick_count:     usize,
    /// interval between ticks in ms
pub tick_interval:  usize,
}

impl<SYNC> Tracker<SYNC> where SYNC: TrackerSync {
    pub fn new(sync: SYNC) -> Self {
        Tracker {
            lpb:            4, // => 4 beats are 1 `Tackt`(de)
            tpl:            10,
            tick_interval:  10,
            lines:          128,
            tracks:         Vec::new(),
            play_line:      -1,
            tick_count:     0,
            sync,
        }
    }

    pub fn tick2song_pos_in_s(&self) -> f32 {
        (((self.tick_count as f64)
          * (self.tick_interval as f64))
         / 1000.0) as f32
    }

    pub fn add_track(&mut self, t: Track) {
        self.sync.add_track(t.clone());
        self.tracks.push(t);
    }

    pub fn reset_pos(&mut self) {
        self.tick_count = 0;
        self.play_line  = -1;
        self.resync_tracks();
    }

    pub fn play_head(&mut self, a: PlayHeadAction) {
        self.sync.play_head(a);
    }

    pub fn tick_to_prev_line<T>(&mut self, output: &mut T)
        where T: OutputHandler {

        if self.play_line > 0 {
            self.tick_count =
                ((self.play_line - 1) * self.tpl as i32) as usize;
        };

        self.resync_tracks();
        self.handle_tick_count_change(output);
    }

    pub fn tick_to_next_line<T>(&mut self, output: &mut T)
        where T: OutputHandler {

        self.tick_count =
            if self.play_line < 0 {
                self.tpl as usize
            } else {
                ((self.play_line + 1) * self.tpl as i32) as usize
            };

        self.handle_tick_count_change(output);
    }

    pub fn handle_tick_count_change<T>(&mut self, output: &mut T)
        where T: OutputHandler {

        let mut new_play_line = self.tick_count / self.tpl;
        let fract_ticks =
            ((self.tick_count - (new_play_line * self.tpl)) as f64)
            / self.tpl as f64;

        if new_play_line >= self.lines {
            new_play_line = 0;
            self.tick_count = 1;
            self.play_line = -1;
            self.resync_tracks();
        }

        if new_play_line as i32 != self.play_line {
            output.emit_play_line(new_play_line as i32);

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
            buf[idx] = t.get_value(new_play_line, fract_ticks);
        }
    }

    pub fn tick<T>(&mut self, output: &mut T)
        where T: OutputHandler {

        self.tick_count += 1;
        self.handle_tick_count_change(output);
    }

    pub fn set_int(&mut self, track_idx: usize, line: usize, int: Interpolation) {
        self.sync.set_int(track_idx, line, int);
        self.tracks[track_idx].set_int(line, int);
    }

    pub fn set_value(&mut self, track_idx: usize, line: usize, value: f32) {
        self.sync.set_value(track_idx, line, value);
        self.tracks[track_idx].set_value(line, value);
    }

    fn resync_tracks(&mut self) {
        for t in self.tracks.iter_mut() { t.desync(); }
    }

    pub fn remove_value(&mut self, track_idx: usize, line: usize) {
        self.sync.remove_value(track_idx, line);
        self.tracks[track_idx].remove_value(line);
    }
}


