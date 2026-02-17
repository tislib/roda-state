use crate::book_level_entry::BookLevelEntry;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct BookLevelTopEntry {
    pub size: u64,
    pub price: i64,
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct BookLevelTop {
    pub ts: u64,
    pub ts_recv: u64,
    pub symbol: u64, // or instrument_id
    pub asks: [BookLevelTopEntry; 5],
    pub bids: [BookLevelTopEntry; 5],
    pub _pad: u64,
}

impl BookLevelTop {
    pub(crate) fn adjust(&mut self, entry: BookLevelEntry) {
        self.ts = entry.ts;
        self.ts_recv = entry.ts_recv;
        let levels = match entry.side {
            b'A' => &mut self.asks,
            b'B' => &mut self.bids,
            _ => return,
        };

        if let Some(existing_idx) = levels.iter().position(|l| l.price == entry.price) {
            if entry.volume == 0 {
                for i in existing_idx..4 {
                    levels[i] = levels[i + 1];
                }
                levels[4] = BookLevelTopEntry::default();
            } else {
                levels[existing_idx].size = entry.volume;
            }
            return;
        }

        if entry.volume > 0 {
            // PASS ONLY THE SLICE: This avoids borrowing 'self' again
            Self::insert_if_better(entry, levels);
        }
    }

    // Removed '&mut self' and changed to a static helper
    fn insert_if_better(entry: BookLevelEntry, levels: &mut [BookLevelTopEntry; 5]) {
        let is_ask = entry.side == b'A';

        let pos = levels.iter().position(|l| {
            if l.price == 0 {
                return true;
            }
            if is_ask {
                entry.price < l.price
            } else {
                entry.price > l.price
            }
        });

        if let Some(i) = pos {
            for j in (i + 1..5).rev() {
                levels[j] = levels[j - 1];
            }
            levels[i] = BookLevelTopEntry {
                price: entry.price,
                size: entry.volume,
            };
        }
    }
}

impl From<BookLevelEntry> for BookLevelTop {
    fn from(entry: BookLevelEntry) -> Self {
        Self {
            ts: entry.ts,
            ts_recv: 0,
            symbol: entry.symbol,
            asks: [BookLevelTopEntry::default(); 5],
            bids: [BookLevelTopEntry::default(); 5],
            _pad: 0,
        }
    }
}
