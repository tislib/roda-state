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
use roda_state::Appendable;

pub fn import_mbo_file(
    file: PathBuf,
    market_store: &mut impl Appendable<LightMboEntry>,
) -> Result<(), Box<dyn Error>> {
    info!("[Writer] Starting Feed Handler for {:?}...", file);
    let start = Instant::now();
    let mut count = 0u64;

    // 1. Setup Decoder
    let mut decoder = Decoder::from_zstd_file(&file)?;

    // 3. Hot Loop
    while let Some(record) = decoder.decode_record_ref()? {
        if record.header().rtype == rtype::MBO {
            let msg = record.get::<MboMsg>().unwrap();
            market_store.append(&LightMboEntry::from(msg));
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
