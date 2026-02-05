# Provider Convenience API Design

## Overview

Extend infracost-rs to provide convenience methods and built-in defaults for common cloud resources across GCP, AWS, and Azure. Users can query prices with minimal configuration, and the library automatically falls back to sensible defaults when no API key is available.

## Goals

- **Zero-config pricing**: Users without API keys get reasonable default prices
- **Multi-tenant support**: Per-request API key injection for SaaS applications
- **Provider-native naming**: Use each cloud's terminology (GCP disk vs AWS EBS)
- **Easy extensibility**: Adding new resources follows a simple, consistent pattern
- **Verified queries**: Integration tests confirm each resource query works

## User API

### Basic Usage

```rust
// Simple - defaults automatic when no API key
let price = client.gcp().disk(DiskType::PdSsd).region("us-central1").fetch_price().await?;

// With per-request API key (multi-tenant apps)
let price = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
    .api_key(user.infracost_key)
    .fetch_price()
    .await?;

// Override default fallback
let price = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
    .override_default(0.20)
    .fetch_price()
    .await?;

// Require API (fail if unavailable)
let price = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
    .api_key(key)
    .require_api()
    .fetch_price()
    .await?;

// Get full result with source info
let result = client.gcp().disk(DiskType::PdSsd).region("us-central1").fetch().await?;
println!("Price: ${}/{} (from {:?})", result.price, result.unit, result.source);
```

### Full API Surface

#### GCP

```rust
client.gcp().disk(DiskType::PdStandard).region("us-central1").fetch_price().await?;
client.gcp().disk(DiskType::PdSsd).region("us-central1").fetch_price().await?;
client.gcp().disk(DiskType::PdBalanced).region("us-central1").fetch_price().await?;
client.gcp().disk(DiskType::PdExtreme).region("us-central1").fetch_price().await?;
client.gcp().disk("pd-ssd").region("us-central1").fetch_price().await?;  // String escape hatch

client.gcp().snapshot().region("us-central1").fetch_price().await?;
client.gcp().static_ip().region("us-central1").fetch_price().await?;
client.gcp().nat_gateway().region("us-central1").fetch_price().await?;
client.gcp().forwarding_rule().region("us-central1").fetch_price().await?;
```

#### AWS

```rust
client.aws().ebs(EbsType::Gp3).region("us-east-1").fetch_price().await?;
client.aws().ebs(EbsType::Gp2).region("us-east-1").fetch_price().await?;
client.aws().ebs(EbsType::Io2).region("us-east-1").fetch_price().await?;
client.aws().ebs(EbsType::St1).region("us-east-1").fetch_price().await?;
client.aws().ebs("gp3").region("us-east-1").fetch_price().await?;  // String escape hatch

client.aws().snapshot().region("us-east-1").fetch_price().await?;
client.aws().elastic_ip().region("us-east-1").fetch_price().await?;
client.aws().nat_gateway().region("us-east-1").fetch_price().await?;
client.aws().alb().region("us-east-1").fetch_price().await?;
```

#### Azure

```rust
client.azure().managed_disk(ManagedDiskType::PremiumSsd).region("eastus").fetch_price().await?;
client.azure().managed_disk(ManagedDiskType::StandardHdd).region("eastus").fetch_price().await?;

client.azure().snapshot().region("eastus").fetch_price().await?;
client.azure().public_ip().region("eastus").fetch_price().await?;
client.azure().nat_gateway().region("eastus").fetch_price().await?;
client.azure().load_balancer().region("eastus").fetch_price().await?;
```

### Builder Methods (All Resources)

| Method | Description |
|--------|-------------|
| `.region(r)` | Set region (required for most resources) |
| `.api_key(k)` | Per-request API key injection |
| `.override_default(p)` | Custom fallback price |
| `.require_api()` | Fail if API unavailable |
| `.fetch_price()` | Returns `Result<f64>` |
| `.fetch()` | Returns `Result<PriceResult>` with source info |

## Behavior Matrix

| Scenario | Behavior |
|----------|----------|
| Has API key | Call API → return live price |
| No API key | Skip API → return built-in default |
| API fails + no `require_api()` | Return built-in default |
| API fails + `require_api()` | Return error |
| `override_default(x)` | Use `x` as fallback instead of built-in |

## Module Structure

