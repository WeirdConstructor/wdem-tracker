use crate::tracker::*;
use crate::track::{Track, Row, Interpolation, TrackSerialized};
use crate::vval_opin::vv2opin;
use wave_sickle::new_slaughter;
use crate::audio_dev_thread::start_audio_thread;
use wctr_signal_ops::*;
use crate::scopes::{Scopes, SCOPE_SAMPLES, SCOPE_WIDTH};
use crate::audio::AudioFrontend;

pub struct TrackerThreadOutput {
    pub pos:                    i32,
    pub song_pos_s:             f32,
    pub cpu:                    (f64, f64, f64),
    pub audio_scope_samples:    Vec<Vec<f32>>,
    pub audio_scope_done:       bool,
        track_notes:            Vec<u8>,
        events:                 Vec<(usize, u8, u8)>,
}

impl TrackerThreadOutput {
    pub fn new() -> Self {
        TrackerThreadOutput {
            pos: 0,
            song_pos_s: 0.0,
            cpu: (0.0, 0.0, 0.0),
            events: Vec::new(),
            track_notes: Vec::new(),
            audio_scope_samples: Vec::new(),
            audio_scope_done: false,
        }
    }

    pub fn collect_audio_scope_samples(&mut self, sample_rate: usize, bufs: &Vec<Vec<f32>>) {
        if bufs.len() != self.audio_scope_samples.len() {
            self.audio_scope_samples.resize(bufs.len(), Vec::new());
        }

        // 2 times freq samples because of stereo signal!
        let a4_buf_len = 2 * ((sample_rate as f64 / 440.0).ceil() as usize);

        for (ab, ass) in bufs.iter().zip(self.audio_scope_samples.iter_mut()) {
            if a4_buf_len != ass.capacity() {
                ass.reserve(a4_buf_len);
            }

            let mut ass_len = ass.len();

            if ass_len >= a4_buf_len {
                self.audio_scope_done = true;
                return;
            }

            let rest = a4_buf_len - ass_len;
            let rest = if ab.len() < rest { ab.len() } else { rest };
            if rest > 0 {
                ass.extend_from_slice(&ab[0..rest]);
                ass_len = ass.len();
            }

            if ass_len >= a4_buf_len {
                self.audio_scope_done = true;
                return;
            }
        }
    }
}

impl OutputHandler for TrackerThreadOutput {
    fn emit_event(&mut self, track_idx: usize, row: &Row) {
        if row.note > 0 {
            if track_idx >= self.track_notes.len() {
                self.track_notes.resize(track_idx + 1, 0);
            }

            if row.note > 1 {
                self.events.push(
                    (track_idx, row.note, row.note));
            }

            if self.track_notes[track_idx] > 0 {
                self.events.push((track_idx, 1, self.track_notes[track_idx]));
            }

            if row.note == 1 {
                self.track_notes[track_idx] = 0;
            } else {
                self.track_notes[track_idx] = row.note;
            }
        }
        //d// println!("EMIT: {}: {}/{}", track_idx, val, flags);
    }

    fn emit_play_line(&mut self, play_line: i32) {
        //d// println!("EMIT PLAYLINE OUT {}", play_line);
        self.pos = play_line;
    }

    fn song_pos(&mut self) -> &mut f32 { return &mut self.song_pos_s; }
}

fn calc_cpu_percentage(millis: u128, interval_ms: u128) -> f64 {
    ((millis * 100000)
     / ((interval_ms * 1000) as u128)) as f64 / 1000.0
}

use wlambda;
use wlambda::{VVal, GlobalEnv, EvalContext, Env};

struct AudioThreadWLambdaContext {
    pub sim: Simulator,
    pub track_values: std::rc::Rc<std::cell::RefCell<Vec<f32>>>,
    pub sample_rate: usize,
}

