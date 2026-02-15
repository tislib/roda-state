# Roda

Ultra-high-performance, low-latency state computer for real-time analytics and event-driven systems. Roda lets you build
deterministic streaming pipelines with cache-friendly dataflows, wait-free reads, and explicit memory bounds—ideal for
IoT, telemetry, industrial automation, and any workload where microseconds matter.

> Status: Early design and API preview. Examples and tests illustrate the intended DX. Expect rapid iteration and
> breaking changes.

---

## Why Roda?

- Deterministic performance: Explicit store sizes, preallocated buffers, back-pressure free write path by design goals.
- Low latency by construction: Reader APIs are designed for zero/constant allocations and predictable access patterns.
- Declarative pipelines: Express processing in terms of partitions, reductions, and sliding windows.
- Indexable state: Build direct indexes for O(1) lookups into rolling state.
- Simple concurrency model: Long-lived workers with single-writer/multi-reader patterns.

## Core Concepts

- **Engine:** Orchestrates workers (long-lived tasks) that advance your pipelines.
- **Store<T>:** A bounded, cache-friendly append-only buffer that holds your state. You choose the capacity up front.
    - `push(value)`: Append a new item (typically by a single writer thread).
    - `reader()`: Returns a `StoreReader` view appropriate for consumers.
    - `direct_index<Key>()`: Build a secondary index over the store.
- **StoreReader<T>:** A cursor-based handle for consuming state from a `Store`.
    - `next()`: Advance the cursor to the next available item.
    - `get()`, `get_at(at)`, `get_last()`: Retrieve a copy of the state.
    - `get_window::<N>(at)`: Retrieve a fixed-size window of state.
    - `with(|state| ...)`, `with_at(at, |state| ...)`, `with_last(|state| ...)`: Execute a closure with a borrowed reference.
- **Aggregator<In, Out, Key = ()>:** A partitioned reducer for turning event streams into rolling state.
    - `from(&reader)`: Set the input source.
    - `to(&mut store)`: Set the output target.
    - `partition_by(|in| Key)`: Assign each input to a partition.
    - `reduce(|idx, in, out| ...)`: Merge an input into the current output for its partition; `idx` is 0-based within the partition window.
- **Window<In, Out>:** A fixed-size sliding window over the input store.
    - `from(&reader)`: Set the input source.
    - `to(&mut store)`: Set the output target.
    - `reduce(window_size, |window: &[In]| -> Option<Out>)`: Compute optional output when the window is advanced.
- **DirectIndex<Key, Value>:** Build and query secondary indexes over a store for O(1) state lookups.
    - `compute(|value| Key)`: Manually update the index for the next available item in the store (typically called inside a worker).

---

For a deep dive into Roda's memory model, zero-copy internals, and execution patterns, see [DESIGN.md](DESIGN.md).

- **Shared-Nothing Strategy:** While data is shared for efficiency, workers maintain independent logic and state to avoid contention.
- **Microsecond Precision:** Built specifically for systems where every microsecond of jitter impacts the bottom line.
- **Cache-Friendly:** Data layout is optimized for CPU cache lines, minimizing cache misses during pipeline execution.
- **Built-in Indexing:** O(1) secondary lookups without the overhead of a general-purpose database.

## Architecture at a Glance

Roda is designed as a **Shared-Memory, Single-Writer Multi-Reader (SWMR)** system:
- **Zero-Copy:** Data stays in memory-mapped stores; consumers receive borrowed views.
- **Lock-Free:** Coordination happens via Atomic Sequence Counters with Acquire/Release semantics.
- **Deterministic:** Memory is pre-allocated; no allocations on the hot path.
- **Declarative:** Pipelines are built by connecting `Store`, `Aggregator`, and `Window` primitives.

## Features

- **Blazing Fast:** Designed for microsecond-level latency using memory-mapped buffers.
- **Zero-Copy:** Data is borrowed directly from shared memory regions; no unnecessary allocations on the hot path.
- **Lock-Free:** Single-Writer Multi-Reader (SWMR) pattern with atomic coordination.
- **Deterministic:** Explicit memory management and pre-allocated stores prevent GC pauses or unexpected heap allocations.
- **Declarative API:** Build complex data processing pipelines using `Aggregator`, `Window`, and `Index` primitives.

## Quick Start

Add `roda-state` to your `Cargo.toml`:

```toml
[dependencies]
roda-state = "0.1"
```

Or if you're working from this repository:

```toml
[dependencies]
roda-state = { path = "." }
```

Run the example:

