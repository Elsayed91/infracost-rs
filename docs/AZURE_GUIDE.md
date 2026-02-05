# Azure Pricing Guide

Convenient API for querying Microsoft Azure resource pricing.

## Quick Start

```rust
use infracost_rs::Client;
use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?; // or Client::anonymous()

    let price = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch_price()
        .await?;

    println!("Premium SSD P10 price: ${}/month", price);
    Ok(())
}
```

## Available Resources

### Managed Disks

Query pricing for Azure Managed Disks. Unlike AWS/GCP, Azure uses fixed disk sizes with fixed monthly prices.

```rust
use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};

// Premium SSD (P-series)
let premium_disks = [
    ManagedDiskSize::P1,   // 4 GB
    ManagedDiskSize::P10,  // 128 GB
    ManagedDiskSize::P30,  // 1 TB
    ManagedDiskSize::P50,  // 4 TB
];

for size in premium_disks {
    let result = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, size)
        .region("eastus")
        .fetch()
        .await?;

    println!("{:?}: ${:.2}/{} (source: {:?})",
        size, result.price, result.unit, result.source);
}

// Standard SSD (E-series)
let standard_ssd = [
    ManagedDiskSize::E1,   // 4 GB
    ManagedDiskSize::E10,  // 128 GB
    ManagedDiskSize::E30,  // 1 TB
];

// Standard HDD (S-series)
let standard_hdd = [
    ManagedDiskSize::S4,   // 32 GB
    ManagedDiskSize::S10,  // 128 GB
    ManagedDiskSize::S30,  // 1 TB
];
```

**Disk types:**
- `ManagedDiskType::PremiumSsd` - High-performance SSD (P-series)
- `ManagedDiskType::StandardSsd` - Balanced performance SSD (E-series)
- `ManagedDiskType::StandardHdd` - Cost-effective HDD (S-series)

**String conversion:**
```rust
let disk_type = "premium_ssd".parse::<ManagedDiskType>()?;
let size = "P10".parse::<ManagedDiskSize>()?;

let disk = client.azure().managed_disk(disk_type, size).region("eastus");
```

### Snapshots

Query pricing for Standard HDD disk snapshots.

```rust
let result = client
    .azure()
    .snapshot()
    .region("eastus")
    .fetch()
    .await?;

println!("Snapshot: ${:.4}/{}", result.price, result.unit);
```

### Public IP

Query pricing for Standard static public IPv4 addresses.

```rust
let result = client
    .azure()
    .public_ip()
    .region("eastus")
    .fetch()
    .await?;

println!("Public IP: ${:.4}/{} (~${:.2}/month)",
    result.price, result.unit, result.price * 730.0);
```

## Managed Disk Sizes Reference

### Premium SSD (P-series)

| Size | Capacity | Typical Use Case |
|------|----------|------------------|
| P1   | 4 GB     | Small workloads |
| P2   | 8 GB     | Development |
| P4   | 32 GB    | Small databases |
| P6   | 64 GB    | Testing |
| P10  | 128 GB   | Standard workloads |
| P15  | 256 GB   | Mid-size databases |
| P20  | 512 GB   | Production workloads |
| P30  | 1 TB     | Large databases |
| P40  | 2 TB     | Enterprise |
| P50  | 4 TB     | Large-scale storage |

### Standard SSD (E-series)

| Size | Capacity | Typical Use Case |
|------|----------|------------------|
| E1   | 4 GB     | Dev/test |
| E2   | 8 GB     | Small apps |
| E4   | 32 GB    | Web servers |
| E6   | 64 GB    | Standard apps |
| E10  | 128 GB   | Production apps |
| E15  | 256 GB   | Medium workloads |
| E20  | 512 GB   | Large apps |
| E30  | 1 TB     | Data processing |
| E40  | 2 TB     | Analytics |
| E50  | 4 TB     | Large datasets |

### Standard HDD (S-series)

| Size | Capacity | Typical Use Case |
|------|----------|------------------|
| S4   | 32 GB    | Backups |
| S6   | 64 GB    | Archive |
| S10  | 128 GB   | Infrequent access |
| S15  | 256 GB   | Long-term storage |
| S20  | 512 GB   | Backup storage |
| S30  | 1 TB     | Archival |
| S40  | 2 TB     | Cold storage |
| S50  | 4 TB     | Large backups |

## Usage Patterns

### Without API Key (Defaults)

Anonymous clients return built-in defaults based on current Azure pricing:

```rust
let client = Client::anonymous();

let result = client
    .azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .region("eastus")
    .fetch()
    .await?;

assert!(result.is_from_default());  // true
assert_eq!(result.price, 19.71);    // Built-in default
```

### With API Key (Live Prices)

Clients with API keys fetch live prices from Infracost:

```rust
let client = Client::from_env()?;  // Reads INFRACOST_API_KEY

let result = client
    .azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .region("eastus")
    .fetch()
    .await?;

assert!(result.is_from_api());  // true
```

