use crate::measure::LatencyMeasurer;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

pub static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

pub struct E2ELatencyMeasurer {
    pub measurer: LatencyMeasurer,
}

impl E2ELatencyMeasurer {
    pub fn new(sample_size: u64) -> Self {
        E2ELatencyMeasurer {
            measurer: LatencyMeasurer::new(sample_size),
        }
    }

    #[inline(always)]
    pub fn get_relative_nanos() -> u64 {
        START_TIME.elapsed().as_nanos() as u64
    }

    pub fn add_tracker(&self) -> u64 {
        Self::get_relative_nanos()
    }

    pub fn measure(&mut self, tracker: u64) {
        let nanos = Self::get_relative_nanos() - tracker;
        self.measurer.measure(Duration::from_nanos(nanos));
    }
}
