//! In-memory cache using moka.

use super::PriceCache;
use crate::types::Product;
use async_trait::async_trait;
use moka::future::Cache;
use std::time::Duration;

/// In-memory cache using moka.
///
/// Thread-safe, async-ready, with automatic expiration.
///
/// # Example
///
/// ```ignore
/// use infracost_rs::Client;
/// use infracost_rs::cache::MemoryCache;
///
/// let client = Client::builder()
///     .with_cache(MemoryCache::new())
///     .build()?;
/// ```
pub struct MemoryCache {
    cache: Cache<String, Vec<Product>>,
}

impl MemoryCache {
    /// Create a new in-memory cache with default settings.
    ///
    /// Defaults: 10,000 max entries, 24-hour TTL.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Create a builder for custom configuration.
    pub fn builder() -> MemoryCacheBuilder {
        MemoryCacheBuilder::default()
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PriceCache for MemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<Product>> {
        self.cache.get(key).await
    }

    async fn set(&self, key: &str, products: &[Product], _ttl: Duration) {
        // TTL is set at cache construction time for moka
        self.cache.insert(key.to_string(), products.to_vec()).await;
    }

    async fn clear(&self) {
        self.cache.invalidate_all();
    }
}

/// Builder for MemoryCache.
pub struct MemoryCacheBuilder {
    max_capacity: u64,
    ttl: Duration,
}

impl Default for MemoryCacheBuilder {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            ttl: super::DEFAULT_CACHE_TTL,
        }
    }
}

impl MemoryCacheBuilder {
    /// Set maximum number of entries.
    pub fn max_capacity(mut self, capacity: u64) -> Self {
        self.max_capacity = capacity;
        self
    }

    /// Set time-to-live for entries.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Build the cache.
    pub fn build(self) -> MemoryCache {
        MemoryCache {
            cache: Cache::builder()
                .max_capacity(self.max_capacity)
                .time_to_live(self.ttl)
                .build(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_cache_get_set() {
        let cache = MemoryCache::new();
        let products = vec![Product {
            product_hash: "test".to_string(),
            vendor_name: "aws".to_string(),
            service: "AmazonEC2".to_string(),
            product_family: None,
            region: Some("us-east-1".to_string()),
            sku: "ABC123".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        cache
            .set("test-key", &products, Duration::from_secs(60))
            .await;
        let cached = cache.get("test-key").await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_memory_cache_miss() {
        let cache = MemoryCache::new();
        let cached = cache.get("nonexistent").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_clear() {
        let cache = MemoryCache::new();
        let products = vec![];

        cache.set("key1", &products, Duration::from_secs(60)).await;
        cache.set("key2", &products, Duration::from_secs(60)).await;

        assert!(cache.get("key1").await.is_some());

        cache.clear().await;

        // Note: moka's invalidate_all is async, may need sync point
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(cache.get("key1").await.is_none());
    }
}
