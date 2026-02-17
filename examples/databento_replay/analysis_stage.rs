use crate::book_level_entry::BookLevelEntry;
use crate::book_level_top::BookLevelTop;
use crate::imbalance_signal::ImbalanceSignal;
use fxhash::FxHashMap;
use roda_state::measure::LatencyMeasurer;
use roda_state::{OutputCollector, Stage};
use spdlog::prelude::*;
use std::time::{Duration, Instant};

pub struct AnalysisStage {
    book_tops: FxHashMap<u64, BookLevelTop>,
    last_print: Instant,
    counter: u64,
    // Tick-to-Signal Latency Measurer
    tts_measurer: LatencyMeasurer,
}

impl Default for AnalysisStage {
    fn default() -> Self {
        Self {
            book_tops: FxHashMap::default(),
            last_print: Instant::now(),
            counter: 0,
            tts_measurer: LatencyMeasurer::new(1), // Sample every 1000th tick
        }
    }
}

impl AnalysisStage {
    /// SIMD-friendly weighted imbalance calculation
    #[inline(always)]
    fn calculate_weighted_imbalance(book_top: &BookLevelTop) -> (f64, f64, f64) {
        const WEIGHTS: [f64; 5] = [1.0, 0.8, 0.6, 0.4, 0.2];
        let mut bid_vol = 0.0;
        let mut ask_vol = 0.0;

        for (i, &weight) in WEIGHTS.iter().enumerate() {
            bid_vol += book_top.bids[i].size as f64 * weight;
            ask_vol += book_top.asks[i].size as f64 * weight;
        }

        let total_vol = bid_vol + ask_vol;
        if total_vol > 0.0 {
            ((bid_vol - ask_vol) / total_vol, bid_vol, ask_vol)
        } else {
            (0.0, 0.0, 0.0)
        }
    }
}

impl Stage<BookLevelEntry, ImbalanceSignal> for AnalysisStage {
    fn process<C>(&mut self, entry: &BookLevelEntry, collector: &mut C)
    where
        C: OutputCollector<ImbalanceSignal>,
    {
        self.counter += 1;
        let book_top = self
            .book_tops
            .entry(entry.symbol)
            .or_insert_with(|| BookLevelTop {
                symbol: entry.symbol,
                ..Default::default()
            });
        book_top.adjust(*entry);

        let (imbalance, bid_vol, ask_vol) = Self::calculate_weighted_imbalance(book_top);

        if bid_vol + ask_vol > 0.0 {
            // Produce the signal
            collector.push(&ImbalanceSignal {
                ts: entry.ts,
                ts_recv: entry.ts_recv,
                symbol: entry.symbol,
                imbalance,
                bid_vol,
                ask_vol,
                _pad: [0; 2],
            });

            if imbalance.abs() > 0.98 && self.last_print.elapsed() > Duration::from_millis(500) {
                info!(
                    "[Sym:{}] High Imbalance: {:.4} (B:{:.0} A:{:.0})",
                    entry.symbol, imbalance, bid_vol, ask_vol
                );
                self.last_print = Instant::now();
            }
        }

        // Record tick-to-signal latency
        if self.counter.is_multiple_of(1000) {
            let now_nanos = crate::latency_tracker::get_relative_nanos();
            let tts_latency = now_nanos.saturating_sub(entry.ts_recv);
            self.tts_measurer.measure(Duration::from_nanos(tts_latency));
        }
    }
}

impl Drop for AnalysisStage {
    fn drop(&mut self) {
        info!(
            "[System] Final Imbalance Signals processed: {}",
            self.counter
        );
        info!(
            "[Analysis] TTS Latency (Tick-to-Signal): {}",
            self.tts_measurer.format_stats()
        );
    }
}
