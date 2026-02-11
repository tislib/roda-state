use roda_core::RodaEngine;

#[test]
fn test_push_then_read_single() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);

    store.push(42).expect("failed to push value");

    // Expect to read the only pushed value
    assert_eq!(store.with(|v| *v), 42);
}

#[test]
fn test_multiple_push_read_in_order() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);

    // Push a small sequence
    for v in [1u32, 2, 3, 4, 5] {
        store.push(v).expect("failed to push value");
    }

    // Expect reads to yield in the same FIFO order as pushes
    for expected in [1u32, 2, 3, 4, 5] {
        assert_eq!(store.with(|v| *v), expected);
    }
}

#[test]
fn test_interleaved_push_and_read() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);

    store.push(10).expect("failed to push value");
    assert_eq!(store.with(|v| *v), 10);

    store.push(20).expect("failed to push value");
    store.push(30).expect("failed to push value");
    assert_eq!(store.with(|v| *v), 20);

    store.push(40).expect("failed to push value");
    assert_eq!(store.with(|v| *v), 30);
    assert_eq!(store.with(|v| *v), 40);
}

#[test]
fn test_stores_are_isolated_by_type() {
    let engine = RodaEngine::new();

    let u_store = engine.store::<u32>(1024);
    let i_store = engine.store::<i64>(1024);

    u_store.push(1).expect("failed to push value");
    i_store.push(-1).expect("failed to push value");

    // Each store should yield its own sequence independently
    assert_eq!(u_store.with(|v| *v), 1);
    assert_eq!(i_store.with(|v| *v), -1);

    u_store.push(2).expect("failed to push value");
    i_store.push(-2).expect("failed to push value");

    assert_eq!(u_store.with(|v| *v), 2);
    assert_eq!(i_store.with(|v| *v), -2);
}

#[test]
fn test_push_after_partial_reads() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);

    store.push(100).expect("failed to push value");
    store.push(200).expect("failed to push value");

    // Read one, then push more, ensure order continues
    assert_eq!(store.with(|v| *v), 100);

    store.push(300).expect("failed to push value");
    store.push(400).expect("failed to push value");

    assert_eq!(store.with(|v| *v), 200);
    assert_eq!(store.with(|v| *v), 300);
    assert_eq!(store.with(|v| *v), 400);
}
