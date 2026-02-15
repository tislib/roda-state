use assert_no_alloc::*;
use roda_state::JournalStoreOptions;
use roda_state::RodaEngine;

#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: AllocDisabler = AllocDisabler;

#[test]
fn test_store_push_no_alloc() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "no_alloc_push",
        size: 1024,
        in_memory: true,
    });

    assert_no_alloc(|| {
        store.append(42);
    });
}

#[test]
fn test_store_reader_next_no_alloc() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "no_alloc_next",
        size: 1024,
        in_memory: true,
    });
    store.append(42);
    let reader = store.reader();

    assert_no_alloc(|| {
        reader.next();
    });
}

#[test]
fn test_store_reader_get_no_alloc() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "no_alloc_get",
        size: 1024,
        in_memory: true,
    });
    store.append(42);
    let reader = store.reader();
    reader.next();

    assert_no_alloc(|| {
        let _ = reader.get();
    });
}

#[test]
fn test_store_reader_get_window_no_alloc() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "no_alloc_window",
        size: 1024,
        in_memory: true,
    });
    store.append(42);
    store.append(43);
    let reader = store.reader();

    assert_no_alloc(|| {
        let res = reader.get_window::<2>(0).unwrap();
        assert_eq!(res[0], 42);
        assert_eq!(res[1], 43);
    });
}

#[test]
fn test_store_reader_get_at_no_alloc() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "no_alloc_get_at",
        size: 1024,
        in_memory: true,
    });
    store.append(42);
    let reader = store.reader();

    assert_no_alloc(|| {
        let _ = reader.get_at(0);
    });
}

#[test]
fn test_store_reader_get_last_no_alloc() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "no_alloc_get_last",
        size: 1024,
        in_memory: true,
    });
    store.append(42);
    let reader = store.reader();

    assert_no_alloc(|| {
        let _ = reader.get_last();
    });
}
