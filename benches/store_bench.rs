use bytemuck::{Pod, Zeroable};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use roda_state::RodaEngine;
use roda_state::components::{Engine, Store, StoreOptions, StoreReader};
use std::hint::black_box;

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
struct LargeState {
    data: [u64; 16], // 128 bytes
}

fn bench_push(c: &mut Criterion) {
    let engine = RodaEngine::new();
    let mut group = c.benchmark_group("push");

    // 1GB buffer to ensure we don't overflow during benchmarking
    let size = 1024 * 1024 * 1024;
    let mut store_u64 = engine.store::<u64>(StoreOptions {
        name: "bench_push_u64",
        size,
        in_memory: true,
    });

    group.throughput(Throughput::Elements(1));
    group.bench_function("push_u64", |b| {
        let mut val = 0u64;
        b.iter(|| {
            store_u64.push(black_box(val));
            val += 1;
        });
    });

    let mut store_large = engine.store::<LargeState>(StoreOptions {
        name: "bench_push_large",
        size,
        in_memory: true,
    });

    group.bench_function("push_128b", |b| {
        let val = LargeState { data: [42; 16] };
        b.iter(|| {
            store_large.push(black_box(val));
        });
    });

    group.finish();
}

fn bench_fetch(c: &mut Criterion) {
    let engine = RodaEngine::new();
    let mut group = c.benchmark_group("fetch");

    let size = 1024 * 1024 * 100; // 100MB
    let mut store = engine.store::<u64>(StoreOptions {
        name: "bench_fetch",
        size,
        in_memory: true,
    });

    // Pre-fill some data
    for i in 0..10000 {
        store.push(i as u64);
    }
    let reader = store.reader();

    group.throughput(Throughput::Elements(1));
    group.bench_function("get_at_u64", |b| {
        b.iter(|| {
            black_box(reader.get_at(black_box(5000)));
        });
    });

    group.bench_function("get_last_u64", |b| {
        b.iter(|| {
            black_box(reader.get_last());
        });
    });

    let mut store_large = engine.store::<LargeState>(StoreOptions {
        name: "bench_fetch_large",
        size,
        in_memory: true,
    });
    for _ in 0..10000 {
        store_large.push(LargeState { data: [42; 16] });
    }
    let reader_large = store_large.reader();

    group.bench_function("get_at_128b", |b| {
        b.iter(|| {
            black_box(reader_large.get_at(black_box(5000)));
        });
    });

    group.bench_function("next_get_u64", |b| {
        b.iter(|| {
            if reader.next() {
                black_box(reader.get());
            }
        });
    });

    group.finish();
}

fn bench_window(c: &mut Criterion) {
    let engine = RodaEngine::new();
    let mut group = c.benchmark_group("window");

    let size = 1024 * 1024 * 100; // 100MB
    let mut store = engine.store::<u64>(StoreOptions {
        name: "bench_window",
        size,
        in_memory: true,
    });

    // Pre-fill some data
    for i in 0..10000 {
        store.push(i as u64);
    }
    let reader = store.reader();

    group.throughput(Throughput::Elements(1));
    group.bench_function("get_window_10", |b| {
        b.iter(|| {
            black_box(reader.get_window::<10>(black_box(5000)));
        });
    });

    group.bench_function("get_window_100", |b| {
        b.iter(|| {
            black_box(reader.get_window::<100>(black_box(5000)));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_push, bench_fetch, bench_window);
criterion_main!(benches);
