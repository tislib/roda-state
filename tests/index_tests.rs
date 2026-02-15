use bytemuck::{Pod, Zeroable};
use roda_state::RodaEngine;
use roda_state::journal_store::JournalStoreOptions;
use std::thread;
use std::time::Duration;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Pod, Zeroable)]
struct ComplexKey {
    id: u32,
    category: u32,
}

#[test]
fn test_index_multiple_values() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<u32>();

    for i in 0..5 {
        store.append(i);
    }

    // Index them all
    for _ in 0..5 {
        index.compute(|x| x * 10);
    }

    let reader = index.reader();
    for i in 0..5 {
        assert_eq!(reader.get(&(i * 10)), Some(i));
    }
}

#[test]
fn test_multiple_indices_on_same_store() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });

    let index_double = store.direct_index::<u32>();
    let index_triple = store.direct_index::<u32>();

    store.append(10);

    index_double.compute(|x| x * 2);
    index_triple.compute(|x| x * 3);

    let reader_double = index_double.reader();
    let reader_triple = index_triple.reader();

    assert_eq!(reader_double.get(&20), Some(10));
    assert_eq!(reader_triple.get(&30), Some(10));
}

#[test]
fn test_index_complex_key() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<ComplexKey>();

    store.append(100);
    index.compute(|&val| ComplexKey {
        id: val,
        category: 1,
    });

    let reader = index.reader();
    assert_eq!(
        reader.get(&ComplexKey {
            id: 100,
            category: 1
        }),
        Some(100)
    );
    assert_eq!(
        reader.get(&ComplexKey {
            id: 100,
            category: 2
        }),
        None
    );
}

#[test]
fn test_index_shallow_clone_sharing() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<u32>();
    let clone1 = index.reader();
    let clone2 = index.reader();

    store.append(42);
    index.compute(|&x| x);

    assert_eq!(clone1.get(&42), Some(42));
    assert_eq!(clone2.get(&42), Some(42));
}

#[test]
fn test_index_collision_overwrite() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<u32>();

    // Both 10 and 20 will map to key 1
    store.append(10);
    store.append(20);

    index.compute(|_| 1);
    index.compute(|_| 1);

    let reader = index.reader();
    // Usually a direct index mapping should store the latest value for a given key
    assert_eq!(reader.get(&1), Some(20));
}

#[test]
fn test_index_not_found() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<u32>();

    store.append(10);
    index.compute(|x| x + 1);

    let reader = index.reader();
    assert_eq!(reader.get(&11), Some(10));
    assert_eq!(reader.get(&999), None);
}

#[test]
fn test_concurrent_push_and_index() {
    let mut engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<u32>();
    let index_reader = index.reader();

    // Spawn a worker to index everything that comes in
    engine.run_worker(move || {
        for _ in 0..10 {
            index.compute(|&x| x);
        }
    });

    // Push values from another thread (main thread)
    for i in 0..10 {
        store.append(i);
        // Give worker some time to process
        thread::sleep(Duration::from_millis(1));
    }

    // Give some extra time for the last ones to be indexed
    thread::sleep(Duration::from_millis(20));

    for i in 0..10 {
        assert_eq!(index_reader.get(&i), Some(i));
    }
}

#[test]
fn test_run_worker_with_multiple_stores() {
    let mut engine = RodaEngine::new();
    let mut store_u32 = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let mut store_string = engine.new_journal_store::<[u8; 16]>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });

    let index_u32 = store_u32.direct_index::<u32>();
    let index_string = store_string.direct_index::<usize>();

    // Prepare read-only readers for assertions after workers complete
    let index_u32_reader = index_u32.reader();
    let index_string_reader = index_string.reader();

    for _ in 0..10 {
        store_u32.append(100);
    }

    let mut pushed_u32 = false;
    engine.run_worker(move || {
        if !pushed_u32 {
            store_u32.append(100);
            pushed_u32 = true;
        }
        index_u32.compute(|&x| x);
    });

    let mut pushed_string = false;
    engine.run_worker(move || {
        if !pushed_string {
            let mut bytes = [0u8; 16];
            bytes[..5].copy_from_slice(b"hello");
            store_string.append(bytes);
            pushed_string = true;
        }
        index_string.compute(|s: &[u8; 16]| s.iter().take_while(|&&b| b != 0).count());
    });

    // Wait for workers
    thread::sleep(Duration::from_millis(50));

    assert_eq!(index_u32_reader.get(&100), Some(100));
    let res_bytes = index_string_reader.get(&5).unwrap();
    assert_eq!(&res_bytes[..5], b"hello");
}

