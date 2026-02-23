# GCP

```rust
use infracost_rs::Client;
use infracost_rs::providers::gcp::DiskType;

let client = Client::from_env()?; // or Client::anonymous() for defaults
```

## Persistent Disk

```rust
// Unit price
let r = client.gcp().disk(DiskType::PdSsd).region("us-central1").fetch().await?;
// r.price = 0.17, r.unit = "GiB-month"

// Monthly cost (500 GB)
let r = client.gcp().disk(DiskType::PdSsd)
    .region("us-central1")
    .size_gb(500)
    .fetch_monthly().await?;
// r.price = 85.0

// pd-extreme with IOPS
let r = client.gcp().disk(DiskType::PdExtreme)
    .size_gb(500)
    .iops(15000)
    .fetch_monthly().await?;
// storage ($0.125 * 500) + IOPS ($0.065 * 15000) = $1037.50

// Regional disk (2x price)
let r = client.gcp().disk(DiskType::PdSsd)
    .size_gb(500)
    .regional(true)
    .fetch_monthly().await?;
// r.price = 170.0

// String shorthand
let r = client.gcp().disk("pd-ssd").region("us-central1").fetch().await?;

// Types: PdStandard, PdSsd, PdBalanced, PdExtreme
```

## Snapshots

```rust
use infracost_rs::providers::gcp::SnapshotType;

// Standard snapshot
let r = client.gcp().snapshot(SnapshotType::Standard).region("us-central1").fetch().await?;
// r.price = 0.05, r.unit = "GiB-month"

let r = client.gcp().snapshot(SnapshotType::Standard).size_gb(100).fetch_monthly().await?;
// r.price = 5.0

// Archive snapshot (cheaper storage, optional retrieval cost)
let r = client.gcp().snapshot(SnapshotType::Archive).region("us-central1").fetch().await?;
// r.price = 0.019, r.unit = "GiB-month"

let r = client.gcp().snapshot(SnapshotType::Archive)
    .size_gb(500)
    .retrieval_size_gb(100)  // optional one-time retrieval
    .fetch_monthly().await?;
// storage ($0.019 * 500) + retrieval ($0.019 * 100) = $11.40

// String shorthand
let r = client.gcp().snapshot("archive").region("us-central1").fetch().await?;
```

## Static IP

```rust
let r = client.gcp().static_ip().region("us-central1").fetch().await?;
// r.price = 0.01, r.unit = "hour"

let r = client.gcp().static_ip().fetch_monthly().await?;
// r.price = 7.30
```

## NAT Gateway

```rust
let r = client.gcp().nat_gateway().region("us-central1").fetch().await?;
// r.price = 0.0014, r.unit = "hour"

let r = client.gcp().nat_gateway()
    .data_processed_gb(1000)
    .fetch_monthly().await?;
// uptime ($0.0014 * 730) + data ($0.045 * 1000) = $46.022
```

## Forwarding Rule (Load Balancer)

```rust
let r = client.gcp().forwarding_rule().region("us-central1").fetch().await?;
// r.price = 0.025, r.unit = "hour"

let r = client.gcp().forwarding_rule()
    .data_processed_gb(1000)
    .fetch_monthly().await?;
// uptime ($0.025 * 730) + data ($0.008 * 1000) = $26.25
```

## Common Patterns

```rust
// Override default fallback
client.gcp().disk("pd-ssd").override_default(0.20).fetch().await?;

// Per-request API key
client.gcp().disk("pd-ssd").api_key("ico-xxx").fetch().await?;

// Check source
if result.is_from_api() { /* live price */ }
if result.is_from_default() { /* offline fallback */ }
```
