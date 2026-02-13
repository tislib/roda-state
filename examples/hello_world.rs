use bytemuck::{Pod, Zeroable};
use roda_state::components::{Engine, Index, Store, StoreOptions, StoreReader};
use roda_state::{Aggregator, RodaEngine, Window};
use std::cmp::min;
// ==============================================================================
// 1. DATA CONTRACT
// ==============================================================================

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Tick {
    pub symbol: u64,
    pub price: f64,
    pub timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct OHLC {
    pub symbol: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeKey {
    pub symbol: u64,
    pub timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Signal {
    pub symbol: u64,
    pub timestamp: u64,
    pub direction: i32,
    pub size: u32,
}

// ==============================================================================
// 2. DECLARATIVE PIPELINE EXAMPLE
// ==============================================================================

fn main() {
    let engine = RodaEngine::new();

    // A. RESOURCES
    let tick_store = engine.store::<Tick>(StoreOptions {
        name: "ticks",
        size: 1_000_000,
        in_memory: true,
    });
    let tick_reader = tick_store.reader();
    let mut ohlc_store = engine.store::<OHLC>(StoreOptions {
        name: "ohlc",
        size: 10_000,
        in_memory: true,
    });
    let ohlc_reader = ohlc_store.reader();
    let mut simple_strategy = engine.store::<Signal>(StoreOptions {
        name: "simple_strategy",
        size: 10_000,
        in_memory: true,
    });

    // The Index tracks where specific candles live in the ring buffer
    let ohlc_index = ohlc_store.direct_index::<TimeKey>();

    // B. PIPELINE
    let ohlc_pipeline: Aggregator<Tick, OHLC, TimeKey> = Aggregator::new();
    let simple_strategy_pipeline: Window<OHLC, Signal> = Window::new();

    // C. WORKER
    engine.run_worker(move || {
        tick_reader.next();

        // 1. PARTITION: Map the Tick to a Candle ID (Construct the Key)
        ohlc_pipeline
            .from(&tick_reader)
            .to(&mut ohlc_store)
            .partition_by(|tick| TimeKey {
                symbol: tick.symbol,
                timestamp: tick.timestamp / 100_000,
            })
            .reduce(|index, tick, candle| {
                if index == 0 {
                    // Init (First tick in bucket)
                    candle.open = tick.price;
                    candle.high = tick.price;
                    candle.low = tick.price;
                    candle.close = tick.price;

                    // Set Identity
                    candle.symbol = tick.symbol;
                    candle.timestamp = (tick.timestamp / 100_000) * 100_000;
                } else {
                    // Update
                    candle.high = tick.price.max(candle.high);
                    candle.low = tick.price.min(candle.low);
                    candle.close = tick.price;
                }
            });

        // 3. INDEX: Ensure the new candle is discoverable
        // Note: Input is 'candle' (OHLC), not 'tick'
        ohlc_index.compute(|candle| TimeKey {
            symbol: candle.symbol,
            timestamp: candle.timestamp / 100_000,
        });
    });

    engine.run_worker(move || {
        ohlc_reader.next();

        simple_strategy_pipeline
            .from(&ohlc_reader)
            .to(&mut simple_strategy)
            .reduce(2, |candle| {
                let cur = candle[1];
                let prev = candle[0];

                if cur.close > prev.close {
                    return Some(Signal {
                        symbol: cur.symbol,
                        timestamp: cur.timestamp,
                        direction: 1,
                        size: min(100, (cur.close - prev.close) as u32),
                    });
                }

                None
            })
    });
}
