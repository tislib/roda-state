use crate::book_level_entry::BookLevelEntry;
use crate::book_level_top::BookLevelTop;
use crate::imbalance_signal::ImbalanceSignal;
use roda_state::{OutputCollector, Stage};
use spdlog::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct AnalysisStage {
    book_tops: HashMap<u64, BookLevelTop>,
    last_print: Instant,
    counter: u64,
}

impl Default for AnalysisStage {
    fn default() -> Self {
        Self {
            book_tops: HashMap::new(),
            last_print: Instant::now(),
            counter: 0,
        }
    }
}

impl Stage<BookLevelEntry, ImbalanceSignal> for AnalysisStage {
    fn process<C>(&mut self, entry: BookLevelEntry, collector: &mut C)
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
        book_top.adjust(entry);

        let mut bid_vol = 0.0;
        let mut ask_vol = 0.0;

        for (i, level) in book_top.bids.iter().enumerate() {
            if level.price == 0 {
                break;
            }
            let weight = 1.0 - (i as f64 * 0.2);
            bid_vol += level.size as f64 * weight;
        }

        for (i, level) in book_top.asks.iter().enumerate() {
            if level.price == 0 {
                break;
            }
            let weight = 1.0 - (i as f64 * 0.2);
            ask_vol += level.size as f64 * weight;
        }

        let total_vol = bid_vol + ask_vol;
        if total_vol > 0.0 {
            let imbalance = (bid_vol - ask_vol) / total_vol;

            // Produce the signal
            collector.push(ImbalanceSignal {
                ts: entry.ts,
                symbol: entry.symbol,
                imbalance,
                bid_vol,
                ask_vol,
            });

            if imbalance.abs() > 0.95 && self.last_print.elapsed() > Duration::from_millis(500) {
                info!(
                    "[Sym:{}] Imbalance: {:.2} (B: {:.0}, A: {:.0})",
                    entry.symbol, imbalance, bid_vol, ask_vol
                );
                self.last_print = Instant::now();
            }
        }
    }
}

impl Drop for AnalysisStage {
    fn drop(&mut self) {
        info!(
            "[System] Final Imbalance Signals processed: {}",
            self.counter
        );
    }
}
