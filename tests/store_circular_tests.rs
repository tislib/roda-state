use roda_state::RodaEngine;
use roda_state::components::{Engine, Store, StoreOptions, StoreReader};

#[test]
fn test_store_wrap_around() {
    let engine = RodaEngine::new();
    let size = 64; // 8 u64s
    let mut store = engine.store::<u64>(StoreOptions {
        name: "test_wrap",
        size,
        in_memory: true,
    });
    let reader = store.reader();

    // Fill the store
    for i in 0..8 {
        store.push(i as u64);
    }

    // Read all
    for i in 0..8 {
        assert_eq!(reader.get_at(i), Some(i as u64));
    }

    // Push one more, should overwrite index 0
    store.push(8);

    // index 0 should be gone (overwritten by 8)
    assert_eq!(reader.get_at(0), None);
    // index 8 should be present
    assert_eq!(reader.get_at(8), Some(8));
    
    // get_last should be 8
    assert_eq!(reader.get_last(), Some(8));
}

#[test]
fn test_reader_lapping_catch_up() {
    let engine = RodaEngine::new();
    let size = 64; // 8 u64s
    let mut store = engine.store::<u64>(StoreOptions {
        name: "test_lapping",
        size,
        in_memory: true,
    });
    let reader = store.reader();

    // Push 4 items
    for i in 0..4 {
        store.push(i as u64);
    }

    // Reader is at index 0. Advance it once.
    assert!(reader.next()); // reads 0
    assert_eq!(reader.get(), Some(0));

    // Now push 8 more items, which will overwrite the current position of the reader (index 1)
    // Buffer has [8, 9, 10, 11, 4, 5, 6, 7] (logical indices 8, 9, 10, 11, 4, 5, 6, 7)
    // Current next_index in reader is 1.
    // min_offset for write_index 12*8 = 96 is 96 - 64 = 32.
    // index 1 has offset 8. 8 < 32, so it's lapped.
    for i in 4..12 {
        store.push(i as u64);
    }

    // Calling next() should detect lapping and catch up to the oldest available data (index 4)
    assert!(reader.next()); 
    // It should skip 1, 2, 3 and jump to the oldest available element after being lapped.
    // My implementation sets it to min_offset / size + 1.
    // min_offset = 32. new_index = 32 / 8 = 4. next_index set to 5.
    // So get() should return element 4.
    assert_eq!(reader.get(), Some(4));
}

#[test]
fn test_get_window_lapping() {
    let engine = RodaEngine::new();
    let size = 64; // 8 u64s
    let mut store = engine.store::<u64>(StoreOptions {
        name: "test_window_lapping",
        size,
        in_memory: true,
    });
    let reader = store.reader();

    for i in 0..12 {
        store.push(i as u64);
    }
    // Write index is 12*8 = 96. min_offset is 32 (index 4).
    
    // Window at index 4 (length 4) should be [4, 5, 6, 7]
    let win = reader.get_window::<4>(4).unwrap();
    assert_eq!(win, [4, 5, 6, 7]);

    // Window at index 3 should be None (partially overwritten)
    assert!(reader.get_window::<4>(3).is_none());
}

#[test]
fn test_large_rolling_push() {
    let engine = RodaEngine::new();
    let size = 1024; // 128 u64s
    let mut store = engine.store::<u64>(StoreOptions {
        name: "test_large_rolling",
        size,
        in_memory: true,
    });
    let reader = store.reader();

    for i in 0..1000 {
        store.push(i as u64);
    }

    assert_eq!(reader.get_last(), Some(999));
    
    // The buffer holds last 128 elements.
    // 1000 - 128 = 872. 
    // Indices 872 to 999 should be available.
    assert_eq!(reader.get_at(872), Some(872));
    assert_eq!(reader.get_at(871), None);
}
