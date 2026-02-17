//! End-to-end latency measurer built on top of `LatencyMeasurer`.
//!
//! It provides a zero-allocation tracker based on a monotonic start time,
//! suitable for measuring cross-stage latencies.
use crate::measure::LatencyMeasurer;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Monotonic start time used to compute relative nanoseconds.
pub static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Measures end-to-end latencies between `add_tracker` and `measure` calls.
pub struct E2ELatencyMeasurer {
    pub measurer: LatencyMeasurer,
}

impl E2ELatencyMeasurer {
    /// Creates a new measurer with the given sampling rate.
    pub fn new(sample_size: u64) -> Self {
        E2ELatencyMeasurer {
            measurer: LatencyMeasurer::new(sample_size),
        }
    }

    /// Returns nanoseconds elapsed since process start.
    #[inline(always)]
    pub fn nanos_since_start() -> u64 {
        START_TIME.elapsed().as_nanos() as u64
    }

    /// Starts a latency measurement and returns a tracker token.
    pub fn add_tracker(&self) -> u64 {
        Self::nanos_since_start()
    }

    /// Completes the measurement using the given tracker token.
    pub fn measure(&mut self, tracker: u64) {
        let nanos = Self::nanos_since_start() - tracker;
        self.measurer.measure(Duration::from_nanos(nanos));
    }
}
