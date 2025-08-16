use core::time::Duration;
use spin::RwLock;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

pub struct Clock {
    base_time: AtomicU64,
    current_time: AtomicU64,
    rate: RwLock<f64>,
    running: AtomicBool,
    paused_time: AtomicU64,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            base_time: AtomicU64::new(0),
            current_time: AtomicU64::new(0),
            rate: RwLock::new(1.0),
            running: AtomicBool::new(false),
            paused_time: AtomicU64::new(0),
        }
    }

    pub fn start(&self) {
        self.base_time.store(Self::get_system_time(), Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.paused_time.store(self.get_time(), Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.base_time.store(0, Ordering::SeqCst);
        self.current_time.store(0, Ordering::SeqCst);
        self.paused_time.store(0, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);
        *self.rate.write() = 1.0;
    }

    pub fn get_time(&self) -> u64 {
        if !self.running.load(Ordering::SeqCst) {
            return self.paused_time.load(Ordering::SeqCst);
        }

        let base = self.base_time.load(Ordering::SeqCst);
        if base == 0 {
            return 0;
        }

        let current = Self::get_system_time();
        let elapsed = current - base;
        let rate = *self.rate.read();
        
        (elapsed as f64 * rate) as u64
    }

    pub fn set_rate(&self, rate: f64) {
        if rate > 0.0 {
            *self.rate.write() = rate;
        }
    }

    pub fn get_rate(&self) -> f64 {
        *self.rate.read()
    }

    pub fn wait_until(&self, target_time: u64) {
        while self.get_time() < target_time && self.running.load(Ordering::SeqCst) {
            core::hint::spin_loop();
        }
    }

    pub fn to_stream_time(&self, buffer_time: u64) -> u64 {
        let clock_time = self.get_time();
        buffer_time.saturating_add(clock_time)
    }

    pub fn to_buffer_time(&self, stream_time: u64) -> u64 {
        let clock_time = self.get_time();
        stream_time.saturating_sub(clock_time)
    }

    fn get_system_time() -> u64 {
        #[cfg(feature = "std")]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        }
        #[cfg(not(feature = "std"))]
        {
            0
        }
    }
}

pub struct ClockSync {
    master_clock: Option<Clock>,
    slave_clocks: RwLock<Vec<Clock>>,
    sync_threshold: Duration,
}

impl ClockSync {
    pub fn new() -> Self {
        Self {
            master_clock: None,
            slave_clocks: RwLock::new(Vec::new()),
            sync_threshold: Duration::from_millis(40),
        }
    }

    pub fn set_master(&mut self, clock: Clock) {
        self.master_clock = Some(clock);
    }

    pub fn add_slave(&self, clock: Clock) {
        self.slave_clocks.write().push(clock);
    }

    pub fn sync(&self) {
        if let Some(ref master) = self.master_clock {
            let master_time = master.get_time();
            
            for slave in self.slave_clocks.read().iter() {
                let slave_time = slave.get_time();
                let diff = (master_time as i64 - slave_time as i64).abs();
                
                if diff > self.sync_threshold.as_nanos() as i64 {
                    slave.current_time.store(master_time, Ordering::SeqCst);
                }
            }
        }
    }
}

pub struct Timestamp {
    pts: Option<i64>,
    dts: Option<i64>,
    duration: Option<i64>,
    timebase: Rational,
}

impl Timestamp {
    pub fn new(timebase: Rational) -> Self {
        Self {
            pts: None,
            dts: None,
            duration: None,
            timebase,
        }
    }

    pub fn set_pts(&mut self, pts: i64) {
        self.pts = Some(pts);
    }

    pub fn set_dts(&mut self, dts: i64) {
        self.dts = Some(dts);
    }

    pub fn set_duration(&mut self, duration: i64) {
        self.duration = Some(duration);
    }

    pub fn pts_time(&self) -> Option<Duration> {
        self.pts.map(|pts| self.to_duration(pts))
    }

    pub fn dts_time(&self) -> Option<Duration> {
        self.dts.map(|dts| self.to_duration(dts))
    }

    pub fn duration_time(&self) -> Option<Duration> {
        self.duration.map(|d| self.to_duration(d))
    }

    fn to_duration(&self, timestamp: i64) -> Duration {
        let nanos = (timestamp as i128 * self.timebase.num as i128 * 1_000_000_000) 
            / self.timebase.den as i128;
        Duration::from_nanos(nanos as u64)
    }

    pub fn rescale(&self, new_timebase: Rational) -> Self {
        let scale = (self.timebase.num as i64 * new_timebase.den as i64) 
            / (self.timebase.den as i64 * new_timebase.num as i64);
        
        Self {
            pts: self.pts.map(|p| p * scale),
            dts: self.dts.map(|d| d * scale),
            duration: self.duration.map(|d| d * scale),
            timebase: new_timebase,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rational {
    pub num: u32,
    pub den: u32,
}

impl Rational {
    pub fn new(num: u32, den: u32) -> Self {
        let gcd = Self::gcd(num, den);
        Self {
            num: num / gcd,
            den: den / gcd,
        }
    }

    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 {
            a
        } else {
            Self::gcd(b, a % b)
        }
    }

    pub fn as_f64(&self) -> f64 {
        self.num as f64 / self.den as f64
    }
}