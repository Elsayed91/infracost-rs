# Optional Caching Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add pluggable cache backends (in-memory, Redis) to infracost-rs for caching pricing API responses. Also refactor the `require_api` pattern to be cleaner.

**Architecture:** Cache at `ProductFilter` level via `PriceCache` trait. Trait always compiled, implementations feature-gated. Cache check happens after API key validation in `execute_query()`, ensuring keyless users never touch cache.

**Tech Stack:** Rust, async_trait, moka (in-memory cache), redis crate

---

## Task 0: Remove require_api and Add error_on_fallback to Client

**Background:** The `require_api` method is duplicated across all 15+ providers. It's redundant because `PriceResult.source` already tells you if you got a default. We'll remove it from providers and add a single client-level `error_on_fallback` option for those who need strict mode.

**Files:**
- Modify: `src/client.rs`
- Modify: `src/providers/aws/ebs.rs`
- Modify: `src/providers/aws/alb.rs`
- Modify: `src/providers/aws/nat_gateway.rs`
- Modify: `src/providers/aws/elastic_ip.rs`
- Modify: `src/providers/aws/snapshot.rs`
- Modify: `src/providers/gcp/disk.rs`
- Modify: `src/providers/gcp/nat_gateway.rs`
- Modify: `src/providers/gcp/static_ip.rs`
- Modify: `src/providers/gcp/snapshot.rs`
- Modify: `src/providers/gcp/forwarding_rule.rs`
- Modify: `src/providers/azure/managed_disk.rs`
- Modify: `src/providers/azure/public_ip.rs`
- Modify: `src/providers/azure/snapshot.rs`

**Step 1: Add error_on_fallback to ClientInner and ClientBuilder**

In `src/client.rs`, update `ClientInner`:

```rust
struct ClientInner {
    http: reqwest::Client,
    api_key: Option<String>,
    endpoint: String,
    error_on_fallback: bool,
}
```

Update `ClientBuilder`:

```rust
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    api_key: Option<String>,
    endpoint: Option<String>,
    timeout: Duration,
    error_on_fallback: bool,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            timeout: DEFAULT_TIMEOUT,
            error_on_fallback: false,
        }
    }
}
```

Add method to `ClientBuilder`:

```rust
/// Error instead of returning default prices when API is unavailable.
///
/// By default, the library gracefully falls back to built-in default prices
/// when the API fails or no API key is provided. Enable this for strict mode
/// where you want errors instead of potentially stale defaults.
///
/// Note: You can also check `PriceResult.source` to see if a price came from
/// the API or from defaults, which gives you more flexibility.
pub fn error_on_fallback(mut self, enabled: bool) -> Self {
    self.error_on_fallback = enabled;
    self
}
```

Update `Client::new()`, `Client::anonymous()`, and `ClientBuilder::build()` to include:

```rust
error_on_fallback: false,  // for new() and anonymous()
error_on_fallback: self.error_on_fallback,  // for build()
```

Add accessor method to `Client`:

```rust
/// Check if this client will error on fallback to defaults.
pub fn error_on_fallback(&self) -> bool {
    self.inner.error_on_fallback
}
```

**Step 2: Remove require_api from AWS EBS provider**

In `src/providers/aws/ebs.rs`:

1. Remove the `require_api` field from `EbsBuilder`
2. Remove the `require_api()` method
3. Update `fetch()` method - remove all `!self.require_api` checks, replace with `!self.client.error_on_fallback()`
4. Update `fetch_storage_price()`, `fetch_iops_price()`, `fetch_throughput_price()` - same replacement

The pattern changes from:
```rust
// Old
Ok(_) if !self.require_api => Ok(PriceResult::from_default(...)),
Err(_) if !self.require_api => Ok(PriceResult::from_default(...)),
```

To:
```rust
// New
Ok(_) if !self.client.error_on_fallback() => Ok(PriceResult::from_default(...)),
Err(_) if !self.client.error_on_fallback() => Ok(PriceResult::from_default(...)),
```

And the early return changes from:
```rust
// Old
if effective_key.is_none() && !self.require_api {
    return Ok(PriceResult::from_default(default_price, unit));
}
```

