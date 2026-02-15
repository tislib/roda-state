use bytemuck::{Pod, Zeroable};
use criterion::{Criterion, criterion_group, criterion_main};
use roda_state::measure::LatencyMeasurer;
use roda_state::{Aggregator, JournalStoreOptions, RodaEngine, Window};
use std::hint::black_box;

#[derive(Clone, Copy, Zeroable, Pod, Default)]
#[repr(C)]
struct RawData {
    id: u32,
    _pad: u32,
    value: f64,
}

#[derive(Clone, Copy, Zeroable, Pod, Default)]
#[repr(C)]
struct AggregatedData {
    id: u32,
    _pad: u32,
    sum: f64,
    count: u64,
}

fn bench_index(c: &mut Criterion) {
    let engine = RodaEngine::new();
    let mut group = c.benchmark_group("index");

    let size = 16 * 1024 * 1024 * 1024;
    let mut store = engine.new_journal_store::<RawData>(JournalStoreOptions {
        name: "bench_index_store",
        size,
        in_memory: true,
    });

    // Fill data
    for i in 0..10000 {
        store.append(RawData {
            id: i as u32,
            value: i as f64,
            ..Default::default()
        });
    }

    let index = store.direct_index::<u32>();

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("index_compute_10k", |b| {
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            let reader = store.reader();
            let index = store.direct_index::<u32>();
            while reader.next() {
                index.compute(|data| data.id);
            }
        });
    });
    println!("index_compute_10k latency:{}", measurer.format_stats());

    // Pre-compute index for lookup bench
    let reader = store.reader();
    while reader.next() {
        index.compute(|data| data.id);
    }
    let index_reader = index.reader();

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("index_lookup", |b| {
        let mut i = 0u32;
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            black_box(index_reader.get(&(i % 10_000)));
            i += 1;
        });
    });
    println!("index_lookup latency:{}", measurer.format_stats());

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("index_incremental_compute", |b| {
        let mut i = 10000u32;
        let reader = store.reader();
        // Skip already pushed
        for _ in 0..10000 {
            reader.next();
        }

        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            store.append(RawData {
                id: i,
                value: i as f64,
                ..Default::default()
            });
            reader.next();
            index.compute(|data| data.id);
            i += 1;
        });
    });
    println!(
        "index_incremental_compute latency:{}",
        measurer.format_stats()
    );

    group.finish();
}

fn bench_aggregator(c: &mut Criterion) {
    let engine = RodaEngine::new();
    let mut group = c.benchmark_group("aggregator");

    for num_partitions in [10, 100, 1000] {
        let mut source = engine.new_journal_store::<RawData>(JournalStoreOptions {
            name: "bench_agg_source",
            size: 8 * 1024 * 1024 * 1024,
            in_memory: true,
        });
        let mut target = engine.new_journal_store::<AggregatedData>(JournalStoreOptions {
            name: "bench_agg_target",
            size: 8 * 1024 * 1024 * 1024,
            in_memory: true,
        });

        let source_reader = source.reader();
        let aggregator: Aggregator<RawData, AggregatedData, u32> = Aggregator::new();

        let mut measurer = LatencyMeasurer::new(1000);
        group.bench_function(
            format!("aggregator_reduce_step_{}_partitions", num_partitions),
            |b| {
                let mut i = 0u32;
                b.iter(|| {
                    let _latency_guard = measurer.measure_with_guard();
                    source.append(RawData {
                        id: i % num_partitions,
                        value: 1.0,
                        ..Default::default()
                    });
                    source_reader.next();
                    aggregator
                        .from(&source_reader)
                        .to(&mut target)
                        .partition_by(|r| r.id)
                        .reduce(|_idx, r, s, _keep| {
                            s.id = r.id;
                            s.sum += r.value;
                            s.count += 1;
                        });
                    i += 1;
                });
            },
        );
        println!(
            "aggregator_reduce_step_{}_partitions latency:{}",
            num_partitions,
            measurer.format_stats()
        );
    }

    group.finish();
}

fn bench_window(c: &mut Criterion) {
    let engine = RodaEngine::new();
    let mut group = c.benchmark_group("window_component");

    let size = 8 * 1024 * 1024 * 1024;
    let mut source = engine.new_journal_store::<RawData>(JournalStoreOptions {
        name: "bench_window_source",
        size,
        in_memory: true,
    });
    let mut target = engine.new_journal_store::<RawData>(JournalStoreOptions {
        name: "bench_window_target",
        size,
        in_memory: true,
    });

    let source_reader = source.reader();
    let window: Window<RawData, RawData> = Window::new();

    for window_size in [10, 100] {
        let mut measurer = LatencyMeasurer::new(1000);
        group.bench_function(format!("window_reduce_size_{}", window_size), |b| {
            let mut i = 0u32;
            b.iter(|| {
                let _latency_guard = measurer.measure_with_guard();
                source.append(RawData {
                    id: i,
                    value: i as f64,
                    ..Default::default()
                });
                source_reader.next();
                window
                    .from(&source_reader)
                    .to(&mut target)
                    .reduce(window_size, |data| {
                        let sum: f64 = data.iter().map(|d| d.value).sum();
                        Some(RawData {
                            id: data.last().unwrap().id,
                            value: sum / data.len() as f64,
                            ..Default::default()
                        })
                    });
                i += 1;
            });
        });
        println!(
            "window_reduce_size_{} latency:{}",
            window_size,
            measurer.format_stats()
        );
    }

    group.finish();
}

fn bench_mixed(c: &mut Criterion) {
    let engine = RodaEngine::new();
    let mut group = c.benchmark_group("mixed_pipeline");

    let size = 8 * 1024 * 1024 * 1024;
    let mut s1 = engine.new_journal_store::<RawData>(JournalStoreOptions {
        name: "mixed_s1",
        size,
        in_memory: true,
    });
    let mut s2 = engine.new_journal_store::<AggregatedData>(JournalStoreOptions {
        name: "mixed_s2",
        size,
        in_memory: true,
    });
    let mut s3 = engine.new_journal_store::<AggregatedData>(JournalStoreOptions {
        name: "mixed_s3",
        size,
        in_memory: true,
    });

    let r1 = s1.reader();
    let r2 = s2.reader();

    let aggregator: Aggregator<RawData, AggregatedData, u32> = Aggregator::new();
    let window: Window<AggregatedData, AggregatedData> = Window::new();

    let mut measurer = LatencyMeasurer::new(1000);
    group.bench_function("mixed_pipeline", |b| {
        let mut i = 0u32;
        b.iter(|| {
            let _latency_guard = measurer.measure_with_guard();
            // Push to S1
            s1.append(RawData {
                id: i % 10,
                value: 1.0,
                ..Default::default()
            });

            // Aggregator: S1 -> S2
            r1.next();
            aggregator
                .from(&r1)
                .to(&mut s2)
                .partition_by(|r| r.id)
                .reduce(|_idx, r, s, _keep| {
                    s.id = r.id;
                    s.sum += r.value;
                    s.count += 1;
                });

            // Window: S2 -> S3
            r2.next();
            window.from(&r2).to(&mut s3).reduce(5, |data| {
                let sum: f64 = data.iter().map(|d| d.sum).sum();
                Some(AggregatedData {
                    id: 0, // Mixed
                    sum,
                    count: data.iter().map(|d| d.count).sum(),
                    ..Default::default()
                })
            });

            i += 1;
        });
    });
    println!("mixed_pipeline latency:{}", measurer.format_stats());

    group.finish();
}

criterion_group!(
    benches,
    bench_index,
    bench_aggregator,
    bench_window,
    bench_mixed
);
criterion_main!(benches);
