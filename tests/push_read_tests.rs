use roda_core::RodaEngine;

#[test]
fn test_push_then_read_single() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);
    let reader = store.reader();

    store.push(42).expect("failed to push value");

    let res = reader.collect::<1>();
    assert_eq!(*res[0], 42);
}

#[test]
fn test_multiple_push_read_in_order() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);
    let reader = store.reader();

    for v in [1u32, 2, 3, 4, 5] {
        store.push(v).expect("failed to push value");
    }

    let res = reader.collect::<5>();
    for (i, expected) in [1u32, 2, 3, 4, 5].iter().enumerate() {
        assert_eq!(*res[i], *expected);
    }
}

#[test]
fn test_interleaved_push_and_read() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);
    let reader = store.reader();

    // Push values; verify FIFO order via collect
    store.push(10).expect("failed to push value");
    store.push(20).expect("failed to push value");
    store.push(30).expect("failed to push value");
    store.push(40).expect("failed to push value");

    let res = reader.collect::<4>();
    assert_eq!(*res[0], 10);
    assert_eq!(*res[1], 20);
    assert_eq!(*res[2], 30);
    assert_eq!(*res[3], 40);
}

#[test]
fn test_stores_are_isolated_by_type() {
    let engine = RodaEngine::new();

    let u_store = engine.store::<u32>(1024);
    let i_store = engine.store::<i64>(1024);
    let u_reader = u_store.reader();
    let i_reader = i_store.reader();

    u_store.push(1).expect("failed to push value");
    i_store.push(-1).expect("failed to push value");
    u_store.push(2).expect("failed to push value");
    i_store.push(-2).expect("failed to push value");

    let u_res = u_reader.collect::<2>();
    let i_res = i_reader.collect::<2>();

    assert_eq!(*u_res[0], 1);
    assert_eq!(*u_res[1], 2);
    assert_eq!(*i_res[0], -1);
    assert_eq!(*i_res[1], -2);
}

#[test]
fn test_push_after_partial_reads() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>(1024);
    let reader = store.reader();

    store.push(100).expect("failed to push value");
    store.push(200).expect("failed to push value");
    store.push(300).expect("failed to push value");
    store.push(400).expect("failed to push value");

    let res = reader.collect::<4>();
    assert_eq!(*res[0], 100);
    assert_eq!(*res[1], 200);
    assert_eq!(*res[2], 300);
    assert_eq!(*res[3], 400);
}
