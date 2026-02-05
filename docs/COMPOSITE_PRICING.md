# Implementing Composite Pricing

Guide for adding total monthly cost calculation to resources with multiple pricing components.

## Overview

Many cloud resources have multiple pricing components:
- **EBS gp3**: Storage + IOPS + Throughput
- **NAT Gateway**: Hourly + Data processing
- **ALB**: Hourly + LCU charges
- **io2**: Storage + Tiered IOPS

This guide explains how to add `fetch_monthly()` support to calculate accurate total costs.

## API Key

An Infracost API key is required for research and testing.

**If you don't have an API key:**
- Ask the user/maintainer for the test API key
- Or obtain one from https://www.infracost.io/

```bash
export INFRACOST_API_KEY="<your-api-key>"
```

## Step 1: Research Pricing Components

### 1.1 Discover Available Products

Use `irs` to explore what pricing components exist for a resource:

```bash
# Find all products for a resource type
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'volumeApiName=gp3' \
  -a 'servicecode=AmazonEC2' \
  --limit 10 -f json | jq '.[] | {product_family, usagetype: (.attributes[] | select(.key == "usagetype") | .value), usd: .prices[0].usd, unit: .prices[0].unit}'
```

### 1.2 Identify Unique Filters

For each component, find the **minimal set of attributes** that return exactly ONE product:

```bash
# Test a specific filter combination
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'usagetype=EBS:VolumeP-IOPS.gp3' \
  -a 'servicecode=AmazonEC2' \
  --limit 5 -f json | jq 'length'
```

**Target: Exactly 1 result per component.**

### 1.3 Document Findings

Create a table of discovered components:

| Component | Filter | Default Price | Unit |
|-----------|--------|---------------|------|
| Storage | `usagetype=EBS:VolumeUsage.gp3` | $0.08 | GB-Mo |
| IOPS | `usagetype=EBS:VolumeP-IOPS.gp3` | $0.005 | IOPS-Mo |
| Throughput | `usagetype=EBS:VolumeP-Throughput.gp3` | $0.04 | MiBps-Mo |

## Step 2: Validate with External Sources

**IMPORTANT:** Always cross-reference Infracost data with official pricing pages.

### 2.1 Check Official Pricing

Search for official pricing documentation:
- AWS: `site:aws.amazon.com pricing <service>`
- Azure: `site:azure.microsoft.com pricing <service>`
- GCP: `site:cloud.google.com pricing <service>`

### 2.2 Verify Baseline/Free Tier Rules

Many resources include baseline allocations. Research these carefully:

| Resource | Baseline | Source |
|----------|----------|--------|
| gp3 | 3000 IOPS, 125 MiBps included | AWS EBS pricing page |
| io2 | No baseline, all IOPS billed | AWS EBS pricing page |
| gp2 | IOPS tied to size (3 IOPS/GB) | AWS EBS pricing page |

**Key question:** Is the "free tier" per-resource or account-wide?
- Per-resource baseline (like gp3): Include in calculation
- Account-wide free tier: Let user handle externally

### 2.3 Check for Tiered Pricing

Some resources have tiered pricing (e.g., io2 IOPS):

```bash
# Find tiered pricing
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'volumeApiName=io2' \
  -a 'servicecode=AmazonEC2' \
  --limit 10 -f json | jq '.[] | select(.product_family == "System Operation") | {usagetype: (.attributes[] | select(.key == "usagetype") | .value), usd: .prices[0].usd}'
```

Example io2 tiers:
- `EBS:VolumeP-IOPS.io2` → $0.065 (1-32,000 IOPS)
- `EBS:VolumeP-IOPS.io2.tier2` → $0.0455 (32,001-64,000)
- `EBS:VolumeP-IOPS.io2.tier3` → $0.03185 (64,001+)

## Step 3: Implementation Pattern

### 3.1 Add Builder Fields

