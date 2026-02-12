use roda_state::RodaEngine;
use roda_state::components::{Engine, Store, StoreOptions, StoreReader};

#[test]
fn test_reader_next_and_with_logic() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions {
        name: "logic_test",
        size: 1024,
        in_memory: true,
    });
    let reader = store.reader();

    // Initially, next() should be false and with() should be None
    assert!(!reader.next());
    assert!(reader.with(|&x| x).is_none());

    // Push one value
    store.push(100);

    // next() should now be true
    assert!(reader.next());
    // after next(), with() should return the value
    assert_eq!(reader.with(|&x| x), Some(100));

    // next() should now be false again until another push
    assert!(!reader.next());
    // but with() should still return the last successfully read value
    assert_eq!(reader.with(|&x| x), Some(100));

    // Push another value
    store.push(200);

    // next() should be true
    assert!(reader.next());
    // with() should return the new value
    assert_eq!(reader.with(|&x| x), Some(200));
}

#[test]
fn test_reader_get_at_and_last() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions {
        name: "logic_test_2",
        size: 1024,
        in_memory: true,
    });
    let reader = store.reader();

    store.push(10);
    store.push(20);
    store.push(30);

    assert_eq!(reader.get_at(0), Some(10));
    assert_eq!(reader.get_at(1), Some(20));
    assert_eq!(reader.get_at(2), Some(30));
    assert_eq!(reader.get_at(3), None);

    assert_eq!(reader.get_last(), Some(30));
}
