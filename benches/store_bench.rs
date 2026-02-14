use bytemuck::{Pod, Zeroable};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use roda_state::measure::LatencyMeasurer;
use roda_state::{JournalStoreOptions, RodaEngine};
use std::hint::black_box;

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
struct LargeState {
    data: [u64; 16], // 128 bytes
}

fn bench_push(c: &mut Criterion) {
    let mut engine = RodaEngine::new();
    engine.enable_latency_stats(true);
    let mut group = c.benchmark_group("append");

    // 1GB buffer to ensure we don't overflow during benchmarking
    let size = 16 * 1024 * 1024 * 1024;
    let mut store_u64 = engine.new_journal_store::<u64>(JournalStoreOptions {
        name: "bench_push_u64",
        size,
        in_memory: true,
    });

    group.throughput(Throughput::Elements(1));
    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("push_u64", |b| {
        let mut val = 0u64;
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            store_u64.append(black_box(val));
            val += 1;
        });
    });
    println!("push_u64 latency:{}", measurer.format_stats());

    let mut store_large = engine.new_journal_store::<LargeState>(JournalStoreOptions {
        name: "bench_push_large",
        size,
        in_memory: true,
    });

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("push_128b", |b| {
        let val = LargeState { data: [42; 16] };
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            store_large.append(black_box(val));
        });
    });
    println!("push_128b latency:{}", measurer.format_stats());

    group.finish();
}

fn bench_fetch(c: &mut Criterion) {
    let mut engine = RodaEngine::new();
    engine.enable_latency_stats(true);
    let mut group = c.benchmark_group("fetch");

    let size = 1024 * 1024 * 100; // 100MB
    let mut store = engine.new_journal_store::<u64>(JournalStoreOptions {
        name: "bench_fetch",
        size,
        in_memory: true,
    });

    // Pre-fill some data
    for i in 0..10000 {
        store.append(i as u64);
    }
    let reader = store.reader();

    group.throughput(Throughput::Elements(1));
    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("get_at_u64", |b| {
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            black_box(reader.get_at(black_box(5000)));
        });
    });
    println!("get_at_u64 latency:{}", measurer.format_stats());

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("get_last_u64", |b| {
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            black_box(reader.get_last());
        });
    });
    println!("get_last_u64 latency:{}", measurer.format_stats());

    let mut store_large = engine.new_journal_store::<LargeState>(JournalStoreOptions {
        name: "bench_fetch_large",
        size,
        in_memory: true,
    });
    for _ in 0..10000 {
        store_large.append(LargeState { data: [42; 16] });
    }
    let reader_large = store_large.reader();

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("get_at_128b", |b| {
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            black_box(reader_large.get_at(black_box(5000)));
        });
    });
    println!("get_at_128b latency:{}", measurer.format_stats());

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("next_get_u64", |b| {
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            if reader.next() {
                black_box(reader.get());
            }
        });
    });
    println!("next_get_u64 latency:{}", measurer.format_stats());

    group.finish();
}

fn bench_window(c: &mut Criterion) {
    let mut engine = RodaEngine::new();
    engine.enable_latency_stats(true);
    let mut group = c.benchmark_group("window");

    let size = 1024 * 1024 * 100; // 100MB
    let mut store = engine.new_journal_store::<u64>(JournalStoreOptions {
        name: "bench_window",
        size,
        in_memory: true,
    });

    // Pre-fill some data
    for i in 0..10000 {
        store.append(i as u64);
    }
    let reader = store.reader();

    group.throughput(Throughput::Elements(1));
    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("get_window_10", |b| {
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            black_box(reader.get_window::<10>(black_box(5000)));
        });
    });
    println!("get_window_10 latency:{}", measurer.format_stats());

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("get_window_100", |b| {
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            black_box(reader.get_window::<100>(black_box(5000)));
        });
    });
    println!("get_window_100 latency:{}", measurer.format_stats());

    group.finish();
}

criterion_group!(benches, bench_push, bench_fetch, bench_window);
criterion_main!(benches);
