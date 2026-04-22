use crate::error::Result;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

pub struct LLMCache {
    cache: Mutex<LruCache<String, String>>,
}

impl LLMCache {
    pub fn new(max_size: usize) -> Self {
        let cache_size = NonZeroUsize::new(max_size).unwrap_or(NonZeroUsize::new(1000).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(cache_size)),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.lock().unwrap().get(key).cloned()
    }

    pub fn put(&self, key: String, value: String) {
        self.cache.lock().unwrap().put(key, value);
    }

    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }
}
