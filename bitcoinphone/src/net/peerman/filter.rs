use sv::util::BloomFilter;
use std::sync::{RwLock, Arc};

const MAX_BLOOM_HIT_RATE: f64 = 1e-7;

pub struct BloomFilterState {
    hit_rate: f64,
    max_items: f64,
    current_filter: RwLock<BloomFilter>,
}

impl BloomFilterState {
    pub fn new(items: f64) -> Arc<BloomFilterState> {
        let filter = BloomFilter::new(items, MAX_BLOOM_HIT_RATE)
            .unwrap();

        return Arc::from(BloomFilterState{
            hit_rate: MAX_BLOOM_HIT_RATE,
            max_items: items,
            current_filter: RwLock::from(filter),
        });
    }

    pub fn get_filter(self: Arc<Self>) -> BloomFilter {
        return (*self.current_filter
            .read()
            .unwrap())
            .clone();
    }

    pub fn update_filter(self: Arc<Self>, item: &[u8]) {
        self.current_filter
            .write()
            .unwrap()
            .add(item);
    }
}
