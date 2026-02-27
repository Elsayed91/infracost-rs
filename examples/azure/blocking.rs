//! Azure pricing using the blocking (synchronous) API
//!
//! Demonstrates all Azure builders using `infracost_rs::blocking::Client`,
//! which is a synchronous wrapper suitable for non-async contexts.
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example azure_blocking
//! ```

use infracost_rs::blocking::Client;
use infracost_rs::providers::azure::{ManagedDiskSize, ManagedDiskType};

fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key)?,
        Err(_) => {
            println!("(No API key — using built-in defaults)\n");
            Client::anonymous()?
        }
    };

    println!("=== Azure Blocking API ===\n");

    // Managed Disk
    let r = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch_monthly()?;
    println!("Premium SSD P10   ${:.2}/month  ({:?})", r.price, r.source);

    let r = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P30)
        .region("eastus")
        .fetch_monthly()?;
    println!("Premium SSD P30   ${:.2}/month  ({:?})", r.price, r.source);

    let r = client
        .azure()
        .managed_disk(ManagedDiskType::StandardSsd, ManagedDiskSize::E10)
        .region("eastus")
        .fetch_monthly()?;
    println!("Standard SSD E10  ${:.2}/month  ({:?})", r.price, r.source);

    let r = client
        .azure()
        .managed_disk(ManagedDiskType::StandardHdd, ManagedDiskSize::S10)
        .region("eastus")
        .fetch_monthly()?;
    println!("Standard HDD S10  ${:.2}/month  ({:?})", r.price, r.source);

    // Snapshot
    let r = client
        .azure()
        .snapshot()
        .region("eastus")
        .size_gb(200)
        .fetch_monthly()?;
    println!(
        "Snapshot          ${:.2}/month (200 GB)  ({:?})",
        r.price, r.source
    );

    // Public IP
    let r = client
        .azure()
        .public_ip()
        .region("eastus")
        .fetch_monthly()?;
    println!("Public IP         ${:.2}/month  ({:?})", r.price, r.source);

    // NAT Gateway
    let r = client
        .azure()
        .nat_gateway()
        .region("eastus")
        .data_processed_gb(500)
        .fetch_monthly()?;
    println!(
        "NAT Gateway       ${:.2}/month (500 GB)  ({:?})",
        r.price, r.source
    );

    // Load Balancer Rules
    let r = client
        .azure()
        .load_balancer_rules()
        .region("eastus")
        .rule_count(10)
        .fetch_monthly()?;
    println!(
        "LB Rules          ${:.2}/month (10 rules)  ({:?})",
        r.price, r.source
    );

    Ok(())
}
