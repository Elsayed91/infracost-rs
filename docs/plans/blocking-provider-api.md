# Implementation Plan: Blocking Provider Convenience API

## Overview

Extend `blocking::Client` to expose the high-level provider convenience API (`gcp()`, `aws()`, `azure()`) with their fluent builders and `fetch_monthly()` methods, mirroring the existing async provider API.

## Architecture

All blocking wrappers follow the same pattern already established by `BlockingProductQueryBuilder`:
1. Wrap the inner async type
2. Store `Arc<Runtime>` for executing async calls
3. Forward builder methods to inner async builder
4. Terminal methods (fetch/fetch_price/fetch_monthly) call `runtime.block_on(inner.method())`

**Key constraint**: The async builder types hold `&'a Client` references with lifetime `'a`. The blocking wrappers cannot wrap them directly because the blocking `Client` owns the `crate::Client` (not borrowed). Instead, we construct the async builders on the fly inside the terminal methods (fetch/fetch_price/fetch_monthly), or we make the blocking builders own their parameters and construct the async chain at execution time.

**Chosen approach**: Each blocking builder stores its own parameters (not the async builder) and constructs the full async chain inside terminal methods. This avoids lifetime issues entirely.

## File: `src/blocking.rs` - All changes go here

### Step 1: Fix ClientBuilder (add missing cache methods)

Add to `ClientBuilder`:
```rust
pub fn with_cache<C: PriceCache + 'static>(mut self, cache: C) -> Self
pub fn cache_ttl(mut self, ttl: Duration) -> Self
pub fn error_on_fallback(mut self, enabled: bool) -> Self
```

### Step 2: Add provider methods to blocking Client

```rust
impl Client {
    pub fn gcp(&self) -> BlockingGcpProvider { ... }
    pub fn aws(&self) -> BlockingAwsProvider { ... }
    pub fn azure(&self) -> BlockingAzureProvider { ... }
}
```

### Step 3: GCP Blocking Provider and Builders

**BlockingGcpProvider** with methods:
- `disk(disk_type) -> BlockingGcpDiskBuilder`
- `snapshot() -> BlockingGcpSnapshotBuilder`
- `static_ip() -> BlockingGcpStaticIpBuilder`
- `nat_gateway() -> BlockingGcpNatGatewayBuilder`
- `forwarding_rule() -> BlockingGcpForwardingRuleBuilder`

Each builder mirrors the async builder's methods exactly.

### Step 4: AWS Blocking Provider and Builders

**BlockingAwsProvider** with methods:
- `ebs(ebs_type) -> BlockingAwsEbsBuilder`
- `snapshot() -> BlockingAwsSnapshotBuilder`
- `elastic_ip() -> BlockingAwsElasticIpBuilder`
- `nat_gateway() -> BlockingAwsNatGatewayBuilder`
- `alb() -> BlockingAwsAlbBuilder`

### Step 5: Azure Blocking Provider and Builders

**BlockingAzureProvider** with methods:
- `managed_disk(disk_type, size) -> BlockingAzureManagedDiskBuilder`
- `snapshot() -> BlockingAzureSnapshotBuilder`
- `public_ip() -> BlockingAzurePublicIpBuilder`

## Builder Pattern (same for all)

Each blocking builder:
1. Stores `inner: crate::Client` (cloned) and `runtime: Arc<Runtime>`
2. Stores the same parameters as the async builder (region, api_key, override_default, etc.)
3. Builder methods set parameters and return `Self`
4. Terminal methods construct the async builder chain and call `runtime.block_on()`

Example for GcpDiskBuilder:
```rust
pub struct BlockingGcpDiskBuilder {
    client: crate::Client,
    runtime: Arc<Runtime>,
    disk_type: DiskType,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
    iops: Option<u64>,
}

impl BlockingGcpDiskBuilder {
    pub fn region(mut self, region: impl Into<String>) -> Self { ... }
    pub fn api_key(mut self, key: impl Into<String>) -> Self { ... }
    pub fn override_default(mut self, price: f64) -> Self { ... }
    pub fn size_gb(mut self, size: u64) -> Self { ... }
    pub fn iops(mut self, iops: u64) -> Self { ... }

    pub fn fetch(self) -> Result<PriceResult> {
        let mut builder = self.client.gcp().disk(self.disk_type);
        if let Some(r) = self.region { builder = builder.region(r); }
        if let Some(k) = self.api_key { builder = builder.api_key(k); }
        if let Some(p) = self.override_default { builder = builder.override_default(p); }
        if let Some(s) = self.size_gb { builder = builder.size_gb(s); }
        if let Some(i) = self.iops { builder = builder.iops(i); }
        self.runtime.block_on(builder.fetch())
    }

    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
    }

    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut builder = self.client.gcp().disk(self.disk_type);
        // ... set all fields ...
        self.runtime.block_on(builder.fetch_monthly())
    }
}
```

## Testing Strategy

### Unit tests (in blocking.rs #[cfg(test)])
- Test that each blocking builder can be constructed and chained
- Test with anonymous client (returns defaults, no API call)
- Verify defaults match async versions

### Integration tests (tests/blocking_integration.rs)
- Real API calls with INFRACOST_API_KEY
- Test all providers, all resource types
- Test fetch, fetch_price, fetch_monthly for each
- Test builder with cache support
- Compare blocking vs async results for same queries
