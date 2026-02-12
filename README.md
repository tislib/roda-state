# Roda

Ultra‑high‑performance, low‑latency state computer for real‑time analytics and trading systems. Roda lets you build
deterministic streaming pipelines with cache‑friendly dataflows, wait‑free reads, and explicit memory bounds — ideal for
HFT, market microstructure research, telemetry, and any workload where microseconds matter.

> Status: early design and API preview. Examples and tests illustrate the intended DX. Expect rapid iteration and
> breaking changes.

---

## Why Roda?

- Deterministic performance: explicit store sizes, preallocated ring buffers, back‑pressure free write path by design
  goals.
- Low latency by construction: reader APIs are designed for zero/constant allocations and predictable access patterns.
- Declarative pipelines: express processing in terms of partitions, reductions, and sliding windows.
- Indexable state: build direct indexes for O(1) lookups into rolling state.
- Simple concurrency model: long‑lived workers with single‑writer/multi‑reader patterns.

## Core concepts

- Engine: orchestrates workers (long‑lived tasks) that advance your pipelines.
- Store<T>: a bounded, cache‑friendly ring buffer that holds your state. You choose the capacity up front.
    - push(value): append a new item (typically by a single writer thread)
    - reader(): returns a `StoreReader` view appropriate for consumers
    - direct_index<Key>(): build a secondary index over the store
- StoreReader<T>: a cursor‑based handle for consuming state from a `Store`.
    - next(): advance the cursor to the next available item
    - get(), get_at(at), get_last(): retrieve a copy of the state
    - get_window::<N>(at): retrieve a fixed‑size window of state
    - with(|state| ...), with_at(at, |state| ...), with_last(|state| ...): execute a closure with a borrowed reference
- Aggregator<In, Out, Key = ()>: a partitioned reducer for turning event streams into rolling state.
    - from(&reader): set the input source
    - to(&mut store): set the output target
    - partition_by(|in| Key): assign each input to a partition
    - reduce(|idx, in, out| ...): merge an input into the current output for its partition; idx is 0‑based within the
      partition window
- Window<In, Out>: a fixed‑size sliding window over the input store.
    - from(&reader): set the input source
    - to(&mut store): set the output target
    - reduce(window_size, |window: &[In]| -> Option<Out>): compute optional output when the window is advanced
- DirectIndex<Key, Value>: build and query secondary indexes over a store for O(1) state lookups.
    - compute(|value| Key): manually update the index for the next available item in the store (typically called inside a worker)

---

For a deep dive into Roda's memory model, zero-copy internals, and execution patterns, see [DESIGN.md](DESIGN.md).

## Architecture at a Glance

Roda is designed as a **Shared-Memory, Single-Writer Multi-Reader (SWMR)** system:
- **Zero-Copy:** Data stays in memory-mapped stores; consumers receive borrowed views.
- **Lock-Free:** Coordination happens via Atomic Sequence Counters with Acquire/Release semantics.
- **Deterministic:** Memory is pre-allocated; no allocations on the hot path.
- **Declarative:** Pipelines are built by connecting `Store`, `Aggregator`, and `Window` primitives.

## Quick start

Using the crate directly:

```bash
# Run the end‑to‑end example
cargo run --example hello_world
```

Or add roda‑state to your own Cargo.toml while working from this repository:

```toml
[dependencies]
roda-state = { path = "." }
```

## Example: from ticks to OHLC to trading signals

Below is a trimmed version of examples/hello_world.rs that demonstrates a two‑stage pipeline: aggregate ticks into OHLC
candles, then derive a simple momentum signal via a sliding window.

```rust
use bytemuck::{Pod, Zeroable};
use roda_state::components::{Index, Store, StoreReader};
use roda_state::{Aggregator, RodaEngine, Window};

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Tick {
    symbol: u64,
    price: f64,
    timestamp: u64
}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct OHLC {
    symbol: u64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    timestamp: u64
}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
struct Signal {
    symbol: u64,
    timestamp: u64,
    direction: i32,
    size: u32
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TimeKey {
    symbol: u64,
    timestamp: u64
}

fn main() {
    let engine = RodaEngine::new();

    // Allocate bounded stores (explicit memory profile)
    let tick_store = engine.store::<Tick>(1_000_000);
    let tick_reader = tick_store.reader();
    let mut ohlc_store = engine.store::<OHLC>(10_000);
    let ohlc_reader = ohlc_store.reader();
    let mut signal_store = engine.store::<Signal>(10_000);

    // Index to locate candles by (symbol, time)
    let ohlc_index = ohlc_store.direct_index::<TimeKey>();

    // Declare pipelines
    let mut ohlc_pipeline: Aggregator<Tick, OHLC, TimeKey> = Aggregator::new();
    let mut strategy_pipeline: Window<OHLC, Signal> = Window::new();

    // Worker 1: aggregate ticks → OHLC and maintain index
    engine.run_worker(move || {
        tick_reader.next();
        ohlc_pipeline
            .from(&tick_reader)
            .to(&mut ohlc_store)
            .partition_by(|t| TimeKey { symbol: t.symbol, timestamp: t.timestamp / 100_000 })
            .reduce(|i, t, c| {
                if i == 0 {
                    c.open = t.price;
                    c.high = t.price;
                    c.low = t.price;
                    c.close = t.price;
                    c.symbol = t.symbol;
                    c.timestamp = (t.timestamp / 100_000) * 100_000;
                } else {
                    c.high = c.high.max(t.price);
                    c.low = c.low.min(t.price);
                    c.close = t.price;
                }
            });
        ohlc_index.compute(|c| TimeKey { symbol: c.symbol, timestamp: c.timestamp / 100_000 });
    });

    // Worker 2: 2‑bar momentum signal
    engine.run_worker(move || {
        ohlc_reader.next();
        strategy_pipeline
            .from(&ohlc_reader)
            .to(&mut signal_store)
            .reduce(2, |w| {
                let prev = w[0];
                let cur = w[1];
                (cur.close > prev.close).then(|| Signal { 
                    symbol: cur.symbol, 
                    timestamp: cur.timestamp, 
                    direction: 1, 
                    size: ((cur.close - prev.close) as u32).min(100) 
                })
            });
    });
}
```

Explore the full example in examples/hello_world.rs for more context.

## Contributing

Contributions are welcome! If you have ideas, issues, or benchmarks:

- Open an issue to discuss the use‑case and constraints
- Keep PRs focused and measured; include micro‑benchmarks when changing hot paths
- Follow the existing code style and formatting

## License

Licensed under the Apache License, Version 2.0. See the LICENSE file for details.
