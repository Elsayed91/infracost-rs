# Azure

```rust
use infracost_rs::Client;

let client = Client::from_env()?; // or Client::anonymous() for defaults
```

## Managed Disks

Fixed sizes with fixed monthly prices (no per-GB pricing).

```rust
use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};

// Premium SSD P10 (128 GB)
let r = client.azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .region("eastus")
    .fetch().await?;
// r.price = 19.71, r.unit = "month"

// fetch_monthly returns the same thing for fixed pricing
let r = client.azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .fetch_monthly().await?;

// String conversion
let dtype = "premium_ssd".parse::<ManagedDiskType>()?;
let size = "P10".parse::<ManagedDiskSize>()?;

// Or string shorthand
let r = client.azure().managed_disk("premium_ssd", "P10").fetch().await?;
```

**Types:** `PremiumSsd` (P-series), `StandardSsd` (E-series), `StandardHdd` (S-series)

**Sizes:** P1-P50, E1-E50, S4-S50 (see Azure docs for capacity mapping)

## Snapshots

```rust
let r = client.azure().snapshot().region("eastus").fetch().await?;
// r.price = 0.05, r.unit = "GB-month"

let r = client.azure().snapshot().size_gb(200).fetch_monthly().await?;
// r.price = 10.0
```

## Public IP

```rust
let r = client.azure().public_ip().region("eastus").fetch().await?;
// r.price = 0.005, r.unit = "hour"

let r = client.azure().public_ip().fetch_monthly().await?;
// r.price = 3.65
```

## NAT Gateway

```rust
let r = client.azure().nat_gateway().region("eastus").fetch().await?;
// r.price = 0.045, r.unit = "hour"

let r = client.azure().nat_gateway()
    .data_processed_gb(1000)
    .fetch_monthly().await?;
// uptime ($0.045 * 730) + data ($0.045 * 1000) = $77.85
```

## Load Balancer Rules

Two-tier hourly pricing: first 5 rules at $0.025/rule/hr, additional rules at $0.01/rule/hr.

```rust
// Per-rule hourly rate (first tier)
let r = client.azure().load_balancer_rules().region("eastus").fetch().await?;
// r.price = 0.025, r.unit = "hour"

// Monthly with rule count
let r = client.azure().load_balancer_rules()
    .rule_count(8)
    .fetch_monthly().await?;
// (5 * $0.025 + 3 * $0.01) * 730 = $113.15
```

## Common Patterns

```rust
// Override default fallback
client.azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .override_default(25.00)
    .fetch().await?;

// Per-request API key
client.azure().snapshot().api_key("ico-xxx").fetch().await?;

// Check source
if result.is_from_api() { /* live price */ }
if result.is_from_default() { /* offline fallback */ }
```