fn eval_audio_script(mut msgh: wlambda::threads::MsgHandle, ctxref: std::rc::Rc<std::cell::RefCell<AudioThreadWLambdaContext>>) {
    let genv = GlobalEnv::new_default();

    genv.borrow_mut().add_func(
        "p", |env: &mut Env, _argc: usize| {
            println!("{}", env.arg(0).s_raw());
            Ok(VVal::Bol(true))
        }, Some(1), Some(1));

    genv.borrow_mut().add_func(
        "signal_group", |env: &mut Env, _argc: usize| {
            let name = env.arg(0).s_raw();
            env.with_user_do(|ctx: &mut AudioThreadWLambdaContext| {
                Ok(VVal::Int(ctx.sim.add_group(&name) as i64))
            })
        }, Some(1), Some(1));

    genv.borrow_mut().add_func(
        "input", |env: &mut Env, _argc: usize| {
            let op_name = env.arg(0).s_raw();
            let in_name = env.arg(1).s_raw();
            let op_in   = vv2opin(env.arg(2).clone());
            if op_in.is_none() {
                return Ok(VVal::err_msg(
                        &format!("bad op description: {}", env.arg(2).s())));
            }
            env.with_user_do(|ctx: &mut AudioThreadWLambdaContext| {
                let op_idx = ctx.sim.get_op_index(&op_name);
                if op_idx.is_none() {
                    return Ok(VVal::err_msg(
                            &format!("bad op name: {}", op_name)));
                }
                ctx.sim.set_op_input(op_idx.unwrap(), &in_name, op_in.unwrap().clone(), true);
                ctx.sim.set_op_input(op_idx.unwrap(), &in_name, op_in.unwrap().clone(), false);
                Ok(VVal::Bol(true))
            })
        }, Some(3), Some(3));

    genv.borrow_mut().add_func(
        "op", |env: &mut Env, _argc: usize| {
            let op_type     = env.arg(0).s_raw();
            let op_name     = env.arg(1).s_raw();
            let group_index = env.arg(2).i() as usize;
            env.with_user_do(|ctx: &mut AudioThreadWLambdaContext| {
                let op : Box<dyn Op> =
                    match &op_type[..] {
                        "sin" => {
                            let s = ops::Sin::new();
                            Box::new(s)
                        },
                        "slaughter" => {
                            let s = new_slaughter(ctx.sample_rate as f64);
                            Box::new(s)
                        },
                        "audio_send" => {
                            let s = ops::AudioSend::new();
                            Box::new(s)
                        },
                        _ => { return Ok(VVal::Nul); }
                    };

                match ctx.sim.add_op(op, op_name.clone(), group_index) {
                    Some(i) => Ok(VVal::Int(i as i64)),
                    None    => Ok(VVal::Nul),
                }
            })
        }, Some(3), Some(3));

    genv.borrow_mut().add_func(
        "track_proxy", |env: &mut Env, _argc: usize| {
            let track_count = env.arg(0).i() as usize;
            let group_index = env.arg(1).i() as usize;
            println!("TR {} , {}", track_count, group_index);
            env.with_user_do(|ctx: &mut AudioThreadWLambdaContext| {
                let oprox = ops::OutProxy::new(track_count);
                ctx.track_values = oprox.values.clone();
                ctx.sim.add_op(Box::new(oprox), String::from("T"), group_index);
                Ok(VVal::Bol(true))
            })
        }, Some(2), Some(2));

    let mut wl_eval_ctx =
        wlambda::compiler::EvalContext::new_with_user(genv, ctxref);

    //    match wl_eval_ctx.eval_file("tracker.wl") {
//        Ok(_) => (),
//        Err(e) => { panic!(format!("AUDIO SCRIPT ERROR: {}", e)); }
//    }

    println!("RUN");
    msgh.run(&mut wl_eval_ctx);
    println!("RUN DONE");
}


