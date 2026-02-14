use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use dbn::Record;
use dbn::decode::{DbnDecoder as Decoder, DecodeRecordRef};
use dbn::enums::rtype;
use dbn::record::MboMsg;
use spdlog::prelude::*;

// Use your specific high-level API modules
use crate::light_mbo_entry::LightMboEntry;
use roda_state::components::Appendable;
use roda_state::measure::latency_measurer::LatencyMeasurer;

pub fn import_mbo_file(
    file: PathBuf,
    market_store: &mut impl Appendable<LightMboEntry>,
) -> Result<(), Box<dyn Error>> {
    info!("[Writer] Starting Feed Handler for {:?}...", file);
    let mut latency_measurer = LatencyMeasurer::new(1);
    let start = Instant::now();
    let mut count = 0u64;

    // 1. Setup Decoder
    let mut decoder = Decoder::from_zstd_file(&file)?;

    // 3. Hot Loop
    while let Some(record) = decoder.decode_record_ref()? {
        let _latency_guard = latency_measurer.measure_with_guard();
        if record.header().rtype == rtype::MBO {
            let msg = record.get::<MboMsg>().unwrap();
            market_store.append(LightMboEntry::from(msg));
            count += 1;
        }
    }

    let duration = start.elapsed();
    info!(
        "[Writer] Finished! Pushed {} updates in {:?}",
        count, duration
    );
    // info!("[Writer] Store size: {}", market_store.size());
    info!("[Latency/Import]{}", latency_measurer.format_stats());
    Ok(())
}
