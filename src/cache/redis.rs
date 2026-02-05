//! Redis-backed cache for shared caching across instances.

use super::{DEFAULT_CACHE_TTL, PriceCache};
use crate::types::Product;
use async_trait::async_trait;
use redis::AsyncCommands;
use std::time::Duration;

/// Redis-backed cache for shared caching across instances.
///
/// # Example
///
/// ```ignore
/// use infracost_rs::Client;
/// use infracost_rs::cache::RedisCache;
///
/// let client = Client::builder()
///     .with_cache(RedisCache::new("redis://localhost:6379")?)
///     .build()?;
/// ```
pub struct RedisCache {
    client: redis::Client,
    ttl: Duration,
}

impl RedisCache {
    /// Create a new Redis cache.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL (e.g., "redis://localhost:6379")
    pub fn new(url: &str) -> Result<Self, redis::RedisError> {
        Ok(Self {
            client: redis::Client::open(url)?,
            ttl: DEFAULT_CACHE_TTL,
        })
    }

    /// Set custom TTL for cached entries.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
}

#[async_trait]
impl PriceCache for RedisCache {
    async fn get(&self, key: &str) -> Option<Vec<Product>> {
        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to Redis for cache get: {}", e);
                return None;
            }
        };

        let data: Option<String> = match conn.get(key).await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to get cache key: {}", e);
                return None;
            }
        };

        data.and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn set(&self, key: &str, products: &[Product], ttl: Duration) {
        let Ok(mut conn) = self.client.get_multiplexed_async_connection().await else {
            tracing::warn!("Failed to connect to Redis for cache set");
            return;
        };

        let Ok(json) = serde_json::to_string(products) else {
            tracing::warn!("Failed to serialize products for cache");
            return;
        };

        let ttl_secs = ttl.as_secs().max(1); // Minimum 1 second TTL
        let result: Result<(), _> = conn.set_ex(key, json, ttl_secs).await;

        if let Err(e) = result {
            tracing::warn!("Failed to set cache: {}", e);
        }
    }

    async fn clear(&self) {
        let Ok(mut conn) = self.client.get_multiplexed_async_connection().await else {
            tracing::warn!("Failed to connect to Redis for cache clear");
            return;
        };

        // Only clear infracost keys, not entire Redis
        let keys: Result<Vec<String>, _> = redis::cmd("KEYS")
            .arg("infracost:*")
            .query_async(&mut conn)
            .await;

        match keys {
            Ok(keys) => {
                for key in keys {
                    let _: Result<(), _> = conn.del(&key).await;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to list cache keys for clear: {}", e);
            }
        }
    }
}

// Note: Redis tests require a running Redis instance and are in integration tests
