# Azure

```rust
use infracost_rs::Client;
use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};

let client = Client::from_env()?; // or Client::anonymous() for defaults
```

## Managed Disks

Azure disks have fixed sizes with fixed monthly prices (no per-GB pricing).

```rust
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
// r.price = 19.71

// String conversion
let dtype = "premium_ssd".parse::<ManagedDiskType>()?;
let size = "P10".parse::<ManagedDiskSize>()?;
```

**Types:** PremiumSsd (P-series), StandardSsd (E-series), StandardHdd (S-series)

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
