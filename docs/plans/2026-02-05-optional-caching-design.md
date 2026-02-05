# Optional Caching for infracost-rs

**Date:** 2026-02-05
**Status:** Approved

## Overview

Add optional caching to the infracost-rs library with pluggable backends (in-memory and Redis). Caching operates at the `ProductFilter` level, meaning each distinct API query gets its own cache entry.

## Goals

1. Reduce latency for repeated pricing queries (Infracost API takes 1-3 seconds)
2. Enable shared caching across users (global cache for all users with API keys)
3. Zero overhead when caching is not configured
4. Feature-gated dependencies (moka, redis only compiled when needed)

## Key Behaviors

- **Users without API keys never touch the cache** — they either get defaults or errors
- **Users with API keys share the cache** — API key is not part of cache key
- **Multi-dimensional resources work naturally** — each price component (storage, IOPS, throughput, etc.) is a separate query with a separate cache entry

## Architecture

### Module Structure

```
src/cache/
├── mod.rs      # PriceCache trait + cache_key() impl (always compiled)
├── memory.rs   # MemoryCache using moka (feature: cache-memory)
└── redis.rs    # RedisCache (feature: cache-redis)
```

### The Trait

```rust
#[async_trait]
pub trait PriceCache: Send + Sync {
    async fn get(&self, key: &str) -> Option<Vec<Product>>;
    async fn set(&self, key: &str, products: &[Product], ttl: Duration);
    async fn clear(&self) {}  // optional, default no-op
}
```

The trait is always compiled. Only implementations are feature-gated.

### Cache Key Generation

Cache keys are generated from `ProductFilter` by hashing a canonical string of all fields (attributes sorted for determinism).

```rust
impl ProductFilter {
    pub fn cache_key(&self) -> String {
        // Sort attributes for determinism
        let mut attrs: Vec<String> = self.attribute_filters.iter()
            .map(|a| format!("{}={}~{}", a.key, a.value, a.value_regex))
            .collect();
        attrs.sort();

        let canonical = format!(
            "{}:{}:{}:{}:{}:{}",
            self.vendor_name.unwrap_or(""),
            self.service.unwrap_or(""),
            self.region.unwrap_or(""),
            self.product_family.unwrap_or(""),
            self.sku.unwrap_or(""),
            attrs.join(";")
        );

        format!("infracost:v1:{:x}", hash(canonical))
    }
}
```

### Client Integration

**ClientInner changes:**
```rust
struct ClientInner {
    http: reqwest::Client,
    api_key: Option<String>,
    endpoint: String,
    cache: Option<Arc<dyn PriceCache>>,  // new
    cache_ttl: Duration,                  // new
}
```

**ClientBuilder additions:**
```rust
impl ClientBuilder {
    pub fn with_cache<C: PriceCache + 'static>(mut self, cache: C) -> Self;
    pub fn cache_ttl(mut self, ttl: Duration) -> Self;
}
```

**execute_query integration:**
```rust
pub(crate) async fn execute_query(&self, filter: ProductFilter, api_key_override: Option<&str>) -> Result<Vec<Product>> {
    // 1. API key check (existing) - keyless users never reach cache
    let api_key = api_key_override
        .or(self.inner.api_key.as_deref())
        .ok_or(Error::MissingApiKey)?;

    // 2. Check cache (new)
    let cache_key = filter.cache_key();
    if let Some(cache) = &self.inner.cache {
        if let Some(cached) = cache.get(&cache_key).await {
            return Ok(cached);
        }
    }

    // 3. Execute GraphQL query (existing)
    let products = /* ... existing HTTP logic ... */;

    // 4. Cache result (new)
    if let Some(cache) = &self.inner.cache {
        cache.set(&cache_key, &products, self.inner.cache_ttl).await;
    }

    Ok(products)
}
```

### Cache Implementations

**MemoryCache (feature: cache-memory):**
- Uses `moka` crate for high-performance concurrent caching
- Default capacity: 10,000 entries
- Configurable TTL (default: 24 hours)

**RedisCache (feature: cache-redis):**
- Uses `redis` crate with async support
- Serializes `Vec<Product>` as JSON
- Uses `SET EX` for TTL

### Cargo.toml Changes

```toml
[features]
cache-memory = ["moka"]
cache-redis = ["redis"]

[dependencies]
moka = { version = "0.12", features = ["future"], optional = true }
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"], optional = true }
```

## Usage Examples

**In-memory cache:**
```rust
let client = Client::builder()
    .with_cache(MemoryCache::new())
    .build()?;
```

**Redis cache (shared across instances):**
```rust
let client = Client::builder()
    .with_cache(RedisCache::new("redis://localhost:6379")?)
    .cache_ttl(Duration::from_secs(12 * 3600))
    .build()?;
```

**Per-request with user's API key:**
```rust
let price = client.aws().ebs("gp3")
    .region("us-east-1")
    .api_key(&user_api_key)
    .fetch()
    .await?;
```

**Strict mode (error instead of defaults):**
```rust
let client = Client::builder()
    .error_on_fallback(true)
    .build()?;

// Now keyless requests or API failures will error instead of returning defaults
// Useful for testing/debugging or when you must have real prices
```

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| No cache configured | Zero overhead, `if let Some` doesn't match |
| Cache get fails | Returns `None`, proceeds to API |
| Cache set fails | Silently ignored, API result still returned |
| Empty API response | Cached (prevents repeated misses) |
| No API key | Error before cache check |

## Multi-Dimensional Resources

Resources with multiple price components (EBS, NAT Gateway, ALB, etc.) naturally work because each component queries with a different `ProductFilter`:

| Resource | Components | Cache Entries |
|----------|------------|---------------|
| AWS EBS gp3 | storage, IOPS, throughput | 3 |
| AWS NAT Gateway | hourly, data processing | 2 |
| AWS ALB | hourly, LCU | 2 |
| GCP Disk | capacity | 1 |
| Azure Managed Disk | fixed price | 1 |

## Implementation Plan

0. **Refactor:** Remove `require_api` from all providers, add `error_on_fallback` to Client
1. Create `src/cache/mod.rs` with `PriceCache` trait and `cache_key()` impl
2. Create `src/cache/memory.rs` with `MemoryCache`
3. Create `src/cache/redis.rs` with `RedisCache`
4. Update `src/client.rs` to integrate caching
5. Update `src/lib.rs` with exports
6. Update `Cargo.toml` with features and dependencies
7. Add tests for cache key generation and integration
