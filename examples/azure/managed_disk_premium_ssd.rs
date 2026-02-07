//! Azure Premium SSD Managed Disk pricing (P-series, fixed monthly)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example azure_managed_disk_premium_ssd
//! ```

use infracost_rs::Client;
use infracost_rs::providers::azure::ManagedDiskSize;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key),
        Err(_) => {
            println!("(No API key — using built-in defaults)\n");
            Client::anonymous()
        }
    };

    let regions = [
        ("eastus", "Americas"),
        ("westeurope", "Europe"),
        ("southeastasia", "Asia-Pacific"),
    ];

    let sizes = [
        (ManagedDiskSize::P4, "P4 (32 GB)"),
        (ManagedDiskSize::P10, "P10 (128 GB)"),
        (ManagedDiskSize::P20, "P20 (512 GB)"),
        (ManagedDiskSize::P30, "P30 (1 TB)"),
        (ManagedDiskSize::P50, "P50 (4 TB)"),
    ];

    println!("=== Azure Premium SSD (P-series) ===\n");

    for (size, label) in &sizes {
        println!("{}:", label);
        for (region, geo) in &regions {
            let r = client
                .azure()
                .managed_disk("premium-ssd", *size)
                .region(*region)
                .fetch_monthly()
                .await?;
            println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
        println!();
    }

    Ok(())
}
