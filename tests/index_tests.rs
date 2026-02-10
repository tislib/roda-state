use roda_core::RodaEngine;
use std::thread;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct ComplexKey {
    id: u32,
    category: u8,
}

#[test]
fn test_index_multiple_values() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<u32>();

    for i in 0..5 {
        store.push(i).expect("failed to push");
    }

    // Index them all
    for _ in 0..5 {
        index.compute(|x| x * 10);
    }

    for i in 0..5 {
        assert_eq!(index.get(&(i * 10)), Some(&i));
    }
}

#[test]
fn test_multiple_indices_on_same_store() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    
    let index_double = store.direct_index::<u32>();
    let index_triple = store.direct_index::<u32>();

    store.push(10).expect("failed to push");

    index_double.compute(|x| x * 2);
    index_triple.compute(|x| x * 3);

    assert_eq!(index_double.get(&20), Some(&10));
    assert_eq!(index_triple.get(&30), Some(&10));
}

#[test]
fn test_index_complex_key() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<ComplexKey>();

    store.push(100).expect("failed to push");
    index.compute(|&val| ComplexKey { id: val, category: 1 });

    assert_eq!(index.get(&ComplexKey { id: 100, category: 1 }), Some(&100));
    assert_eq!(index.get(&ComplexKey { id: 100, category: 2 }), None);
}

#[test]
fn test_index_shallow_clone_sharing() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<u32>();
    let clone1 = index.shallow_clone();
    let clone2 = clone1.shallow_clone();

    store.push(42).expect("failed to push");
    index.compute(|&x| x);

    assert_eq!(clone1.get(&42), Some(&42));
    assert_eq!(clone2.get(&42), Some(&42));
}

#[test]
fn test_index_collision_overwrite() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<u32>();

    // Both 10 and 20 will map to key 1
    store.push(10).expect("failed to push");
    store.push(20).expect("failed to push");

    index.compute(|_| 1);
    index.compute(|_| 1);

    // Usually a direct index mapping should store the latest value for a given key
    assert_eq!(index.get(&1), Some(&20));
}

#[test]
fn test_index_not_found() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<u32>();

    store.push(10).expect("failed to push");
    index.compute(|x| x + 1);

    assert_eq!(index.get(&11), Some(&10));
    assert_eq!(index.get(&999), None);
}

#[test]
fn test_concurrent_push_and_index() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<u32>();
    let index_clone = index.shallow_clone();

    // Spawn a worker to index everything that comes in
    engine.run_worker(move || {
        for _ in 0..10 {
            index.compute(|&x| x);
        }
    });

    // Push values from another thread (main thread)
    for i in 0..10 {
        store.push(i).expect("failed to push");
        // Give worker some time to process
        thread::sleep(Duration::from_millis(1));
    }

    // Give some extra time for the last ones to be indexed
    thread::sleep(Duration::from_millis(20));

    for i in 0..10 {
        assert_eq!(index_clone.get(&i), Some(&i));
    }
}

#[test]
fn test_run_worker_with_multiple_stores() {
    let engine = RodaEngine::new();
    let store_u32 = engine.store::<u32>();
    let store_string = engine.store::<String>();

    let index_u32 = store_u32.direct_index::<u32>();
    let index_string = store_string.direct_index::<usize>();

    // Prepare read-only clones for assertions after workers complete
    let index_u32_reader = index_u32.shallow_clone();
    let index_string_reader = index_string.shallow_clone();

    engine.run_worker(move || {
        store_u32.push(100).expect("push failed");
        index_u32.compute(|&x| x);
    });

    engine.run_worker(move || {
        store_string.push("hello".to_string()).expect("push failed");
        index_string.compute(|s| s.len());
    });

    // Wait for workers
    thread::sleep(Duration::from_millis(50));

    assert_eq!(index_u32_reader.get(&100), Some(&100));
    assert_eq!(index_string_reader.get(&5), Some(&"hello".to_string()));
}

#[test]
fn test_multiple_workers_reading_index_only_original_computes() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<u32>();

    let reader1 = index.shallow_clone();
    let reader2 = index.shallow_clone();

    store.push(1).expect("push failed");
    store.push(2).expect("push failed");

    // Only the original index can compute; shallow clones are read-only
    engine.run_worker(move || {
        index.compute(|&x| x * 10);
        index.compute(|&x| x * 10);
    });

    thread::sleep(Duration::from_millis(50));

    assert_eq!(reader1.get(&10), Some(&1));
    assert_eq!(reader2.get(&20), Some(&2));
}


#[test]
#[should_panic]
fn test_shallow_clone_cannot_compute() {
    let engine = RodaEngine::new();
    let store = engine.store::<u32>();
    let index = store.direct_index::<u32>();
    let shallow = index.shallow_clone();

    // Attempt to compute using a shallow clone must fail (read-only)
    shallow.compute(|&x| x);
}
