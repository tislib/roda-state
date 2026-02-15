use roda_state::JournalStoreOptions;
use roda_state::RodaEngine;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_store_reader_edge_cases() {
    let mut engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "edge_cases",
        size: 1024,
        in_memory: true,
    });
    let reader = store.reader();

    // 1. get_at out of bounds on empty store
    assert_eq!(reader.get_at(0), None);
    assert_eq!(reader.get_at(1), None);

    // 2. get_last on empty store
    assert_eq!(reader.get_last(), None);

    // 3. get_window out of bounds on empty store
    assert_eq!(reader.get_window::<1>(0), None);

    // 4. get before next()
    assert_eq!(reader.get(), None);

    store.append(42);

    // 5. get before next() but after push
    assert_eq!(reader.get(), None);

    // 6. next() then get()
    assert!(reader.next());
    assert_eq!(reader.get(), Some(42));

    // 7. next() again (should be false)
    assert!(!reader.next());
    // get() should still return last successful read
    assert_eq!(reader.get(), Some(42));

    // 8. get_at valid
    assert_eq!(reader.get_at(0), Some(42));
    assert_eq!(reader.get_at(1), None);

    // 9. get_last valid
    assert_eq!(reader.get_last(), Some(42));

    // 10. get_window valid
    assert_eq!(reader.get_window::<1>(0), Some(&[42][..]));

    // 11. with_at and with_last
    assert_eq!(reader.with_at(0, |&v| v), Some(42));
    assert_eq!(reader.with_last(|&v| v), Some(42));
}

#[test]
fn test_store_full_capacity() {
    let mut engine = RodaEngine::new();
    let num_items = 10;
    let mut store = engine.new_journal_store::<u64>(JournalStoreOptions {
        name: "full_capacity",
        size: num_items,
        in_memory: true,
    });

    for i in 0..num_items {
        store.append(i as u64);
    }

    let reader = store.reader();
    for i in 0..num_items {
        assert!(reader.next());
        assert_eq!(reader.get(), Some(i as u64));
    }
    assert!(!reader.next());

    // This should panic if it exceeds capacity
    // However, looking at journal_store:
    // self.storage.append(&state);
    // and MmapJournal::append
    // Let's see what happens if we push one more.
}

#[test]
#[should_panic(expected = "Store is full")]
fn test_store_overflow_panic() {
    let mut engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u64>(JournalStoreOptions {
        name: "overflow",
        size: 1,
        in_memory: true,
    });

    store.append(1);
    store.append(2); // Should panic here
}

#[test]
fn test_store_concurrent_load() {
    let engine = Arc::new(RodaEngine::new());
    let store_options = JournalStoreOptions {
        name: "concurrent_load",
        size: 1024 * 1024,
        in_memory: true,
    };
    let mut store = engine.new_journal_store::<u32>(store_options);

    let num_readers = 4;
    let num_pushes = 1000;
    let barrier = Arc::new(Barrier::new(num_readers + 1));

    let mut readers = Vec::new();
    for i in 0..num_readers {
        let b = barrier.clone();
        let reader = store.reader();
        readers.push(thread::spawn(move || {
            b.wait();
            let mut count = 0;
            let mut last_val = None;
            while count < num_pushes {
                if reader.next() {
                    let val = reader.get().unwrap();
                    if let Some(prev) = last_val {
                        assert!(
                            val > prev,
                            "Reader {} saw non-monotonic values: {} then {}",
                            i,
                            prev,
                            val
                        );
                    }
                    last_val = Some(val);
                    count += 1;
                } else {
                    thread::yield_now();
                }
            }
            count
        }));
    }

    barrier.wait();
    for i in 1..=num_pushes {
        store.append(i as u32);
    }

    let mut total_read = 0;
    for handle in readers {
        total_read += handle.join().unwrap();
    }

    assert_eq!(total_read, num_readers * num_pushes);
}
