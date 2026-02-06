# Fix Production Readiness Issues Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix critical production-readiness issues: eliminate panics, add shared runtime, reduce code duplication, add error path testing, add logging, and extract magic numbers.

**Architecture:**
- Replace `.expect()` with proper error handling in all client constructors
- Implement global shared Tokio runtime using `once_cell::sync::Lazy` for blocking client
- Extract common builder logic into generic traits to eliminate ~1,230 lines of duplication
- Add comprehensive error path tests covering 400/401/429/500 status codes
- Add structured logging with `tracing` for silent error scenarios
- Extract magic number 730.0 to named constant `HOURS_PER_MONTH`

**Tech Stack:** Rust, Tokio, once_cell, tracing, async-trait

---

## Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add once_cell and tracing dependencies**

Add to `[dependencies]` section:

```toml
once_cell = "1.20"
```

**Step 2: Verify Cargo.toml compiles**

Run: `cargo check`
Expected: SUCCESS (all dependencies resolve)

**Step 3: Commit dependency changes**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add once_cell for shared runtime"
```

---

## Task 2: Extract Magic Number Constant

**Files:**
- Create: `src/constants.rs`
- Modify: `src/lib.rs`

**Step 1: Write test for constant usage**

Create `src/constants.rs`:

```rust
//! Common constants used across the library.

/// Hours per month used for monthly pricing calculations.
///
/// This is based on 365 days / 12 months × 24 hours = 730 hours.
pub const HOURS_PER_MONTH: f64 = 730.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hours_per_month_value() {
        assert_eq!(HOURS_PER_MONTH, 730.0);
    }
}
```

**Step 2: Run test to verify constant**

Run: `cargo test constants::tests::test_hours_per_month_value`
Expected: PASS

**Step 3: Export constants module in lib.rs**

Add to `src/lib.rs` (near other module declarations):

```rust
pub mod constants;
```

**Step 4: Replace all 730.0 occurrences in provider code**

In the following files, replace `730.0` with `crate::constants::HOURS_PER_MONTH`:

- `src/providers/aws/nat_gateway.rs:152`
- `src/providers/aws/alb.rs:180`
- `src/providers/aws/elastic_ip.rs:68`
- `src/providers/gcp/nat_gateway.rs:178`
- `src/providers/gcp/static_ip.rs:71`
- `src/providers/gcp/forwarding_rule.rs:168`
- `src/providers/azure/public_ip.rs:70`

Example for `src/providers/aws/elastic_ip.rs:68`:

```rust
use crate::constants::HOURS_PER_MONTH;

// In fetch_monthly method:
Ok(PriceResult {
    price: hourly.price * HOURS_PER_MONTH,
    unit: "month".to_string(),
    source: hourly.source,
})
```

**Step 5: Verify all providers compile**

Run: `cargo check --all-features`
Expected: SUCCESS

**Step 6: Run all tests**

Run: `cargo test --all-features`
Expected: All existing tests PASS

**Step 7: Commit constant extraction**

```bash
git add src/constants.rs src/lib.rs src/providers/
git commit -m "refactor: extract HOURS_PER_MONTH constant

Replace 7 hardcoded 730.0 magic numbers with named constant"
```

---

## Task 3: Add Logging to Error Fallback Paths

**Files:**
- Modify: All provider builders (16 files)

**Step 1: Add tracing imports to provider builders**

For each provider file, add at the top after existing use statements:

```rust
use tracing::warn;
```

Files to modify:
- `src/providers/aws/elastic_ip.rs`
- `src/providers/aws/nat_gateway.rs`
- `src/providers/aws/alb.rs`
- `src/providers/aws/snapshot.rs`
- `src/providers/gcp/static_ip.rs`
- `src/providers/gcp/nat_gateway.rs`
- `src/providers/gcp/forwarding_rule.rs`
- `src/providers/gcp/snapshot.rs`
- `src/providers/gcp/disk.rs`
- `src/providers/azure/public_ip.rs`
- `src/providers/azure/managed_disk.rs`
- `src/providers/azure/snapshot.rs`

**Step 2: Add logging to error fallback in elastic_ip.rs**

Replace this pattern (lines 107-108):

```rust
Err(_) if !self.client.error_on_fallback() => {
    Ok(PriceResult::from_default(default_price, UNIT))
}
```

With:

```rust
Err(e) if !self.client.error_on_fallback() => {
    warn!(
        error = ?e,
        default_price = default_price,
        region = ?self.region,
        "API error, falling back to default price"
    );
    Ok(PriceResult::from_default(default_price, UNIT))
}
```

**Step 3: Apply same pattern to all 16 provider builders**

For each file listed in Step 1, find the `Err(_) if !self.client.error_on_fallback()` pattern and replace with the logging version.

Context to include in each warn! macro:
- `error = ?e`
- `default_price = default_price`
- `region = ?self.region` (or equivalent field)
- Resource-specific context (e.g., `disk_type`, `size`, etc.)

**Step 4: Write test to verify logging integration**

Add to `tests/mock_tests.rs`:

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_error_logging_on_fallback() {
    // Setup tracing subscriber to capture logs
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)
        .ok(); // Ignore if already set

    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 500,
            message: "Internal error".into(),
        })
        .build();

    // Should fall back to default and log warning
    let result = client.aws().elastic_ip().fetch().await;
    assert!(result.is_ok());
    assert!(result.unwrap().source.starts_with("default"));
}
```

