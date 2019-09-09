use crate::audio::*;
use std::sync::Arc;

pub fn start_audio_thread(audio_dev: Arc<AudioDev>) {

    let ad = audio_dev.clone();
    std::thread::spawn(move || {
        use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
        let host = cpal::default_host();
        let event_loop = host.event_loop();
        let device = host.default_output_device().expect("no output device available");
        let format = device.default_output_format().expect("proper default format");
        println!("FORMAT: {:?}", format);
        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
        event_loop.play_stream(stream_id).expect("failed to play_stream");

        let sample_rate = if let cpal::SampleRate(r) = format.sample_rate {
            r
        } else {
            44100
        };

        let channels = format.channels as usize;

        let mut avg_buf_len = 0;
        let mut avg_buf_cnt = 0;
        let avg_buf_len_samples = 10;
        let mut startup = true;

        let mut last_call_instant = std::time::Instant::now();
        let mut cnt = 0;

        use cpal::{StreamData, UnknownTypeOutputBuffer};
        event_loop.run(move |stream_id, stream_result| {
            let stream_data = match stream_result {
                Ok(data) => data,
                Err(err) => {
                    eprintln!("an error occurred on stream {:?}: {}", stream_id, err);
                    return;
                }
            };

            match stream_data {
                StreamData::Output { buffer: UnknownTypeOutputBuffer::U16(mut buffer) } => {
                    println!("FOFOE3");
                    for elem in buffer.iter_mut() {
                        *elem = u16::max_value() / 2;
                    }
                },
                StreamData::Output { buffer: UnknownTypeOutputBuffer::I16(mut buffer) } => {
                    println!("FOFOE2");
                    for elem in buffer.iter_mut() {
                        *elem = 0;
                    }
                },
                StreamData::Output { buffer: UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    if startup {
                        if avg_buf_cnt < avg_buf_len_samples {
                            avg_buf_len += buffer.len();
                            avg_buf_cnt += 1;

                            for elem in buffer.iter_mut() {
                                *elem = 0.0;
                            }

                            return;
                        } else {
                            audio_dev.backend_ready(
                                sample_rate as usize,
                                ((avg_buf_len / avg_buf_cnt) as f64 * 1.5).ceil() as usize);
                            println!("AVG BUF SIZE: {}", avg_buf_len / avg_buf_cnt);
                            startup = false;
                        }
                    }
                    let m = std::time::Instant::now();

                    audio_dev.get_stereo_samples(&mut buffer);

                    cnt += 1;
                    if cnt % 200 == 0 {
                        println!("Audio time ms: cycle={}us, wait={}us ",
                                last_call_instant.elapsed().as_micros(),
                                m.elapsed().as_micros());
                    }
                    last_call_instant = std::time::Instant::now();

//                    for elem in buffer.iter_mut() {
//                        *elem = 0.0;
//                    }
                },
                _ => (),
            }
        });
    });
}

