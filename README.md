# Roda

Ultra-high-performance, low-latency state computer for real-time analytics and event-driven systems. Roda lets you build deterministic streaming pipelines with cache-friendly dataflows, wait-free reads, and explicit memory bounds—ideal for IoT, telemetry, industrial automation, and any workload where microseconds matter.

> **Status:** Early design and API preview. Examples and tests illustrate the intended DX. Expect rapid iteration and breaking changes.

---

## Example: From Sensor Readings to Alerts

```rust
use roda_state::{StageEngine, pipe, stateful, delta};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Reading { sensor_id: u64, value: f64, timestamp: u64 }

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Summary { sensor_id: u64, avg: f64, count: u64 }

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable, Debug)]
struct Alert { sensor_id: u64, severity: i32 }

fn main() {
    // 1. Build a multistage pipeline
    let engine = StageEngine::<Reading, Reading>::with_capacity(1_000_000)
        .add_stage(pipe![
            stateful(
                |r| r.sensor_id,
                |r| Summary { sensor_id: r.sensor_id, avg: r.value, count: 1 },
                |s, r| {
                    s.avg = (s.avg * s.count as f64 + r.value) / (s.count + 1) as f64;
                    s.count += 1;
                }
            )
        ])
        .add_stage(pipe![
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

    // 2. Ingest data
    engine.send(&Reading { sensor_id: 1, value: 10.0, timestamp: 1 });
    engine.send(&Reading { sensor_id: 1, value: 20.0, timestamp: 2 });

    // 3. Receive processed alerts
    std::thread::sleep(std::time::Duration::from_millis(10));
    while let Some(alert) = engine.try_receive() {
        println!("{:?}", alert);
    }
}
```

---

## Examples

Explore more detailed implementations in the [examples](examples) folder:

- [**Service Health Monitoring**](examples/service_health/README.md): Demonstrates noise filtering, stateful aggregation, and alert suppression.
- [**Real-Time Sensor Data**](examples/sensor_test/README.md): Showcases statistical windowing and end-to-end latency tracking.
- [**High-Performance MBO Replay**](examples/databento_replay/README.md): A production-ready market data replay and alpha generation system with CPU pinning and zero-allocation hot paths.

---

## Why Roda?

- **Deterministic performance:** Explicit store sizes, preallocated buffers, back-pressure free write path.
- **Low latency by construction:** Reader APIs are designed for zero/constant allocations and predictable access patterns.
- **Multistage Pipelines:** Orchestrate processing stages in dedicated threads with `StageEngine`.
- **Declarative Composition:** Build complex logic using the `pipe!` macro and reusable components.
- **Simple Concurrency:** Single-writer/multi-reader patterns with lock-free coordination.

---

## Performance: Why it is so fast?

Roda is designed for microsecond-level latency by adhering to **Mechanical Sympathy** principles:

- **Static Dispatch:** Everything is resolved at compile time. The `pipe!` macro and generic stages eliminate virtual function calls (`dyn`), allowing the compiler to inline and optimize the entire data flow across component boundaries.
- **Non-blocking Pipelining via `mmap`:** Stages communicate through shared memory-mapped regions. Data written by one stage is immediately visible to the next without kernel-level context switches, syscalls, or expensive memory copies.
- **Single-Writer Multi-Reader (SWMR):** Only the **write index** is atomic and shared between threads. Each reader maintains its own **local read index**, eliminating write-side contention and minimizing cache coherence traffic across CPU cores.
- **Wait-Free Reads:** Readers poll the atomic write index using `Acquire/Release` memory ordering. They never block or wait for other readers or the writer, ensuring predictable, jitter-free processing even under heavy load.
- **Append-only Journal:** Data is stored in pre-allocated, contiguous buffers. This ensures linear memory access patterns, which are highly efficient for CPU prefetchers and maximize cache hit rates.
- **Zero-Copy Principles:** Data is never moved or copied between stages. Consumers receive borrowed views (`&T`) directly into the shared memory regions, eliminating allocation overhead and reducing memory bandwidth pressure.

---

## Core API: The `pipe!` macro

The `pipe!` macro chains processing components into a single execution stage. Each component is executed sequentially for every incoming item.

### `stateful`
Maintains per-key state for partitioned reduction or aggregation.
```rust
stateful(
    |r| r.id,            // Key selector: groups data by ID
    |r| State::new(r),   // Initializer: creates state for a new key
    |s, r| s.update(r)   // Mutator: updates existing state with new input
)
```

### `delta`
Compares the current incoming item with the previous one for the same key. Useful for anomaly detection or calculating rates of change.
```rust
delta(
    |s| s.id,            // Key selector
    |curr, prev| {       // Comparison logic: receives Current and Option<Previous>
        if let Some(p) = prev && curr.val > p.val * 2.0 {
            return Some(Alert::new(curr));
        }
        None
    }
)
```

### `map` & `filter`
Standard functional primitives for transformation and conditional dropping.
```rust
pipe![
    map(|x| x.value * 2),
    filter(|x| *x > 100)
]
```

### `dedup_by`
Filters out redundant items if the calculated key matches the last seen key for that partition.
```rust
dedup_by(|r| r.id)
```

---

## Core Concepts

- **StageEngine:** The primary entry point for building pipelines. It manages a sequence of stages, each running in its own thread.
- **JournalStore<T>:** A bounded, cache-friendly append-only buffer. Data stays in memory-mapped regions; consumers receive borrowed views.
- **SlotStore<T>:** A bounded store for state that needs to be updated by "slots" or addresses, rather than appended.
- **Stage & Pipe:**
    - **Stage:** A unit of execution (thread) that processes items from an input store to an output store.
    - **Pipe:** Composable logic that can be chained within a single stage.

---

## Quick Start

Add `roda-state` to your `Cargo.toml`:

```toml
[dependencies]
roda-state = "0.1"
```

For a deep dive into Roda's memory model, zero-copy internals, and execution patterns, see [DESIGN.md](DESIGN.md).

---

## License

Licensed under the Apache License, Version 2.0. See the LICENSE file for details.
