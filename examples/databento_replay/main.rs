use clap::Parser;
use spdlog::kv::Key;
use spdlog::prelude::*;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
// Use your specific high-level API modules
use roda_state::JournalStoreOptions;
use roda_state::components::{Appendable, IterativeReadable};
use roda_state::{Aggregator, DirectIndex, RodaEngine};

mod book_level_entry;
mod importer;
mod light_mbo_entry;

use crate::book_level_entry::BookLevelEntry;
use importer::import_mbo_file;
use light_mbo_entry::LightMboEntry;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    file: PathBuf,
}

// ==============================================================================
// 2. THE PIPELINE IMPLEMENTATION
// ==============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut engine = RodaEngine::new();
    engine.enable_latency_stats(true);
    info!("[System] Booting Roda Data Bento Replay...");

    // 1. Market Data Store (The "River" of MBO updates)
    let mut market_store = engine.new_journal_store::<LightMboEntry>(JournalStoreOptions {
        name: "market_data",
        size: 30000000 * size_of::<LightMboEntry>(),
        in_memory: true,
    });

    let mut market_book_store = engine.new_journal_store::<BookLevelEntry>(JournalStoreOptions {
        name: "market_book",
        size: 30000000 * size_of::<BookLevelEntry>(),
        in_memory: true,
    });

    let market_book_store_reader = market_book_store.reader();
    let final_reader = market_book_store.reader();
    let market_book_store_index = market_book_store.direct_index();
    let market_book_store_index_reader = market_book_store_index.reader();
    let market_book_store_index_reader2 = market_book_store_index.reader();

    let mut market_book_aggregator: Aggregator<LightMboEntry, BookLevelEntry, _> =
        Aggregator::new();

    let market_reader = market_store.reader();

    // Prepare Book Level
    engine.run_worker(move || {
        if market_reader.next() {
            market_book_aggregator
                .from(&market_reader)
                .to(&mut market_book_store)
                .partition_by(|entry| (entry.instrument_id, entry.side, entry.price))
                .reduce(|_, entry, book, keep| {
                    book.side = entry.side;
                    book.price = entry.price;
                    book.symbol = entry.instrument_id as u64;
                    match entry.action {
                        // Add: New liquidity
                        b'A' => {
                            book.volume = book.volume.saturating_add(entry.size as u64);
                        }
                        // Cancel, Fill, or Trade: Remove liquidity
                        // Note: Check your feed docs. Usually 'F' is the one that reduces the book.
                        b'C' | b'F' | b'T' => {
                            book.volume = book.volume.saturating_sub(entry.size as u64);
                        }
                        // Clear Book: Wipe level
                        b'R' => {
                            book.volume = 0;
                        }
                        // Modify: This is tricky without order-id tracking.
                        // For a showcase, if you don't have 'old_size', ignoring it is
                        // safer than guessing, but your book will slowly drift.
                        b'M' | b'N' => {}

                        _ => {}
                    }

                    if book.volume == 0 {
                        market_book_store_index.delete(&(book.side, book.price));
                        *keep = false;
                    }
                });
            market_book_store_index.compute(|entry| (entry.side, entry.price));
        }
    });

    // Prepare Weighted L5 and OB Imbalance
    engine.run_worker(move || {
        if market_book_store_reader.next() {
            // 1. Get Bids: Everything <= (b'B', MAX)
            // We go REV to get Highest Price first
            // 1. Get Bids (Highest Bids first)
            // Range: From (b'B', 0) to (b'B', i64::MAX)
            let bids = market_book_store_index_reader
                .range((
                    std::ops::Bound::Included(&(66, 0)),
                    std::ops::Bound::Included(&(66, i64::MAX)),
                ))
                .rev() // Start at highest price
                .take(5);

            // --- 2. GET ASKS (Lowest prices first) ---
            // Range: From (b'A', 0) to (b'A', i64::MAX)
            let asks = market_book_store_index_reader
                .range((
                    std::ops::Bound::Included((65, 0)),
                    std::ops::Bound::Included((65, i64::MAX)),
                ))
                .take(5); // Already starts at lowest price

            let mut bid_vol = 0.0;
            let mut ask_vol = 0.0;

            // 3. Sum Bids
            for (i, (_key, state)) in bids.enumerate() {
                let weight = 1.0 - (i as f64 * 0.2);
                bid_vol += state.volume as f64 * weight;
            }

            // 4. Sum Asks
            for (i, (_key, state)) in asks.enumerate() {
                let weight = 1.0 - (i as f64 * 0.2);
                ask_vol += state.volume as f64 * weight;
            }

            // 5. Compute Final Imbalance
            let total_vol = bid_vol + ask_vol;
            if total_vol > 0.0 {
                let imbalance = (bid_vol - ask_vol) / total_vol;
                if imbalance > 0.95 {
                    println!(
                        "Imbalance: {:.2} (B: {:.0}, A: {:.0})",
                        imbalance, bid_vol, ask_vol
                    );
                    println!("{:?}", market_book_store_index_reader.size());
                }
            }
        }
    });

    import_mbo_file(args.file, &mut market_store)?;

    info!("[System] Waiting for all workers to finish...");

    engine.await_idle(Duration::from_mins(100));

    info!(
        "[System] Book Size: {}",
        market_book_store_index_reader2.size()
    );

    info!("[System] Done!");

    Ok(())
}
