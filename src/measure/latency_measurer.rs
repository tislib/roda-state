use hdrhistogram::Histogram;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    pub count: u64,
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub p50: u64,
    pub p90: u64,
    pub p99: u64,
    pub p999: u64,
    pub p9999: u64,
}

pub struct LatencyMeasurerGuard<'a> {
    measurer: &'a mut LatencyMeasurer,
    start: Option<Instant>,
}

impl Drop for LatencyMeasurerGuard<'_> {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            self.measurer.measure_local(start.elapsed());
        }
    }
}

/// A latency measurer that uses hdrhistogram.
pub struct LatencyMeasurer {
    histogram: Histogram<u64>,
    sum: u64,
    step_instant: Instant,
    sample_rate: u64,
    step: u64,
}

impl LatencyMeasurer {
    pub fn new(sample_rate: u64) -> Self {
        assert!(sample_rate > 0, "sample_rate must be positive");
        // Range: 1ns to 1,000s (1,000,000,000,000 ns)
        // 3 significant figures
        let histogram = Histogram::<u64>::new_with_bounds(1, 1_000_000_000_000, 3).unwrap();
        Self {
            histogram,
            sum: 0,
            sample_rate,
            step_instant: Instant::now(),
            step: 0,
        }
    }

    pub fn measure(&mut self, duration: Duration) {
        self.step += 1;
        if !self.step.is_multiple_of(self.sample_rate) {
            return;
        }

        self.measure_local(duration);
    }

    fn measure_local(&mut self, duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        let nanos = nanos.clamp(1, 1_000_000_000_000);

        self.histogram.record(nanos).unwrap();
        self.sum += nanos;
    }

    pub fn measure_with_guard(&mut self) -> LatencyMeasurerGuard<'_> {
        self.step += 1;
        if !self.step.is_multiple_of(self.sample_rate) {
            return LatencyMeasurerGuard {
                measurer: self,
                start: None,
            };
        }
        LatencyMeasurerGuard {
            measurer: self,
            start: Some(Instant::now()),
        }
    }

    pub fn step_measure(&mut self) {
        self.step += 1;
        if !self.step.is_multiple_of(self.sample_rate) {
            return;
        }
        let elapsed = self.step_instant.elapsed();
        self.measure(elapsed);
        self.step_instant = Instant::now();
    }

    pub fn reset(&mut self) {
        self.histogram.reset();
        self.sum = 0;
    }

    pub fn get_stats(&self) -> LatencyStats {
        let count = self.histogram.len();
        if count == 0 {
            return LatencyStats::default();
        }

        LatencyStats {
            count,
            min: self.histogram.min(),
            max: self.histogram.max(),
            mean: self.histogram.mean(),
            p50: self.histogram.value_at_quantile(0.5),
            p90: self.histogram.value_at_quantile(0.9),
            p99: self.histogram.value_at_quantile(0.99),
            p999: self.histogram.value_at_quantile(0.999),
            p9999: self.histogram.value_at_quantile(0.9999),
        }
    }

    pub fn format_stats(&self) -> String {
        let stats = self.get_stats();
        if stats.count == 0 {
            return "No stats collected yet".into();
        }

        format!(
            "\tmin={},\tmax={},\tmean={},\tp50={},\tp90={},\tp99={},\tp999={},\tp9999={}",
            Self::format_duration(stats.min as f64),
            Self::format_duration(stats.max as f64),
            Self::format_duration(stats.mean),
            Self::format_duration(stats.p50 as f64),
            Self::format_duration(stats.p90 as f64),
            Self::format_duration(stats.p99 as f64),
            Self::format_duration(stats.p999 as f64),
            Self::format_duration(stats.p9999 as f64),
        )
    }

    fn format_duration(nanos: f64) -> String {
        if nanos < 1000.0 {
            if nanos == nanos.floor() {
                format!("{:.0}ns", nanos)
            } else {
                format!("{:.1}ns", nanos)
            }
        } else if nanos < 1_000_000.0 {
            let val = nanos / 1000.0;
            if val == val.floor() {
                format!("{:.0}us", val)
            } else {
                format!("{:.1}us", val)
            }
        } else if nanos < 1_000_000_000.0 {
            let val = nanos / 1_000_000.0;
            if val == val.floor() {
                format!("{:.0}ms", val)
            } else {
                let s = format!("{:.2}ms", val);
                if s.ends_with("0ms") {
                    format!("{:.1}ms", val)
                } else {
                    s
                }
            }
        } else {
            let val = nanos / 1_000_000_000.0;
            if val == val.floor() {
                format!("{:.0}s", val)
            } else {
                let s = format!("{:.2}s", val);
                if s.ends_with("0s") {
                    format!("{:.1}s", val)
                } else {
                    s
                }
            }
        }
    }

    pub fn is_outlier(&self, duration: Duration) -> bool {
        let stats = self.get_stats();
        if stats.count < 100 {
            return false;
        }
        duration.as_nanos() as u64 > stats.p999
    }
}
