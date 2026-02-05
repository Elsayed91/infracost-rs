# SQLx SQLite & PostgreSQL Cache Implementation Design

**Date:** 2026-02-05
**Status:** Approved

## Overview

Add two new cache backends using SQLx: SQLite (file-based) and PostgreSQL. Both implementations follow the existing `PriceCache` trait pattern and integrate with the existing caching architecture.

## Key Decisions

| Decision | Choice |
|----------|--------|
| SQLite storage | File-based (not in-memory) |
| SQLite input | Accept `Arc<SqlitePool>` or file path |
| PostgreSQL input | Accept `Arc<PgPool>` or connection string |
| TTL handling | Expiration timestamp column |
| Cleanup strategy | Lazy, non-blocking (spawned task) |
| Migrations | SQLx migrations, compile-time checked queries |
| Feature flags | Separate: `cache-sqlite`, `cache-postgres` |
| Data serialization | JSON (consistent with Redis) |

## Database Schema

Identical for both SQLite and PostgreSQL:

```sql
CREATE TABLE infracost_cache (
    key TEXT PRIMARY KEY,
    data TEXT NOT NULL,        -- JSON serialized Vec<Product>
    expires_at BIGINT NOT NULL -- Unix timestamp (seconds)
);

CREATE INDEX idx_infracost_cache_expires_at ON infracost_cache(expires_at);
```

### Migration Structure

```
migrations/
├── sqlite/
│   └── 20240101000000_create_cache_table.sql
└── postgres/
    └── 20240101000000_create_cache_table.sql
```

Compile-time checked queries generate `.sqlx/` folder for offline builds.

## Module Structure

```
src/cache/
├── mod.rs          # PriceCache trait + re-exports (existing)
├── memory.rs       # Moka cache (existing)
├── redis.rs        # Redis cache (existing)
├── sqlite.rs       # New SQLite cache
└── postgres.rs     # New PostgreSQL cache
```

## API Design

### SQLite Cache

```rust
// From existing pool
let cache = SqliteCache::from_pool(existing_pool).await?;

// From file path (creates pool internally)
let cache = SqliteCache::new("./cache.db").await?;

// Builder pattern for configuration
let cache = SqliteCache::builder()
    .path("./cache.db")
    .max_connections(5)
    .build()
    .await?;
```

### PostgreSQL Cache

```rust
// From existing pool
let cache = PostgresCache::from_pool(existing_pool).await?;

// From connection string
let cache = PostgresCache::new("postgres://localhost/mydb").await?;

// Builder pattern
let cache = PostgresCache::builder()
    .url("postgres://localhost/mydb")
    .max_connections(10)
    .build()
    .await?;
```

Both run migrations automatically on construction.

## PriceCache Implementation

### Core Operations

```rust
#[async_trait]
impl PriceCache for SqliteCache {
    async fn get(&self, key: &str) -> Option<Vec<Product>> {
        // 1. Query: SELECT data FROM infracost_cache
        //    WHERE key = ? AND expires_at > current_unix_timestamp
        // 2. Deserialize JSON to Vec<Product>
        // 3. Spawn non-blocking cleanup task
        // 4. Return None on any error (graceful degradation)
    }

    async fn set(&self, key: &str, products: &[Product], ttl: Duration) {
        // 1. Serialize products to JSON
        // 2. Calculate expires_at = now + ttl
        // 3. UPSERT: INSERT OR REPLACE (SQLite) / ON CONFLICT (Postgres)
        // 4. Log warning on error, don't propagate
    }

    async fn clear(&self) {
        // DELETE FROM infracost_cache
    }
}
```

### Non-blocking Cleanup

```rust
fn spawn_cleanup(&self) {
    let pool = self.pool.clone();
    tokio::spawn(async move {
        // DELETE FROM infracost_cache WHERE expires_at < now
        // Runs in background, errors logged but ignored
    });
}
```

## Dependencies

### Cargo.toml

```toml
[features]
cache-sqlite = ["sqlx/sqlite"]
cache-postgres = ["sqlx/postgres"]

[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "json"], optional = true }
```

### Module Gating

```rust
// src/cache/mod.rs
#[cfg(feature = "cache-sqlite")]
mod sqlite;
#[cfg(feature = "cache-postgres")]
mod postgres;

#[cfg(feature = "cache-sqlite")]
pub use sqlite::{SqliteCache, SqliteCacheBuilder};
#[cfg(feature = "cache-postgres")]
pub use postgres::{PostgresCache, PostgresCacheBuilder};
```

## Testing Strategy

### Unit Tests (in-module)

```rust
// src/cache/sqlite.rs
#[cfg(test)]
mod tests {
    // Uses in-memory SQLite for fast isolated tests
    // Tests: get/set, expiration, clear, concurrent access
}

// src/cache/postgres.rs
#[cfg(test)]
mod tests {
    // Uses testcontainers or requires running Postgres
    // Same test cases as SQLite
}
```

### Integration Tests

```rust
// tests/integration.rs

#[cfg(feature = "cache-sqlite")]
mod sqlite_cache_tests {
    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_with_real_api() {
        // Full end-to-end: Client + SQLite cache + real Infracost API
    }
}

#[cfg(feature = "cache-postgres")]
mod postgres_cache_tests {
    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_with_real_api() {
        // Full end-to-end: Client + Postgres cache + real Infracost API
    }
}
```

### Test Coverage

1. Basic get/set operations
2. Cache miss returns None
3. Expiration works correctly
4. Clear removes all entries
5. Graceful handling of connection errors
6. Integration with Client (cache hit/miss flow)

## Docker Compose

### infracost-rs/docker-compose.yml

```yaml
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

### Local Testing Commands

```bash
# Start services
docker-compose up -d redis postgres

# Run SQLite tests (no service needed)
INFRACOST_API_KEY=xxx cargo test --test integration --features cache-sqlite -- --ignored

# Run PostgreSQL tests
DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache \
INFRACOST_API_KEY=xxx cargo test --test integration --features cache-postgres -- --ignored

# Run all cache tests
INFRACOST_API_KEY=xxx \
DATABASE_URL=postgres://infracost:infracost@localhost/infracost_cache \
cargo test --test integration --features cache-redis,cache-sqlite,cache-postgres -- --ignored
```

## CI Configuration

### .github/workflows/ci.yml

```yaml
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
```

## Implementation Checklist

- [ ] Add SQLx dependency with sqlite/postgres features
- [ ] Create migrations for SQLite
- [ ] Create migrations for PostgreSQL
- [ ] Implement `SqliteCache` with builder
- [ ] Implement `PostgresCache` with builder
- [ ] Add feature flags and module gating
- [ ] Write unit tests for SQLite
- [ ] Write unit tests for PostgreSQL
- [ ] Write integration tests
- [ ] Update docker-compose.yml (infracost-rs)
- [ ] Update docker-compose.yml (parent)
- [ ] Update CI workflow
- [ ] Generate .sqlx files for offline compilation
- [ ] Update lib.rs re-exports
- [ ] Run full integration tests with API key
