use bytemuck::{Pod, Zeroable};
use criterion::{Criterion, criterion_group, criterion_main};
use roda_state::StageEngine;
use roda_state::pipe;
use roda_state::{delta, stateful};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Reading {
    pub sensor_id: u64,
    pub value: f64,
    pub timestamp: u64,
}

impl Reading {
    pub fn from(sensor_id: u64, value: f64, timestamp: u64) -> Self {
        Self {
            sensor_id,
            value,
            timestamp,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Summary {
    pub sensor_id: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: u64,
    pub timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SensorKey {
    pub sensor_id: u64,
    pub timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Alert {
    pub sensor_id: u64,
    pub timestamp: u64,
    pub severity: i32,
    pub _pad0: i32,
}

impl SensorKey {
    #[inline(always)]
    pub fn from_reading(r: &Reading) -> Self {
        Self {
            sensor_id: r.sensor_id,
            timestamp: (r.timestamp / 100_000) * 100_000,
        }
    }
}

impl Summary {
    #[inline(always)]
    pub fn init(r: &Reading) -> Self {
        Self {
            sensor_id: r.sensor_id,
            min: r.value,
            max: r.value,
            avg: r.value,
            count: 1,
            timestamp: (r.timestamp / 100_000) * 100_000,
        }
    }

    #[inline(always)]
    pub fn update(&mut self, r: &Reading) {
        if r.value < self.min {
            self.min = r.value;
        }
        if r.value > self.max {
            self.max = r.value;
        }
        self.avg = (self.avg * self.count as f64 + r.value) / (self.count + 1) as f64;
        self.count += 1;
    }
}

fn bench_sensor_pipeline(c: &mut Criterion) {
    let num_readings = 1_000_000;
    let num_sensors = 1000;

    let mut readings = Vec::with_capacity(num_readings);
    for i in 0..num_readings {
        let sensor_id = (i % num_sensors) as u64;
        let value = if i > 0 && i % 1000 == 0 {
            50.0
        } else {
            10.0 + (i as f64 * 0.0001)
        };
        readings.push(Reading::from(sensor_id, value, i as u64 * 10_000));
    }

    let mut group = c.benchmark_group("sensor_pipeline");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("stage_engine", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;
            for _ in 0..iters {
                let engine = StageEngine::<Reading, Reading>::with_capacity(num_readings + 1000);
                let mut engine = engine
                    .add_stage_with_capacity(
                        num_readings + 1000,
                        pipe![stateful(
                            SensorKey::from_reading,
                            Summary::init,
                            |state, r| state.update(r)
                        )],
                    )
                    .add_stage_with_capacity(
                        num_readings + 1000,
                        pipe![delta(
                            |s: &Summary| s.sensor_id,
                            |curr, prev| {
                                if let Some(p) = prev
                                    && curr.avg > p.avg * 1.5
                                {
                                    return Some(Alert {
                                        sensor_id: curr.sensor_id,
                                        timestamp: curr.timestamp,
                                        severity: 1,
                                        ..Default::default()
                                    });
                                }
                                None
                            }
                        )],
                    );

                let start = Instant::now();
                for &r in &readings {
                    engine.send(&r);
                }
                engine.await_idle(Duration::from_secs(5));
                total_duration += start.elapsed();

                // Drain alerts
                while let Some(alert) = engine.try_receive() {
                    black_box(alert);
                }
            }
            total_duration
        });
    });

    group.bench_function("pure_rust", |b| {
        b.iter(|| {
            let mut summaries: HashMap<SensorKey, Summary> = HashMap::new();
            let mut last_summaries: HashMap<u64, Summary> = HashMap::new();
            let mut alerts = Vec::new();

            for &r in &readings {
                let key = SensorKey::from_reading(&r);
                let summary = summaries.entry(key).or_insert_with(|| Summary::init(&r));

                summary.update(&r);
                let curr_summary = *summary;

                if let Some(prev) = last_summaries.get(&r.sensor_id)
                    && curr_summary.avg > prev.avg * 1.5
                {
                    alerts.push(Alert {
                        sensor_id: curr_summary.sensor_id,
                        timestamp: curr_summary.timestamp,
                        severity: 1,
                        ..Default::default()
                    });
                }
                last_summaries.insert(r.sensor_id, curr_summary);
            }
            black_box(alerts);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_sensor_pipeline);
criterion_main!(benches);
