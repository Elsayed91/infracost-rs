# GCP Pricing Guide

Convenient API for querying Google Cloud Platform resource pricing.

## Quick Start

```rust
use infracost_rs::Client;
use infracost_rs::providers::gcp::DiskType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?; // or Client::anonymous()

    let price = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch_price()
        .await?;

    println!("PD-SSD price: ${}/GB-month", price);
    Ok(())
}
```

## Available Resources

### Persistent Disk

Query pricing for GCP Persistent Disk storage.

```rust
use infracost_rs::providers::gcp::DiskType;

// All disk types
let types = [
    DiskType::PdStandard,   // Standard persistent disk
    DiskType::PdSsd,        // SSD persistent disk
    DiskType::PdBalanced,   // Balanced persistent disk
    DiskType::PdExtreme,    // Extreme persistent disk
];

for disk_type in types {
    let result = client
        .gcp()
        .disk(disk_type)
        .region("us-central1")
        .fetch()
        .await?;

    println!("{:?}: ${:.4}/{} (source: {:?})",
        disk_type, result.price, result.unit, result.source);
}
```

**String conversion:**
```rust
let disk = client.gcp().disk("pd-ssd").region("us-central1");
```

### Snapshot

Query pricing for disk snapshots.

```rust
let result = client
    .gcp()
    .snapshot()
    .region("us-central1")
    .fetch()
    .await?;

println!("Snapshot: ${:.4}/{}", result.price, result.unit);
```

### Static IP

Query pricing for reserved static IP addresses.

```rust
let result = client
    .gcp()
    .static_ip()
    .region("us-central1")
    .fetch()
    .await?;

println!("Static IP: ${:.4}/{} (~${:.2}/month)",
    result.price, result.unit, result.price * 730.0);
```

### NAT Gateway

Query pricing for Cloud NAT gateway uptime.

```rust
let result = client
    .gcp()
    .nat_gateway()
    .region("us-central1")
    .fetch()
    .await?;

println!("NAT Gateway: ${:.4}/{}", result.price, result.unit);
```

### Forwarding Rule

Query pricing for load balancer forwarding rules.

```rust
let result = client
    .gcp()
    .forwarding_rule()
    .region("us-central1")
    .fetch()
    .await?;

println!("Forwarding Rule: ${:.4}/{}", result.price, result.unit);
```

## Usage Patterns

### Without API Key (Defaults)

Anonymous clients return built-in defaults based on current GCP pricing:

```rust
let client = Client::anonymous();

let result = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
    .fetch()
    .await?;

assert!(result.is_from_default());  // true
assert_eq!(result.price, 0.17);     // Built-in default
```

### With API Key (Live Prices)

Clients with API keys fetch live prices from Infracost:

```rust
let client = Client::from_env()?;  // Reads INFRACOST_API_KEY

let result = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
    .fetch()
    .await?;

assert!(result.is_from_api());  // true
```

### Per-Request API Key (Multi-Tenant)

Inject API keys per-request for SaaS applications:

```rust
let client = Client::anonymous();

let result = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
    .api_key(user_api_key)  // Per-request key
    .fetch()
    .await?;
```

### Override Defaults

Provide custom fallback prices:

```rust
let result = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
    .override_default(0.20)  // Custom default
    .fetch()
    .await?;
```

### Require API (No Fallback)

Force API calls to fail if unavailable:

```rust
let result = client
    .gcp()
    .disk(DiskType::PdSsd)
    .region("us-central1")
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
let result = client.gcp().disk(DiskType::PdSsd).region("us-central1").fetch().await?;

println!("Price: ${:.4}/{}", result.price, result.unit);
println!("Source: {:?}", result.source);

if result.is_from_default() {
    println!("⚠️  Using default price (no API key)");
}
```

## Regions

All methods support GCP regions:

```rust
let regions = ["us-central1", "us-east1", "europe-west1", "asia-southeast1"];

for region in regions {
    let price = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region(region)
        .fetch_price()
        .await?;

    println!("{}: ${:.4}/GB-month", region, price);
}
```

## Complete Example

```rust
use infracost_rs::Client;
use infracost_rs::providers::gcp::DiskType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env().unwrap_or_else(|_| {
        println!("No API key, using defaults");
        Client::anonymous()
    });

    println!("=== GCP Pricing ===\n");

    // Disk pricing
    for disk_type in [DiskType::PdStandard, DiskType::PdSsd, DiskType::PdBalanced] {
        let result = client
            .gcp()
            .disk(disk_type)
            .region("us-central1")
            .fetch()
            .await?;

        println!("{:?}: ${:.4}/{} ({})",
            disk_type,
            result.price,
            result.unit,
            if result.is_from_api() { "API" } else { "default" }
        );
    }

    // Network pricing
    let static_ip = client.gcp().static_ip().region("us-central1").fetch().await?;
    let nat = client.gcp().nat_gateway().region("us-central1").fetch().await?;

    println!("\nStatic IP: ${:.4}/{}", static_ip.price, static_ip.unit);
    println!("NAT Gateway: ${:.4}/{}", nat.price, nat.unit);

    Ok(())
}
```