```rust
pub struct ResourceBuilder<'a> {
    // ... existing fields ...

    // Volume specs for monthly cost calculation
    size_gb: Option<u64>,
    iops: Option<u64>,
    throughput_mibps: Option<u64>,
}
```

### 3.2 Add Builder Methods

```rust
/// Set the volume size in GB (required for `fetch_monthly`).
pub fn size_gb(mut self, size: u64) -> Self {
    self.size_gb = Some(size);
    self
}

/// Set provisioned IOPS.
/// For gp3: baseline 3000 IOPS is included; you only pay for IOPS above that.
pub fn iops(mut self, iops: u64) -> Self {
    self.iops = Some(iops);
    self
}
```

### 3.3 Add Type Metadata

```rust
impl ResourceType {
    /// Get baseline IOPS (included in storage price)
    fn baseline_iops(&self) -> u64 {
        match self {
            Self::Gp3 => 3000,
            _ => 0,
        }
    }

    /// Whether this type supports provisioned IOPS
    fn supports_iops(&self) -> bool {
        matches!(self, Self::Gp3 | Self::Io2)
    }

    /// Get default IOPS price
    fn default_iops_price(&self) -> Option<f64> {
        match self {
            Self::Gp3 => Some(0.005),
            Self::Io2 => Some(0.065),
            _ => None,
        }
    }
}
```

### 3.4 Add Component Fetch Methods

```rust
async fn fetch_iops_price(&self, region: &str, volume_type: &str) -> Result<Option<f64>> {
    if !self.resource_type.supports_iops() {
        return Ok(None);
    }

    let default = self.resource_type.default_iops_price();

    if !self.client.has_api_key() && self.api_key.is_none() && !self.require_api {
        return Ok(default);
    }

    let filter = ProductFilter::builder()
        .vendor("aws")
        .region(region)
        .attribute("usagetype", format!("EBS:VolumeP-IOPS.{}", volume_type))
        .attribute("servicecode", "AmazonEC2")
        .build();

    match self.client.query_products_with_key(filter, self.api_key.as_deref()).await {
        Ok(products) if !products.is_empty() => {
            Ok(Some(products[0].first_nonzero_price_or(default.unwrap_or(0.0))))
        }
        _ if !self.require_api => Ok(default),
        Err(e) => Err(e),
        Ok(_) => Ok(default),
    }
}
```

### 3.5 Add fetch_monthly Method

```rust
pub async fn fetch_monthly(self) -> Result<PriceResult> {
    let size_gb = self.size_gb
        .ok_or_else(|| crate::Error::validation("size_gb is required for fetch_monthly"))?;

    let region = self.region.as_deref().unwrap_or("us-east-1");

    // Query all price components
    let storage_price = self.fetch_storage_price(region).await?;
    let iops_price = self.fetch_iops_price(region).await?;
    let throughput_price = self.fetch_throughput_price(region).await?;

    // Calculate costs with baseline subtraction
    let storage_cost = size_gb as f64 * storage_price;

    let iops_cost = if self.resource_type.supports_iops() {
        let provisioned = self.iops.unwrap_or(self.resource_type.baseline_iops());
        let billable = provisioned.saturating_sub(self.resource_type.baseline_iops());
        billable as f64 * iops_price.unwrap_or(0.0)
    } else {
        0.0
    };

    let total = storage_cost + iops_cost + throughput_cost;

    Ok(PriceResult {
        price: total,
        unit: "month".to_string(),
        source: if self.client.has_api_key() { PriceSource::Api } else { PriceSource::Default },
    })
}
```

## Step 4: Unit Conversions

Watch for unit mismatches between API and user input:

| API Unit | User Input | Conversion |
|----------|------------|------------|
| GiBps-mo | MiBps | `api_price / 1024` |
| GB-Mo | GB | None |
| IOPS-Mo | IOPS | None |

Example:
```rust
// API returns $40.96/GiBps-mo, user provides MiBps
let price_gibps = products[0].first_nonzero_price_or(default * 1024.0);
Ok(Some(price_gibps / 1024.0))  // Convert to per-MiBps
```