```
src/
├── lib.rs                        # Re-exports
├── client.rs                     # + .gcp(), .aws(), .azure() methods
├── providers/
│   ├── mod.rs                    # PriceResult, PriceSource
│   ├── gcp/
│   │   ├── mod.rs                # GcpProvider
│   │   ├── disk.rs               # DiskType, DiskBuilder, defaults
│   │   ├── snapshot.rs           # SnapshotBuilder, defaults
│   │   ├── static_ip.rs          # StaticIpBuilder, defaults
│   │   ├── nat_gateway.rs        # NatGatewayBuilder, defaults
│   │   └── forwarding_rule.rs    # ForwardingRuleBuilder, defaults
│   ├── aws/
│   │   ├── mod.rs                # AwsProvider
│   │   ├── ebs.rs                # EbsType, EbsBuilder, defaults
│   │   ├── snapshot.rs
│   │   ├── elastic_ip.rs
│   │   ├── nat_gateway.rs
│   │   └── alb.rs
│   └── azure/
│       ├── mod.rs                # AzureProvider
│       ├── managed_disk.rs
│       ├── snapshot.rs
│       ├── public_ip.rs
│       ├── nat_gateway.rs
│       └── load_balancer.rs
tests/
└── integration/
    ├── mod.rs                    # Test utilities
    ├── gcp/                      # One test file per resource
    │   ├── mod.rs
    │   ├── disk.rs
    │   ├── snapshot.rs
    │   ├── static_ip.rs
    │   ├── nat_gateway.rs
    │   └── forwarding_rule.rs
    ├── aws/
    │   ├── mod.rs
    │   ├── ebs.rs
    │   ├── snapshot.rs
    │   ├── elastic_ip.rs
    │   ├── nat_gateway.rs
    │   └── alb.rs
    └── azure/
        └── ...
```

## Self-Contained Resource Module Pattern

Each resource module contains its types, defaults, and builder:

```rust
// providers/gcp/disk.rs
use crate::{Client, ProductFilter, Result};
use super::PriceResult;

// ============================================================
// Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskType {
    PdStandard,
    PdSsd,
    PdBalanced,
    PdExtreme,
}

impl DiskType {
    fn resource_group(&self) -> &'static str {
        match self {
            Self::PdStandard => "PDStandard",
            Self::PdSsd => "SSD",
            Self::PdBalanced => "PDBalanced",
            Self::PdExtreme => "PDExtreme",
        }
    }

    fn default_price(&self) -> f64 {
        match self {
            Self::PdStandard => 0.04,
            Self::PdSsd => 0.17,
            Self::PdBalanced => 0.10,
            Self::PdExtreme => 0.125,
        }
    }

    fn unit(&self) -> &'static str {
        "GB-month"
    }
}

impl From<&str> for DiskType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().replace('-', "").as_str() {
            "pdssd" | "ssd" => Self::PdSsd,
            "pdbalanced" | "balanced" => Self::PdBalanced,
            "pdextreme" | "extreme" => Self::PdExtreme,
            _ => Self::PdStandard,
        }
    }
}

// ============================================================
// Builder
// ============================================================

pub struct DiskBuilder<'a> {
    client: &'a Client,
    disk_type: DiskType,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    require_api: bool,
}

impl<'a> DiskBuilder<'a> {
    pub(crate) fn new(client: &'a Client, disk_type: DiskType) -> Self {
        Self {
            client,
            disk_type,
            region: None,
            api_key: None,
            override_default: None,
            require_api: false,
        }
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    pub fn require_api(mut self) -> Self {
        self.require_api = true;
        self
    }

    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    pub async fn fetch(self) -> Result<PriceResult> {
        let default_price = self.override_default.unwrap_or_else(|| self.disk_type.default_price());

        // No API key and not required → return default immediately
        if self.api_key.is_none() && !self.require_api {
            return Ok(PriceResult::from_default(default_price, self.disk_type.unit()));
        }

        // Try API
        let filter = self.build_filter();
        match self.client.query_products_with_key(filter, self.api_key.as_deref()).await {
            Ok(products) if !products.is_empty() => {
                let price = products[0].first_nonzero_price_or(default_price);
                Ok(PriceResult::from_api(price, self.disk_type.unit()))
            }
            _ if !self.require_api => {
                Ok(PriceResult::from_default(default_price, self.disk_type.unit()))
            }
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    fn build_filter(&self) -> ProductFilter {
        ProductFilter::builder()
            .vendor("gcp")
            .service("Compute Engine")
            .region(self.region.as_deref().unwrap_or("us-central1"))
            .product_family("Storage")
            .attribute("resourceGroup", self.disk_type.resource_group())
            .build()
    }
}
```

## PriceResult Type

```rust
#[derive(Debug, Clone)]
pub struct PriceResult {
    pub price: f64,
    pub unit: String,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceSource {
    Api,
    Default,
    UserOverride,
}

impl PriceResult {
    pub(crate) fn from_api(price: f64, unit: &str) -> Self {
        Self { price, unit: unit.to_string(), source: PriceSource::Api }
    }

    pub(crate) fn from_default(price: f64, unit: &str) -> Self {
        Self { price, unit: unit.to_string(), source: PriceSource::Default }
    }

    pub fn is_from_api(&self) -> bool {
        self.source == PriceSource::Api
    }

    pub fn is_from_default(&self) -> bool {
        self.source == PriceSource::Default
    }
}
```

