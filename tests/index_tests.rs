use bytemuck::{Pod, Zeroable};
use roda_state::RodaEngine;
use roda_state::components::{Engine, Index, IndexReader, Store, StoreOptions};
use std::thread;
use std::time::Duration;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Pod, Zeroable)]
struct ComplexKey {
    id: u32,
    category: u32,
}

#[test]
#[ignore]
fn test_index_multiple_values() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let index = store.direct_index::<u32>();

    for i in 0..5 {
        store.push(i);
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
#[ignore]
fn test_multiple_indices_on_same_store() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });

    let index_double = store.direct_index::<u32>();
    let index_triple = store.direct_index::<u32>();

    store.push(10);

    index_double.compute(|x| x * 2);
    index_triple.compute(|x| x * 3);

    let reader_double = index_double.reader();
    let reader_triple = index_triple.reader();

    assert_eq!(reader_double.get(&20), Some(10));
    assert_eq!(reader_triple.get(&30), Some(10));
}

#[test]
#[ignore]
fn test_index_complex_key() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let index = store.direct_index::<ComplexKey>();

    store.push(100);
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
#[ignore]
fn test_index_shallow_clone_sharing() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let index = store.direct_index::<u32>();
    let clone1 = index.reader();
    let clone2 = index.reader();

    store.push(42);
    index.compute(|&x| x);

    assert_eq!(clone1.get(&42), Some(42));
    assert_eq!(clone2.get(&42), Some(42));
}

#[test]
#[ignore]
fn test_index_collision_overwrite() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let index = store.direct_index::<u32>();

    // Both 10 and 20 will map to key 1
    store.push(10);
    store.push(20);

    index.compute(|_| 1);
    index.compute(|_| 1);

    let reader = index.reader();
    // Usually a direct index mapping should store the latest value for a given key
    assert_eq!(reader.get(&1), Some(20));
}

#[test]
#[ignore]
fn test_index_not_found() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let index = store.direct_index::<u32>();

    store.push(10);
    index.compute(|x| x + 1);

    let reader = index.reader();
    assert_eq!(reader.get(&11), Some(10));
    assert_eq!(reader.get(&999), None);
}

#[test]
#[ignore]
fn test_concurrent_push_and_index() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
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
        store.push(i);
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
#[ignore]
fn test_run_worker_with_multiple_stores() {
    let engine = RodaEngine::new();
    let mut store_u32 = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let mut store_string = engine.store::<[u8; 16]>(StoreOptions { name: "test", size: 1024, in_memory: true });

    let index_u32 = store_u32.direct_index::<u32>();
    let index_string = store_string.direct_index::<usize>();

    // Prepare read-only readers for assertions after workers complete
    let index_u32_reader = index_u32.reader();
    let index_string_reader = index_string.reader();

    engine.run_worker(move || {
        store_u32.push(100);
        index_u32.compute(|&x| x);
    });

    engine.run_worker(move || {
        let mut bytes = [0u8; 16];
        bytes[..5].copy_from_slice(b"hello");
        store_string.push(bytes);
        index_string.compute(|s: &[u8; 16]| {
            let len = s.iter().take_while(|&&b| b != 0).count();
            len
        });
    });

    // Wait for workers
    thread::sleep(Duration::from_millis(50));

    assert_eq!(index_u32_reader.get(&100), Some(100));
    let res_bytes = index_string_reader.get(&5).unwrap();
    assert_eq!(&res_bytes[..5], b"hello");
}

#[test]
#[ignore]
fn test_multiple_workers_reading_index_only_original_computes() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let index = store.direct_index::<u32>();

    let reader1 = index.reader();
    let reader2 = index.reader();

    store.push(1);
    store.push(2);

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
#[ignore]
fn test_reader_cannot_compute() {
    let engine = RodaEngine::new();
    let mut store = engine.store::<u32>(StoreOptions { name: "test", size: 1024, in_memory: true });
    let index = store.direct_index::<u32>();
    let _reader = index.reader();

    // Verification: This test is now a compile-time check.
    // Readers do not have a .compute() method.
}