To:
```rust
// New
if effective_key.is_none() && !self.client.error_on_fallback() {
    return Ok(PriceResult::from_default(default_price, unit));
}
```

**Step 3: Repeat for all other providers**

Apply the same changes to all provider files:
- Remove `require_api` field
- Remove `require_api()` method
- Replace `!self.require_api` with `!self.client.error_on_fallback()`

**Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/client.rs src/providers/
git commit -m "refactor: replace require_api with client-level error_on_fallback

- Remove duplicated require_api from all 15 providers
- Add single error_on_fallback option to ClientBuilder
- Consumers can still check PriceResult.source for more flexibility"
```

---

## Task 1: Add Cache Key Generation to ProductFilter

**Files:**
- Modify: `src/types.rs`
- Test: `src/types.rs` (inline tests)

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` module at the bottom of `src/types.rs`:

```rust
#[test]
fn test_cache_key_deterministic() {
    let filter1 = ProductFilter {
        vendor_name: Some("aws".to_string()),
        service: Some("AmazonEC2".to_string()),
        region: Some("us-east-1".to_string()),
        product_family: Some("Storage".to_string()),
        sku: None,
        attribute_filters: vec![
            AttributeFilter::exact("volumeApiName", "gp3"),
            AttributeFilter::exact("servicecode", "AmazonEC2"),
        ],
    };

    let filter2 = ProductFilter {
        vendor_name: Some("aws".to_string()),
        service: Some("AmazonEC2".to_string()),
        region: Some("us-east-1".to_string()),
        product_family: Some("Storage".to_string()),
        sku: None,
        attribute_filters: vec![
            AttributeFilter::exact("volumeApiName", "gp3"),
            AttributeFilter::exact("servicecode", "AmazonEC2"),
        ],
    };

    assert_eq!(filter1.cache_key(), filter2.cache_key());
}

#[test]
fn test_cache_key_attribute_order_independent() {
    let filter1 = ProductFilter {
        vendor_name: Some("aws".to_string()),
        attribute_filters: vec![
            AttributeFilter::exact("a", "1"),
            AttributeFilter::exact("b", "2"),
        ],
        ..Default::default()
    };

    let filter2 = ProductFilter {
        vendor_name: Some("aws".to_string()),
        attribute_filters: vec![
            AttributeFilter::exact("b", "2"),
            AttributeFilter::exact("a", "1"),
        ],
        ..Default::default()
    };

    assert_eq!(filter1.cache_key(), filter2.cache_key());
}

#[test]
fn test_cache_key_different_filters() {
    let filter1 = ProductFilter {
        vendor_name: Some("aws".to_string()),
        region: Some("us-east-1".to_string()),
        ..Default::default()
    };

    let filter2 = ProductFilter {
        vendor_name: Some("aws".to_string()),
        region: Some("us-west-2".to_string()),
        ..Default::default()
    };

    assert_ne!(filter1.cache_key(), filter2.cache_key());
}

#[test]
fn test_cache_key_format() {
    let filter = ProductFilter {
        vendor_name: Some("gcp".to_string()),
        ..Default::default()
    };

    let key = filter.cache_key();
    assert!(key.starts_with("infracost:v1:"));
    assert_eq!(key.len(), "infracost:v1:".len() + 16); // 64-bit hex = 16 chars
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test cache_key -- --nocapture`
Expected: FAIL with "no method named `cache_key` found"

**Step 3: Write minimal implementation**

Add this `impl` block to `ProductFilter` in `src/types.rs` (after the existing `impl ProductFilter` block):

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

