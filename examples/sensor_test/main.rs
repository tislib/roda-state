use bytemuck::{Pod, Zeroable};
use roda_state::JournalStoreOptions;
use roda_state::components::{Appendable, IterativeReadable};
use roda_state::{Aggregator, RodaEngine, Window};
use std::thread;
use std::time::Duration;

/// Raw sensor reading
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

/// Statistical summary of readings for a time window
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

/// Key used for partitioning and indexing summaries
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SensorKey {
    pub sensor_id: u64,
    pub timestamp: u64,
}

/// Alert generated when an anomaly is detected
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Alert {
    pub sensor_id: u64,
    pub timestamp: u64,
    pub severity: i32,
    pub _pad0: i32,
}

fn main() {
    let mut engine = RodaEngine::new();

    // 1. SETUP STORES
    // Stores are bounded, pre-allocated buffers for your state.
    let mut reading_store = engine.new_journal_store::<Reading>(JournalStoreOptions {
        name: "readings",
        size: 1000,
        in_memory: true,
    });
    let reading_reader = reading_store.reader();

    let mut summary_store = engine.new_journal_store::<Summary>(JournalStoreOptions {
        name: "summaries",
        size: 100,
        in_memory: true,
    });
    let summary_reader = summary_store.reader();

    let mut alert_store = engine.new_journal_store::<Alert>(JournalStoreOptions {
        name: "alerts",
        size: 100,
        in_memory: true,
    });
    let alert_reader_for_print = alert_store.reader();

    // Secondary index to look up summaries by sensor and time
    let summary_index = summary_store.direct_index::<SensorKey>();
    let summary_index_reader = summary_index.reader();

    // 2. DEFINE PIPELINES
    let summary_pipeline: Aggregator<Reading, Summary, SensorKey> = Aggregator::new();
    let alert_pipeline: Window<Summary, Alert> = Window::new();

    // 3. WORKER: Aggregate readings into summaries
    engine.run_worker(move || {
        reading_reader.next(); // Wait for data

        summary_pipeline
            .from(&reading_reader)
            .to(&mut summary_store)
            .partition_by(|r| SensorKey {
                sensor_id: r.sensor_id,
                timestamp: r.timestamp / 100_000,
            })
            .reduce(|idx, r, s, _keep| {
                if idx == 0 {
                    *s = Summary {
                        sensor_id: r.sensor_id,
                        min: r.value,
                        max: r.value,
                        avg: r.value,
                        count: 1,
                        timestamp: (r.timestamp / 100_000) * 100_000,
                    };
                } else {
                    s.min = s.min.min(r.value);
                    s.max = s.max.max(r.value);
                    s.avg = (s.avg * s.count as f64 + r.value) / (s.count + 1) as f64;
                    s.count += 1;
                }
            });

        // Update the index so summaries can be found by key
        summary_index.compute(|s| SensorKey {
            sensor_id: s.sensor_id,
            timestamp: s.timestamp / 100_000,
        });
    });

    // 4. WORKER: Detect anomalies from summaries
    engine.run_worker(move || {
        summary_reader.next(); // Wait for data

        alert_pipeline
            .from(&summary_reader)
            .to(&mut alert_store)
            .reduce(2, |window| {
                let (prev, cur) = (window[0], window[1]);

                // Alert if average value jumps by more than 50%
                if cur.avg > prev.avg * 1.5 {
                    Some(Alert {
                        sensor_id: cur.sensor_id,
                        timestamp: cur.timestamp,
                        severity: 1,
                        ..Default::default()
                    })
                } else {
                    None
                }
            });
    });

    // 5. INGEST DATA
    println!("Pushing sensor readings...");
    let readings = [
        Reading::from(1, 10.0, 10_000),
        Reading::from(1, 12.0, 20_000),
        Reading::from(1, 12.0, 30_000),
        Reading::from(1, 12.0, 40_000),
        Reading::from(1, 20.0, 110_000), // Average jump here
        Reading::from(1, 22.0, 120_000),
        Reading::from(1, 22.0, 220_000),
        Reading::from(1, 22.0, 320_000),
    ];

    for r in readings {
        reading_store.append(r);
    }

    // Give workers a moment to process
    thread::sleep(Duration::from_millis(100));

    // 6. DISPLAY RESULTS
    println!("\nSummaries in Index:");
    for (_, summary) in summary_index_reader.iter() {
        println!(
            "Sensor {} at {}: Avg={:.2}, Count={}",
            summary.sensor_id, summary.timestamp, summary.avg, summary.count
        );
    }

    println!("\nAlerts Detected:");
    while alert_reader_for_print.next() {
        if let Some(alert) = alert_reader_for_print.get() {
            println!(
                "ALERT: Sensor {} anomaly at {}",
                alert.sensor_id, alert.timestamp
            );
        }
    }
}