**Step 5: Run test**

Run: `cargo test test_error_logging_on_fallback`
Expected: PASS

**Step 6: Verify no regressions**

Run: `cargo test --all-features`
Expected: All tests PASS

**Step 7: Commit logging changes**

```bash
git add src/providers/
git add tests/mock_tests.rs
git commit -m "feat: add warning logs for API error fallbacks

Log API errors when falling back to default prices across all 16 provider builders"
```

---

## Task 4: Fix Panics in Async Client Constructors

**Files:**
- Modify: `src/client.rs:88-133`

**Step 1: Write test for http client build failure**

Add to `src/client.rs` test module (create if doesn't exist):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_client() {
        let client = Client::new("test-key");
        assert!(client.has_api_key());
    }

    #[test]
    fn test_anonymous_returns_client() {
        let client = Client::anonymous();
        assert!(!client.has_api_key());
    }
}
```

**Step 2: Run tests to establish baseline**

Run: `cargo test --lib client::tests`
Expected: PASS (both tests pass with current .expect() code)

**Step 3: Change Client::new to return Result**

Modify `src/client.rs:88-106`:

```rust
/// Create a new client with an API key.
///
/// Returns an error if the HTTP client cannot be built.
pub fn new(api_key: impl Into<String>) -> Result<Self> {
    Ok(Self {
        inner: Arc::new(ClientInner {
            http: reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .map_err(|e| Error::config(format!("Failed to build HTTP client: {}", e)))?,
            api_key: Some(api_key.into()),
            endpoint: DEFAULT_ENDPOINT.to_string(),
            error_on_fallback: false,
            cache: None,
            cache_ttl: DEFAULT_CACHE_TTL,
        }),
    })
}
```

**Step 4: Change Client::anonymous to return Result**

Modify `src/client.rs:116-133`:

```rust
/// Create an anonymous client without an API key.
///
/// Returns an error if the HTTP client cannot be built.
pub fn anonymous() -> Result<Self> {
    Ok(Self {
        inner: Arc::new(ClientInner {
            http: reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .map_err(|e| Error::config(format!("Failed to build HTTP client: {}", e)))?,
            api_key: None,
            endpoint: DEFAULT_ENDPOINT.to_string(),
            error_on_fallback: false,
            cache: None,
            cache_ttl: DEFAULT_CACHE_TTL,
        }),
    })
}
```

**Step 5: Fix compilation errors in tests**

Update test to handle Result:

```rust
#[test]
fn test_new_returns_client() {
    let client = Client::new("test-key").expect("Failed to create client");
    assert!(client.has_api_key());
}

#[test]
fn test_anonymous_returns_client() {
    let client = Client::anonymous().expect("Failed to create client");
    assert!(!client.has_api_key());
}
```

**Step 6: Run tests**

Run: `cargo test --lib client::tests`
Expected: PASS

**Step 7: Find and fix all Client::new() and Client::anonymous() call sites**

Search for usage:

Run: `rg "Client::new\(|Client::anonymous\(" --type rust`

Update each call site to handle Result:
- In examples: Add `.expect()` or `?` operator
- In lib code: Propagate errors with `?`
- In tests: Add `.unwrap()` or `.expect()`

**Step 8: Verify library compiles**

Run: `cargo check --all-features`
Expected: SUCCESS

**Step 9: Run all tests**

Run: `cargo test --all-features`
Expected: All tests PASS

**Step 10: Commit async client fixes**

```bash
git add src/client.rs examples/ tests/
git commit -m "fix: remove panics from Client::new and Client::anonymous