impl ProductFilter {
    /// Generate a deterministic cache key for this filter.
    ///
    /// The key is a hash of all filter fields, with attributes sorted
    /// for determinism. Two filters with the same values will always
    /// produce the same cache key.
    pub fn cache_key(&self) -> String {
        let mut attrs: Vec<String> = self
            .attribute_filters
            .iter()
            .map(|a| {
                let value = a.value.as_deref().unwrap_or("");
                let regex = a.value_regex.as_deref().unwrap_or("");
                format!("{}={}~{}", a.key, value, regex)
            })
            .collect();
        attrs.sort();

        let canonical = format!(
            "{}:{}:{}:{}:{}:{}",
            self.vendor_name.as_deref().unwrap_or(""),
            self.service.as_deref().unwrap_or(""),
            self.region.as_deref().unwrap_or(""),
            self.product_family.as_deref().unwrap_or(""),
            self.sku.as_deref().unwrap_or(""),
            attrs.join(";")
        );

        let mut hasher = DefaultHasher::new();
        canonical.hash(&mut hasher);
        format!("infracost:v1:{:x}", hasher.finish())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test cache_key -- --nocapture`
Expected: PASS (4 tests)

**Step 5: Commit**

```bash
git add src/types.rs
git commit -m "feat(cache): add cache_key() method to ProductFilter"
```

---

## Task 2: Create PriceCache Trait

**Files:**
- Create: `src/cache/mod.rs`
- Modify: `src/lib.rs`

**Step 1: Create the cache module**

Create `src/cache/mod.rs`:

```rust
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
#[cfg(feature = "cache-redis")]
mod redis;

#[cfg(feature = "cache-memory")]
pub use memory::MemoryCache;
#[cfg(feature = "cache-redis")]
pub use redis::RedisCache;

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
```

**Step 2: Export the module in lib.rs**

Add after `pub mod providers;` in `src/lib.rs`:

```rust
pub mod cache;
```

Add to the pub use section:

```rust
pub use cache::PriceCache;
```

**Step 3: Run to verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cache/mod.rs src/lib.rs
git commit -m "feat(cache): add PriceCache trait"
```

---

## Task 3: Integrate Cache into Client

**Files:**
- Modify: `src/client.rs`

**Step 1: Write the failing test**

Add to `src/client.rs` tests:

```rust
#[test]
fn test_client_builder_with_cache_ttl() {
    use std::time::Duration;

    let client = Client::builder()
        .api_key("test-key")
        .cache_ttl(Duration::from_secs(3600))
        .build()
        .unwrap();

    assert!(client.has_api_key());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_client_builder_with_cache_ttl`
Expected: FAIL with "no method named `cache_ttl`"

**Step 3: Update ClientInner and ClientBuilder**

In `src/client.rs`, update the imports at the top:

```rust
use crate::cache::{PriceCache, DEFAULT_CACHE_TTL};
use std::sync::Arc;
```

Update `ClientInner`:

```rust
struct ClientInner {
    http: reqwest::Client,
    api_key: Option<String>,
    endpoint: String,
    cache: Option<Arc<dyn PriceCache>>,
    cache_ttl: Duration,
}
```

Update `ClientBuilder`:

```rust
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    api_key: Option<String>,
    endpoint: Option<String>,
    timeout: Duration,
    cache_ttl: Duration,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            timeout: DEFAULT_TIMEOUT,
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }
}
```

Add methods to `ClientBuilder`:

```rust
/// Set cache TTL (default: 24 hours).
pub fn cache_ttl(mut self, ttl: Duration) -> Self {
    self.cache_ttl = ttl;
    self
}
```

Update `ClientBuilder::build()`:

```rust
pub fn build(self) -> Result<Client> {
    let http = reqwest::Client::builder()
        .timeout(self.timeout)
        .build()
        .map_err(|e| Error::config(format!("Failed to build HTTP client: {}", e)))?;

    Ok(Client {
        inner: Arc::new(ClientInner {
            http,
            api_key: self.api_key,
            endpoint: self
                .endpoint
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            cache: None,
            cache_ttl: self.cache_ttl,
        }),
    })
}
```

Update `Client::new()`:

```rust
pub fn new(api_key: impl Into<String>) -> Self {
    Self {
        inner: Arc::new(ClientInner {
            http: reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .expect("Failed to build HTTP client"),
            api_key: Some(api_key.into()),
            endpoint: DEFAULT_ENDPOINT.to_string(),
            cache: None,
            cache_ttl: DEFAULT_CACHE_TTL,
        }),
    }
}
```

Update `Client::anonymous()`:

```rust
pub fn anonymous() -> Self {
    Self {
        inner: Arc::new(ClientInner {
            http: reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .expect("Failed to build HTTP client"),
            api_key: None,
            endpoint: DEFAULT_ENDPOINT.to_string(),
            cache: None,
            cache_ttl: DEFAULT_CACHE_TTL,
        }),
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_client_builder_with_cache_ttl`
Expected: PASS

**Step 5: Commit**

```bash
git add src/client.rs
git commit -m "feat(cache): add cache fields to ClientInner and ClientBuilder"
```

---

## Task 4: Add with_cache Method to ClientBuilder

**Files:**
- Modify: `src/client.rs`

**Step 1: Remove Debug derive from ClientBuilder**

The `Arc<dyn PriceCache>` is not Debug, so we need a manual impl. Update `ClientBuilder`:

```rust
pub struct ClientBuilder {
    api_key: Option<String>,
    endpoint: Option<String>,
    timeout: Duration,
    cache: Option<Arc<dyn PriceCache>>,
    cache_ttl: Duration,
}

impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("endpoint", &self.endpoint)
            .field("timeout", &self.timeout)
            .field("has_cache", &self.cache.is_some())
            .field("cache_ttl", &self.cache_ttl)
            .finish()
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            timeout: DEFAULT_TIMEOUT,
            cache: None,
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }
}
```

Add `with_cache` method:

```rust
/// Enable caching with a custom cache implementation.
pub fn with_cache<C: PriceCache + 'static>(mut self, cache: C) -> Self {
    self.cache = Some(Arc::new(cache));
    self
}
```

Update `build()` to use the cache:

```rust
pub fn build(self) -> Result<Client> {
    let http = reqwest::Client::builder()
        .timeout(self.timeout)
        .build()
        .map_err(|e| Error::config(format!("Failed to build HTTP client: {}", e)))?;

    Ok(Client {
        inner: Arc::new(ClientInner {
            http,
            api_key: self.api_key,
            endpoint: self
                .endpoint
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            cache: self.cache,
            cache_ttl: self.cache_ttl,
        }),
    })
}
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/client.rs
git commit -m "feat(cache): add with_cache() method to ClientBuilder"
```

---

## Task 5: Integrate Cache Check in execute_query

**Files:**
- Modify: `src/client.rs`

**Step 1: Update execute_query method**

Update the `execute_query` method in `src/client.rs`:

```rust
pub(crate) async fn execute_query(
    &self,
    filter: ProductFilter,
    api_key_override: Option<&str>,
) -> Result<Vec<Product>> {
    let api_key = api_key_override
        .or(self.inner.api_key.as_deref())
        .ok_or(Error::MissingApiKey)?;

    // Check cache first (only for authenticated requests)
    let cache_key = filter.cache_key();
    if let Some(cache) = &self.inner.cache {
        if let Some(cached) = cache.get(&cache_key).await {
            tracing::debug!(key = %cache_key, "cache hit");
            return Ok(cached);
        }
        tracing::debug!(key = %cache_key, "cache miss");
    }

    let gql_filter: GqlProductFilter = filter.into();
    let operation = ProductQuery::build(ProductQueryVariables {
        filter: Some(gql_filter),
    });

    // Serialize to JSON, removing null fields (Infracost API quirk)
    let mut operation_json =
        serde_json::to_value(&operation).map_err(|e| Error::config(e.to_string()))?;
    remove_nulls(&mut operation_json);

    tracing::debug!("Sending GraphQL query to Infracost API");
    if tracing::enabled!(tracing::Level::TRACE)
        && let Ok(json_str) = serde_json::to_string_pretty(&operation_json)
    {
        tracing::trace!("Query: {}", json_str);
    }

    let response = self
        .inner
        .http
        .post(&self.inner.endpoint)
        .header("X-Api-Key", api_key)
        .header(
            "User-Agent",
            concat!("infracost-rs/", env!("CARGO_PKG_VERSION")),
        )
        .header("Content-Type", "application/json")
        .json(&operation_json)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::api(
            status.as_u16(),
            if error_text.is_empty() {
                status.to_string()
            } else {
                error_text
            },
        ));
    }

    let response_text = response.text().await?;
    tracing::trace!(
        "Response: {}",
        &response_text[..response_text.len().min(1000)]
    );

    let gql_response: cynic::GraphQlResponse<ProductQuery> =
        serde_json::from_str(&response_text)?;

    if let Some(errors) = gql_response.errors {
        let error_msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        return Err(Error::graphql(error_msgs.join("; ")));
    }

    let data = gql_response
        .data
        .ok_or_else(|| Error::graphql("No data in response"))?;

    let products: Vec<Product> = data
        .products
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .map(Product::from)
        .collect();

    tracing::debug!("Query returned {} products", products.len());

    // Cache the result
    if let Some(cache) = &self.inner.cache {
        cache.set(&cache_key, &products, self.inner.cache_ttl).await;
    }

    Ok(products)
}
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/client.rs
git commit -m "feat(cache): integrate cache check in execute_query"
```

---

## Task 6: Implement MemoryCache

**Files:**
- Create: `src/cache/memory.rs`
- Modify: `Cargo.toml`

**Step 1: Add moka dependency**

Add to `Cargo.toml` in `[features]`:

```toml
cache-memory = ["moka"]
```

Add to `[dependencies]`:

```toml
moka = { version = "0.12", features = ["future"], optional = true }
```

**Step 2: Create memory.rs**

Create `src/cache/memory.rs`:

```rust
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

        cache.set("test-key", &products, Duration::from_secs(60)).await;
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
```

**Step 3: Run tests with feature**

Run: `cargo test --features cache-memory`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/cache/memory.rs Cargo.toml
git commit -m "feat(cache): implement MemoryCache with moka"
```

---

## Task 7: Implement RedisCache

**Files:**
- Create: `src/cache/redis.rs`
- Modify: `Cargo.toml`

**Step 1: Add redis dependency**

Add to `Cargo.toml` in `[features]`:

```toml
cache-redis = ["redis"]
```

Add to `[dependencies]`:

```toml
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"], optional = true }
```

**Step 2: Create redis.rs**

Create `src/cache/redis.rs`:

```rust
//! Redis-backed cache for shared caching across instances.

use super::{PriceCache, DEFAULT_CACHE_TTL};
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
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .ok()?;

        let data: Option<String> = conn.get(key).await.ok()?;
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
            return;
        };

