use std::sync::{Arc, Mutex, Condvar};


struct AudioQueue {
    backend_ready:      bool,
    sample_rate:        usize,
    samples_per_period: usize,
    bufs:               std::collections::VecDeque<Vec<f32>>,
    empty_bufs:         Vec<Vec<f32>>,
    cur_buf:            Option<Vec<f32>>,
    cur_buf_idx:        usize,
}

struct AudioDev {
    mx: Mutex<AudioQueue>,
    cv: Condvar,
    cv_put: Condvar,
}

impl AudioDev {
    fn new() -> Self {
        AudioDev {
            mx: Mutex::new(AudioQueue {
                backend_ready:      false,
                sample_rate:        0,
                samples_per_period: 0,
                bufs:               std::collections::VecDeque::new(),
                empty_bufs:         Vec::new(),
                cur_buf:            None,
            }),
            cv: Condvar::new(),
            cv_put: Condvar::new(),
        }
    }

    fn backend_ready(&mut self, sample_rate: usize, samples_per_period: usize) {
        let mut aq = self.mx.lock().unwrap();
        if aq.backend_ready { return; }

        aq.samples_rate       = samples_rate;
        aq.samples_per_period = samples_per_period;
        aq.backend_ready      = true;
        self.cv.notify_one();
    }

    fn next_stereo_sample(&mut self, sidx: &mut usize, b: &mut Box<Vec<f32>>) -> (f32, f32) {
        if b.empty() {
            let mut aq = self.mx.lock().unwrap();
            while aq.bufs.empty() {
                self.cv_put.wait(aq).unwrap();
            }

            std::mem::replace(b.as_mut(), aq.bufs.pop_front().unwrap());
            *sidx = 0;

        } else if *sidx >= b.len() {
            let mut aq = self.mx.lock().unwrap();
            while aq.bufs.empty() {
                self.cv_put.wait(aq).unwrap();
            }

            let v = std::mem::replace(b.as_mut(), aq.bufs.pop_front().unwrap());
            *sidx = 0;
            aq.empty_bufs.push(v);
        }

        let r = (b[*sidx], b[*sidx + 1]);
        *sidx += 2;
        r
    }

    fn fill_backend_buffer(&mut self, buf: [
}

struct AudioFrontend {
    dev: Arc<AudioDev>,
    put_samples_interval_ms: usize,
    sample_rate: usize,
    audio_buf_size: usize,
    internal_buf_count: usize,
}

impl AudioFrontend {
    fn new(put_samples_interval_ms: usize) -> Self {
        AudioFrontend {
            dev:                Arc::new(AudioDev::new()),
            sample_rate:        0,
            put_samples_interval_ms,
            internal_buf_count: 0,
            audio_buf_size:     0,
        }
    }

    fn wait_backend_ready(&mut self) {
        let mut ad = self.dev.mx.lock().unwrap();
        while !ad.backend_ready {
            self.dev.cv.wait(ad).unwrap();
        }

        self.sample_rate = ad.sample_rate;

        let samples_per_interval =
            ((ad.sample_rate * self.put_samples_interval_ms) as f64).ceil()
            / 1000.0;

        self.audio_buf_size = samples_per_interval;

        self.internal_buf_count =
            ((1.5 * (ad.samples_per_period as f64))
            / (self.audio_buf_size as f64)).ceil() as usize;

        ad.empty_bufs.resize(
            self.internal_buf_count,
            [Vec::new(), Vec::new()]);
        for b in ad.empty_bufs.iter_mut() {
            b[0].resize(self.audio_buf_size, 0.0);
            b[1].resize(self.audio_buf_size, 0.0);
        }
    }

    fn get_sample_rate(&self) -> usize { self.sample_rate }

    fn put_samples_blocking(buf: &[Vec<f32>; 2]) {
        let mut ad = self.dev.mx.lock().unwrap();
        while !ad.empty_bufs.empty() {
            self.dev.cv.wait(ad).unwrap();
        }

        let buf = ad.empty_bufs.pop().unwrap();
        buf[0][..].copy_from_slice(&buf[0][..]);
        buf[1][..].copy_from_slice(&buf[1][..]);
        ad.bufs.push_back(buf);
    }
}

