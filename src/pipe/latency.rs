use crate::measure::latency_measurer::LatencyMeasurer;
use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use spdlog::info;
use std::marker::PhantomData;

/// A pipe that measures the latency of an inner stage.
pub struct Latency<In, Out, S> {
    name: String,
    report_interval: usize,
    stage: S,
    measurer: LatencyMeasurer,
    count: usize,
    _phantom: PhantomData<(In, Out)>,
}

impl<In, Out, S> Latency<In, Out, S>
where
    In: Pod + Send,
    Out: Pod + Send,
    S: Stage<In, Out>,
{
    pub fn new(
        name: impl Into<String>,
        report_interval: usize,
        sample_rate: u64,
        stage: S,
    ) -> Self {
        Latency {
            name: name.into(),
            report_interval,
            stage,
            measurer: LatencyMeasurer::new(sample_rate),
            count: 0,
            _phantom: PhantomData,
        }
    }
}

impl<In, Out, S> Stage<In, Out> for Latency<In, Out, S>
where
    In: Pod + Send,
    Out: Pod + Send,
    S: Stage<In, Out>,
{
    #[inline(always)]
    fn process<C>(&mut self, data: &In, collector: &mut C)
    where
        C: OutputCollector<Out>,
    {
        {
            let _guard = self.measurer.measure_with_guard();
            self.stage.process(data, collector);
        }
        self.count += 1;
        if self.count % self.report_interval == 0 {
            info!("[{}] Latency: {}", self.name, self.measurer.format_stats());
        }
    }
}

pub fn latency<In, Out, S>(
    name: impl Into<String>,
    interval: usize,
    example_size: usize,
    stage: S,
) -> Latency<In, Out, S>
where
    In: Pod + Send,
    Out: Pod + Send,
    S: Stage<In, Out>,
{
    Latency::new(name, interval, example_size as u64, stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_latency_logic() {
        let mut pipe = latency("test", 2, 1, |x: &u32| {
            thread::sleep(Duration::from_millis(10));
            Some(*x as u64)
        });

        let mut out = Vec::new();

        // Process 1st item
        {
            let mut collector = |x: &u64| out.push(*x);
            pipe.process(&1u32, &mut collector);
        }
        assert_eq!(out, vec![1]);

        // Process 2nd item - should trigger print
        {
            let mut collector = |x: &u64| out.push(*x);
            pipe.process(&2u32, &mut collector);
        }
        assert_eq!(out, vec![1, 2]);

        let stats = pipe.measurer.get_stats();
        assert_eq!(stats.count, 2);
        assert!(stats.min >= 10_000_000); // at least 10ms in nanos
    }
}
