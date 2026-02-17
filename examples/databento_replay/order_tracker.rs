use crate::light_mbo_delta::MboDelta;
use crate::light_mbo_entry::LightMboEntry;
use fxhash::FxHashMap;
use roda_state::{OutputCollector, Stage};

#[derive(Default)]
pub struct OrderTracker {
    orders: FxHashMap<u64, LightMboEntry>,
}

impl Stage<LightMboEntry, MboDelta> for OrderTracker {
    #[inline(always)]
    fn process<C>(&mut self, entry: &LightMboEntry, collector: &mut C)
    where
        C: OutputCollector<MboDelta>,
    {
        match entry.action {
            // Add
            b'A' => {
                self.orders.insert(entry.order_id, *entry);
                collector.push(&MboDelta {
                    ts: entry.ts,
                    ts_recv: entry.ts_recv,
                    price: entry.price,
                    delta: entry.size as i32,
                    instrument_id: entry.instrument_id,
                    side: entry.side,
                    is_clear: 0,
                    ..Default::default()
                });
            }
            // Cancel, Fill, or Trade
            b'C' | b'F' | b'T' => {
                // For Cancel/Fill, the message size is the size of the event.
                // We should also update our internal tracking if the order isn't completely gone.
                // But DBN MBO usually means order is gone on 'C'. On 'F' it might stay if partial.
                // "The 'F' message represents a fill... If the order is fully filled, it is removed from the book."
                // In DBN, if it's a partial fill, there might be a follow up or the remaining size is what matters.

                // For simplicity and matching the previous 'delta' pipe logic:
                // If it's a Cancel or full Fill, we emit a negative delta.
                collector.push(&MboDelta {
                    ts: entry.ts,
                    ts_recv: entry.ts_recv,
                    price: entry.price,
                    delta: -(entry.size as i32),
                    instrument_id: entry.instrument_id,
                    side: entry.side,
                    is_clear: 0,
                    ..Default::default()
                });

                if entry.action == b'C' {
                    self.orders.remove(&entry.order_id);
                } else if let Some(order) = self.orders.get_mut(&entry.order_id) {
                    order.size = order.size.saturating_sub(entry.size);
                    if order.size == 0 {
                        self.orders.remove(&entry.order_id);
                    }
                }
            }
            // Modify
            b'M' => {
                if let Some(old_order) = self.orders.get_mut(&entry.order_id) {
                    if old_order.price != entry.price {
                        // Price changed: remove old volume, add new volume
                        collector.push(&MboDelta {
                            ts: entry.ts,
                            ts_recv: entry.ts_recv,
                            price: old_order.price,
                            delta: -(old_order.size as i32),
                            instrument_id: entry.instrument_id,
                            side: entry.side,
                            is_clear: 0,
                            ..Default::default()
                        });
                        collector.push(&MboDelta {
                            ts: entry.ts,
                            ts_recv: entry.ts_recv,
                            price: entry.price,
                            delta: entry.size as i32,
                            instrument_id: entry.instrument_id,
                            side: entry.side,
                            is_clear: 0,
                            ..Default::default()
                        });
                    } else {
                        // Price same, size changed
                        collector.push(&MboDelta {
                            ts: entry.ts,
                            ts_recv: entry.ts_recv,
                            price: entry.price,
                            delta: entry.size as i32 - old_order.size as i32,
                            instrument_id: entry.instrument_id,
                            side: entry.side,
                            is_clear: 0,
                            ..Default::default()
                        });
                    }
                    *old_order = *entry;
                } else {
                    // We missed the Add? Treat as Add.
                    self.orders.insert(entry.order_id, *entry);
                    collector.push(&MboDelta {
                        ts: entry.ts,
                        ts_recv: entry.ts_recv,
                        price: entry.price,
                        delta: entry.size as i32,
                        instrument_id: entry.instrument_id,
                        side: entry.side,
                        is_clear: 0,
                        ..Default::default()
                    });
                }
            }
            // Clear Book
            b'R' => {
                self.orders
                    .retain(|_, v| v.instrument_id != entry.instrument_id);
                collector.push(&MboDelta {
                    ts: entry.ts,
                    ts_recv: entry.ts_recv,
                    instrument_id: entry.instrument_id,
                    is_clear: 1,
                    ..Default::default()
                });
            }
            _ => {}
        }
    }
}
