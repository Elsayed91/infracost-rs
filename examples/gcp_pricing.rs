//! Example: GCP resource pricing with convenience API
//!
//! Run without API key (returns defaults):
//!   cargo run --example gcp_pricing
//!
//! Run with API key (fetches live prices):
//!   INFRACOST_API_KEY="ico-xxx" cargo run --example gcp_pricing

use infracost_rs::Client;
use infracost_rs::providers::gcp::DiskType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let has_api_key = std::env::var("INFRACOST_API_KEY").is_ok();

    println!("=== GCP Pricing Example ===\n");
    println!(
        "API Key: {}\n",
        if has_api_key {
            "provided"
        } else {
            "not provided (using defaults)"
        }
    );

    let client = if has_api_key {
        Client::from_env()?
    } else {
        Client::anonymous()
    };

    // Test all disk types
    println!("--- Disk Pricing (per GB-month) ---");
    for disk_type in [
        DiskType::PdStandard,
        DiskType::PdSsd,
        DiskType::PdBalanced,
        DiskType::PdExtreme,
    ] {
        let result = client
            .gcp()
            .disk(disk_type)
            .region("us-central1")
            .fetch()
            .await?;
        println!(
            "{:?}: ${:.4}/{} (source: {:?})",
            disk_type, result.price, result.unit, result.source
        );
    }

    // Test snapshot
    println!("\n--- Snapshot Pricing (per GB-month) ---");
    let result = client
        .gcp()
        .snapshot()
        .region("us-central1")
        .fetch()
        .await?;
    println!(
        "Snapshot: ${:.4}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test snapshot monthly cost
    println!("\n--- Snapshot Monthly Cost (100 GB) ---");
    let result = client
        .gcp()
        .snapshot()
        .region("us-central1")
        .size_gb(100)
        .fetch_monthly()
        .await?;
    println!(
        "Snapshot (100 GB): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test static IP
    println!("\n--- Static IP Pricing (per hour) ---");
    let result = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch()
        .await?;
    println!(
        "Static IP: ${:.4}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test static IP monthly cost
    println!("\n--- Static IP Monthly Cost ---");
    let result = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch_monthly()
        .await?;
    println!(
        "Static IP: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test NAT Gateway - hourly pricing
    println!("\n--- NAT Gateway Uptime Pricing (per hour) ---");
    let result = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .fetch()
        .await?;
    println!(
        "NAT Gateway: ${:.4}/{} (~${:.2}/month) (source: {:?})",
        result.price,
        result.unit,
        result.price * 730.0,
        result.source
    );

    // Test NAT Gateway - monthly composite pricing
    println!("\n--- NAT Gateway Monthly Cost (uptime + data processing) ---");
    let result = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .data_processed_gb(1000) // 1000 GB per month
        .fetch_monthly()
        .await?;
    println!(
        "NAT Gateway (1000 GB data): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test NAT Gateway - monthly with no data processing
    let result = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .fetch_monthly()
        .await?;
    println!(
        "NAT Gateway (uptime only): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test Forwarding Rule - hourly pricing
    println!("\n--- Forwarding Rule Uptime Pricing (per hour) ---");
    let result = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch()
        .await?;
    println!(
        "Forwarding Rule: ${:.4}/{} (~${:.2}/month) (source: {:?})",
        result.price,
        result.unit,
        result.price * 730.0,
        result.source
    );

    // Test Forwarding Rule - monthly composite pricing
    println!("\n--- Forwarding Rule Monthly Cost (uptime + data processing) ---");
    let result = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .data_processed_gb(1000) // 1000 GB per month
        .fetch_monthly()
        .await?;
    println!(
        "Forwarding Rule (1000 GB data): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test Forwarding Rule - monthly with no data processing
    let result = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch_monthly()
        .await?;
    println!(
        "Forwarding Rule (uptime only): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test pd-extreme with IOPS - monthly composite pricing
    println!("\n--- pd-extreme Monthly Cost (storage + IOPS) ---");

    // Example 1: Small pd-extreme disk with moderate IOPS
    let result = client
        .gcp()
        .disk(DiskType::PdExtreme)
        .region("us-central1")
        .size_gb(500)
        .iops(15000)
        .fetch_monthly()
        .await?;
    println!(
        "pd-extreme (500 GB, 15000 IOPS): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: storage (500 * $0.125) + IOPS (15000 * $0.065) = $62.50 + $975.00");

    // Example 2: Large pd-extreme disk with high IOPS
    let result = client
        .gcp()
        .disk(DiskType::PdExtreme)
        .region("us-central1")
        .size_gb(1000)
        .iops(100000)
        .fetch_monthly()
        .await?;
    println!(
        "pd-extreme (1000 GB, 100000 IOPS): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: storage (1000 * $0.125) + IOPS (100000 * $0.065) = $125.00 + $6500.00");

    // Example 3: pd-extreme storage only (no IOPS)
    let result = client
        .gcp()
        .disk(DiskType::PdExtreme)
        .region("us-central1")
        .size_gb(500)
        .fetch_monthly()
        .await?;
    println!(
        "pd-extreme (500 GB, no IOPS): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Example 4: Compare with other disk types (storage only)
    println!("\n--- Disk Monthly Cost Comparison (500 GB) ---");
    for disk_type in [
        DiskType::PdStandard,
        DiskType::PdBalanced,
        DiskType::PdSsd,
        DiskType::PdExtreme,
    ] {
        let result = client
            .gcp()
            .disk(disk_type)
            .region("us-central1")
            .size_gb(500)
            .fetch_monthly()
            .await?;
        println!(
            "{:?}: ${:.2}/{} (source: {:?})",
            disk_type, result.price, result.unit, result.source
        );
    }

    Ok(())
}