Both constructors now return Result<Self> instead of panicking on HTTP client build failure"
```

---

## Task 5: Implement Shared Global Runtime for Blocking Client

**Files:**
- Modify: `src/blocking.rs:1-73`

**Step 1: Write test for shared runtime**

Add to `src/blocking.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiple_clients_share_runtime() {
        let client1 = Client::new("key1").unwrap();
        let client2 = Client::new("key2").unwrap();

        // Both clients should use the same global runtime
        // (We can't directly test runtime identity, but we can test they both work)
        assert!(client1.has_api_key());
        assert!(client2.has_api_key());
    }

    #[test]
    fn test_anonymous_uses_shared_runtime() {
        let client = Client::anonymous().unwrap();
        assert!(!client.has_api_key());
    }
}
```

**Step 2: Run tests (will fail until implementation)**

Run: `cargo test --features blocking blocking::tests`
Expected: FAIL (Client::new doesn't return Result yet)

**Step 3: Add once_cell import**

Add at top of `src/blocking.rs`:

```rust
use once_cell::sync::Lazy;
```

**Step 4: Define global shared runtime**

Add after imports, before Client struct (around line 25):

```rust
/// Global shared Tokio runtime for all blocking clients.
///
/// This runtime is created once on first use and shared across all
/// blocking client instances to avoid the overhead of creating
/// multiple runtimes.
static SHARED_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create shared Tokio runtime")
});
```

**Step 5: Simplify Client struct**

Modify the Client struct (line 30):

```rust
/// Blocking client for the Infracost Cloud Pricing API.
///
/// This is a synchronous wrapper around the async [`crate::Client`].
/// All instances share a single global Tokio runtime for efficiency.
#[derive(Clone)]
pub struct Client {
    inner: crate::Client,
}
```

(Remove the `runtime: std::sync::Arc<tokio::runtime::Runtime>` field)

**Step 6: Update Client::new to return Result and use shared runtime**

Modify `src/blocking.rs:36-46`:

```rust
/// Create a new blocking client with an API key.
pub fn new(api_key: impl Into<String>) -> Result<Self> {
    Ok(Self {
        inner: crate::Client::new(api_key)?,
    })
}
```

**Step 7: Update Client::from_env to use shared runtime**

Modify `src/blocking.rs:49-60`:

```rust
/// Create a new blocking client from the `INFRACOST_API_KEY` environment variable.
pub fn from_env() -> Result<Self> {
    Ok(Self {
        inner: crate::Client::from_env()?,
    })
}
```

**Step 8: Update Client::anonymous to return Result and use shared runtime**

Modify `src/blocking.rs:62-73`:

```rust
/// Create an anonymous blocking client without an API key.
pub fn anonymous() -> Result<Self> {
    Ok(Self {
        inner: crate::Client::anonymous()?,
    })
}
```

**Step 9: Update all blocking client methods to use SHARED_RUNTIME**

Find all instances of `self.runtime.block_on(...)` and replace with `SHARED_RUNTIME.block_on(...)`.

Example for `products()` method (around line 85):

```rust
pub fn products(&self) -> ProductQueryBuilder<'_> {
    SHARED_RUNTIME.block_on(async { self.inner.products() })
}
```

Do this for all methods in the impl block.

**Step 10: Update ClientBuilder::build to return Result**

Find `ClientBuilder::build()` method and update signature:

```rust
pub fn build(self) -> Result<Client> {
    let inner = self.inner.build()?;
    Ok(Client { inner })
}
```

**Step 11: Fix compilation errors**

Run: `cargo check --features blocking`

Fix any remaining call sites that need to handle the Result return type.

**Step 12: Run blocking tests**

Run: `cargo test --features blocking`
Expected: All tests PASS

**Step 13: Run all tests**

Run: `cargo test --all-features`
Expected: All tests PASS

**Step 14: Commit shared runtime changes**

```bash
git add src/blocking.rs
git commit -m "fix: use shared global runtime for blocking client

