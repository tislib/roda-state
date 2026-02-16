use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use dbn::Record;
use dbn::decode::{DbnDecoder as Decoder, DecodeRecordRef};
use dbn::enums::rtype;
use dbn::record::MboMsg;
use spdlog::prelude::*;

// Use your specific high-level API modules
use crate::light_mbo_entry::LightMboEntry;
use roda_state::Appendable;

pub fn import_mbo_file(
    file: PathBuf,
    market_store: &mut impl Appendable<LightMboEntry>,
    simulate_live: bool,
) -> Result<(), Box<dyn Error>> {
    info!(
        "[Writer] Starting Feed Handler for {:?} (Simulate Live: {})...",
        file, simulate_live
    );
    let start = Instant::now();
    let mut count = 0u64;

    // 1. Setup Decoder
    let mut decoder = Decoder::from_zstd_file(&file)?;

    let mut first_ts = None;
    let mut first_now = Instant::now();

    while let Some(record) = decoder.decode_record_ref()? {
        if record.header().rtype == rtype::MBO {
            let msg = record.get::<MboMsg>().unwrap();

            if simulate_live {
                if first_ts.is_none() {
                    first_ts = Some(msg.hd.ts_event);
                    first_now = Instant::now();
                    // Small warm-up delay to let threads stabilize
                    std::thread::sleep(Duration::from_millis(50));
                }

                let elapsed_market = msg.hd.ts_event - first_ts.unwrap();
                let elapsed_now = first_now.elapsed().as_nanos() as u64;

                if elapsed_market > elapsed_now {
                    let sleep_dur = Duration::from_nanos(elapsed_market - elapsed_now);
                    if sleep_dur > Duration::from_secs(1) { // reset
                        first_ts = None;
                        first_now = Instant::now();
                    } else if sleep_dur > Duration::from_micros(10) {
                        std::thread::sleep(sleep_dur);
                    }
                }
            } else if count == 0 {
                // Warm-up for backtest mode
                std::thread::sleep(Duration::from_millis(50));
            }

            let ts_recv = crate::latency_tracker::get_relative_nanos();
            market_store.append(&LightMboEntry::from_msg(msg, ts_recv));
            count += 1;
        }
    }

    let duration = start.elapsed();
    info!(
        "[Writer] Finished! Pushed {} updates in {:?}",
        count, duration
    );
    Ok(())
}
