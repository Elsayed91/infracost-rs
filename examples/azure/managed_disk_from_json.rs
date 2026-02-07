//! Azure Managed Disk pricing from `az disk show` JSON output
//!
//! ```bash
//! # With real JSON from Azure CLI:
//! # az disk show --name my-disk --resource-group my-rg --output json | \
//! #   cargo run --example azure_managed_disk_from_json
//!
//! # With defaults (no API key needed):
//! cargo run --example azure_managed_disk_from_json
//! ```

use infracost_rs::Client;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key),
        Err(_) => {
            println!("(No API key — using built-in defaults)\n");
            Client::anonymous()
        }
    };

    // Simulated `az disk show` JSON output (Premium SSD P10)
    let premium_disk = serde_json::json!({
        "sku": { "name": "Premium_LRS", "tier": "Premium" },
        "diskSizeGb": 128,
        "location": "eastus",
        "name": "my-premium-disk",
        "provisioningState": "Succeeded"
    });

    println!("=== Azure Managed Disk from JSON ===\n");

    println!("Premium SSD (128 GB -> P10, eastus):");
    let r = client
        .azure()
        .managed_disk_from_json(&premium_disk)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})\n", r.price, r.source);

    // Standard SSD (256 GB -> E15)
    let standard_ssd = serde_json::json!({
        "sku": { "name": "StandardSSD_LRS" },
        "diskSizeGb": 256,
        "location": "westeurope",
        "name": "my-standard-ssd"
    });

    println!("Standard SSD (256 GB -> E15, westeurope):");
    let r = client
        .azure()
        .managed_disk_from_json(&standard_ssd)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})\n", r.price, r.source);

    // Standard HDD (64 GB -> S6)
    let standard_hdd = serde_json::json!({
        "sku": { "name": "Standard_LRS" },
        "diskSizeGb": 64,
        "location": "southeastasia",
        "name": "my-standard-hdd"
    });

    println!("Standard HDD (64 GB -> S6, southeastasia):");
    let r = client
        .azure()
        .managed_disk_from_json(&standard_hdd)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})", r.price, r.source);

    Ok(())
}
