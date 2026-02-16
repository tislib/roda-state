# Roda

Ultra-high-performance, low-latency state computer for real-time analytics and event-driven systems. Roda lets you build
deterministic streaming pipelines with cache-friendly dataflows, wait-free reads, and explicit memory bounds—ideal for
IoT, telemetry, industrial automation, and any workload where microseconds matter.

> Status: Early design and API preview. Examples and tests illustrate the intended DX. Expect rapid iteration and
> breaking changes.

---

## Why Roda?

- **Deterministic performance:** Explicit store sizes, preallocated buffers, back-pressure free write path.
- **Low latency by construction:** Reader APIs are designed for zero/constant allocations and predictable access patterns.
- **Multistage Pipelines:** Orchestrate processing stages in dedicated threads with `StageEngine`.
- **Declarative Composition:** Build complex logic using the `pipe!` macro and reusable components like `stateful`, `delta`, and `dedup_by`.
- **Simple Concurrency:** Single-writer/multi-reader patterns with lock-free coordination.

## Core Concepts

- **StageEngine:** The primary entry point for building pipelines. It manages a sequence of stages, each running in its own thread.
- **JournalStore<T>:** A bounded, cache-friendly append-only buffer. Data stays in memory-mapped regions; consumers receive borrowed views.
- **SlotStore<T>:** A bounded store for state that needs to be updated by "slots" or addresses, rather than appended.
- **Stage & Pipe:**
    - **Stage:** A trait for processing items from `In` to `Out`.
    - **pipe!:** A macro to chain multiple processing steps into a single stage or across stages.
- **Pipe Components:**
    - `map`: Simple 1-to-1 transformation.
    - `filter`: Drop items based on a predicate.
    - `stateful`: Partitioned reduction/aggregation with per-key state.
    - `delta`: Compare current item with the previous one for the same key.
    - `dedup_by`: Drop redundant items based on a custom key.

---

For a deep dive into Roda's memory model, zero-copy internals, and execution patterns, see [DESIGN.md](DESIGN.md).

---

## Quick Start

Add `roda-state` to your `Cargo.toml`:

```toml
[dependencies]
roda-state = "0.1"
```

## Example: From Sensor Readings to Alerts

```rust
use roda_state::{StageEngine, pipe, stateful, delta};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Reading {
    sensor_id: u64,
    value: f64,
    timestamp: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Summary {
    sensor_id: u64,
    avg: f64,
    count: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable, Debug)]
struct Alert {
    sensor_id: u64,
    severity: i32,
}

fn main() {
    // 1. Initialize StageEngine
    let engine = StageEngine::<Reading, Reading>::with_capacity(1_000_000);

    // 2. Add Aggregation Stage: Reading -> Summary
    let engine = engine.add_stage(pipe![
        stateful(
            |r| r.sensor_id,
            |r| Summary { sensor_id: r.sensor_id, avg: r.value, count: 1 },
            |s, r| {
                s.avg = (s.avg * s.count as f64 + r.value) / (s.count + 1) as f64;
                s.count += 1;
            }
        )
    ]);

    // 3. Add Anomaly Detection Stage: Summary -> Alert
    let mut engine = engine.add_stage(pipe![
        delta(
            |s: &Summary| s.sensor_id,
            |curr, prev| {
                if let Some(p) = prev && curr.avg > p.avg * 1.5 {
                    return Some(Alert { sensor_id: curr.sensor_id, severity: 1 });
                }
                None
            }
        )
    ]);

    // 4. Ingest & Receive
    engine.send(&Reading { sensor_id: 1, value: 10.0, timestamp: 1 });
    engine.send(&Reading { sensor_id: 1, value: 20.0, timestamp: 2 }); // Jumps by 2x

    // Give workers a moment to process
    std::thread::sleep(std::time::Duration::from_millis(10));

    while let Some(alert) = engine.try_receive() {
        println!("{:?}", alert);
    }
}
```

## Features

- **Blazing Fast:** Designed for microsecond-level latency using memory-mapped buffers.
- **Zero-Copy:** Data is borrowed directly from shared memory regions; no unnecessary allocations on the hot path.
- **Lock-Free:** Single-Writer Multi-Reader (SWMR) pattern with atomic coordination.
- **Deterministic:** Explicit memory management and pre-allocated stores prevent GC pauses.
- **Declarative API:** Build complex data processing pipelines using the `pipe!` macro.

---

## License

Licensed under the Apache License, Version 2.0. See the LICENSE file for details.