### Per-Request API Key (Multi-Tenant)

Inject API keys per-request for SaaS applications:

```rust
let client = Client::anonymous();

let result = client
    .azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .region("eastus")
    .api_key(user_api_key)  // Per-request key
    .fetch()
    .await?;
```

### Override Defaults

Provide custom fallback prices:

```rust
let result = client
    .azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .region("eastus")
    .override_default(25.00)  // Custom default
    .fetch()
    .await?;
```

### Require API (No Fallback)

Force API calls to fail if unavailable:

```rust
let result = client
    .azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .region("eastus")
    .require_api()  // Error if API fails
    .fetch()
    .await?;
```

## Price Result

All `fetch()` methods return `PriceResult`:

```rust
pub struct PriceResult {
    pub price: f64,
    pub unit: String,
    pub source: PriceSource,  // Api, Default, or UserOverride
}

impl PriceResult {
    pub fn is_from_api(&self) -> bool;
    pub fn is_from_default(&self) -> bool;
    pub fn is_from_user_override(&self) -> bool;
}
```

**Usage:**
```rust
let result = client
    .azure()
    .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    .region("eastus")
    .fetch()
    .await?;

println!("Price: ${:.2}/{}", result.price, result.unit);
println!("Source: {:?}", result.source);

if result.is_from_default() {
    println!("⚠️  Using default price (no API key)");
}
```

## Regions

All methods support Azure regions:

```rust
let regions = ["eastus", "westus2", "westeurope", "southeastasia"];

for region in regions {
    let price = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region(region)
        .fetch_price()
        .await?;

    println!("{}: ${:.2}/month", region, price);
}
```

## Cost Estimation Example

Calculate total monthly cost for a VM setup:

```rust
use infracost_rs::Client;
use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let region = "eastus";

    // OS disk: Premium SSD P10 (128 GB)
    let os_disk = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region(region)
        .fetch_price()
        .await?;

    // Data disk: Premium SSD P30 (1 TB)
    let data_disk = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P30)
        .region(region)
        .fetch_price()
        .await?;

    // Snapshots: 200 GB
    let snapshot_price = client.azure().snapshot().region(region).fetch_price().await?;
    let snapshot_cost = snapshot_price * 200.0;

    // Public IP: 730 hours/month
    let ip_price = client.azure().public_ip().region(region).fetch_price().await?;
    let ip_cost = ip_price * 730.0;

    let total = os_disk + data_disk + snapshot_cost + ip_cost;

    println!("=== Monthly Cost Estimate ({}) ===", region);
    println!("OS Disk (P10):         ${:.2}", os_disk);
    println!("Data Disk (P30):       ${:.2}", data_disk);
    println!("Snapshots (200 GB):    ${:.2}", snapshot_cost);
    println!("Public IP:             ${:.2}", ip_cost);
    println!("---");
    println!("Total:                 ${:.2}", total);

    Ok(())
}
```

## Complete Example

```rust
use infracost_rs::Client;
use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env().unwrap_or_else(|_| {
        println!("No API key, using defaults");
        Client::anonymous()
    });

    println!("=== Azure Pricing ===\n");

    // Premium SSD pricing
    println!("--- Premium SSD ---");
    for size in [ManagedDiskSize::P1, ManagedDiskSize::P10, ManagedDiskSize::P30] {
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, size)
            .region("eastus")
            .fetch()
            .await?;

        println!("{:?}: ${:.2}/{} ({})",
            size,
            result.price,
            result.unit,
            if result.is_from_api() { "API" } else { "default" }
        );
    }

    // Standard SSD pricing
    println!("\n--- Standard SSD ---");
    for size in [ManagedDiskSize::E1, ManagedDiskSize::E10, ManagedDiskSize::E30] {
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::StandardSsd, size)
            .region("eastus")
            .fetch()
            .await?;

        println!("{:?}: ${:.2}/{}", size, result.price, result.unit);
    }

    // Other resources
    let snapshot = client.azure().snapshot().region("eastus").fetch().await?;
    let public_ip = client.azure().public_ip().region("eastus").fetch().await?;

    println!("\n--- Other Resources ---");
    println!("Snapshot: ${:.4}/{}", snapshot.price, snapshot.unit);
    println!("Public IP: ${:.4}/{}", public_ip.price, public_ip.unit);

    Ok(())
}
```

## Disk Type Comparison

| Feature | Premium SSD | Standard SSD | Standard HDD |
|---------|-------------|--------------|--------------|
| Performance | Highest | Medium | Basic |
| IOPS | Up to 20,000 | Up to 6,000 | Up to 2,000 |
| Throughput | Up to 900 MB/s | Up to 750 MB/s | Up to 500 MB/s |
| Use Case | Production | Dev/Test | Backup/Archive |
| Price | $$$ | $$ | $ |
