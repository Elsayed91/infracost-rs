//! Azure NAT Gateway pricing (uptime + data processing)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example azure_nat_gateway
//! ```

use infracost_rs::Client;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = if let Ok(key) = std::env::var("INFRACOST_API_KEY") {
        Client::new(key)?
    } else {
        println!("(No API key — using built-in defaults)\n");
        Client::anonymous()?
    };

    let regions = [
        ("eastus", "Americas"),
        ("westeurope", "Europe"),
        ("southeastasia", "Asia-Pacific"),
    ];

    println!("=== Azure NAT Gateway ===\n");

    println!("Hourly uptime price:");
    for (region, geo) in &regions {
        let r = client.azure().nat_gateway().region(*region).fetch().await?;
        println!("  {:<15} ${:.4}/hr  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly cost (uptime + data processing):");
    for gb in [0, 100, 500, 1000] {
        println!("  {} GB processed:", gb);
        for (region, geo) in &regions {
            let r = client
                .azure()
                .nat_gateway()
                .region(*region)
                .data_processed_gb(gb)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    Ok(())
}
