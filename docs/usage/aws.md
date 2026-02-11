# AWS

```rust
use infracost_rs::Client;
use infracost_rs::providers::aws::EbsType;

let client = Client::from_env()?; // or Client::anonymous() for defaults
```

## EBS Volumes

```rust
// Unit price
let r = client.aws().ebs(EbsType::Gp3).region("us-east-1").fetch().await?;
// r.price = 0.08, r.unit = "GB-month"

// Monthly cost (500 GB, 6000 IOPS, 250 MiBps throughput)
let r = client.aws().ebs(EbsType::Gp3)
    .region("us-east-1")
    .size_gb(500)
    .iops(6000)        // 3000 included free
    .throughput_mibps(250) // 125 included free
    .fetch_monthly().await?;
// r.price = 60.0, r.unit = "month"

// String shorthand
let r = client.aws().ebs("gp3").region("us-east-1").fetch().await?;

// Types: Gp3, Gp2, Io2, St1, Sc1
```

## Snapshots

```rust
let r = client.aws().snapshot().region("us-east-1").fetch().await?;
// r.price = 0.05, r.unit = "GB-month"

let r = client.aws().snapshot().size_gb(200).fetch_monthly().await?;
// r.price = 10.0
```

## Elastic IP

```rust
let r = client.aws().elastic_ip().region("us-east-1").fetch().await?;
// r.price = 0.005, r.unit = "hour"

let r = client.aws().elastic_ip().fetch_monthly().await?;
// r.price = 3.65
```

## NAT Gateway

```rust
let r = client.aws().nat_gateway().region("us-east-1").fetch().await?;
// r.price = 0.045, r.unit = "hour"

let r = client.aws().nat_gateway()
    .data_processed_gb(1000)
    .fetch_monthly().await?;
// uptime ($0.045 * 730) + data ($0.045 * 1000)
```

## ALB

```rust
let r = client.aws().alb().region("us-east-1").fetch().await?;
// r.price = 0.0225, r.unit = "hour"
```

## Common Patterns

```rust
// Override default fallback
client.aws().ebs("gp3").override_default(0.10).fetch().await?;

// Per-request API key
client.aws().ebs("gp3").api_key("ico-xxx").fetch().await?;

// Check source
if result.is_from_api() { /* live price */ }
if result.is_from_default() { /* offline fallback */ }
```