## Integration Test Pattern

Each resource has integration tests that verify the API query works:

```rust
// tests/integration/gcp/disk.rs
use infracost_rs::{Client, providers::gcp::DiskType};

fn client() -> Client {
    Client::from_env().expect("INFRACOST_API_KEY must be set")
}

#[tokio::test]
async fn test_pd_ssd_returns_valid_price() {
    let result = client()
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .api_key(std::env::var("INFRACOST_API_KEY").unwrap())
        .require_api()
        .fetch()
        .await
        .expect("Should fetch pd-ssd price");

    assert!(result.price > 0.0, "Price should be positive");
    assert!(result.price < 1.0, "Price should be reasonable");
    assert_eq!(result.unit, "GB-month");
    assert!(result.is_from_api(), "Should be from API");

    println!("pd-ssd price: ${}/GB-month", result.price);
}

#[tokio::test]
async fn test_default_fallback_without_api_key() {
    let result = Client::anonymous()
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch()
        .await
        .expect("Should return default price");

    assert!(result.is_from_default());
    assert_eq!(result.price, 0.17);
}
```

## Adding New Resources

### Checklist

1. Create `providers/<provider>/<resource>.rs`
2. Define types with `default_price()` and query params
3. Implement builder with standard methods
4. Add to `providers/<provider>/mod.rs`
5. Create `tests/integration/<provider>/<resource>.rs`
6. Run integration tests to verify

### Example: Adding GCP Cloud SQL

**1. Create `providers/gcp/cloud_sql.rs`:**

```rust
#[derive(Debug, Clone, Copy)]
pub enum CloudSqlTier {
    DbN1Standard1,
    DbN1Standard2,
    DbN1Standard4,
}

impl CloudSqlTier {
    fn sku(&self) -> &'static str {
        match self {
            Self::DbN1Standard1 => "db-n1-standard-1",
            Self::DbN1Standard2 => "db-n1-standard-2",
            Self::DbN1Standard4 => "db-n1-standard-4",
        }
    }

    fn default_price(&self) -> f64 {
        match self {
            Self::DbN1Standard1 => 0.0965,
            Self::DbN1Standard2 => 0.1930,
            Self::DbN1Standard4 => 0.3860,
        }
    }

    fn unit(&self) -> &'static str {
        "hours"
    }
}

pub struct CloudSqlBuilder<'a> { /* same pattern */ }
```

**2. Add to `providers/gcp/mod.rs`:**

```rust
mod cloud_sql;
pub use cloud_sql::{CloudSqlTier, CloudSqlBuilder};

impl<'a> GcpProvider<'a> {
    pub fn cloud_sql(self, tier: impl Into<CloudSqlTier>) -> CloudSqlBuilder<'a> {
        CloudSqlBuilder::new(self.client, tier.into())
    }
}
```

**3. Create `tests/integration/gcp/cloud_sql.rs`** with tests.

## Default Prices

### GCP

| Resource | Type | Default Price | Unit |
|----------|------|---------------|------|
| Disk | pd-standard | $0.04 | GB-month |
| Disk | pd-ssd | $0.17 | GB-month |
| Disk | pd-balanced | $0.10 | GB-month |
| Disk | pd-extreme | $0.125 | GB-month |
| Snapshot | - | $0.026 | GB-month |
| Static IP | - | $7.30 | month |
| NAT Gateway | - | $32.00 | month |
| Forwarding Rule | - | $18.00 | month |

### AWS

| Resource | Type | Default Price | Unit |
|----------|------|---------------|------|
| EBS | gp3 | $0.08 | GB-month |
| EBS | gp2 | $0.10 | GB-month |
| EBS | io2 | $0.125 | GB-month |
| EBS | st1 | $0.045 | GB-month |
| Snapshot | - | $0.05 | GB-month |
| Elastic IP | - | $3.65 | month |
| NAT Gateway | - | $32.40 | month |
| ALB | - | $16.20 | month |

### Azure

| Resource | Type | Default Price | Unit |
|----------|------|---------------|------|
| Managed Disk | Premium SSD | $0.132 | GB-month |
| Managed Disk | Standard HDD | $0.04 | GB-month |
| Snapshot | - | $0.05 | GB-month |
| Public IP | - | $3.65 | month |
| NAT Gateway | - | $32.00 | month |
| Load Balancer | - | $18.00 | month |

## Implementation Order

1. Core infrastructure (`providers/mod.rs`, `PriceResult`)
2. GCP provider with disk resource (most tested via arbiter-core)
3. Remaining GCP resources
4. AWS provider and resources
5. Azure provider and resources
6. Integration tests for all
