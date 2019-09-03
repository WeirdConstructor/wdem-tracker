use std::sync::{Arc, Mutex, Condvar};

struct AudioQueue {
    backend_ready:      bool,
    sample_rate:        usize,
    samples_per_period: usize,
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
                samples_per_period: 0,
                samples:            std::collections::VecDeque::new(),
            }),
            cv: Condvar::new(),
            cv_put: Condvar::new(),
        }
    }

    pub fn backend_ready(&mut self, sample_rate: usize, samples_per_period: usize) {
        let mut aq = self.mx.lock().unwrap();
        if aq.backend_ready { return; }

        aq.sample_rate        = sample_rate;
        aq.samples_per_period = samples_per_period;
        aq.backend_ready      = true;
        self.cv.notify_one();
    }

    pub fn get_stereo_samples(&mut self, stereo_out: &mut [f32]) {
        let mut aq = self.mx.lock().unwrap();
        while aq.samples.len() < stereo_out.len() {
            aq = self.cv_put.wait(aq).unwrap();
        }

        let len = stereo_out.len();
        for (i, s) in aq.samples.drain(0..len).enumerate() {
            stereo_out[i] = s;
        }
    }
}

pub struct AudioFrontend {
    dev:                Arc<AudioDev>,
    max_buffer_fill:    usize,
    sample_rate:        usize,
    samples_per_period: usize,
}

impl AudioFrontend {
    pub fn new() -> Self {
        AudioFrontend {
            dev:                Arc::new(AudioDev::new()),
            sample_rate:        0,
            samples_per_period: 0,
            max_buffer_fill:    0,
        }
    }

    pub fn wait_backend_ready(&mut self) {
        let mut ad = self.dev.mx.lock().unwrap();
        while !ad.backend_ready {
            ad = self.dev.cv.wait(ad).unwrap();
        }

        self.sample_rate        = ad.sample_rate;
        self.samples_per_period = ad.samples_per_period;

        self.max_buffer_fill =
            (self.samples_per_period as f64 * 1.5) as usize;
    }

    pub fn get_sample_rate(&self) -> usize { self.sample_rate }

    pub fn get_latency_in_samples(&self) -> usize {
        // div by 2 cause of stereo out
        self.max_buffer_fill / 2
    }

    pub fn put_samples_blocking(&mut self, buf: &[f32]) {
        let mut ad = self.dev.mx.lock().unwrap();
        while ad.samples.len() >= self.max_buffer_fill {
            ad = self.dev.cv.wait(ad).unwrap();
        }

        for s in buf.iter() { ad.samples.push_back(*s); }
    }
}
