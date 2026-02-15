use clap::Parser;
use spdlog::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

use roda_state::{StageEngine, latency, pipe, progress, track_prev};

mod aggregation_stage;
mod analysis_stage;
mod book_level_entry;
mod book_level_top;
mod imbalance_signal;
mod importer;
mod light_mbo_entry;

use crate::aggregation_stage::AggregationStage;
use crate::analysis_stage::AnalysisStage;
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
            track_prev(),
            latency("Aggregation", 10_000_000, 1000, AggregationStage::default())
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
