mod models;

use models::{Alert, Reading, SensorKey, Summary};
use roda_state::StageEngine;
use roda_state::pipe;
use roda_state::{dedup_by, delta, inspect, stateful};
use std::time::Duration;

fn main() {
    println!("--- Starting StageEngine: Service Health Pipeline ---");

    // 1. Initialize StageEngine (Initial entry type is Reading)
    let engine = StageEngine::<Reading, Reading>::with_capacity(1000);

    // 2. Add Aggregation Stage: Reading -> Summary
    // We also include a deduplicator at the start to drop identical raw readings.
    let engine = engine.add_stage(pipe![
        dedup_by(|r: &Reading| (r.sensor_id, (r.value * 1000.0) as u64)), // Noise filter
        stateful(SensorKey::from_reading, Summary::init, Summary::update),
        inspect(|s: &Summary| {
            println!(
                "STAGE 1 [AGG]: Sensor {} Avg updated to {:.2}",
                s.sensor_id, s.avg
            );
        })
    ]);

    // 3. Add Anomaly Detection Stage: Summary -> Alert
    // Uses Delta to compare current state with previous known state for that sensor.
    let mut engine = engine.add_stage(pipe![
        delta(
            |s: &Summary| s.sensor_id,
            |curr, prev| {
                if let Some(p) = prev
                    && curr.avg > p.avg * 1.5
                {
                    // Logic: Alert if the average jumps by more than 50%
                    return Some(Alert {
                        sensor_id: curr.sensor_id,
                        timestamp: curr.timestamp,
                        severity: 1,
                        ..Default::default()
                    });
                }
                None
            }
        ),
        // Deduplicate Alerts: Only notify if the alert is new/changed for this sensor
        dedup_by(|a: &Alert| a.sensor_id),
        inspect(|a: &Alert| {
            println!(
                "STAGE 2 [ALERT]: 🚨 Anomaly detected for Sensor {}!",
                a.sensor_id
            );
        })
    ]);

    // 4. Ingest Data
    println!("\nIngesting readings...");
    let readings = [
        Reading::from(1, 10.0, 10_000),  // Baseline
        Reading::from(1, 10.0, 20_000),  // Duplicate (filtered by dedup)
        Reading::from(1, 11.0, 30_000),  // Small change
        Reading::from(1, 25.0, 110_000), // Spike -> Triggers Alert
        Reading::from(2, 5.0, 10_000),   // New Sensor
    ];

    for r in readings {
        engine.send(&r);
    }

    // Give workers time to finish processing
    engine.await_idle(Duration::from_millis(100));

    // 5. Display Results from the end of the pipeline
    println!("\n--- Final Alert Journal ---");
    while let Some(alert) = engine.try_receive() {
        println!(
            "Received in Main: Alert for Sensor {} at {}",
            alert.sensor_id, alert.timestamp
        );
    }

    println!("\nDone.");
}
