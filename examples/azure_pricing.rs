//! Example: Azure resource pricing with convenience API
//!
//! Run without API key (returns defaults):
//!   cargo run --example azure_pricing
//!
//! Run with API key (fetches live prices):
//!   INFRACOST_API_KEY="ico-xxx" cargo run --example azure_pricing

use infracost_rs::Client;
use infracost_rs::providers::azure::{ManagedDiskSize, ManagedDiskType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let has_api_key = std::env::var("INFRACOST_API_KEY").is_ok();

    println!("=== Azure Pricing Example ===\n");
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

    // Test Public IP hourly pricing
    println!("--- Public IP Pricing (per hour) ---");
    let result = client.azure().public_ip().region("eastus").fetch().await?;
    println!(
        "Public IP: ${:.4}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test Public IP monthly pricing
    println!("\n--- Public IP Monthly Cost Calculation ---");
    let result = client
        .azure()
        .public_ip()
        .region("eastus")
        .fetch_monthly()
        .await?;
    println!(
        "Public IP: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: $0.005 x 730 hours = $3.65");

    // Test Snapshot per-GB pricing
    println!("\n--- Snapshot Pricing (per GB-month) ---");
    let result = client.azure().snapshot().region("eastus").fetch().await?;
    println!(
        "Snapshot: ${:.4}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test Snapshot monthly pricing with size
    println!("\n--- Snapshot Monthly Cost Calculation ---");
    let result = client
        .azure()
        .snapshot()
        .region("eastus")
        .size_gb(100)
        .fetch_monthly()
        .await?;
    println!(
        "Snapshot (100 GB): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: $0.05 x 100 GB = $5.00");

    // Test different snapshot sizes
    println!("\n--- Snapshot Sizes ---");
    for size_gb in [50, 100, 500, 1000] {
        let result = client
            .azure()
            .snapshot()
            .region("eastus")
            .size_gb(size_gb)
            .fetch_monthly()
            .await?;
        println!(
            "  {} GB: ${:.2}/month (source: {:?})",
            size_gb, result.price, result.source
        );
    }

    // Test Managed Disk pricing (Premium SSD)
    println!("\n--- Managed Disk Pricing (Premium SSD P-series) ---");
    for size in [
        ManagedDiskSize::P4,
        ManagedDiskSize::P10,
        ManagedDiskSize::P20,
        ManagedDiskSize::P30,
    ] {
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, size)
            .region("eastus")
            .fetch_monthly()
            .await?;
        println!(
            "{:?}: ${:.2}/{} (source: {:?})",
            size, result.price, result.unit, result.source
        );
    }

    // Test Managed Disk pricing (Standard SSD)
    println!("\n--- Managed Disk Pricing (Standard SSD E-series) ---");
    for size in [
        ManagedDiskSize::E4,
        ManagedDiskSize::E10,
        ManagedDiskSize::E20,
        ManagedDiskSize::E30,
    ] {
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::StandardSsd, size)
            .region("eastus")
            .fetch_monthly()
            .await?;
        println!(
            "{:?}: ${:.2}/{} (source: {:?})",
            size, result.price, result.unit, result.source
        );
    }

    // Test Managed Disk pricing (Standard HDD)
    println!("\n--- Managed Disk Pricing (Standard HDD S-series) ---");
    for size in [
        ManagedDiskSize::S4,
        ManagedDiskSize::S10,
        ManagedDiskSize::S20,
        ManagedDiskSize::S30,
    ] {
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::StandardHdd, size)
            .region("eastus")
            .fetch_monthly()
            .await?;
        println!(
            "{:?}: ${:.2}/{} (source: {:?})",
            size, result.price, result.unit, result.source
        );
    }

    // Demonstrate that fetch() and fetch_monthly() are equivalent for managed disks
    println!("\n--- Managed Disk: fetch() vs fetch_monthly() ---");
    let fetch_result = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch()
        .await?;
    let monthly_result = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch_monthly()
        .await?;
    println!(
        "P10 fetch():        ${:.2}/{} (source: {:?})",
        fetch_result.price, fetch_result.unit, fetch_result.source
    );
    println!(
        "P10 fetch_monthly(): ${:.2}/{} (source: {:?})",
        monthly_result.price, monthly_result.unit, monthly_result.source
    );
    println!(
        "  Note: Managed disks are already priced monthly, so both methods return the same result"
    );

    Ok(())
}