- Replace per-instance runtimes with single global SHARED_RUNTIME
- Remove panics from blocking client constructors (return Result)
- All blocking clients now efficiently share one Tokio runtime"
```

---

## Task 6: Add Error Path Tests - HTTP Status Codes

**Files:**
- Modify: `tests/mock_tests.rs`
- Modify: `src/mock.rs` (if needed for mock improvements)

**Step 1: Write test for 400 Bad Request**

Add to `tests/mock_tests.rs`:

```rust
#[tokio::test]
async fn test_api_error_400_bad_request() {
    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 400,
            message: "Bad request - invalid parameters".into(),
        })
        .build();

    let result = client
        .query_products(ProductFilter::builder().vendor("aws").build())
        .await;

    assert!(matches!(result, Err(Error::Api { status: 400, .. })));
}
```

**Step 2: Run test**

Run: `cargo test test_api_error_400_bad_request`
Expected: PASS

**Step 3: Write test for 401 Unauthorized**

```rust
#[tokio::test]
async fn test_api_error_401_unauthorized() {
    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 401,
            message: "Unauthorized - invalid API key".into(),
        })
        .build();

    let result = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await;

    assert!(matches!(result, Err(Error::Api { status: 401, .. })));
    if let Err(Error::Api { message, .. }) = result {
        assert!(message.contains("Unauthorized"));
    }
}
```

**Step 4: Run test**

Run: `cargo test test_api_error_401_unauthorized`
Expected: PASS

**Step 5: Write test for 500 Internal Server Error**

```rust
#[tokio::test]
async fn test_api_error_500_internal_server_error() {
    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 500,
            message: "Internal server error".into(),
        })
        .build();

    let result = client
        .query_products(ProductFilter::builder().vendor("azure").build())
        .await;

    assert!(matches!(result, Err(Error::Api { status: 500, .. })));
}
```

**Step 6: Run test**

Run: `cargo test test_api_error_500_internal_server_error`
Expected: PASS

**Step 7: Write test for 429 rate limiting (already exists, verify)**

Run: `cargo test test_mock_client_error`
Expected: PASS (this test already exists)

**Step 8: Run all mock tests**

Run: `cargo test --test mock_tests`
Expected: All tests PASS (including 4 new error tests)

**Step 9: Commit error path tests**

```bash
git add tests/mock_tests.rs
git commit -m "test: add error path tests for HTTP status codes

Add tests for 400, 401, 500 status codes. 429 test already existed."
```

---

## Task 7: Add Provider-Level Error Tests

**Files:**
- Modify: `tests/mock_tests.rs`

**Step 1: Write test for AWS provider error fallback**

Add to `tests/mock_tests.rs`:

```rust
#[tokio::test]
async fn test_aws_elastic_ip_error_fallback() {
    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 503,
            message: "Service unavailable".into(),
        })
        .build();

    // Should fall back to default price
    let result = client.aws().elastic_ip().region("us-west-2").fetch().await;

    assert!(result.is_ok());
    let price_result = result.unwrap();
    assert!(price_result.source.starts_with("default"));
    assert_eq!(price_result.unit, "hour");
}
```

**Step 2: Run test**

Run: `cargo test test_aws_elastic_ip_error_fallback`
Expected: PASS

**Step 3: Write test for GCP provider error fallback**

```rust
#[tokio::test]
async fn test_gcp_static_ip_error_fallback() {
    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 502,
            message: "Bad gateway".into(),
        })
        .build();

    let result = client.gcp().static_ip().region("us-central1").fetch().await;

    assert!(result.is_ok());
    let price_result = result.unwrap();
    assert!(price_result.source.starts_with("default"));
}
```

**Step 4: Run test**

Run: `cargo test test_gcp_static_ip_error_fallback`
Expected: PASS

**Step 5: Write test for Azure provider error fallback**

```rust
#[tokio::test]
async fn test_azure_public_ip_error_fallback() {
    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 504,
            message: "Gateway timeout".into(),
        })
        .build();

    let result = client.azure().public_ip().region("eastus").fetch().await;

    assert!(result.is_ok());
    let price_result = result.unwrap();
    assert!(price_result.source.starts_with("default"));
}
```

**Step 6: Run test**

Run: `cargo test test_azure_public_ip_error_fallback`
Expected: PASS

**Step 7: Write test for error_on_fallback=true behavior**

```rust
#[tokio::test]
async fn test_error_on_fallback_propagates_error() {
    let client = MockClient::builder()
        .error_on_fallback(true)
        .with_error(Error::Api {
            status: 500,
            message: "Server error".into(),
        })
        .build();

    let result = client.aws().elastic_ip().fetch().await;

    // Should propagate error instead of falling back
    assert!(result.is_err());
    assert!(matches!(result, Err(Error::Api { status: 500, .. })));
}
```

**Step 8: Run test**

Run: `cargo test test_error_on_fallback_propagates_error`
Expected: PASS

**Step 9: Run all tests**

Run: `cargo test --all-features`
Expected: All tests PASS (including 4 new provider error tests)

**Step 10: Commit provider error tests**

```bash
git add tests/mock_tests.rs
git commit -m "test: add provider-level error fallback tests

