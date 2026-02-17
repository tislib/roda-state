use roda_state::JournalStoreOptions;
use roda_state::RodaEngine;

#[test]
#[should_panic(expected = "Store is full")]
fn test_journal_panic_when_full() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u64>(JournalStoreOptions {
        name: "full_test",
        size: 2, // Can hold only 2 u64
        in_memory: true,
    });

    store.append(&1);
    store.append(&2);
    store.append(&3); // This should panic
}

#[test]
fn test_journal_no_circularity() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u64>(JournalStoreOptions {
        name: "no_circular_test",
        size: 2,
        in_memory: true,
    });
    let reader = store.reader();

    store.append(&1);
    store.append(&2);

    assert_eq!(reader.get_at(0), Some(1));
    assert_eq!(reader.get_at(1), Some(2));

    // In the old circular store, if we pushed more, it would overwrite.
    // Here it just panics, so we just verify we can read what we pushed.
}
