//! Validation tool for YAML-defined pricing catalog.
//!
//! Iterates all resources across multiple regions, queries the Infracost API,
//! and reports whether each returns a valid price within expected range.
//!
//! # Usage
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --features cli --bin validate-prices
//! ```

use infracost_rs::Client;
use infracost_rs::catalog::{aws_catalog, azure_catalog, engine::PricingEngine, gcp_catalog};

const GCP_REGIONS: &[&str] = &[
    "us-central1",
    "us-east1",
    "europe-west1",
    "asia-east1",
    "asia-southeast1",
    "australia-southeast1",
    "southamerica-east1",
];

const AWS_REGIONS: &[&str] = &[
    "us-east-1",
    "us-west-2",
    "eu-west-1",
    "ap-southeast-1",
    "ap-northeast-1",
    "sa-east-1",
    "ca-central-1",
];

const AZURE_REGIONS: &[&str] = &[
    "eastus",
    "westus2",
    "westeurope",
    "southeastasia",
    "japaneast",
    "brazilsouth",
    "canadacentral",
];

struct ValidationResult {
    vendor: String,
    resource: String,
    component: String,
    region: String,
    price: f64,
    default: f64,
    min: Option<f64>,
    max: Option<f64>,
    status: Status,
}

enum Status {
    Ok,
    ZeroPrice,
    OutOfRange,
    Error(String),
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Ok => write!(f, "OK"),
            Status::ZeroPrice => write!(f, "ZERO"),
            Status::OutOfRange => write!(f, "OUT_OF_RANGE"),
            Status::Error(e) => write!(f, "ERROR: {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::from_env()?;
    let mut results = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    println!("Validating YAML pricing catalog...\n");

    // GCP
    let gcp = gcp_catalog();
    for resource in &gcp.resources {
        for region in GCP_REGIONS {
            for component in &resource.cost_components {
                let r = validate_component(&client, &gcp.vendor, resource, component, region).await;
                print_result(&r);
                match r.status {
                    Status::Ok => passed += 1,
                    _ => failed += 1,
                }
                results.push(r);
            }
        }
    }

    // AWS
    let aws = aws_catalog();
    for resource in &aws.resources {
        for region in AWS_REGIONS {
            for component in &resource.cost_components {
                let r = validate_component(&client, &aws.vendor, resource, component, region).await;
                print_result(&r);
                match r.status {
                    Status::Ok => passed += 1,
                    _ => failed += 1,
                }
                results.push(r);
            }
        }
    }

    // Azure
    let azure = azure_catalog();
    for resource in &azure.resources {
        for region in AZURE_REGIONS {
            for component in &resource.cost_components {
                let r =
                    validate_component(&client, &azure.vendor, resource, component, region).await;
                print_result(&r);
                match r.status {
                    Status::Ok => passed += 1,
                    _ => failed += 1,
                }
                results.push(r);
            }
        }
    }

    println!(
        "\nValidation complete: {} passed, {} failed",
        passed, failed
    );

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

async fn validate_component(
    client: &Client,
    vendor: &str,
    resource: &infracost_rs::catalog::types::ResourceDef,
    component: &infracost_rs::catalog::types::CostComponentDef,
    region: &str,
) -> ValidationResult {
    let result = PricingEngine::fetch_component_price(
        client,
        component,
        vendor,
        region,
        None,
        component.default_price,
        None,
    )
    .await;

    let (price, status) = match result {
        Ok(pr) => {
            let p = pr.price;
            if p == 0.0 {
                (p, Status::ZeroPrice)
            } else if let (Some(min), Some(max)) = (component.min_price, component.max_price) {
                if p < min || p > max {
                    (p, Status::OutOfRange)
                } else {
                    (p, Status::Ok)
                }
            } else {
                (p, Status::Ok)
            }
        }
        Err(e) => (0.0, Status::Error(e.to_string())),
    };

    ValidationResult {
        vendor: vendor.to_string(),
        resource: resource.name.clone(),
        component: component.name.clone(),
        region: region.to_string(),
        price,
        default: component.default_price,
        min: component.min_price,
        max: component.max_price,
        status,
    }
}

fn print_result(r: &ValidationResult) {
    let range = match (r.min, r.max) {
        (Some(min), Some(max)) => format!(" (range: ${:.4}-${:.4})", min, max),
        _ => String::new(),
    };
    println!(
        "{}/{}/{:<20} {:<22} ${:<10.4} (default: ${:.4}) {}{}",
        r.vendor, r.resource, r.component, r.region, r.price, r.default, r.status, range
    );
}
