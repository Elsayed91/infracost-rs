# AWS

```rust
use infracost_rs::Client;

let client = Client::from_env()?; // or Client::anonymous() for defaults
```

## EC2 Instance

Any instance type, no hardcoded list. Defaults to Linux/Shared/NA.

```rust
// Hourly rate
let r = client.aws().ec2_instance("t3.micro").region("us-east-1").fetch().await?;
// r.price = 0.0104, r.unit = "hour"

// Monthly
let r = client.aws().ec2_instance("m5.xlarge")
    .region("us-east-1")
    .fetch_monthly().await?;

// Windows
let r = client.aws().ec2_instance("m5.xlarge")
    .operating_system("Windows")
    .fetch_monthly().await?;

// Dedicated tenancy
let r = client.aws().ec2_instance("m5.xlarge")
    .tenancy("Dedicated")
    .fetch_monthly().await?;

// Pre-installed software
let r = client.aws().ec2_instance("m5.xlarge")
    .pre_installed_sw("SQL Std")
    .fetch_monthly().await?;
```

Defaults: `operating_system = "Linux"`, `tenancy = "Shared"`, `pre_installed_sw = "NA"`.

## RDS

Instance compute + storage pricing. Supports MySQL, PostgreSQL, MariaDB, Oracle, SQL Server, Aurora.

```rust
use infracost_rs::providers::aws::RdsStorageType;

// Hourly instance rate
let r = client.aws().rds("db.t3.micro").region("us-east-1").fetch().await?;

// Full monthly (instance + storage)
let r = client.aws().rds("db.r5.large")
    .engine("postgres")
    .deployment_option("Multi-AZ")
    .storage_type(RdsStorageType::Gp3)
    .allocated_storage_gb(100)
    .iops(6000)                   // gp3 baseline: 3000 included free
    .storage_throughput_mbps(250) // gp3 baseline: 125 included free
    .fetch_monthly().await?;

// String shorthand for storage type
let r = client.aws().rds("db.t3.micro")
    .engine("mysql")
    .storage_type("io1")
    .allocated_storage_gb(200)
    .iops(3000)
    .fetch_monthly().await?;
```

**Engines:** `"mysql"`, `"postgres"`, `"mariadb"`, `"oracle"`, `"sqlserver"`, `"aurora-mysql"`, `"aurora-postgresql"`

**Storage types:** `Gp3` (default), `Gp2`, `Io1`, `Io2`, `Magnetic`

**Deployment:** `"Single-AZ"` (default), `"Multi-AZ"`

## EBS Volumes

```rust
use infracost_rs::providers::aws::EbsType;

// Unit price
let r = client.aws().ebs(EbsType::Gp3).region("us-east-1").fetch().await?;
// r.price = 0.08, r.unit = "GB-month"

// Monthly cost (500 GB, 6000 IOPS, 250 MiBps throughput)
let r = client.aws().ebs(EbsType::Gp3)
    .region("us-east-1")
    .size_gb(500)
    .iops(6000)            // 3000 included free
    .throughput_mibps(250) // 125 included free
    .fetch_monthly().await?;
// r.price = 60.0, r.unit = "month"

// String shorthand
let r = client.aws().ebs("gp3").region("us-east-1").fetch().await?;
```

**Types:** `Gp3`, `Gp2`, `Io2`, `St1`, `Sc1`

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
