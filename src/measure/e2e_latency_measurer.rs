use std::sync::LazyLock;
use std::time::{Duration, Instant};
use crate::measure::LatencyMeasurer;

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
    fn get_relative_nanos() -> u64 {
        START_TIME.elapsed().as_nanos() as u64
    }

    fn add_tracker(&self) -> u64 {
        Self::get_relative_nanos()
    }

    fn measure(&mut self, tracker: u64) {
        let nanos = Self::get_relative_nanos() - tracker;
        self.measurer.measure(Duration::from_nanos(nanos));
    }
}