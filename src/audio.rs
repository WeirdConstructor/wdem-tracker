use std::sync::{Arc, Mutex, Condvar};

struct AudioQueue {
    backend_ready:      bool,
    sample_rate:        usize,
    ringbuf_len:        usize,
    samples:            std::collections::VecDeque<f32>,
}

pub struct AudioDev {
    mx: Mutex<AudioQueue>,
    cv: Condvar,
    cv_put: Condvar,
}

impl AudioDev {
    pub fn new() -> Self {
        AudioDev {
            mx: Mutex::new(AudioQueue {
                backend_ready:      false,
                sample_rate:        0,
                ringbuf_len: 0,
                samples:            std::collections::VecDeque::new(),
            }),
            cv: Condvar::new(),
            cv_put: Condvar::new(),
        }
    }

    pub fn backend_ready(&self, sample_rate: usize, ringbuf_len: usize) {
        let mut aq = self.mx.lock().unwrap();
        if aq.backend_ready { return; }

        aq.sample_rate        = sample_rate;
        aq.ringbuf_len        = ringbuf_len;
        aq.backend_ready      = true;
        self.cv.notify_one();
    }

    pub fn get_stereo_samples(&self, stereo_out: &mut [f32]) {
        let mut aq = self.mx.lock().unwrap();
        let len =
            if stereo_out.len() < aq.ringbuf_len {
                stereo_out.len()
            } else {
                aq.ringbuf_len
            };

        while aq.samples.len() < len {
            aq = self.cv_put.wait(aq).unwrap();
        }

        for (i, s) in aq.samples.drain(0..len).enumerate() {
            stereo_out[i] = s;
        }
        if stereo_out.len() > len {
            for i in len..stereo_out.len() {
                stereo_out[i] = 0.0;
            }
        }
        self.cv_put.notify_one();
    }
}

pub struct AudioFrontend {
    dev:                Arc<AudioDev>,
    sample_rate:        usize,
    ringbuf_len:        usize,
}

impl AudioFrontend {
    pub fn new() -> Self {
        AudioFrontend {
            dev:                Arc::new(AudioDev::new()),
            sample_rate:        0,
            ringbuf_len:        0,
        }
    }

    pub fn get_dev(&self) -> Arc<AudioDev> { self.dev.clone() }

    pub fn wait_backend_ready(&mut self) {
        let mut ad = self.dev.mx.lock().unwrap();
        while !ad.backend_ready {
            ad = self.dev.cv.wait(ad).unwrap();
        }

        self.sample_rate = ad.sample_rate;
        self.ringbuf_len = ad.ringbuf_len;
    }

    pub fn get_sample_rate(&self) -> usize { self.sample_rate }

    pub fn get_latency_in_samples(&self) -> usize {
        // div by 2 cause of stereo out
        self.ringbuf_len / 2
    }

    pub fn put_samples_blocking(&mut self, buf: &[f32]) {
        let mut ad = self.dev.mx.lock().unwrap();
        while ad.samples.len() >= self.ringbuf_len {
            ad = self.dev.cv_put.wait(ad).unwrap();
        }

        for s in buf.iter() { ad.samples.push_back(*s); }
        self.dev.cv_put.notify_one();
    }
}