```bash
cargo run --example sensor_test
```

## Example: From Sensor Readings to Summaries to Alerts

Below is a trimmed version of `examples/sensor_test.rs` that demonstrates a two-stage pipeline: aggregate raw sensor readings into statistical summaries, then derive alerts when anomalies are detected via a sliding window.

```rust
use bytemuck::{Pod, Zeroable};
use roda_state::components::{Engine, Index, Store, StoreOptions, StoreReader};
use roda_state::{Aggregator, RodaEngine, Window};
use std::thread;
use std::time::Duration;

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Reading {
    sensor_id: u64,
    value: f64,
    timestamp: u64,
}

impl Reading {
    fn from(sensor_id: u64, value: f64, timestamp: u64) -> Self {
        Self { sensor_id, value, timestamp }
    }
}


#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Summary {
    sensor_id: u64,
    min: f64,
    max: f64,
    avg: f64,
    count: u64,
    timestamp: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Alert {
    sensor_id: u64,
    timestamp: u64,
    severity: i32,
    _pad0: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
struct SensorKey {
    sensor_id: u64,
    timestamp: u64,
}

fn main() {
    let engine = RodaEngine::new();

    // 1. Allocate bounded stores
    let mut reading_store = engine.store::<Reading>(StoreOptions {
        name: "readings",
        size: 1_000_000,
        in_memory: true,
    });
    let reading_reader = reading_store.reader();

    let mut summary_store = engine.store::<Summary>(StoreOptions {
        name: "summaries",
        size: 10_000,
        in_memory: true,
    });
    let summary_reader = summary_store.reader();

    let mut alert_store = engine.store::<Alert>(StoreOptions {
        name: "alerts",
        size: 10_000,
        in_memory: true,
    });
    let alert_reader_for_print = alert_store.reader();

    let summary_index = summary_store.direct_index::<SensorKey>();

    // 2. Declare pipelines
    let summary_pipeline: Aggregator<Reading, Summary, SensorKey> = Aggregator::new();
    let alert_pipeline: Window<Summary, Alert> = Window::new();

    // 3. Worker 1: aggregate readings -> summaries and maintain index
    engine.run_worker(move || {
        reading_reader.next();
        summary_pipeline
            .from(&reading_reader)
            .to(&mut summary_store)
            .partition_by(|r| SensorKey { 
                sensor_id: r.sensor_id, 
                timestamp: r.timestamp / 100_000 
            })
            .reduce(|i, r, s| {
                if i == 0 {
                    *s = Summary {
                        sensor_id: r.sensor_id,
                        min: r.value, max: r.value, avg: r.value, count: 1,
                        timestamp: (r.timestamp / 100_000) * 100_000,
                    };
                } else {
                    s.min = s.min.min(r.value);
                    s.max = s.max.max(r.value);
                    s.avg = (s.avg * s.count as f64 + r.value) / (s.count + 1) as f64;
                    s.count += 1;
                }
            });
        
        summary_index.compute(|s| SensorKey { 
            sensor_id: s.sensor_id, 
            timestamp: s.timestamp / 100_000 
        });
    });

    // 4. Worker 2: alert on average jumps
    engine.run_worker(move || {
        summary_reader.next();
        alert_pipeline
            .from(&summary_reader)
            .to(&mut alert_store)
            .reduce(2, |w| {
                let (prev, cur) = (w[0], w[1]);
                (cur.avg > prev.avg * 1.5).then(|| Alert { 
                    sensor_id: cur.sensor_id, 
                    timestamp: cur.timestamp, 
                    severity: 1,
                    ..Default::default()
                })
            });
    });

    // 5. Data Ingestion
    reading_store.push(Reading::from(1, 10.0, 10_000));
    reading_store.push(Reading::from(1, 12.0, 20_000));
    reading_store.push(Reading::from(1, 20.0, 110_000));
    reading_store.push(Reading::from(1, 22.0, 120_000));

    thread::sleep(Duration::from_millis(100));

    // 6. Print Results
    while alert_reader_for_print.next() {
        if let Some(a) = alert_reader_for_print.get() {
            println!("{:?}", a);
        }
    }
}
```

Explore the full example in `examples/sensor_test.rs` for more context.

## Contributing

Contributions are welcome! If you have ideas, issues, or benchmarks:

- Open an issue to discuss the use-case and constraints
- Keep PRs focused and measured; include micro-benchmarks when changing hot paths
- Follow the existing code style and formatting

## License

Licensed under the Apache License, Version 2.0. See the LICENSE file for details.
