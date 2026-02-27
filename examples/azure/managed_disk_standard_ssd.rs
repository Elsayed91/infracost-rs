//! Azure Standard SSD Managed Disk pricing (E-series, fixed monthly)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example azure_managed_disk_standard_ssd
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
        (ManagedDiskSize::E4, "E4 (32 GB)"),
        (ManagedDiskSize::E10, "E10 (128 GB)"),
        (ManagedDiskSize::E20, "E20 (512 GB)"),
        (ManagedDiskSize::E30, "E30 (1 TB)"),
        (ManagedDiskSize::E50, "E50 (4 TB)"),
    ];

    println!("=== Azure Standard SSD (E-series) ===\n");

    for (size, label) in &sizes {
        println!("{}:", label);
        for (region, geo) in &regions {
            let r = client
                .azure()
                .managed_disk("standard-ssd", *size)
                .region(*region)
                .fetch_monthly()
                .await?;
            println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
        println!();
    }

    Ok(())
}
