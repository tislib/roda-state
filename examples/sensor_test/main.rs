use bytemuck::{Pod, Zeroable};
use roda_state::StageEngine;
use roda_state::pipe;
use std::collections::HashMap;
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
    println!("Starting Sensor Multistage Pipeline with Closures (StageEngine)...");

    // 1. Initialize StageEngine
    // StageEngine starts as a passthrough for Reading
    let engine = StageEngine::<Reading, Reading>::with_capacity(1000);

    // 2. Add Aggregation Stage: Reading -> Summary
    // Redesigned as a pipeline of closures
    let mut summaries: HashMap<SensorKey, Summary> = HashMap::new();
    let engine = engine.add_stage(pipe![
        move |r: Reading| {
            let key = SensorKey {
                sensor_id: r.sensor_id,
                timestamp: (r.timestamp / 100_000) * 100_000,
            };
            
            let entry = summaries.entry(key);
            let summary = match entry {
                std::collections::hash_map::Entry::Vacant(e) => {
                    let s = Summary {
                        sensor_id: r.sensor_id,
                        min: r.value,
                        max: r.value,
                        avg: r.value,
                        count: 1,
                        timestamp: key.timestamp,
                    };
                    e.insert(s);
                    s
                }
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let s = e.get_mut();
                    s.min = s.min.min(r.value);
                    s.max = s.max.max(r.value);
                    s.avg = (s.avg * s.count as f64 + r.value) / (s.count + 1) as f64;
                    s.count += 1;
                    *s
                }
            };
            Some(summary)
        },
        |s: Summary| {
            println!(
                "AGGREGATOR: Sensor {} at {}: Avg={:.2}, Count={}",
                s.sensor_id, s.timestamp, s.avg, s.count
            );
            Some(s)
        }
    ]);

    // 3. Add Anomaly Detection Stage: Summary -> Alert
    // Redesigned as a closure (which is also a pipeline of one)
    let mut last_summaries: HashMap<u64, Summary> = HashMap::new();
    let mut engine = engine.add_stage(pipe![
        move |s: Summary| {
            let prev = last_summaries.get(&s.sensor_id).copied();
            last_summaries.insert(s.sensor_id, s);
            
            if let Some(prev) = prev {
                // Alert if average value jumps by more than 50%
                if s.avg > prev.avg * 1.5 {
                    return Some(Alert {
                        sensor_id: s.sensor_id,
                        timestamp: s.timestamp,
                        severity: 1,
                        ..Default::default()
                    });
                }
            }
            None
        }
    ]);

    // 4. INGEST DATA
    println!("\nPushing sensor readings...");
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
        engine.send(r);
    }

    // Give workers a moment to process
    engine.await_idle(Duration::from_millis(100));

    // 5. DISPLAY FINAL RESULTS
    println!("\nAlerts Detected:");
    let total_alerts = engine.output_size();
    if total_alerts == 0 {
        println!("No alerts detected.");
    } else {
        for _ in 0..total_alerts {
            if let Some(alert) = engine.receive() {
                println!(
                    "ALERT: Sensor {} anomaly at {}",
                    alert.sensor_id, alert.timestamp
                );
            }
        }
    }
    
    println!("\nDone!");
}
