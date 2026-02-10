use std::sync::mpsc;
use std::time::Duration;

use roda::RodaEngine;

#[test]
fn test_engine_new() {
    // Should construct without panic
    let _engine = RodaEngine::new();
}

#[test]
fn test_run_worker_executes_closure() {
    let engine = RodaEngine::new();
    let (tx, rx) = mpsc::channel::<&'static str>();

    engine.run_worker(move || {
        // Simulate some work
        tx.send("done").ok();
    });

    // The worker thread is detached; wait for the signal with a timeout
    let received = rx.recv_timeout(Duration::from_secs(2)).expect("worker did not execute in time");
    assert_eq!(received, "done");
}