## Step 5: Testing Requirements

### 5.1 Unit Tests (No API Key)

Test calculation logic with default prices:

```rust
#[tokio::test]
async fn test_gp3_fetch_monthly_full_spec() {
    // 500 GB gp3 with 6000 IOPS and 250 MiBps throughput
    // Cost = (500 * $0.08) + (3000 * $0.005) + (125 * $0.04) = $60/month
    let client = Client::anonymous();
    let result = client.aws().ebs(EbsType::Gp3)
        .size_gb(500)
        .iops(6000)
        .throughput_mibps(250)
        .fetch_monthly().await.unwrap();

    assert_eq!(result.price, 60.0);
    assert_eq!(result.unit, "month");
}

#[tokio::test]
async fn test_baseline_no_extra_charge() {
    // Exactly baseline values = no extra IOPS/throughput cost
    let client = Client::anonymous();
    let result = client.aws().ebs(EbsType::Gp3)
        .size_gb(100)
        .iops(3000)      // baseline
        .throughput_mibps(125)  // baseline
        .fetch_monthly().await.unwrap();

    assert_eq!(result.price, 8.0);  // storage only
}

#[tokio::test]
async fn test_fetch_monthly_requires_size() {
    let client = Client::anonymous();
    let result = client.aws().ebs(EbsType::Gp3)
        .iops(6000)  // no size_gb
        .fetch_monthly().await;

    assert!(result.is_err());
}
```

### 5.2 Integration Tests (With API Key)

Verify live API queries return expected prices:

```rust
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_gp3_monthly_with_api() {
    let client = Client::from_env().unwrap();
    let result = client.aws().ebs(EbsType::Gp3)
        .size_gb(500)
        .iops(6000)
        .throughput_mibps(250)
        .fetch_monthly().await.unwrap();

    assert!(result.is_from_api());
    // Allow some variance for price changes
    assert!(result.price > 50.0 && result.price < 70.0);
}
```

Run integration tests:
```bash
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo test --test integration -- --ignored
```

### 5.3 Example Verification

Update the relevant example file and verify output:

```bash
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --example aws_pricing
```

## Step 6: Checklist

Before submitting changes:

- [ ] **Research**
  - [ ] Used `irs` to discover all pricing components
  - [ ] Documented filter for each component (exactly 1 result each)
  - [ ] Verified prices match official documentation
  - [ ] Identified baseline/free tier rules
  - [ ] Checked for tiered pricing

- [ ] **Implementation**
  - [ ] Added builder fields for user inputs
  - [ ] Added builder methods with documentation
  - [ ] Added type metadata (baseline, supports_x, default_price)
  - [ ] Added fetch methods for each component
  - [ ] Added `fetch_monthly()` with correct calculation
  - [ ] Handled unit conversions

- [ ] **Testing**
  - [ ] Unit tests for calculation logic (no API key)
  - [ ] Unit test for baseline behavior
  - [ ] Unit test for validation errors
  - [ ] Integration test with live API
  - [ ] Updated example file

- [ ] **Backward Compatibility**
  - [ ] Existing `fetch()` behavior unchanged
  - [ ] New methods are additive only

## Resources Already Needing This

| Resource | Components | Priority |
|----------|------------|----------|
| AWS io2 | Storage + Tiered IOPS | High |
| AWS NAT Gateway | Hourly + Data processing | Medium |
| AWS ALB | Hourly + LCU | Medium |
| GCP NAT Gateway | Hourly + Data processing | Medium |
| GCP Forwarding Rule | Hourly + Data processing | Medium |

## Common Pitfalls

| Issue | Solution |
|-------|----------|
| Multiple products returned | Add more attribute filters |
| Wrong price (reserved vs on-demand) | Filter by `purchase_option` |
| Unit mismatch | Convert API units to user-friendly units |
| Missing baseline logic | Research official docs for included allocations |
| Tiered pricing ignored | Query all tier products, implement tier logic |
