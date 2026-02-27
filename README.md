# infracost-rs

Rust client for the [Infracost](https://www.infracost.io/) Cloud Pricing API. Typed builders for AWS, GCP, and Azure resources with async/blocking support, offline defaults, and optional caching.

## Extending with Agents

Accurate pricing requires finding the right API query attributes and filters for each resource — a non-trivial process. To streamline this, run `/add-resource` in Claude Code to launch an agentic pipeline: **research** (IRS CLI exploration across regions) → **implement** (YAML manifest + Rust builder) → **review** (spec compliance) → **test** (integration tests across 7+ regions).

## Install

```toml
[dependencies]
infracost-rs = "0.4"

# With blocking API
infracost-rs = { version = "0.4", features = ["blocking"] }

# With caching
infracost-rs = { version = "0.4", features = ["cache-memory"] }
```

```bash
# CLI (binary name: irs)
cargo install infracost-rs --features cli
```

## What's in the box

**23 resources** across 3 cloud providers with typed builders, offline default prices, and monthly cost calculation.

| AWS (7) | GCP (9) | Azure (5) |
|---------|---------|-----------|
| EC2 Instance | Compute Instance | Managed Disk |
| RDS | Cloud SQL | Snapshot |
| EBS | Persistent Disk (+ Hyperdisk) | Public IP |
| Snapshot | BigQuery Storage | NAT Gateway |
| Elastic IP | Snapshot (Standard + Archive) | Load Balancer Rules |
| NAT Gateway | Static IP | |
| ALB | NAT Gateway | |
| | Forwarding Rule | |
| | Backend Service (LB) | |

Every builder returns a `PriceResult` with `.price`, `.unit`, and `.source` (API or default fallback).

## Client

```rust
use infracost_rs::Client;

let client = Client::from_env()?;    // reads INFRACOST_API_KEY
let client = Client::new("ico-xxx"); // explicit key
let client = Client::anonymous();    // must provide key per-request
```

Builder for advanced config:

```rust
use infracost_rs::Client;
use std::time::Duration;

let client = Client::builder()
    .api_key("ico-xxx")
    .endpoint("https://pricing.api.infracost.io/graphql")
    .timeout(Duration::from_secs(30))
    .error_on_fallback(true) // fail instead of returning defaults
    .build()?;
```

## Provider API (typed builders)

The main way to use this library. Each resource has a builder that handles query construction, API filtering, and cost calculation internally.

```rust
// Unit price
let r = client.aws().ebs("gp3").region("us-east-1").fetch().await?;
// r.price = 0.08, r.unit = "GB-month"

// Monthly cost
let r = client.aws().ebs("gp3").region("us-east-1")
    .size_gb(500).iops(6000).throughput_mibps(250)
    .fetch_monthly().await?;
// r.price = 60.0

// GCP compute
let r = client.gcp().compute_instance()
    .machine_type("n2-standard-4")
    .region("us-central1")
    .fetch_monthly().await?;

// Azure disk
let r = client.azure()
    .managed_disk("premium_ssd", "P10")
    .region("eastus")
    .fetch().await?;
```

All builders share: `.region()`, `.api_key()`, `.override_default()`, `.fetch()`, `.fetch_price()`, `.fetch_monthly()`.

Per-provider details: [AWS](docs/usage/aws.md) | [GCP](docs/usage/gcp.md) | [Azure](docs/usage/azure.md)

## Raw Query API

For anything not covered by typed builders, or if you want direct API access:

```rust
let products = client
    .products()
    .vendor("gcp")
    .service("Compute Engine")
    .region("us-central1")
    .attribute("description", "SSD backed PD Capacity")
    .fetch()
    .await?;

let price = products[0].price_f64()?;
```

## Blocking API

Same interface, no async. Requires `blocking` feature.

```rust
use infracost_rs::blocking::Client;

let client = Client::from_env()?;

// Typed builders
let r = client.aws().ebs("gp3").region("us-east-1").fetch()?;
let r = client.gcp().compute_instance().machine_type("n2-standard-4").fetch_monthly()?;

// Raw queries
let products = client.products().vendor("gcp").service("Compute Engine").fetch()?;
```

## Caching

Four backends available. All implement `PriceCache` trait (default TTL: 24h).

```toml
infracost-rs = { version = "0.4", features = ["cache-memory"] }
# or: cache-redis, cache-sqlite, cache-postgres
```

```rust
use infracost_rs::{Client, MemoryCache};
use std::time::Duration;

let client = Client::builder()
    .api_key("ico-xxx")
    .with_cache(MemoryCache::new())
    .cache_ttl(Duration::from_secs(3600))
    .build()?;
```

## Offline Defaults

Every resource has a baked-in default price. If the API is unreachable or no key is provided, builders return the default with `source: PriceSource::Default`. Useful for testing and CI.

```rust
let client = Client::anonymous();
let r = client.aws().ebs("gp3").fetch().await?;
assert!(r.is_from_default());
assert_eq!(r.price, 0.08);

// Force failure instead of fallback
let client = Client::builder()
    .api_key("ico-xxx")
    .error_on_fallback(true)
    .build()?;
```

## Testing with Mocks

```rust
use infracost_rs::mock::MockClient;
use infracost_rs::PricingClient;

let client = MockClient::from_prices(&[
    ("gcp", "Compute Engine", "us-central1", "pd-ssd", 0.170, "GB-month"),
]);

let products = client
    .query_products(ProductFilter::builder().vendor("gcp").build())
    .await?;
```

## CLI

```bash
export INFRACOST_API_KEY=ico-xxx

# Query GCP VM pricing
irs query -v gcp -s "Compute Engine" -r us-central1 -a 'machineType=n2-standard-32'

# Spot/preemptible pricing
irs query -v gcp -s "Compute Engine" -r us-central1 -a 'machineType=n2-standard-32' -p preemptible

# Quiet mode (just the price)
irs query -v gcp -s "Compute Engine" -r us-central1 -a 'machineType=n2-standard-32' -q

# JSON output
irs query --vendor aws --service AmazonEC2 --region us-east-1 --format json

# List services / regions
irs services --vendor gcp
irs regions --vendor gcp --service "Compute Engine"
```

## Features

| Feature | What it does |
|---------|-------------|
| (default) | Async client + typed builders |
| `blocking` | Synchronous wrapper API |
| `cli` | `irs` CLI binary |
| `cache-memory` | In-memory cache (moka) |
| `cache-redis` | Redis cache |
| `cache-sqlite` | SQLite cache |
| `cache-postgres` | PostgreSQL cache |

## License

MIT OR Apache-2.0