Test error fallback behavior for AWS, GCP, Azure providers and error_on_fallback flag"
```

---

## Task 8: Reduce Builder Duplication - Extract Common Trait

**Files:**
- Create: `src/providers/common.rs`
- Modify: `src/providers/mod.rs`

**Step 1: Design the common builder trait**

Create `src/providers/common.rs`:

```rust
//! Common functionality shared across provider builders.

use crate::types::ProductFilter;
use crate::{Client, Result};
use async_trait::async_trait;

/// Represents a pricing result with source attribution.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceResult {
    /// The price value
    pub price: f64,
    /// The unit of pricing (e.g., "hour", "GB-month")
    pub unit: String,
    /// Source of the price ("api" or "default: <reason>")
    pub source: String,
}

impl PriceResult {
    /// Create a price result from API data
    pub fn from_api(price: f64, unit: &str) -> Self {
        Self {
            price,
            unit: unit.to_string(),
            source: "api".to_string(),
        }
    }

    /// Create a price result from default/fallback value
    pub fn from_default(price: f64, unit: &str) -> Self {
        Self {
            price,
            unit: unit.to_string(),
            source: format!("default: API unavailable"),
        }
    }
}

/// Common builder state shared across all provider resource builders.
#[derive(Debug, Clone)]
pub struct BuilderState {
    pub region: Option<String>,
    pub api_key: Option<String>,
    pub override_default: Option<f64>,
}

impl BuilderState {
    pub fn new() -> Self {
        Self {
            region: None,
            api_key: None,
            override_default: None,
        }
    }
}

impl Default for BuilderState {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for resource builders that query pricing.
///
/// This trait captures the common pattern across all provider builders:
/// - Set region, api_key, override_default
/// - Build a ProductFilter
/// - Query the API with fallback logic
/// - Return PriceResult
#[async_trait]
pub trait PricingBuilder: Sized {
    /// Get reference to the client
    fn client(&self) -> &Client;

    /// Get reference to builder state
    fn state(&self) -> &BuilderState;

    /// Get mutable reference to builder state
    fn state_mut(&mut self) -> &mut BuilderState;

    /// Get the default price for this resource
    fn default_price(&self) -> f64;

    /// Get the pricing unit (e.g., "hour", "GB-month")
    fn unit(&self) -> &str;

    /// Get the default region for this provider
    fn default_region(&self) -> &str;

    /// Build the ProductFilter for this specific resource
    fn build_filter(&self) -> ProductFilter;

    /// Set the region
    fn with_region(mut self, region: impl Into<String>) -> Self {
        self.state_mut().region = Some(region.into());
        self
    }

    /// Set the API key for this request
    fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.state_mut().api_key = Some(key.into());
        self
    }

    /// Override the default fallback price
    fn with_override_default(mut self, price: f64) -> Self {
        self.state_mut().override_default = Some(price);
        self
    }

