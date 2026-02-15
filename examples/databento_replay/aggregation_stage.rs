use std::collections::HashMap;
use roda_state::stage::{Stage, OutputCollector};
use crate::light_mbo_entry::LightMboEntry;
use crate::book_level_entry::BookLevelEntry;

pub struct AggregationStage {
    book_volumes: HashMap<(u32, u8, i64), BookLevelEntry>,
}

impl Default for AggregationStage {
    fn default() -> Self {
        Self {
            book_volumes: HashMap::new(),
        }
    }
}

impl Stage<LightMboEntry, BookLevelEntry> for AggregationStage {
    fn process<C>(&mut self, entry: LightMboEntry, collector: &mut C)
    where
        C: OutputCollector<BookLevelEntry>,
    {
        let key = (entry.instrument_id, entry.side, entry.price);
        let book = self.book_volumes.entry(key).or_insert(BookLevelEntry {
            ts: entry.ts,
            symbol: entry.instrument_id as u64,
            price: entry.price,
            volume: 0,
            side: entry.side,
            _pad: [0; 7],
        });

        book.ts = entry.ts;

        match entry.action {
            // Add
            b'A' => {
                book.volume = book.volume.saturating_add(entry.size as u64);
            }
            // Cancel, Fill, or Trade
            b'C' | b'F' | b'T' => {
                book.volume = book.volume.saturating_sub(entry.size as u64);
            }
            // Clear Book
            b'R' => {
                book.volume = 0;
            }
            _ => {}
        }

        // Always push the update so downstream knows about deletions/volume=0
        collector.push(*book);

        if book.volume == 0 {
            self.book_volumes.remove(&key);
        }
    }
}
