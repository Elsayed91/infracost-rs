# SQLx SQLite & PostgreSQL Cache Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add SQLite and PostgreSQL cache backends using SQLx with compile-time checked queries and automatic migrations.

**Architecture:** Two new cache modules (`sqlite.rs`, `postgres.rs`) implementing the `PriceCache` trait. Both use SQLx for database access with compile-time query verification. Connection pooling via SQLx pools. TTL handled via `expires_at` column with lazy non-blocking cleanup.

**Tech Stack:** SQLx 0.8 with sqlite/postgres features, tokio runtime, serde_json for serialization.

---

## Task 1: Add SQLx Dependencies and Feature Flags

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add SQLx dependency and feature flags**

Add to `Cargo.toml` after line 16 (after `cache-redis`):

```toml
cache-sqlite = ["sqlx/sqlite"]
cache-postgres = ["sqlx/postgres"]
```

Add to dependencies section after line 45 (after redis):

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "json"], optional = true }
```

**Step 2: Verify it compiles**

Run: `cargo check --features cache-sqlite,cache-postgres`
Expected: Compilation succeeds (no cache modules yet, just dependencies)

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add SQLx dependency and cache feature flags"
```

---

## Task 2: Create SQLite Migration

**Files:**
- Create: `migrations/sqlite/20240101000000_create_cache_table.sql`

**Step 1: Create migrations directory**

Run: `mkdir -p migrations/sqlite`

**Step 2: Write SQLite migration file**

Create `migrations/sqlite/20240101000000_create_cache_table.sql`:

```sql
-- SQLite cache table for infracost pricing data
CREATE TABLE IF NOT EXISTS infracost_cache (
    key TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_infracost_cache_expires_at ON infracost_cache(expires_at);
```

**Step 3: Commit**

```bash
git add migrations/
git commit -m "feat: add SQLite cache migration"
```

---

## Task 3: Create PostgreSQL Migration

**Files:**
- Create: `migrations/postgres/20240101000000_create_cache_table.sql`

**Step 1: Create migrations directory**

Run: `mkdir -p migrations/postgres`

**Step 2: Write PostgreSQL migration file**

Create `migrations/postgres/20240101000000_create_cache_table.sql`:

```sql
-- PostgreSQL cache table for infracost pricing data
CREATE TABLE IF NOT EXISTS infracost_cache (
    key TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL,
    expires_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_infracost_cache_expires_at ON infracost_cache(expires_at);
```

**Step 3: Commit**

```bash
git add migrations/
git commit -m "feat: add PostgreSQL cache migration"
```

---

## Task 4: Implement SqliteCache

**Files:**
- Create: `src/cache/sqlite.rs`
- Modify: `src/cache/mod.rs`

**Step 1: Create sqlite.rs with full implementation**

Create `src/cache/sqlite.rs`:

```rust
//! SQLite-backed cache for local persistent caching.

use super::PriceCache;
use crate::types::Product;
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;
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
        let options = SqliteConnectOptions::from_str(&self.path)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(self.max_connections)
            .connect_with(options)
            .await?;

        SqliteCache::from_pool(Arc::new(pool)).await
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
    pub async fn from_pool(pool: Arc<SqlitePool>) -> Result<Self, sqlx::Error> {
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

        let result = sqlx::query("SELECT data FROM infracost_cache WHERE key = ? AND expires_at > ?")
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
        cache
            .set("key1", &products, Duration::from_secs(60))
            .await;
        cache
            .set("key2", &products, Duration::from_secs(60))
            .await;

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
```

**Step 2: Update mod.rs to include sqlite module**

Add after line 16 in `src/cache/mod.rs`:

```rust
#[cfg(feature = "cache-sqlite")]
mod sqlite;
```

Add after line 22:

```rust
#[cfg(feature = "cache-sqlite")]
pub use sqlite::{SqliteCache, SqliteCacheBuilder};
```

**Step 3: Run tests**

Run: `cargo test --features cache-sqlite -- sqlite`
Expected: All sqlite tests pass

**Step 4: Commit**