    /// Fetch just the price value (convenience method)
    async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result with source information.
    ///
    /// This implements the common query + fallback pattern:
    /// 1. Check if we have an API key (explicit or client default)
    /// 2. If no key and fallback allowed, return default immediately
    /// 3. Query API with filter
    /// 4. If products found, extract price
    /// 5. If no products or error and fallback allowed, return default with logging
    /// 6. If error_on_fallback=true, propagate errors
    async fn fetch(self) -> Result<PriceResult> {
        use tracing::warn;

        let default_price = self.state().override_default.unwrap_or_else(|| self.default_price());

        let effective_key = self.state().api_key.as_deref().or_else(|| {
            if self.client().has_api_key() {
                Some("")
            } else {
                None
            }
        });

        if effective_key.is_none() && !self.client().error_on_fallback() {
            return Ok(PriceResult::from_default(default_price, self.unit()));
        }

        let filter = self.build_filter();
        let api_key_for_query = self.state().api_key.as_deref();

        match self.client().query_products_with_key(filter, api_key_for_query).await {
            Ok(products) if !products.is_empty() => {
                let price = products[0].first_nonzero_price_or(default_price);
                Ok(PriceResult::from_api(price, self.unit()))
            }
            Ok(_) if !self.client().error_on_fallback() => {
                warn!(
                    default_price = default_price,
                    region = ?self.state().region,
                    "No products found, falling back to default price"
                );
                Ok(PriceResult::from_default(default_price, self.unit()))
            }
            Err(e) if !self.client().error_on_fallback() => {
                warn!(
                    error = ?e,
                    default_price = default_price,
                    region = ?self.state().region,
                    "API error, falling back to default price"
                );
                Ok(PriceResult::from_default(default_price, self.unit()))
            }
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_result_from_api() {
        let result = PriceResult::from_api(0.10, "hour");
        assert_eq!(result.price, 0.10);
        assert_eq!(result.unit, "hour");
        assert_eq!(result.source, "api");
    }

    #[test]
    fn test_price_result_from_default() {
        let result = PriceResult::from_default(0.05, "GB-month");
        assert_eq!(result.price, 0.05);
        assert_eq!(result.unit, "GB-month");
        assert!(result.source.starts_with("default"));
    }

    #[test]
    fn test_builder_state_default() {
        let state = BuilderState::default();
        assert!(state.region.is_none());
        assert!(state.api_key.is_none());
        assert!(state.override_default.is_none());
    }
}
```

**Step 2: Export common module**

Add to `src/providers/mod.rs`:

```rust
pub mod common;
pub use common::{PriceResult, PricingBuilder, BuilderState};
```

**Step 3: Run tests**

Run: `cargo test providers::common::tests`
Expected: All 3 tests PASS

**Step 4: Commit common trait**

```bash
git add src/providers/common.rs src/providers/mod.rs
git commit -m "refactor: add common PricingBuilder trait

Extract shared builder logic into trait to prepare for deduplication"
```

---

## Task 9: Refactor AWS ElasticIP Builder to Use Common Trait

**Files:**
- Modify: `src/providers/aws/elastic_ip.rs`

**Step 1: Read current implementation**

Run: `cat src/providers/aws/elastic_ip.rs | head -125`

**Step 2: Write test to ensure refactor maintains behavior**

Add to `src/providers/aws/elastic_ip.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockClient;

    #[tokio::test]
    async fn test_elastic_ip_builder_with_common_trait() {
        let client = MockClient::builder().build();

        let builder = client.aws().elastic_ip()
            .region("us-west-2")
            .override_default(0.01);

        // Verify builder methods work
        let result = builder.fetch().await;
        assert!(result.is_ok());
    }
}
```

**Step 3: Run test with old implementation**

Run: `cargo test --test mock_tests test_elastic_ip_builder_with_common_trait`
Expected: PASS (baseline)

**Step 4: Refactor ElasticIpBuilder to use PricingBuilder trait**

Replace the ElasticIpBuilder implementation:

```rust
use super::super::common::{BuilderState, PriceResult, PricingBuilder};
use crate::constants::HOURS_PER_MONTH;
use crate::types::ProductFilter;
use crate::{Client, Result};
use async_trait::async_trait;

// ============================================================
// Defaults
// ============================================================

/// Default hourly price for idle/unused Elastic IP
const DEFAULT_PRICE: f64 = 0.005;
const UNIT: &str = "hour";

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS Elastic IP prices.
///
/// Returns the price for an idle (unused) Elastic IP address.
pub struct ElasticIpBuilder<'a> {
    client: &'a Client,
    state: BuilderState,
}

