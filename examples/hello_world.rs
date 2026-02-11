use roda_core::{Aggregator, RodaEngine, Window};
use std::cmp::min;

// ==============================================================================
// 1. DATA CONTRACT
// ==============================================================================

#[derive(Debug, Clone, Copy, Default)]
pub struct Tick {
    pub symbol: u16,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OHLC {
    pub symbol: u16,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeKey {
    pub symbol: u16,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Signal {
    pub symbol: u16,
    pub timestamp: u64,
    pub direction: i8,
    pub size: u16,
}

// ==============================================================================
// 2. DECLARATIVE PIPELINE EXAMPLE
// ==============================================================================

fn main() {
    let engine = RodaEngine::new();

    // A. RESOURCES
    let tick_store = engine.store::<Tick>(1_000_000);
    let ohlc_store = engine.store::<OHLC>(10_000);
    let simple_strategy = engine.store::<Signal>(10_000);

    // The Index tracks where specific candles live in the ring buffer
    let ohlc_index = ohlc_store.direct_index::<TimeKey>();

    // B. PIPELINE
    let mut simple_strategy_pipeline = Window::pipe(ohlc_store.reader(), simple_strategy);
    let mut ohlc_pipeline = Aggregator::pipe(tick_store, ohlc_store);

    // C. WORKER
    engine.run_worker(move || {
        // 1. PARTITION: Map the Tick to a Candle ID (Construct the Key)
        ohlc_pipeline.partition_by(|tick| TimeKey {
            symbol: tick.symbol,
            timestamp: tick.timestamp / 100_000,
        });

        // 2. REDUCE: Merge the Tick into the Candle
        ohlc_pipeline.reduce(|index, tick, candle| {
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
        simple_strategy_pipeline.reduce(2, |candle| {
            let cur = candle[1];
            let prev = candle[0];

            if cur.close > prev.close {
                return Some(Signal {
                    symbol: cur.symbol,
                    timestamp: cur.timestamp,
                    direction: 1,
                    size: min(100, (cur.close - prev.close) as u16),
                });
            }

            None
        })
    });
}