```bash
git add src/cache/sqlite.rs src/cache/mod.rs
git commit -m "feat: implement SqliteCache with PriceCache trait"
```

---

## Task 5: Implement PostgresCache

**Files:**
- Create: `src/cache/postgres.rs`
- Modify: `src/cache/mod.rs`

**Step 1: Create postgres.rs with full implementation**

Create `src/cache/postgres.rs`:

```rust
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
```

**Step 2: Update mod.rs to include postgres module**

Add after the sqlite module line in `src/cache/mod.rs`:

```rust
#[cfg(feature = "cache-postgres")]
mod postgres;
```

Add after the sqlite pub use:

```rust
#[cfg(feature = "cache-postgres")]
pub use postgres::{PostgresCache, PostgresCacheBuilder};
```

**Step 3: Verify compilation**

Run: `cargo check --features cache-postgres`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add src/cache/postgres.rs src/cache/mod.rs
git commit -m "feat: implement PostgresCache with PriceCache trait"
```

---

## Task 6: Update lib.rs Re-exports

**Files:**
- Modify: `src/lib.rs`

**Step 1: Add re-exports for new cache types**

Add after line 89 in `src/lib.rs` (after RedisCache):

```rust
#[cfg(feature = "cache-sqlite")]
pub use cache::SqliteCache;
#[cfg(feature = "cache-postgres")]
pub use cache::PostgresCache;
```

**Step 2: Verify compilation with all features**

Run: `cargo check --all-features`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat: re-export SqliteCache and PostgresCache from lib"
```

---

## Task 7: Add Integration Tests for SQLite

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Add SQLite cache integration tests**

Add at the end of `tests/integration.rs`:

```rust
// ============================================================
// SQLite Cache Integration Tests
// ============================================================

#[cfg(feature = "cache-sqlite")]
mod sqlite_cache_tests {
    use infracost_rs::cache::SqliteCache;
    use infracost_rs::{Client, PriceCache};
    use std::time::Duration;

    fn get_client_with_cache() -> Option<Client> {
        let _ = dotenvy::dotenv();

        // Use temp file for integration tests
        let cache = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(SqliteCache::new("/tmp/infracost_test_cache.db"))
            .ok()?;

        Client::builder()
            .api_key(std::env::var("INFRACOST_API_KEY").ok()?)
            .with_cache(cache)
            .cache_ttl(Duration::from_secs(300))
            .build()
            .ok()
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_basic_operations() {
        let cache = SqliteCache::new("/tmp/infracost_sqlite_test.db")
            .await
            .expect("Should create SQLite cache");

        // Test set and get
        let products = vec![infracost_rs::Product {
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
            .set("sqlite-test-key", &products, Duration::from_secs(60))
            .await;

        let cached = cache.get("sqlite-test-key").await;
        assert!(cached.is_some(), "Should retrieve cached products");
        assert_eq!(cached.unwrap().len(), 1);

        // Clean up
        cache.clear().await;
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_miss() {
        let cache = SqliteCache::new("/tmp/infracost_sqlite_test.db")
            .await
            .expect("Should create SQLite cache");

        let cached = cache.get("nonexistent-key-sqlite-12345").await;
        assert!(cached.is_none(), "Should return None for cache miss");
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_with_client() {
        let client = get_client_with_cache().expect("Requires API key");

        // First call - should hit API and cache the result
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("First query should succeed");

        assert!(result1.is_from_api(), "First call should be from API");
        assert!(result1.price > 0.0, "Should have a price");

        // Second call with same parameters - should be a cache hit
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Second query should succeed");

        // Results should match
        assert_eq!(result1.price, result2.price, "Prices should match");
        assert_eq!(result1.unit, result2.unit, "Units should match");
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_different_queries() {
        let client = get_client_with_cache().expect("Requires API key");

        // Query 1: GCP SSD disk
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 1 should succeed");

        // Query 2: GCP Standard disk (different cache key)
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdStandard)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 2 should succeed");

        // Prices should be different
        assert_ne!(
            result1.price, result2.price,
            "Different disk types should have different prices"
        );
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_multi_provider() {
        let client = get_client_with_cache().expect("Requires API key");

        // Test caching works across different providers
        let gcp_result = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("GCP query should succeed");

        let aws_result = client
            .aws()
            .ebs(infracost_rs::providers::aws::EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .await
            .expect("AWS query should succeed");

        assert!(gcp_result.price > 0.0, "GCP should have price");
        assert!(aws_result.price > 0.0, "AWS should have price");
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_clear() {
        let cache = SqliteCache::new("/tmp/infracost_sqlite_clear_test.db")
            .await
            .expect("Should create SQLite cache");

        // Set some test data
        let products = vec![];
        cache
            .set("infracost:v1:sqlite-test1", &products, Duration::from_secs(60))
            .await;
        cache
            .set("infracost:v1:sqlite-test2", &products, Duration::from_secs(60))
            .await;

        // Verify data was set
        assert!(cache.get("infracost:v1:sqlite-test1").await.is_some());

        // Clear all entries
        cache.clear().await;

        // Give it a moment
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify data was cleared
        assert!(
            cache.get("infracost:v1:sqlite-test1").await.is_none(),
            "Cache should be cleared"
        );
        assert!(
            cache.get("infracost:v1:sqlite-test2").await.is_none(),
            "Cache should be cleared"
        );
    }
}
```

