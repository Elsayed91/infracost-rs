//! SQLite-backed cache for local persistent caching.

use super::PriceCache;
use crate::types::Product;
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// SQLite-backed cache for local persistent caching.
///
/// # Example
///
/// ```ignore
/// use infracost_rs::Client;
/// use infracost_rs::cache::SqliteCache;
///
/// // From file path
/// let cache = SqliteCache::new("./cache.db").await?;
///
/// // From existing pool
/// let cache = SqliteCache::from_pool(pool).await?;
///
/// let client = Client::builder()
///     .with_cache(cache)
///     .build()?;
/// ```
pub struct SqliteCache {
    pool: SqlitePool,
}

/// Builder for configuring SqliteCache.
pub struct SqliteCacheBuilder {
    path: String,
    max_connections: u32,
}

impl Default for SqliteCacheBuilder {
    fn default() -> Self {
        Self {
            path: "infracost_cache.db".to_string(),
            max_connections: 5,
        }
    }
}

impl SqliteCacheBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database file path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set the maximum number of connections in the pool.
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Build the SqliteCache.
    pub async fn build(self) -> Result<SqliteCache, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(&self.path)?.create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(self.max_connections)
            .connect_with(options)
            .await?;

        SqliteCache::from_pool(pool).await
    }
}

impl SqliteCache {
    /// Create a new SQLite cache from a file path.
    ///
    /// Creates the database file if it doesn't exist and runs migrations.
    pub async fn new(path: &str) -> Result<Self, sqlx::Error> {
        SqliteCacheBuilder::new().path(path).build().await
    }

    /// Create a SQLite cache from an existing pool.
    ///
    /// Runs migrations on the provided pool.
    /// Note: `SqlitePool` is internally `Arc`-based, so cloning is cheap.
    pub async fn from_pool(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        // Run migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS infracost_cache (
                key TEXT PRIMARY KEY NOT NULL,
                data TEXT NOT NULL,
                expires_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_infracost_cache_expires_at
            ON infracost_cache(expires_at)
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// Create a builder for more configuration options.
    pub fn builder() -> SqliteCacheBuilder {
        SqliteCacheBuilder::new()
    }

    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    fn spawn_cleanup(&self) {
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let now = Self::current_timestamp();
            let result = sqlx::query("DELETE FROM infracost_cache WHERE expires_at < ?")
                .bind(now)
                .execute(&pool)
                .await;

            if let Err(e) = result {
                tracing::warn!("Failed to cleanup expired cache entries: {}", e);
            }
        });
    }
}

#[async_trait]
impl PriceCache for SqliteCache {
    async fn get(&self, key: &str) -> Option<Vec<Product>> {
        let now = Self::current_timestamp();

        let result =
            sqlx::query("SELECT data FROM infracost_cache WHERE key = ? AND expires_at > ?")
                .bind(key)
                .bind(now)
                .fetch_optional(&self.pool)
                .await;

        // Spawn non-blocking cleanup
        self.spawn_cleanup();

        match result {
            Ok(Some(row)) => {
                let data: String = row.get("data");
                serde_json::from_str(&data).ok()
            }
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("Failed to get cache key {}: {}", key, e);
                None
            }
        }
    }

    async fn set(&self, key: &str, products: &[Product], ttl: Duration) {
        let Ok(json) = serde_json::to_string(products) else {
            tracing::warn!("Failed to serialize products for cache");
            return;
        };

        let expires_at = Self::current_timestamp() + ttl.as_secs() as i64;

        let result = sqlx::query(
            r#"
            INSERT INTO infracost_cache (key, data, expires_at)
            VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET data = excluded.data, expires_at = excluded.expires_at
            "#,
        )
        .bind(key)
        .bind(json)
        .bind(expires_at)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::warn!("Failed to set cache: {}", e);
        }
    }

    async fn clear(&self) {
        let result = sqlx::query("DELETE FROM infracost_cache")
            .execute(&self.pool)
            .await;

        if let Err(e) = result {
            tracing::warn!("Failed to clear cache: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_cache_get_set() {
        let cache = SqliteCache::new(":memory:").await.unwrap();

        let products = vec![Product {
            product_hash: "test-hash".to_string(),
            vendor_name: "test-vendor".to_string(),
            service: "test-service".to_string(),
            product_family: Some("test-family".to_string()),
            region: Some("us-east-1".to_string()),
            sku: "test-sku".to_string(),
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
    async fn test_sqlite_cache_miss() {
        let cache = SqliteCache::new(":memory:").await.unwrap();
        let cached = cache.get("nonexistent").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_cache_expiration() {
        let cache = SqliteCache::new(":memory:").await.unwrap();

        let products = vec![Product {
            product_hash: "test-hash".to_string(),
            vendor_name: "test-vendor".to_string(),
            service: "test-service".to_string(),
            product_family: None,
            region: None,
            sku: "test-sku".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        // Set with 1 second TTL
        cache
            .set("expire-key", &products, Duration::from_secs(1))
            .await;

        // Should exist immediately
        assert!(cache.get("expire-key").await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get("expire-key").await.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_cache_clear() {
        let cache = SqliteCache::new(":memory:").await.unwrap();

        let products = vec![];
        cache.set("key1", &products, Duration::from_secs(60)).await;
        cache.set("key2", &products, Duration::from_secs(60)).await;

        assert!(cache.get("key1").await.is_some());

        cache.clear().await;

        // Give cleanup task a moment
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_cache_update() {
        let cache = SqliteCache::new(":memory:").await.unwrap();

        let products1 = vec![Product {
            product_hash: "hash1".to_string(),
            vendor_name: "vendor1".to_string(),
            service: "service1".to_string(),
            product_family: None,
            region: None,
            sku: "sku1".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        let products2 = vec![Product {
            product_hash: "hash2".to_string(),
            vendor_name: "vendor2".to_string(),
            service: "service2".to_string(),
            product_family: None,
            region: None,
            sku: "sku2".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        cache
            .set("update-key", &products1, Duration::from_secs(60))
            .await;
        cache
            .set("update-key", &products2, Duration::from_secs(60))
            .await;

        let cached = cache.get("update-key").await.unwrap();
        assert_eq!(cached[0].product_hash, "hash2");
    }
}
