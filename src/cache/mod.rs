//! Optional caching for pricing API responses.
//!
//! Enable caching with one of the provided implementations:
//!
//! ```ignore
//! use infracost_rs::Client;
//! use infracost_rs::cache::MemoryCache;
//!
//! let client = Client::builder()
//!     .with_cache(MemoryCache::new())
//!     .build()?;
//! ```

#[cfg(feature = "cache-memory")]
mod memory;
#[cfg(feature = "cache-postgres")]
mod postgres;
#[cfg(feature = "cache-redis")]
mod redis;
#[cfg(feature = "cache-sqlite")]
mod sqlite;

#[cfg(feature = "cache-memory")]
pub use memory::{MemoryCache, MemoryCacheBuilder};
#[cfg(feature = "cache-postgres")]
pub use postgres::{PostgresCache, PostgresCacheBuilder};
#[cfg(feature = "cache-redis")]
pub use redis::RedisCache;
#[cfg(feature = "cache-sqlite")]
pub use sqlite::{SqliteCache, SqliteCacheBuilder};

use crate::types::Product;
use async_trait::async_trait;
use std::time::Duration;

/// Default TTL for cached prices (24 hours).
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Trait for caching pricing API responses.
///
/// Implement this trait to provide custom caching backends.
#[async_trait]
pub trait PriceCache: Send + Sync {
    /// Get cached products for a cache key.
    /// Returns `None` if not cached or expired.
    async fn get(&self, key: &str) -> Option<Vec<Product>>;

    /// Cache products with a TTL.
    async fn set(&self, key: &str, products: &[Product], ttl: Duration);

    /// Clear all cached entries (optional).
    async fn clear(&self) {
        // Default: no-op
    }
}

/// Blanket implementation for Arc-wrapped caches.
///
/// This enables shared ownership of cache implementations,
/// useful for testing and multi-client scenarios.
#[async_trait]
impl<T: PriceCache + ?Sized> PriceCache for std::sync::Arc<T> {
    async fn get(&self, key: &str) -> Option<Vec<Product>> {
        (**self).get(key).await
    }

    async fn set(&self, key: &str, products: &[Product], ttl: Duration) {
        (**self).set(key, products, ttl).await;
    }

    async fn clear(&self) {
        (**self).clear().await;
    }
}
