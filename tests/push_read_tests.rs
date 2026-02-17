use roda_state::JournalStoreOptions;
use roda_state::RodaEngine;

#[test]
fn test_push_then_read_single() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test1",
        size: 1024,
        in_memory: true,
    });
    let reader = store.reader();

    store.append(&42);

    let res = reader.get_window::<1>(0).unwrap();
    assert_eq!(res[0], 42);
}

#[test]
fn test_multiple_push_read_in_order() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test2",
        size: 1024,
        in_memory: true,
    });
    let reader = store.reader();

    for v in [1u32, 2, 3, 4, 5] {
        store.append(&v);
    }

    let res = reader.get_window::<5>(0).unwrap();
    for (i, expected) in [1u32, 2, 3, 4, 5].iter().enumerate() {
        assert_eq!(res[i], *expected);
    }
}

#[test]
fn test_interleaved_push_and_read() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test3",
        size: 1024,
        in_memory: true,
    });
    let reader = store.reader();

    // Push values; verify FIFO order via get_window
    store.append(&10);
    store.append(&20);
    store.append(&30);
    store.append(&40);

    let res = reader.get_window::<4>(0).unwrap();
    assert_eq!(res[0], 10);
    assert_eq!(res[1], 20);
    assert_eq!(res[2], 30);
    assert_eq!(res[3], 40);
}

#[test]
fn test_stores_are_isolated_by_type() {
    let engine = RodaEngine::new();

    let mut u_store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "u32",
        size: 1024,
        in_memory: true,
    });
    let mut i_store = engine.new_journal_store::<i64>(JournalStoreOptions {
        name: "i64",
        size: 1024,
        in_memory: true,
    });
    let u_reader = u_store.reader();
    let i_reader = i_store.reader();

    u_store.append(&1);
    i_store.append(&-1);
    u_store.append(&2);
    i_store.append(&-2);

    let u_res = u_reader.get_window::<2>(0).unwrap();
    let i_res = i_reader.get_window::<2>(0).unwrap();

    assert_eq!(u_res[0], 1);
    assert_eq!(u_res[1], 2);
    assert_eq!(i_res[0], -1);
    assert_eq!(i_res[1], -2);
}

#[test]
fn test_push_after_partial_reads() {
    let engine = RodaEngine::new();
    let mut store = engine.new_journal_store::<u32>(JournalStoreOptions {
        name: "test4",
        size: 1024,
        in_memory: true,
    });
    let reader = store.reader();

    store.append(&100);
    store.append(&200);
    store.append(&300);
    store.append(&400);

    let res = reader.get_window::<4>(0).unwrap();
    assert_eq!(res[0], 100);
    assert_eq!(res[1], 200);
    assert_eq!(res[2], 300);
    assert_eq!(res[3], 400);
}
