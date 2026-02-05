//! PostgreSQL-backed cache for distributed caching.

use super::PriceCache;
use crate::types::Product;
use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// PostgreSQL-backed cache for distributed caching.
///
/// # Example
///
/// ```ignore
/// use infracost_rs::Client;
/// use infracost_rs::cache::PostgresCache;
///
/// // From connection string
/// let cache = PostgresCache::new("postgres://user:pass@localhost/db").await?;
///
/// // From existing pool
/// let cache = PostgresCache::from_pool(pool).await?;
///
/// let client = Client::builder()
///     .with_cache(cache)
///     .build()?;
/// ```
pub struct PostgresCache {
    pool: PgPool,
}

/// Builder for configuring PostgresCache.
pub struct PostgresCacheBuilder {
    url: String,
    max_connections: u32,
}

impl PostgresCacheBuilder {
    /// Create a new builder with the connection URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_connections: 10,
        }
    }

    /// Set the connection URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Set the maximum number of connections in the pool.
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Build the PostgresCache.
    pub async fn build(self) -> Result<PostgresCache, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(self.max_connections)
            .connect(&self.url)
            .await?;

        PostgresCache::from_pool(Arc::new(pool)).await
    }
}

impl PostgresCache {
    /// Create a new PostgreSQL cache from a connection string.
    ///
    /// Runs migrations on the database.
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        PostgresCacheBuilder::new(url).build().await
    }

    /// Create a PostgreSQL cache from an existing pool.
    ///
    /// Runs migrations on the provided pool.
    pub async fn from_pool(pool: Arc<PgPool>) -> Result<Self, sqlx::Error> {
        // Run migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS infracost_cache (
                key TEXT PRIMARY KEY NOT NULL,
                data TEXT NOT NULL,
                expires_at BIGINT NOT NULL
            )
            "#,
        )
        .execute(pool.as_ref())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_infracost_cache_expires_at
            ON infracost_cache(expires_at)
            "#,
        )
        .execute(pool.as_ref())
        .await?;

        Ok(Self {
            pool: Arc::try_unwrap(pool).unwrap_or_else(|arc| (*arc).clone()),
        })
    }

    /// Create a builder for more configuration options.
    pub fn builder(url: impl Into<String>) -> PostgresCacheBuilder {
        PostgresCacheBuilder::new(url)
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
            let result = sqlx::query("DELETE FROM infracost_cache WHERE expires_at < $1")
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
impl PriceCache for PostgresCache {
    async fn get(&self, key: &str) -> Option<Vec<Product>> {
        let now = Self::current_timestamp();

        let result =
            sqlx::query("SELECT data FROM infracost_cache WHERE key = $1 AND expires_at > $2")
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
            VALUES ($1, $2, $3)
            ON CONFLICT(key) DO UPDATE SET data = EXCLUDED.data, expires_at = EXCLUDED.expires_at
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

// Note: PostgreSQL tests require a running PostgreSQL instance and are in integration tests
