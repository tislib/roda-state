mod models;

use crate::models::{Alert, Reading, SensorKey, Summary};
use roda_state::StageEngine;
use roda_state::pipe;
use roda_state::{delta, stateful};
use std::time::Duration;

fn main() {
    println!("Starting Sensor Multistage Pipeline (Optimized)...");

    // 1. Initialize StageEngine
    let engine = StageEngine::<Reading, Reading>::with_capacity(1000);

    // 2. Add Aggregation Stage: Reading -> Summary
    let mut engine = engine
        .add_stage(pipe![
            // Use stateful helper to handle the HashMap and windowing logic
            stateful(SensorKey::from_reading, Summary::init, |state, r| state
                .update(r))
        ])
        .add_stage(pipe![
            // Use delta to compare current summary to previous summary for the same sensor
            delta(
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
            )
        ]);

    // 4. INGEST DATA
    println!("\nPushing sensor readings...");
    let readings = [
        Reading::from(1, 10.0, 10_000),
        Reading::from(1, 12.0, 20_000),
        Reading::from(1, 20.0, 110_000), // Average jump
        Reading::from(1, 22.0, 120_000),
    ];

    for r in readings {
        engine.send(&r);
    }

    engine.await_idle(Duration::from_millis(100));

    // 5. DISPLAY RESULTS
    println!("\nAlerts Detected:");
    while let Some(alert) = engine.receive() {
        println!(
            "ALERT: Sensor {} anomaly at {}",
            alert.sensor_id, alert.timestamp
        );
    }

    println!("\nDone!");
}
