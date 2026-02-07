//! Azure Managed Disk Snapshot pricing
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example azure_snapshot
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

    let regions = [
        ("eastus", "Americas"),
        ("westeurope", "Europe"),
        ("southeastasia", "Asia-Pacific"),
    ];

    println!("=== Azure Snapshot ===\n");

    println!("Unit prices:");
    for (region, geo) in &regions {
        let r = client.azure().snapshot().region(*region).fetch().await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly cost by size:");
    for size in [50, 100, 500, 1000] {
        println!("  {} GB:", size);
        for (region, geo) in &regions {
            let r = client
                .azure()
                .snapshot()
                .region(*region)
                .size_gb(size)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    Ok(())
}
