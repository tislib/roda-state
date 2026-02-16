use crate::book_level_entry::BookLevelEntry;
use crate::light_mbo_delta::MboDelta;
use fxhash::FxHashMap;
use roda_state::{OutputCollector, Stage};

#[derive(Default)]
pub struct AggregationStage {
    book_volumes: FxHashMap<(u32, u8, i64), BookLevelEntry>,
}

impl Stage<MboDelta, BookLevelEntry> for AggregationStage {
    fn process<C>(&mut self, delta: &MboDelta, collector: &mut C)
    where
        C: OutputCollector<BookLevelEntry>,
    {
        if delta.is_clear != 0 {
            self.book_volumes
                .retain(|(inst_id, _, _), _| *inst_id != delta.instrument_id);
            // Notify downstream to clear book levels for both sides
            collector.push(&BookLevelEntry {
                ts: delta.ts,
                ts_recv: delta.ts_recv,
                symbol: delta.instrument_id as u64,
                side: b'B',
                volume: 0,
                ..Default::default()
            });
            collector.push(&BookLevelEntry {
                ts: delta.ts,
                ts_recv: delta.ts_recv,
                symbol: delta.instrument_id as u64,
                side: b'A',
                volume: 0,
                ..Default::default()
            });
            return;
        }

        let key = (delta.instrument_id, delta.side, delta.price);
        let book = self.book_volumes.entry(key).or_insert(BookLevelEntry {
            ts: delta.ts,
            ts_recv: delta.ts_recv,
            symbol: delta.instrument_id as u64,
            price: delta.price,
            volume: 0,
            side: delta.side,
            _pad: [0; 7],
        });

        book.ts = delta.ts;
        book.ts_recv = delta.ts_recv;

        // Apply delta
        let new_volume = (book.volume as i64 + delta.delta as i64).max(0) as u64;
        book.volume = new_volume;

        // Always push the update so downstream knows about deletions/volume=0
        collector.push(book);

        if book.volume == 0 {
            self.book_volumes.remove(&key);
        }
    }
}