        // Only clear infracost keys, not entire Redis
        let keys: Result<Vec<String>, _> = redis::cmd("KEYS")
            .arg("infracost:*")
            .query_async(&mut conn)
            .await;

        if let Ok(keys) = keys {
            for key in keys {
                let _: Result<(), _> = conn.del(&key).await;
            }
        }
    }
}

// Note: Redis tests require a running Redis instance and are in integration tests
```

**Step 3: Verify it compiles**

Run: `cargo build --features cache-redis`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cache/redis.rs Cargo.toml
git commit -m "feat(cache): implement RedisCache"
```

---

## Task 8: Add Feature-Gated Re-exports

**Files:**
- Modify: `src/lib.rs`

**Step 1: Update lib.rs exports**

Update `src/lib.rs` to add conditional re-exports:

```rust
pub use cache::PriceCache;
#[cfg(feature = "cache-memory")]
pub use cache::MemoryCache;
#[cfg(feature = "cache-redis")]
pub use cache::RedisCache;
```

**Step 2: Run tests with both features**

Run: `cargo test --features "cache-memory cache-redis"`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat(cache): add feature-gated re-exports"
```

---

## Task 9: Add Integration Test for Cache

**Files:**
- Modify: `tests/mock_tests.rs`

**Step 1: Add cache integration test**

Add to `tests/mock_tests.rs`:

```rust
#[cfg(feature = "cache-memory")]
mod cache_tests {
    use infracost_rs::{Client, MemoryCache, PriceCache};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    // Custom cache that counts calls
    struct CountingCache {
        inner: MemoryCache,
        get_count: AtomicUsize,
        set_count: AtomicUsize,
    }

