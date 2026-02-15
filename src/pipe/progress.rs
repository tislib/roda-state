use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use spdlog::info;
use std::marker::PhantomData;
use std::time::Instant;

/// A pipe that logs progress information.
pub struct Progress<T> {
    name: String,
    interval: usize,
    count: usize,
    last_instant: Instant,
    start_instant: Instant,
    _phantom: PhantomData<T>,
}

impl<T: Pod + Send> Progress<T> {
    pub fn new(name: impl Into<String>, interval: usize) -> Self {
        assert!(interval > 0, "interval must be greater than 0");
        let now = Instant::now();
        Self {
            name: name.into(),
            interval,
            count: 0,
            last_instant: now,
            start_instant: now,
            _phantom: PhantomData,
        }
    }
}

impl<T: Pod + Send> Stage<T, T> for Progress<T> {
    #[inline(always)]
    fn process<C>(&mut self, data: &T, collector: &mut C)
    where
        C: OutputCollector<T>,
    {
        self.count += 1;
        if self.count.is_multiple_of(self.interval) {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_instant);
            let total_elapsed = now.duration_since(self.start_instant);

            let mps = self.interval as f64 / elapsed.as_secs_f64();
            let total_mps = self.count as f64 / total_elapsed.as_secs_f64();

            info!(
                "[{}] Processed {} messages, Rate: {} msg/s, Avg: {} msg/s",
                self.name,
                format_count(self.count as f64),
                format_count(mps),
                format_count(total_mps)
            );
            self.last_instant = now;
        }
        collector.push(data);
    }
}

pub fn progress<T: Pod + Send>(name: impl Into<String>, interval: usize) -> Progress<T> {
    Progress::new(name, interval)
}

fn format_count(val: f64) -> String {
    if val < 1000.0 {
        if val == val.floor() {
            format!("{:.0}", val)
        } else {
            format!("{:.2}", val)
        }
    } else if val < 1_000_000.0 {
        format!("{:.2}k", val / 1000.0)
    } else if val < 1_000_000_000.0 {
        format!("{:.2}m", val / 1_000_000.0)
    } else if val < 1_000_000_000_000.0 {
        format!("{:.2}b", val / 1_000_000_000.0)
    } else {
        format!("{:.2}t", val / 1_000_000_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_progress_logic() {
        let mut pipe = progress::<u32>("test", 2);
        let mut out = Vec::new();

        // Process 1st item
        pipe.process(&1u32, &mut |x: &u32| out.push(*x));
        assert_eq!(out, vec![1]);

        // Process 2nd item - should trigger print
        thread::sleep(Duration::from_millis(10));
        pipe.process(&2u32, &mut |x: &u32| out.push(*x));
        assert_eq!(out, vec![1, 2]);

        // Process 3rd item
        pipe.process(&3u32, &mut |x: &u32| out.push(*x));
        assert_eq!(out, vec![1, 2, 3]);

        // Process 4th item - should trigger print
        thread::sleep(Duration::from_millis(10));
        pipe.process(&4u32, &mut |x: &u32| out.push(*x));
        assert_eq!(out, vec![1, 2, 3, 4]);
    }
}
