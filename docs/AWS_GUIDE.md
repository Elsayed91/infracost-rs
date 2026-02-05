# AWS Pricing Guide

Convenient API for querying Amazon Web Services resource pricing.

## Quick Start

```rust
use infracost_rs::Client;
use infracost_rs::providers::aws::EbsType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?; // or Client::anonymous()

    let price = client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .fetch_price()
        .await?;

    println!("GP3 price: ${}/GB-month", price);
    Ok(())
}
```

## Available Resources

### EBS Volumes

Query pricing for Elastic Block Store volumes.

```rust
use infracost_rs::providers::aws::EbsType;

// All EBS types
let types = [
    EbsType::Gp3,   // General Purpose SSD (latest)
    EbsType::Gp2,   // General Purpose SSD
    EbsType::Io2,   // Provisioned IOPS SSD
    EbsType::St1,   // Throughput Optimized HDD
    EbsType::Sc1,   // Cold HDD
];

for ebs_type in types {
    let result = client
        .aws()
        .ebs(ebs_type)
        .region("us-east-1")
        .fetch()
        .await?;

    println!("{:?}: ${:.4}/{} (source: {:?})",
        ebs_type, result.price, result.unit, result.source);
}
```

**String conversion:**
```rust
let ebs = client.aws().ebs("gp3").region("us-east-1");
```

### EBS Snapshots

Query pricing for EBS volume snapshots.

```rust
let result = client
    .aws()
    .snapshot()
    .region("us-east-1")
    .fetch()
    .await?;

println!("Snapshot: ${:.4}/{}", result.price, result.unit);
```

### Elastic IP

Query pricing for idle Elastic IP addresses.

```rust
let result = client
    .aws()
    .elastic_ip()
    .region("us-east-1")
    .fetch()
    .await?;

println!("Elastic IP: ${:.4}/{} (~${:.2}/month)",
    result.price, result.unit, result.price * 730.0);
```

### NAT Gateway

Query pricing for NAT Gateway uptime.

```rust
let result = client
    .aws()
    .nat_gateway()
    .region("us-east-1")
    .fetch()
    .await?;

println!("NAT Gateway: ${:.4}/{}", result.price, result.unit);
```

### Application Load Balancer

Query pricing for ALB uptime.

```rust
let result = client
    .aws()
    .alb()
    .region("us-east-1")
    .fetch()
    .await?;

println!("ALB: ${:.4}/{}", result.price, result.unit);
```

## Usage Patterns

### Without API Key (Defaults)

Anonymous clients return built-in defaults based on current AWS pricing:

```rust
let client = Client::anonymous();

let result = client
    .aws()
    .ebs(EbsType::Gp3)
    .region("us-east-1")
    .fetch()
    .await?;

assert!(result.is_from_default());  // true
assert_eq!(result.price, 0.08);     // Built-in default
```

### With API Key (Live Prices)

Clients with API keys fetch live prices from Infracost:

```rust
let client = Client::from_env()?;  // Reads INFRACOST_API_KEY

let result = client
    .aws()
    .ebs(EbsType::Gp3)
    .region("us-east-1")
    .fetch()
    .await?;

assert!(result.is_from_api());  // true
```

### Per-Request API Key (Multi-Tenant)

Inject API keys per-request for SaaS applications:

```rust
let client = Client::anonymous();

let result = client
    .aws()
    .ebs(EbsType::Gp3)
    .region("us-east-1")
    .api_key(user_api_key)  // Per-request key
    .fetch()
    .await?;
```

### Override Defaults

Provide custom fallback prices:

```rust
let result = client
    .aws()
    .ebs(EbsType::Gp3)
    .region("us-east-1")
    .override_default(0.10)  // Custom default
    .fetch()
    .await?;
```

### Require API (No Fallback)

Force API calls to fail if unavailable:

```rust
let result = client
    .aws()
    .ebs(EbsType::Gp3)
    .region("us-east-1")
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
let result = client.aws().ebs(EbsType::Gp3).region("us-east-1").fetch().await?;

println!("Price: ${:.4}/{}", result.price, result.unit);
println!("Source: {:?}", result.source);

if result.is_from_default() {
    println!("⚠️  Using default price (no API key)");
}
```

## Regions

All methods support AWS regions:

```rust
let regions = ["us-east-1", "us-west-2", "eu-west-1", "ap-southeast-1"];

for region in regions {
    let price = client
        .aws()
        .ebs(EbsType::Gp3)
        .region(region)
        .fetch_price()
        .await?;

    println!("{}: ${:.4}/GB-month", region, price);
}
```

## Cost Estimation Example

Calculate total monthly cost for an infrastructure setup:

```rust
use infracost_rs::Client;
use infracost_rs::providers::aws::EbsType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let region = "us-east-1";

    // EBS volumes: 500 GB GP3
    let ebs_price = client.aws().ebs(EbsType::Gp3).region(region).fetch_price().await?;
    let ebs_cost = ebs_price * 500.0;

    // Snapshots: 200 GB
    let snapshot_price = client.aws().snapshot().region(region).fetch_price().await?;
    let snapshot_cost = snapshot_price * 200.0;

    // NAT Gateway: 730 hours/month
    let nat_price = client.aws().nat_gateway().region(region).fetch_price().await?;
    let nat_cost = nat_price * 730.0;

    // ALB: 730 hours/month
    let alb_price = client.aws().alb().region(region).fetch_price().await?;
    let alb_cost = alb_price * 730.0;

    let total = ebs_cost + snapshot_cost + nat_cost + alb_cost;

    println!("=== Monthly Cost Estimate ({}) ===", region);
    println!("EBS (500 GB GP3):      ${:.2}", ebs_cost);
    println!("Snapshots (200 GB):    ${:.2}", snapshot_cost);
    println!("NAT Gateway:           ${:.2}", nat_cost);
    println!("Load Balancer:         ${:.2}", alb_cost);
    println!("---");
    println!("Total:                 ${:.2}", total);

    Ok(())
}
```

## Complete Example

```rust
use infracost_rs::Client;
use infracost_rs::providers::aws::EbsType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env().unwrap_or_else(|_| {
        println!("No API key, using defaults");
        Client::anonymous()
    });

    println!("=== AWS Pricing ===\n");

    // EBS volume pricing
    for ebs_type in [EbsType::Gp3, EbsType::Gp2, EbsType::Io2] {
        let result = client
            .aws()
            .ebs(ebs_type)
            .region("us-east-1")
            .fetch()
            .await?;

        println!("{:?}: ${:.4}/{} ({})",
            ebs_type,
            result.price,
            result.unit,
            if result.is_from_api() { "API" } else { "default" }
        );
    }

    // Network resources
    let eip = client.aws().elastic_ip().region("us-east-1").fetch().await?;
    let nat = client.aws().nat_gateway().region("us-east-1").fetch().await?;
    let alb = client.aws().alb().region("us-east-1").fetch().await?;

    println!("\nElastic IP: ${:.4}/{}", eip.price, eip.unit);
    println!("NAT Gateway: ${:.4}/{}", nat.price, nat.unit);
    println!("ALB: ${:.4}/{}", alb.price, alb.unit);

    Ok(())
}
```
