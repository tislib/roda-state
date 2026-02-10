use roda_core::RodaEngine;

#[test]
fn test_engine_new() {
    // Should construct without panic
    let _engine = RodaEngine::new();
}

#[test]
fn test_simple_push_read() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();

    store.push(1).expect("failed to push value");
    store.push(3).expect("failed to push value");

    assert_eq!(store.with(|value| *value), 1);
    assert_eq!(store.with(|value| *value), 3);
}