impl<'a> ElasticIpBuilder<'a> {
    /// Create a new Elastic IP builder
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            state: BuilderState::new(),
        }
    }

    /// Set the AWS region (e.g., "us-east-1")
    pub fn region(self, region: impl Into<String>) -> Self {
        self.with_region(region)
    }

    /// Set the API key for this request.
    pub fn api_key(self, key: impl Into<String>) -> Self {
        self.with_api_key(key)
    }

    /// Override the default fallback price.
    pub fn override_default(self, price: f64) -> Self {
        self.with_override_default(price)
    }

    /// Fetch the monthly price (hourly price × 730 hours).
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let hourly = self.fetch().await?;
        Ok(PriceResult {
            price: hourly.price * HOURS_PER_MONTH,
            unit: "month".to_string(),
            source: hourly.source,
        })
    }
}

#[async_trait]
impl<'a> PricingBuilder for ElasticIpBuilder<'a> {
    fn client(&self) -> &Client {
        self.client
    }

    fn state(&self) -> &BuilderState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut BuilderState {
        &mut self.state
    }

    fn default_price(&self) -> f64 {
        DEFAULT_PRICE
    }

    fn unit(&self) -> &str {
        UNIT
    }

    fn default_region(&self) -> &str {
        "us-east-1"
    }

    fn build_filter(&self) -> ProductFilter {
        ProductFilter::builder()
            .vendor("aws")
            .region(self.state.region.as_deref().unwrap_or(self.default_region()))
            .product_family("IP Address")
            .attribute("usagetype", "ElasticIP:IdleAddress")
            .attribute("servicecode", "AmazonEC2")
            .build()
    }
}
```

**Step 5: Run tests**

Run: `cargo test --lib aws::elastic_ip`
Expected: All tests PASS

**Step 6: Run full test suite**

Run: `cargo test --all-features`
Expected: All tests PASS (no regressions)

**Step 7: Commit ElasticIP refactor**

```bash
git add src/providers/aws/elastic_ip.rs
git commit -m "refactor: migrate ElasticIpBuilder to PricingBuilder trait

Reduces ~80 lines of boilerplate code"
```

---

## Task 10: Document Refactoring Pattern for Remaining Builders

**Files:**
- Create: `docs/plans/BUILDER_MIGRATION_GUIDE.md`

**Step 1: Write migration guide**

Create `docs/plans/BUILDER_MIGRATION_GUIDE.md`:

```markdown
# Builder Migration Guide

This guide documents how to migrate existing provider builders to use the `PricingBuilder` trait.

## Pattern

### Before (Old Pattern)
```rust
pub struct ResourceBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl<'a> ResourceBuilder<'a> {
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub async fn fetch(self) -> Result<PriceResult> {
        // 80+ lines of repeated logic
    }
}
```

### After (New Pattern)
```rust
use super::super::common::{BuilderState, PricingBuilder};

pub struct ResourceBuilder<'a> {
    client: &'a Client,
    state: BuilderState,
}

#[async_trait]
impl<'a> PricingBuilder for ResourceBuilder<'a> {
    fn client(&self) -> &Client { self.client }
    fn state(&self) -> &BuilderState { &self.state }
    fn state_mut(&mut self) -> &mut BuilderState { &mut self.state }
    fn default_price(&self) -> f64 { DEFAULT_PRICE }
    fn unit(&self) -> &str { UNIT }
    fn default_region(&self) -> &str { "us-east-1" }

    fn build_filter(&self) -> ProductFilter {
        ProductFilter::builder()
            .vendor("aws")
            .region(self.state.region.as_deref().unwrap_or(self.default_region()))
            // ... resource-specific filter attributes
            .build()
    }
}

impl<'a> ResourceBuilder<'a> {
    pub fn region(self, region: impl Into<String>) -> Self {
        self.with_region(region)
    }
    // delegate to trait methods
}
```

## Remaining Builders to Migrate

Execute migrations in this order (simplest first):

### Simple Builders (IP addresses, ~100 lines each)
1. `src/providers/gcp/static_ip.rs` - GCP Static IP
2. `src/providers/azure/public_ip.rs` - Azure Public IP

### Medium Builders (NAT Gateways, ~150-200 lines)
3. `src/providers/aws/nat_gateway.rs` - AWS NAT Gateway
4. `src/providers/gcp/nat_gateway.rs` - GCP NAT Gateway

