use clap::Parser;
use spdlog::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

use roda_state::{StageEngine, pipe};

mod aggregation_stage;
mod analysis_stage;
mod book_level_entry;
mod book_level_top;
mod imbalance_signal;
mod importer;
mod latency_tracker;
mod light_mbo_delta;
mod light_mbo_entry;
mod order_tracker;

use crate::aggregation_stage::AggregationStage;
use crate::analysis_stage::AnalysisStage;
use crate::light_mbo_entry::LightMboEntry;
use crate::order_tracker::OrderTracker;
use importer::import_mbo_file;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    file: PathBuf,

    #[arg(long, default_value_t = false)]
    simulate_live: bool,

    /// Pin worker threads to CPU cores
    #[arg(long, default_value_t = false)]
    pin_cores: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    info!("[System] Booting Roda Data Bento Replay with StageEngine...");

    // 1. Initialize StageEngine with enough capacity for the input
    let mut engine = StageEngine::with_capacity(30_000_000);
    engine.set_pin_cores(args.pin_cores);

    if args.pin_cores {
        info!("[System] CPU Pinning enabled for worker threads");
    }

    // 2. Add Order Tracker Stage: LightMboEntry -> MboDelta
    let engine = engine.add_stage_with_capacity(30_000_000, |x: &LightMboEntry| Some(*x));
    let engine = engine.add_stage_with_capacity(30_000_000, pipe![OrderTracker::default()]);

    // 3. Add Aggregation Stage: MboDelta -> BookLevelEntry
    let engine = engine.add_stage_with_capacity(30_000_000, pipe![AggregationStage::default()]);

    // 4. Add Imbalance Analysis Stage: BookLevelEntry -> ImbalanceSignal
    let mut engine = engine.add_stage_with_capacity(30_000_000, pipe![AnalysisStage::default()]);

    let start = std::time::Instant::now();
    import_mbo_file(args.file, &mut engine, args.simulate_live)?;

    info!("[System] Waiting for all stages to finish processing...");
    engine.await_idle(Duration::from_secs(600));

    let duration = start.elapsed();
    let total_msgs = engine.output_size();
    let meps = total_msgs as f64 / duration.as_secs_f64() / 1_000_000.0;

    info!("[System] Final Imbalance Signals: {}", total_msgs);
    info!(
        "[System] Throughput: {:.2} MEPS (Million Events Per Second)",
        meps
    );
    info!("[System] Done in {:?}", duration);

    Ok(())
}
