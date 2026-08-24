use crate::font::coverage::CoarseCoverageBitmap;
use crate::font::shaping::ShapedRun;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

const DEFAULT_CACHE_CAPACITY: usize = 128;

pub struct BoundedLruCache<K, V> {
    capacity: usize,
    entries: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> BoundedLruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if self.entries.contains_key(key) {
            // Move key to back of order
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.clone());
            self.entries.get(key).cloned()
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.entries.contains_key(&key) {
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
        } else if self.entries.len() >= self.capacity {
            if let Some(evicted_key) = self.order.pop_front() {
                self.entries.remove(&evicted_key);
            }
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}

pub struct FontRuntimeCache {
    pub coverage_cache: Mutex<BoundedLruCache<String, CoarseCoverageBitmap>>,
    pub shape_cache: Mutex<BoundedLruCache<String, Vec<ShapedRun>>>,
    pub document_font_cache: Mutex<BoundedLruCache<String, Vec<String>>>,
}

impl Default for FontRuntimeCache {
    fn default() -> Self {
        Self {
            coverage_cache: Mutex::new(BoundedLruCache::new(DEFAULT_CACHE_CAPACITY)),
            shape_cache: Mutex::new(BoundedLruCache::new(DEFAULT_CACHE_CAPACITY)),
            document_font_cache: Mutex::new(BoundedLruCache::new(DEFAULT_CACHE_CAPACITY)),
        }
    }
}

static GLOBAL_FONT_CACHE: OnceLock<FontRuntimeCache> = OnceLock::new();

pub fn get_font_cache() -> &'static FontRuntimeCache {
    GLOBAL_FONT_CACHE.get_or_init(FontRuntimeCache::default)
}