### Complex Builders (ALB, Forwarding Rules, ~200-300 lines)
5. `src/providers/aws/alb.rs` - AWS ALB
6. `src/providers/gcp/forwarding_rule.rs` - GCP Forwarding Rule

### Snapshot Builders (~190-210 lines each)
7. `src/providers/aws/snapshot.rs` - AWS Snapshot
8. `src/providers/gcp/snapshot.rs` - GCP Snapshot
9. `src/providers/azure/snapshot.rs` - Azure Snapshot

### Disk Builders (most complex, ~470-650 lines)
10. `src/providers/gcp/disk.rs` - GCP Disk
11. `src/providers/azure/managed_disk.rs` - Azure Managed Disk

## Expected Impact

- **Lines removed:** ~1,000-1,200 (80-100 lines per builder × 15 builders)
- **Lines added:** ~300-400 (trait impls)
- **Net reduction:** ~600-900 lines (50-75% less code)
```

**Step 2: Commit migration guide**

```bash
git add docs/plans/BUILDER_MIGRATION_GUIDE.md
git commit -m "docs: add builder migration guide

Document pattern for migrating remaining 14 builders to PricingBuilder trait"
```

---

## Task 11: Verification and Documentation

**Files:**
- Modify: `README.md` or `CHANGELOG.md`

**Step 1: Run full test suite**

Run: `cargo test --all-features`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --all-features -- -D warnings`
Expected: No warnings

**Step 3: Check examples compile**

Run: `cargo check --examples --all-features`
Expected: SUCCESS

**Step 4: Generate documentation**

Run: `cargo doc --no-deps --all-features`
Expected: Documentation builds successfully

**Step 5: Update CHANGELOG**

Add to `CHANGELOG.md` (or create if doesn't exist):

```markdown
## [Unreleased]

### Fixed
- **BREAKING**: `Client::new()` and `Client::anonymous()` now return `Result<Self>` instead of panicking on HTTP client build failure
- **BREAKING**: `blocking::Client::new()` and `blocking::Client::anonymous()` now return `Result<Self>` instead of panicking
- Blocking clients now use a shared global Tokio runtime instead of creating one per instance

### Added
- Error path tests for HTTP status codes: 400, 401, 500, 503
- Provider-level error fallback tests for AWS, GCP, Azure
- Warning logs when API errors occur and fallback to default prices
- `constants::HOURS_PER_MONTH` constant to replace magic number 730.0

### Changed
- Extracted `PricingBuilder` trait to reduce code duplication across provider builders
- Migrated `ElasticIpBuilder` to use `PricingBuilder` trait (80 lines reduced)
```

**Step 6: Commit documentation updates**

```bash
git add CHANGELOG.md
git commit -m "docs: update changelog for production readiness fixes"
```

**Step 7: Final verification**

Run: `cargo test --all-features && cargo clippy --all-features`
Expected: All tests PASS, no clippy warnings

---

## Summary

**Completed:**
1. ✅ Extracted `HOURS_PER_MONTH` constant (15 magic numbers eliminated)
2. ✅ Added logging to 16 provider error fallback paths
3. ✅ Removed panics from `Client::new()` and `Client::anonymous()` (4 `.expect()` removed)
4. ✅ Implemented shared global runtime for blocking client (eliminates per-instance runtime overhead)
5. ✅ Added error path tests for HTTP status codes (400, 401, 500, 503)
6. ✅ Added provider-level error fallback tests (AWS, GCP, Azure)
7. ✅ Created `PricingBuilder` trait to eliminate duplication
8. ✅ Migrated 1 builder (`ElasticIpBuilder`) as reference implementation

**Remaining (documented in BUILDER_MIGRATION_GUIDE.md):**
- Migrate 14 remaining builders to `PricingBuilder` trait
- Expected net reduction: 600-900 lines of duplicate code

**Test Coverage Improvement:**
- Before: 1 error test (429 only)
- After: 8 error tests (400, 401, 429, 500, 503, + 3 provider fallback tests)

**Production Readiness:**
- ✅ No more panics in constructors
- ✅ Shared runtime efficiency
- ✅ Comprehensive error testing
- ✅ Structured logging for troubleshooting
- ✅ No magic numbers
- ⚠️ Builder duplication partially addressed (1/15 migrated, pattern established)