#[test]
fn test_multiple_workers_reading_index_only_original_computes() {
    let mut engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<u32>();

    let reader1 = index.reader();
    let reader2 = index.reader();

    store.append(1);
    store.append(2);

    // Only the original index can compute; shallow clones are read-only
    engine.run_worker(move || {
        index.compute(|&x| x * 10);
        index.compute(|&x| x * 10);
    });

    thread::sleep(Duration::from_millis(50));

    assert_eq!(reader1.get(&10), Some(1));
    assert_eq!(reader2.get(&20), Some(2));
}

#[test]
fn test_index_iterator() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test",
        size: 1024,
        in_memory: true,
    });
    let index = store.direct_index::<u32>();

    for i in 0..5 {
        store.append(i);
        index.compute(|&x| x * 2);
    }

    let reader = index.reader();
    let items: Vec<_> = reader.iter().collect();

    assert_eq!(items.len(), 5);
    let expected = vec![(0, 0), (2, 1), (4, 2), (6, 3), (8, 4)];
    assert_eq!(items, expected);

    // Test Index::iter too
    let items_from_index: Vec<_> = index.iter().collect();
    assert_eq!(items_from_index, expected);
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug, PartialEq)]
struct PriceLevel {
    pub price: i64,
    pub volume: u64,
}

#[test]
fn test_index_navigation() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<PriceLevel>(JournalStoreOptions {
        name: "test_nav",
        size: 1024,
        in_memory: true,
    });

    let index = store.direct_index::<i64>();
    let reader = index.reader();

    // Push some data
    store.append(PriceLevel {
        price: 100,
        volume: 10,
    });
    store.append(PriceLevel {
        price: 200,
        volume: 20,
    });
    store.append(PriceLevel {
        price: 300,
        volume: 30,
    });

    // Compute index
    index.compute(|p| p.price); // for 100
    index.compute(|p| p.price); // for 200
    index.compute(|p| p.price); // for 300

    // Test find_ge
    {
        let key = 150;
        let mut it = reader.find_ge(&key);
        assert_eq!(it.next().unwrap().0, 200);
        assert_eq!(it.next().unwrap().0, 300);
        assert!(it.next().is_none());
    }

    // Test find_le
    {
        let key = 250;
        let mut it = reader.find_le(&key);
        assert_eq!(it.next().unwrap().0, 100);
        assert_eq!(it.next().unwrap().0, 200);
        assert!(it.next().is_none());
    }

    // Test range
    {
        let mut it = reader.range(150..250);
        assert_eq!(it.next().unwrap().0, 200);
        assert!(it.next().is_none());
    }
}

#[test]
fn test_index_navigation_rev() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<PriceLevel>(JournalStoreOptions {
        name: "test_nav_rev",
        size: 1024,
        in_memory: true,
    });

    let index = store.direct_index::<i64>();
    let reader = index.reader();

    store.append(PriceLevel {
        price: 100,
        volume: 10,
    });
    store.append(PriceLevel {
        price: 200,
        volume: 20,
    });
    store.append(PriceLevel {
        price: 300,
        volume: 30,
    });

    index.compute(|p| p.price);
    index.compute(|p| p.price);
    index.compute(|p| p.price);

    // Test find_ge().rev()
    {
        let key = 150;
        let mut it = reader.find_ge(&key).rev();
        assert_eq!(it.next().unwrap().0, 300);
        assert_eq!(it.next().unwrap().0, 200);
        assert!(it.next().is_none());
    }

    // Test find_le().rev()
    {
        let key = 250;
        let mut it = reader.find_le(&key).rev();
        assert_eq!(it.next().unwrap().0, 200);
        assert_eq!(it.next().unwrap().0, 100);
        assert!(it.next().is_none());
    }
}
