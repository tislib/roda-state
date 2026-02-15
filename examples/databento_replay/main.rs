use clap::Parser;
use spdlog::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

use roda_state::{StageEngine, delta, latency, pipe, progress, stateful, track_prev};

mod aggregation_stage;
mod analysis_stage;
mod book_level_entry;
mod book_level_top;
mod imbalance_signal;
mod importer;
mod light_mbo_delta;
mod light_mbo_entry;

use crate::analysis_stage::AnalysisStage;
use crate::book_level_entry::BookLevelEntry;
use crate::light_mbo_delta::MboDelta;
use crate::light_mbo_entry::LightMboEntry;
use importer::import_mbo_file;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    info!("[System] Booting Roda Data Bento Replay with StageEngine...");

    // 1. Initialize StageEngine with enough capacity for the input
    // Using 30M as in original example
    let mut engine = StageEngine::with_capacity(30_000_000);
    engine.enable_latency_stats(true);

    // 2. Add Aggregation Stage: LightMboEntry -> BookLevelEntry
    let engine = engine.add_stage_with_capacity(
        30_000_000,
        pipe![
            progress("Aggregation", 10_000_000),
            delta(
                |entry: &LightMboEntry| entry.order_id, // group by order_id
                |curr, prev| {
                    if let Some(prev) = prev {
                        return Some(MboDelta {
                            ts: curr.ts,
                            price: curr.price,
                            side: curr.side as u64,
                            delta: curr.size as i32 - prev.size as i32,
                            instrument_id: curr.instrument_id,
                        });
                    }
                    None
                }
            ),
            stateful::<(u64, u32), MboDelta, BookLevelEntry>(
                |entry| (entry.side, entry.instrument_id),
                |entry| BookLevelEntry::init(entry),
                |level, entry| BookLevelEntry::update(level, entry)
            )
        ],
    );

    // 3. Add Imbalance Analysis Stage: BookLevelEntry -> ImbalanceSignal
    let mut engine = engine.add_stage_with_capacity(
        30_000_000,
        pipe![
            progress("Imbalance Analysis", 10_000_000),
            latency(
                "Imbalance Analysis",
                10_000_000,
                1000,
                AnalysisStage::default()
            )
        ],
    );

    import_mbo_file(args.file, &mut engine)?;

    info!("[System] Waiting for all stages to finish processing...");
    engine.await_idle(Duration::from_secs(600));

    info!("[System] Final Imbalance Signals: {}", engine.output_size());
    info!("[System] Done!");

    Ok(())
}
