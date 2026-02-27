//! Azure Standard HDD Managed Disk pricing (S-series, fixed monthly)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example azure_managed_disk_standard_hdd
//! ```

use infracost_rs::Client;
use infracost_rs::providers::azure::ManagedDiskSize;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key)?,
        Err(_) => {
            println!("(No API key — using built-in defaults)\n");
            Client::anonymous()?
        }
    };

    let regions = [
        ("eastus", "Americas"),
        ("westeurope", "Europe"),
        ("southeastasia", "Asia-Pacific"),
    ];

    let sizes = [
        (ManagedDiskSize::S4, "S4 (32 GB)"),
        (ManagedDiskSize::S10, "S10 (128 GB)"),
        (ManagedDiskSize::S20, "S20 (512 GB)"),
        (ManagedDiskSize::S30, "S30 (1 TB)"),
        (ManagedDiskSize::S50, "S50 (4 TB)"),
    ];

    println!("=== Azure Standard HDD (S-series) ===\n");

    for (size, label) in &sizes {
        println!("{}:", label);
        for (region, geo) in &regions {
            let r = client
                .azure()
                .managed_disk("standard-hdd", *size)
                .region(*region)
                .fetch_monthly()
                .await?;
            println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
        println!();
    }

    Ok(())
}
