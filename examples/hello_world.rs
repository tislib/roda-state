use roda_core::RodaEngine;

pub type Symbol = [u8; 6];
pub struct SymbolMoment {
    pub timestamp: u64,
    pub symbol: Symbol,
}

pub struct Order {
    pub timestamp: u64,
    pub symbol: Symbol,
    pub quantity: u32,
    pub ask_price: f64,
    pub bid_price: f64,
}

pub struct Price {
    pub symbol: Symbol,
    pub price: f64,
}

fn main() {
    let engine = RodaEngine::new();

    let order_store = engine.store::<Order>();
    let price_store = engine.store::<Price>();
    let order_index = order_store.direct_index::<SymbolMoment>();

    let order_index_shallow = order_index.shallow_clone();

    engine.run_worker(move || {
        order_index.compute();
    });

    engine.run_worker(move || {
        let value = order_index_shallow.get(&SymbolMoment { timestamp: 1, symbol: Symbol::default() });
    });
}