**Step 2: Verify tests compile**

Run: `cargo test --test integration --features cache-sqlite --no-run`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add SQLite cache integration tests"
```

---

## Task 8: Add Integration Tests for PostgreSQL

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Add PostgreSQL cache integration tests**

Add at the end of `tests/integration.rs`:

```rust
// ============================================================
// PostgreSQL Cache Integration Tests
// ============================================================

#[cfg(feature = "cache-postgres")]
mod postgres_cache_tests {
    use infracost_rs::cache::PostgresCache;
    use infracost_rs::{Client, PriceCache};
    use std::time::Duration;

    fn get_postgres_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://infracost:infracost@localhost/infracost_cache".to_string())
    }

    fn get_client_with_cache() -> Option<Client> {
        let _ = dotenvy::dotenv();

        let url = get_postgres_url();
        let cache = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(PostgresCache::new(&url))
            .ok()?;

        Client::builder()
            .api_key(std::env::var("INFRACOST_API_KEY").ok()?)
            .with_cache(cache)
            .cache_ttl(Duration::from_secs(300))
            .build()
            .ok()
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_basic_operations() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        // Test set and get
        let products = vec![infracost_rs::Product {
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
            .set("postgres-test-key", &products, Duration::from_secs(60))
            .await;

        let cached = cache.get("postgres-test-key").await;
        assert!(cached.is_some(), "Should retrieve cached products");
        assert_eq!(cached.unwrap().len(), 1);

        // Clean up
        cache.clear().await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_miss() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        let cached = cache.get("nonexistent-key-postgres-12345").await;
        assert!(cached.is_none(), "Should return None for cache miss");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_with_client() {
        let client = get_client_with_cache().expect("Requires API key and PostgreSQL");

        // First call - should hit API and cache the result
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("First query should succeed");

        assert!(result1.is_from_api(), "First call should be from API");
        assert!(result1.price > 0.0, "Should have a price");

        // Second call with same parameters - should be a cache hit
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Second query should succeed");

        // Results should match
        assert_eq!(result1.price, result2.price, "Prices should match");
        assert_eq!(result1.unit, result2.unit, "Units should match");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_different_queries() {
        let client = get_client_with_cache().expect("Requires API key and PostgreSQL");

        // Query 1: GCP SSD disk
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 1 should succeed");

        // Query 2: GCP Standard disk (different cache key)
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdStandard)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 2 should succeed");

        // Prices should be different
        assert_ne!(
            result1.price, result2.price,
            "Different disk types should have different prices"
        );
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_multi_provider() {
        let client = get_client_with_cache().expect("Requires API key and PostgreSQL");

        // Test caching works across different providers
        let gcp_result = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("GCP query should succeed");

        let aws_result = client
            .aws()
            .ebs(infracost_rs::providers::aws::EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .await
            .expect("AWS query should succeed");

        assert!(gcp_result.price > 0.0, "GCP should have price");
        assert!(aws_result.price > 0.0, "AWS should have price");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_clear() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        // Set some test data
        let products = vec![];
        cache
            .set("infracost:v1:pg-test1", &products, Duration::from_secs(60))
            .await;
        cache
            .set("infracost:v1:pg-test2", &products, Duration::from_secs(60))
            .await;

        // Verify data was set
        assert!(cache.get("infracost:v1:pg-test1").await.is_some());

        // Clear all entries
        cache.clear().await;

        // Give it a moment
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify data was cleared
        assert!(
            cache.get("infracost:v1:pg-test1").await.is_none(),
            "Cache should be cleared"
        );
        assert!(
            cache.get("infracost:v1:pg-test2").await.is_none(),
            "Cache should be cleared"
        );
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_expiration() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        let products = vec![infracost_rs::Product {
            product_hash: "expire-hash".to_string(),
            vendor_name: "test-vendor".to_string(),
            service: "test-service".to_string(),
            product_family: None,
            region: None,
            sku: "expire-sku".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        // Set with 1 second TTL
        cache
            .set("pg-expire-key", &products, Duration::from_secs(1))
            .await;

        // Should exist immediately
        assert!(cache.get("pg-expire-key").await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get("pg-expire-key").await.is_none());
    }
}
```

**Step 2: Verify tests compile**

Run: `cargo test --test integration --features cache-postgres --no-run`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add PostgreSQL cache integration tests"
```

---

## Task 9: Update Docker Compose Files

**Files:**
- Modify: `/Users/islamelsayed/Desktop/the-arbiter-project/infracost-rs/docker-compose.yml`
- Modify: `/Users/islamelsayed/Desktop/the-arbiter-project/docker-compose.yml`

**Step 1: Update infracost-rs docker-compose.yml**

Replace content of `docker-compose.yml`:

```yaml
# Docker Compose for local development and testing
#
# Start all services:
#   docker-compose up -d
#
# Start specific service:
#   docker-compose up -d redis
#   docker-compose up -d postgres
#
# Run integration tests:
#   # Redis tests
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-redis -- --ignored
#
#   # SQLite tests (no service needed)
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-sqlite -- --ignored
#
#   # PostgreSQL tests
#   DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache \
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-postgres -- --ignored
#
#   # All cache tests
#   DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache \
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-redis,cache-sqlite,cache-postgres -- --ignored

services:
  redis:
    image: redis:8-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  postgres:
    image: postgres:16-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: infracost
      POSTGRES_PASSWORD: infracost
      POSTGRES_DB: infracost_cache
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U infracost"]
      interval: 5s
      timeout: 3s
      retries: 5

volumes:
  redis-data:
  postgres-data:
```

**Step 2: Update parent docker-compose.yml**

Replace content of `/Users/islamelsayed/Desktop/the-arbiter-project/docker-compose.yml`:

```yaml
# Docker Compose for local development and testing
#
# Start all services:
#   docker-compose up -d
#
# Start specific service:
#   docker-compose up -d redis
#   docker-compose up -d postgres
#
# Run integration tests:
#   # Redis tests
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-redis -- --ignored
#
#   # SQLite tests (no service needed)
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-sqlite -- --ignored
#
#   # PostgreSQL tests
#   DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache \
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-postgres -- --ignored
#
#   # All cache tests
#   DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache \
#   INFRACOST_API_KEY=your-key cargo test --test integration --features cache-redis,cache-sqlite,cache-postgres -- --ignored

services:
  redis:
    image: redis:8-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  postgres:
    image: postgres:16-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: infracost
      POSTGRES_PASSWORD: infracost
      POSTGRES_DB: infracost_cache
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U infracost"]
      interval: 5s
      timeout: 3s
      retries: 5

volumes:
  redis-data:
  postgres-data:
```

**Step 3: Commit**

```bash
git add docker-compose.yml
git -C /Users/islamelsayed/Desktop/the-arbiter-project add docker-compose.yml
git commit -m "chore: add PostgreSQL service to docker-compose files"
```

---

## Task 10: Update CI Workflow

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Update CI to include PostgreSQL service and all cache tests**

Replace content of `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-features
      - run: cargo clippy --all-features -- -D warnings
      - run: cargo fmt --check

  integration:
    runs-on: ubuntu-latest
    services:
      redis:
        image: redis:8-alpine
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_USER: infracost
          POSTGRES_PASSWORD: infracost
          POSTGRES_DB: infracost_cache
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run integration tests
        env:
          INFRACOST_API_KEY: ${{ secrets.INFRACOST_API_KEY }}
          DATABASE_URL: postgres://infracost:infracost@localhost/infracost_cache
        run: cargo test --test integration --features cache-redis,cache-sqlite,cache-postgres -- --ignored

  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo doc --no-deps --all-features
```

**Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add PostgreSQL service and SQLite/Postgres cache tests"
```

---

## Task 11: Run Unit Tests

**Step 1: Run all unit tests with all features**

Run: `cargo test --all-features`
Expected: All tests pass

**Step 2: Run SQLite-specific tests**

Run: `cargo test --features cache-sqlite -- sqlite`
Expected: All SQLite tests pass

---

## Task 12: Run Integration Tests Locally

**Step 1: Start services**

Run: `docker-compose up -d`
Expected: Redis and PostgreSQL services start successfully

**Step 2: Wait for services to be healthy**

Run: `docker-compose ps`
Expected: Both services show as healthy

**Step 3: Run Redis integration tests**

Run: `INFRACOST_API_KEY="ico-OKwGuWyjLFxXitqTJLHlcTjLmlh88vm7" cargo test --test integration --features cache-redis -- --ignored`
Expected: All Redis tests pass

**Step 4: Run SQLite integration tests**

Run: `INFRACOST_API_KEY="ico-OKwGuWyjLFxXitqTJLHlcTjLmlh88vm7" cargo test --test integration --features cache-sqlite -- --ignored`
Expected: All SQLite tests pass

**Step 5: Run PostgreSQL integration tests**

Run: `DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache INFRACOST_API_KEY="ico-OKwGuWyjLFxXitqTJLHlcTjLmlh88vm7" cargo test --test integration --features cache-postgres -- --ignored`
Expected: All PostgreSQL tests pass

**Step 6: Run all integration tests together**

Run: `DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache INFRACOST_API_KEY="ico-OKwGuWyjLFxXitqTJLHlcTjLmlh88vm7" cargo test --test integration --features cache-redis,cache-sqlite,cache-postgres -- --ignored`
Expected: All cache integration tests pass

---

## Task 13: Final Verification and Cleanup

**Step 1: Run full test suite**

Run: `cargo test --all-features`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --all-features -- -D warnings`
Expected: No warnings

**Step 3: Check formatting**

Run: `cargo fmt --check`
Expected: No formatting issues

**Step 4: Build docs**

Run: `cargo doc --no-deps --all-features`
Expected: Documentation builds successfully

**Step 5: Stop docker services**

Run: `docker-compose down`
Expected: Services stop

**Step 6: Final commit (if any changes)**

```bash
git add -A
git commit -m "chore: final cleanup and verification"
```

---

## Summary

This implementation adds two new cache backends:

1. **SqliteCache** - File-based SQLite caching for local/edge deployments
   - Accepts file path or existing `Arc<SqlitePool>`
   - In-memory option via `:memory:`
   - Builder pattern for configuration

2. **PostgresCache** - PostgreSQL caching for distributed deployments
   - Accepts connection string or existing `Arc<PgPool>`
   - Builder pattern for configuration

Both implementations:
- Implement the `PriceCache` trait
- Auto-run migrations on construction
- Use JSON serialization (consistent with Redis)
- Support TTL via `expires_at` column
- Perform lazy non-blocking cleanup
- Handle errors gracefully (log + return None)