    impl CountingCache {
        fn new() -> Self {
            Self {
                inner: MemoryCache::new(),
                get_count: AtomicUsize::new(0),
                set_count: AtomicUsize::new(0),
            }
        }

        fn get_count(&self) -> usize {
            self.get_count.load(Ordering::SeqCst)
        }

        fn set_count(&self) -> usize {
            self.set_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl PriceCache for CountingCache {
        async fn get(&self, key: &str) -> Option<Vec<infracost_rs::Product>> {
            self.get_count.fetch_add(1, Ordering::SeqCst);
            self.inner.get(key).await
        }

        async fn set(&self, key: &str, products: &[infracost_rs::Product], ttl: Duration) {
            self.set_count.fetch_add(1, Ordering::SeqCst);
            self.inner.set(key, products, ttl).await;
        }
    }

    #[tokio::test]
    async fn test_cache_not_used_without_api_key() {
        // When no API key is provided, cache should never be touched
        let cache = Arc::new(CountingCache::new());

        let client = Client::builder()
            .with_cache(cache.clone())
            .build()
            .unwrap();

        // This should fail with MissingApiKey before reaching cache
        let result = client
            .products()
            .vendor("aws")
            .fetch()
            .await;

        assert!(result.is_err());
        assert_eq!(cache.get_count(), 0);
        assert_eq!(cache.set_count(), 0);
    }
}
```

**Step 2: Run the test**

Run: `cargo test --features cache-memory cache_tests`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/mock_tests.rs
git commit -m "test(cache): add integration test verifying keyless users don't touch cache"
```

---

## Task 10: Update Documentation

**Files:**
- Modify: `src/lib.rs` (doc comments)
- Modify: `README.md`

**Step 1: Update lib.rs module docs**

Update the module-level documentation in `src/lib.rs`:

```rust
//! Rust client for the Infracost Cloud Pricing API.
//!
//! # Basic Usage
//!
//! ```no_run
//! use infracost_rs::Client;
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::from_env()?;
//! let products = client
//!     .products()
//!     .vendor("gcp")
//!     .service("Compute Engine")
//!     .region("us-central1")
//!     .fetch()
//!     .await?;
//! println!("${}", products[0].price_f64()?);
//! # Ok(())
//! # }
//! ```
//!
//! # Caching
//!
//! Enable caching to reduce API latency. Requires feature flags:
//!
//! ```toml
//! [dependencies]
//! infracost-rs = { version = "0.1", features = ["cache-memory"] }
//! ```
//!
//! ```ignore
//! use infracost_rs::{Client, MemoryCache};
//!
//! let client = Client::builder()
//!     .with_cache(MemoryCache::new())
//!     .build()?;
//! ```
//!
//! For shared caching across instances, use Redis:
//!
//! ```ignore
//! use infracost_rs::{Client, RedisCache};
//!
//! let client = Client::builder()
//!     .with_cache(RedisCache::new("redis://localhost:6379")?)
//!     .build()?;
//! ```
//!
//! # Provider Convenience API
//!
//! For common cloud resources, use the provider-specific convenience methods
//! which include built-in default prices:
//!
//! ```no_run
//! use infracost_rs::Client;
//! use infracost_rs::providers::gcp::DiskType;
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::anonymous();
//!
//! // Returns built-in default ($0.17/GB-month) when no API key
//! let price = client
//!     .gcp()
//!     .disk(DiskType::PdSsd)
//!     .region("us-central1")
//!     .fetch_price()
//!     .await?;
//! # Ok(())
//! # }
//! ```
```

**Step 2: Run doc tests**

Run: `cargo test --doc --features "cache-memory cache-redis"`
Expected: Doc tests pass (ignore blocks don't run)

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "docs: add caching documentation to lib.rs"
```

---

## Task 11: Final Verification

**Step 1: Run full test suite**

```bash
cargo test
cargo test --features cache-memory
cargo test --features cache-redis
cargo test --features "cache-memory cache-redis"
cargo clippy --features "cache-memory cache-redis"
cargo doc --features "cache-memory cache-redis"
```

Expected: All pass

**Step 2: Commit any fixes if needed**

**Step 3: Final commit summarizing feature**

```bash
git log --oneline -10
```

Review commits look correct.

---

## Summary

After completing all tasks, the feature branch will have:

1. **Refactored API**: `require_api` removed from all providers, replaced with `ClientBuilder::error_on_fallback()`
2. `cache_key()` method on `ProductFilter`
3. `PriceCache` trait in `src/cache/mod.rs`
4. `MemoryCache` implementation (feature: `cache-memory`)
5. `RedisCache` implementation (feature: `cache-redis`)
6. `ClientBuilder::with_cache()` and `cache_ttl()` methods
7. Cache integration in `execute_query()` (after API key check)
8. Tests verifying keyless users don't touch cache
9. Updated documentation
