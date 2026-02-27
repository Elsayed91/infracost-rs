# GCP

```rust
use infracost_rs::Client;

let client = Client::from_env()?; // or Client::anonymous() for defaults
```

## Compute Instance

Parses any machine type string. Supports predefined (`n2-standard-4`), custom (`n2-custom-4-8192`), and zone-prefixed (`zones/us-central1-a/machineTypes/n2-standard-4`). All 23+ GCP machine families work.

```rust
use infracost_rs::providers::gcp::PurchaseOption;

// Hourly CPU rate
let r = client.gcp().compute_instance()
    .machine_type("n2-standard-4")
    .region("us-central1")
    .fetch().await?;

// Monthly cost (parses cores + memory from machine type)
let r = client.gcp().compute_instance()
    .machine_type("n2-standard-4")
    .fetch_monthly().await?;

// Custom specs (no machine type string needed)
let r = client.gcp().compute_instance()
    .machine_family("n2")
    .cpu_cores(4)
    .memory_gib(16)
    .fetch_monthly().await?;

// Spot/preemptible
let r = client.gcp().compute_instance()
    .machine_type("n2-standard-4")
    .purchase_option(PurchaseOption::Preemptible)
    .fetch_monthly().await?;

// Committed use discounts
let r = client.gcp().compute_instance()
    .machine_type("n2-standard-4")
    .purchase_option(PurchaseOption::Commit1Yr)
    .fetch_monthly().await?;
```

**Purchase options:** `OnDemand` (default), `Preemptible`, `Commit1Yr`, `Commit3Yr`

**Families:** N1, N2, N2D, N4, N4A, N4D, E2, T2A, T2D, C2, C2D, C3, C3D, C4, C4A, C4D, M1, M2, M3, M4, H3, H4D, A2, A3, G2, and more.

## Cloud SQL

Custom instances with CPU + RAM + storage + backup components. Handles MySQL, PostgreSQL, and SQL Server with Zonal/Regional availability.

```rust
use infracost_rs::providers::gcp::{CloudSqlEngine, CloudSqlAvailability};

// CPU hourly rate
let r = client.gcp().cloud_sql()
    .engine(CloudSqlEngine::PostgreSql)
    .region("us-central1")
    .fetch().await?;

// Full monthly cost
let r = client.gcp().cloud_sql()
    .engine(CloudSqlEngine::PostgreSql)
    .availability(CloudSqlAvailability::Regional) // HA, 2x price
    .cpu_count(4)
    .memory_gb(16)
    .storage_gb(100)
    .backup_storage_gb(50)
    .fetch_monthly().await?;

// String shorthand
let r = client.gcp().cloud_sql()
    .engine("postgres")
    .availability("ha")
    .cpu_count(2)
    .memory_gb(8)
    .fetch_monthly().await?;
```

**Engines:** `MySql`, `PostgreSql`, `SqlServer`

**Availability:** `Zonal` (default), `Regional` (HA, 2x price)

## BigQuery Storage

Four storage components across logical and physical billing models. Each dataset uses one model.

```rust
// Active logical storage rate
let r = client.gcp().bigquery_storage().region("us-central1").fetch().await?;

// Logical billing model
let r = client.gcp().bigquery_storage()
    .active_logical_storage_gb(500)
    .long_term_logical_storage_gb(200)
    .fetch_monthly().await?;

// Physical billing model
let r = client.gcp().bigquery_storage()
    .active_physical_storage_gb(100)
    .long_term_physical_storage_gb(500)
    .fetch_monthly().await?;
```

## Persistent Disk

Supports standard PD types and Hyperdisk variants.

```rust
use infracost_rs::providers::gcp::DiskType;

// Unit price
let r = client.gcp().disk(DiskType::PdSsd).region("us-central1").fetch().await?;
// r.price = 0.17, r.unit = "GiB-month"

// Monthly cost (500 GB)
let r = client.gcp().disk(DiskType::PdSsd)
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

// Hyperdisk
let r = client.gcp().disk(DiskType::HyperdiskBalanced)
    .size_gb(1000)
    .iops(3000)
    .throughput(140)
    .fetch_monthly().await?;

// String shorthand
let r = client.gcp().disk("pd-ssd").region("us-central1").fetch().await?;
```

**Types:** `PdStandard`, `PdSsd`, `PdBalanced`, `PdExtreme`, `HyperdiskBalanced`, `HyperdiskExtreme`, `HyperdiskThroughput`, `HyperdiskMl`

## Snapshots

Standard and archive snapshots.

```rust
use infracost_rs::providers::gcp::SnapshotType;

// Standard snapshot
let r = client.gcp().snapshot(SnapshotType::Standard).region("us-central1").fetch().await?;
// r.price = 0.05, r.unit = "GiB-month"

let r = client.gcp().snapshot(SnapshotType::Standard).size_gb(100).fetch_monthly().await?;

// Archive snapshot (cheaper storage, optional retrieval cost)
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

## Forwarding Rule

```rust
let r = client.gcp().forwarding_rule().region("us-central1").fetch().await?;
// r.price = 0.025, r.unit = "hour"

let r = client.gcp().forwarding_rule()
    .data_processed_gb(1000)
    .fetch_monthly().await?;
// uptime ($0.025 * 730) + data ($0.008 * 1000) = $26.25
```

## Backend Service (Load Balancer)

Full LB pricing: data processing + optional forwarding rules. Premium (global) and Standard (regional) tiers.

```rust
use infracost_rs::providers::gcp::BackendServiceTier;

// Data processing rate
let r = client.gcp().backend_service(BackendServiceTier::Premium)
    .region("us-central1")
    .fetch().await?;

// Monthly with data processing
let r = client.gcp().backend_service(BackendServiceTier::Premium)
    .data_processed_gb(1000)
    .fetch_monthly().await?;

// Include forwarding rule cost
let r = client.gcp().backend_service("premium")
    .forwarding_rules(1)
    .data_processed_gb(1000)
    .fetch_monthly().await?;
// ($0.025 * 730) + ($0.008 * 1000) = $26.25/month
```

**Tiers:** `Premium` (global), `Standard` (regional)

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
