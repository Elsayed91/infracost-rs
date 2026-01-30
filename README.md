# infracost-rs

Rust client for the [Infracost](https://www.infracost.io/) Cloud Pricing API.

## Install

```toml
# Library
[dependencies]
infracost-rs = "0.1"

# With blocking API
infracost-rs = { version = "0.1", features = ["blocking"] }
```

```bash
# CLI (binary name: irs)
cargo install infracost-rs --features cli
```

## Library Usage

### Client Modes

```rust
use infracost_rs::Client;
use std::time::Duration;

// From environment variable
let client = Client::from_env()?; // reads INFRACOST_API_KEY

// Explicit API key (stored in client)
let client = Client::new("ico-xxx");

// Anonymous client (must provide key per-request)
let client = Client::anonymous();

// Full builder
let client = Client::builder()
    .api_key("ico-xxx")
    .endpoint("https://pricing.api.infracost.io/graphql")
    .timeout(Duration::from_secs(30))
    .build()?;
```

### Querying

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

### GCP Compute Engine

```rust
let products = client
    .products()
    .vendor("gcp")
    .service("Compute Engine")
    .region("us-central1")
    .attribute("machineType", "n2-standard-32")
    .fetch()
    .await?;

// First price as f64 (on-demand)
let hourly = products[0].price_f64()?;

// Filter by purchase option
let on_demand = products[0]
    .prices()
    .purchase_option("on_demand")
    .first_f64()?;

let spot = products[0]
    .prices()
    .purchase_option("preemptible")
    .first_f64()?;
```

### Per-Request API Key

```rust
// With anonymous client, or to override default key
let products = client
    .products()
    .api_key("ico-different-key")
    .vendor("gcp")
    .fetch()
    .await?;
```

### AWS EC2

```rust
let products = client
    .products()
    .vendor("aws")
    .service("AmazonEC2")
    .region("us-east-1")
    .product_family("Compute Instance")
    .attribute("instanceType", "t3.micro")
    .attribute("operatingSystem", "Linux")
    .attribute("tenancy", "Shared")
    .attribute("capacitystatus", "Used")
    .fetch()
    .await?;

let hourly = products[0]
    .prices()
    .unit("Hrs")
    .description("On Demand")
    .first_f64()?;
```

### Testing with Mocks

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

## CLI Usage

```bash
export INFRACOST_API_KEY=ico-xxx

# Query GCP VM pricing
irs query -v gcp -s "Compute Engine" -r us-central1 -a 'machineType=n2-standard-32'

# Spot/preemptible pricing
irs query -v gcp -s "Compute Engine" -r us-central1 -a 'machineType=n2-standard-32' -p preemptible

# On-demand pricing (explicit)
irs query -v gcp -s "Compute Engine" -r us-central1 -a 'machineType=n2-standard-32' -p on_demand

# Quiet mode (just price)
irs query -v gcp -s "Compute Engine" -r us-central1 -a 'machineType=n2-standard-32' -q

# With attribute filter
irs query --vendor gcp -a 'description=SSD backed PD Capacity'

# JSON output
irs query --vendor aws --service AmazonS3 --region us-east-1 --format json

# List services
irs services --vendor gcp

# List regions
irs regions --vendor gcp --service "Compute Engine"
```

## License

MIT OR Apache-2.0