pub fn start_tracker_thread(
    msgh: wlambda::threads::MsgHandle,
    ext_out: std::sync::Arc<std::sync::Mutex<TrackerThreadOutput>>,
    rcv: std::sync::mpsc::Receiver<TrackerSyncMsg>,
    mut ep: SimulatorCommunicatorEndpoint) -> Scopes {

    let sr = Scopes::new(SCOPE_SAMPLES);
    let rr = sr.sample_row.clone();

    let mut audio_f = AudioFrontend::new();
    let audio_dev = audio_f.get_dev();
    start_audio_thread(audio_dev);

    let mut last_iter = std::time::Instant::now();

    std::thread::spawn(move || {
        audio_f.wait_backend_ready();

        let ctxref =
            std::rc::Rc::new(std::cell::RefCell::new(AudioThreadWLambdaContext {
                sim:          Simulator::new(),
                track_values: std::rc::Rc::new(std::cell::RefCell::new(vec![])),
                sample_rate:  audio_f.get_sample_rate(),
            }));

        eval_audio_script(msgh, ctxref.clone());

        // wlambda API:
        // - (audio thread) setup simulator groups
        // - (audio thread) setup simulator operators and their default vals
        // - (audio thread) setup audio buffers and routings between the audio
        //                  devices.
        // - (audio thread) specify which audio devices receive note events
        //                  from the tracks.
        // - (frontend thread) add tracks
        // - (frontend thread) configure tracker values (needs sync!)
        // - (frontend thread) specify project file name
        // - (frontend thread) turtle setup
        // - (frontend thread) frontend simulator setup (groups, operators, ...)
        //                     (insert backend values via OutProxy)

        let mut ctx = ctxref.borrow_mut();

        let mut o = TrackerThreadOutput::new();
        let mut t = Tracker::new(TrackerNopSync { });

        let sample_buf_len =
            (((audio_f.get_sample_rate() * t.tick_interval) as f64).ceil()
             / 1000.0)
            as usize;

        let ticks_per_audio_scope_update =
            // 1000ms / 100ms / ms_per_tick => 10 times per second
            (100.0 as f64 / (t.tick_interval as f64)).ceil() as usize;

        let mut audio_buffers = ctx.sim.new_group_sample_buffers(sample_buf_len);

        let mut is_playing        = true;
        let mut out_updated       = false;
        let mut micros_min : u128 = 9999999;
        let mut micros_max : u128 = 0;
        let mut micros_sum : u128 = 0;
        let mut micros_cnt : u128 = 0;
        let mut audio_scope_counter : usize = 0;
        loop {
            let now = std::time::Instant::now();

            ep.handle_ui_messages(&mut ctx.sim);

            let r = rcv.try_recv();
            match r {
                Ok(TrackerSyncMsg::AddTrack(track)) => {
                    t.add_track(track.clone());
                    println!("THRD: TRACK ADD TRACK");
                },
                Ok(TrackerSyncMsg::SetInt(track_idx, line, int)) => {
                    t.set_int(track_idx, line, int);
                    println!("THRD: SET VAL");
                },
                Ok(TrackerSyncMsg::SetValue(track_idx, line, v)) => {
                    t.set_value(track_idx, line, v);
                    println!("THRD: SET VAL");
                },
                Ok(TrackerSyncMsg::SetNote(track_idx, line, v)) => {
                    t.set_note(track_idx, line, v);
                    println!("THRD: SET NOTE {}", v);
                },
                Ok(TrackerSyncMsg::SetA(track_idx, line, v)) => {
                    t.set_a(track_idx, line, v);
                    println!("THRD: SET A");
                },
                Ok(TrackerSyncMsg::SetB(track_idx, line, v)) => {
                    t.set_b(track_idx, line, v);
                    println!("THRD: SET B");
                },
                Ok(TrackerSyncMsg::RemoveValue(track_idx, line)) => {
                    t.remove_value(track_idx, line);
                    println!("THRD: REMO VAL");
                },
                Ok(TrackerSyncMsg::DeserializeContents(track_idx, contents)) => {
                    t.deserialize_contents(track_idx, contents);
                },
                Ok(TrackerSyncMsg::PlayHead(a)) => {
                    match a {
                        PlayHeadAction::TogglePause => {
                            is_playing = !is_playing;
                        },
                        PlayHeadAction::Pause    => { is_playing = false; },
                        PlayHeadAction::Play     => { is_playing = true; },
                        PlayHeadAction::NextLine => {
                            println!("NEXT LINE");
                            t.tick_to_next_line(&mut o, &ctx.track_values);
                            out_updated = true;
                            is_playing = false;
                        },
                        PlayHeadAction::PrevLine => {
                            println!("PREV LINE");
                            t.tick_to_prev_line(&mut o, &ctx.track_values);
                            out_updated = true;
                            is_playing = false;
                        },
                        PlayHeadAction::Restart  => {
                            t.reset_pos();
                            is_playing = true;
                        },
                        // _ => (),
                    }
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => (),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return (),
            }

            if is_playing {
                t.tick(&mut o, &ctx.track_values);
                out_updated = true;
                //d// println!("THRD: TICK {}", o.pos);
            }

            if out_updated {
                while !o.events.is_empty() {
                    let e = o.events.pop().unwrap();
                    // TODO: Implement proper mapping of track->group, maybe
                    //       use the A value?!
                    let ev =
                        if e.1 == 1 {
                            signals::Event::NoteOff(e.2)
                        } else {
                            signals::Event::NoteOn(e.1)
                        };
                    ctx.sim.event(0, &ev);
                    ctx.sim.event(1, &ev);
                    ctx.sim.event(2, &ev);
                    ctx.sim.event(3, &ev);
                    ctx.sim.event(4, &ev);
                    ctx.sim.event(5, &ev);
                    ctx.sim.event(6, &ev);
                }

                ctx.sim.exec(o.song_pos_s, rr.clone());
            }

            if is_playing {
                ctx.sim.render(sample_buf_len, 0, &mut audio_buffers);
                if audio_scope_counter > ticks_per_audio_scope_update {
                    o.collect_audio_scope_samples(
                        audio_f.get_sample_rate(), &audio_buffers);
                    audio_scope_counter = 0;
                } else {
                    audio_scope_counter += 1;
                }

            } else {
                ctx.sim.render_silence(sample_buf_len, 0, &mut audio_buffers);
            }

            if out_updated {
                out_updated = false;
                if let Ok(ref mut m) = ext_out.try_lock() {
                    m.pos        = o.pos;
                    m.song_pos_s = o.song_pos_s;
                    m.cpu        = o.cpu;
                    if o.audio_scope_done && !m.audio_scope_done {
                        m.audio_scope_done = o.audio_scope_done;
                        std::mem::swap(
                            &mut m.audio_scope_samples,
                            &mut o.audio_scope_samples);
                        o.audio_scope_done = false;
                    }
                }
            }


            //            std::thread::sleep(
            //                std::time::Duration::from_micros(
            //                    (((t.tick_interval * 1000) as f64) * 0.1) as u64));

            let elap = now.elapsed().as_micros();

            let wait = std::time::Instant::now();
            audio_f.put_samples_blocking(&audio_buffers[0][..]);

            let whole = last_iter.elapsed().as_micros();
            last_iter = std::time::Instant::now();


            micros_sum += elap;
            micros_cnt += 1;
            if micros_min > elap { micros_min = elap; }
            if micros_max < elap { micros_max = elap; }

            if micros_cnt > 200 {
                println!("i elap={}, min={}, max={}, whole={}, wait={}", elap, micros_min, micros_max, whole, wait.elapsed().as_micros());
                o.cpu = (
                    calc_cpu_percentage(micros_sum / micros_cnt, t.tick_interval as u128),
                    calc_cpu_percentage(micros_min, t.tick_interval as u128),
                    calc_cpu_percentage(micros_max, t.tick_interval as u128));

                //                println!("audio thread %cpu: min={:<6}, max={:<6}, {:<6} {:<4} | {:<4} / {:6.2}/{:6.2}/{:6.2}",
//                         micros_min,
//                         micros_max,
//                         micros_sum,
//                         micros_cnt,
//                         micros_sum / micros_cnt,
//                         o.cpu.0,
//                         o.cpu.1,
//                         o.cpu.2);

                micros_cnt = 0;
                micros_sum = 0;
                micros_min = 9999999;
                micros_max = 0;
            }

//            std::thread::sleep(
//                std::time::Duration::from_millis(
//                    t.tick_interval as u64));
        }
    });

    sr
}

#[derive(Debug, Clone)]
pub enum TrackerSyncMsg {
    AddTrack(Track),
    SetValue(usize, usize, f32),
    SetNote(usize, usize, u8),
    SetA(usize, usize, u8),
    SetB(usize, usize, u8),
    SetInt(usize, usize, Interpolation),
    RemoveValue(usize, usize),
    PlayHead(PlayHeadAction),
    DeserializeContents(usize, TrackSerialized),
}

pub struct ThreadTrackSync {
    send: std::sync::mpsc::Sender<TrackerSyncMsg>,
}

impl ThreadTrackSync {
    pub fn new(send: std::sync::mpsc::Sender<TrackerSyncMsg>) -> Self {
        ThreadTrackSync { send }
    }
}

impl TrackerSync for ThreadTrackSync {
    fn add_track(&mut self, t: Track) {
        self.send.send(TrackerSyncMsg::AddTrack(t))
            .expect("tracker thread communication");
    }
    fn set_int(&mut self, track_idx: usize, line: usize, int: Interpolation) {
        self.send.send(TrackerSyncMsg::SetInt(track_idx, line, int))
            .expect("tracker thread communication");
    }
    fn set_value(&mut self, track_idx: usize, line: usize, value: f32) {
        self.send.send(TrackerSyncMsg::SetValue(track_idx, line, value))
            .expect("tracker thread communication");
    }
    fn set_note(&mut self, track_idx: usize, line: usize, value: u8) {
        self.send.send(TrackerSyncMsg::SetNote(track_idx, line, value))
            .expect("tracker thread communication");
    }
    fn set_a(&mut self, track_idx: usize, line: usize, value: u8) {
        self.send.send(TrackerSyncMsg::SetA(track_idx, line, value))
            .expect("tracker thread communication");
    }
    fn set_b(&mut self, track_idx: usize, line: usize, value: u8) {
        self.send.send(TrackerSyncMsg::SetB(track_idx, line, value))
            .expect("tracker thread communication");
    }
    fn remove_value(&mut self, track_idx: usize, line: usize) {
        self.send.send(TrackerSyncMsg::RemoveValue(track_idx, line))
            .expect("tracker thread communication");
    }
    fn play_head(&mut self, act: PlayHeadAction) {
        self.send.send(TrackerSyncMsg::PlayHead(act))
            .expect("tracker thread communication");
    }
    fn deserialize_contents(&mut self, track_idx: usize, contents: TrackSerialized) {
        self.send.send(TrackerSyncMsg::DeserializeContents(track_idx, contents))
            .expect("tracker thread communication");
    }
}

