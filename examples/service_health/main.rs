mod models;

use models::{Alert, Reading, ServiceKey, Summary};
use roda_state::StageEngine;
use roda_state::pipe;
use roda_state::{dedup_by, delta, stateful};
use std::time::{Duration, Instant};

fn main() {
    println!("--- Starting StageEngine: Service Health Pipeline ---");
    let start_time = Instant::now();

    // 1. Initialize StageEngine (Initial entry type is Reading)
    let engine = StageEngine::<Reading, Reading>::with_capacity(100_000_100);

    // 2. Add Aggregation Stage: Reading -> Summary
    // We also include a deduplicator at the start to drop identical raw readings.
    let engine = engine.add_stage(pipe![
        dedup_by(|r: &Reading| (r.service_id, (r.value * 1000.0) as u64)), // Noise filter
        stateful(ServiceKey::from_reading, Summary::init, Summary::update),
    ]);

    // 3. Add Anomaly Detection Stage: Summary -> Alert
    // Uses Delta to compare current state with previous known state for that sensor.
    let mut engine = engine.add_stage(pipe![
        delta(
            |s: &Summary| s.service_id,
            |curr, prev| {
                if let Some(p) = prev
                    && curr.avg() > p.avg() * 1.5
                {
                    // Logic: Alert if the average jumps by more than 50%
                    return Some(Alert {
                        service_id: curr.service_id,
                        timestamp: curr.timestamp,
                        severity: 1,
                        ..Default::default()
                    });
                }
                None
            }
        ),
        // Deduplicate Alerts: Only notify if the alert is new/changed for this sensor
        dedup_by(|a: &Alert| a.service_id),
    ]);

    // 4. Ingest Data
    println!("\nIngesting readings...");
    // Trigger an initial alert for sensor 2
    engine.send(&Reading::from(2, 10.0, 0));
    engine.send(&Reading::from(2, 100.0, 1));

    let count = 100_000_000;
    for i in 0..count {
        engine.send(&Reading::from(1, 10.0, i as u64));
    }
    let readings_count = count + 2;

    // Give workers time to finish processing
    engine.await_idle(Duration::from_millis(100));

    let duration = start_time.elapsed();
    println!("Pipeline completed in {}ms", duration.as_millis());
    println!(
        "Throughput: {}/s",
        readings_count as f64 / duration.as_secs_f64()
    );

    // 5. Display Results from the end of the pipeline
    println!("\n--- Final Alert Journal ---");
    while let Some(alert) = engine.try_receive() {
        println!(
            "Received in Main: Alert for Service {} at {}",
            alert.service_id, alert.timestamp
        );
    }

    println!("\nDone.");
}
