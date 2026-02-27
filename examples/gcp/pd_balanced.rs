//! GCP PD Balanced pricing
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_pd_balanced
//! ```

use infracost_rs::Client;
use infracost_rs::providers::gcp::DiskType;

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
        ("us-central1", "Americas"),
        ("europe-west1", "Europe"),
        ("asia-southeast1", "Asia-Pacific"),
    ];

    println!("=== GCP PD Balanced ===\n");

    println!("Unit prices:");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .disk(DiskType::PdBalanced)
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly cost by size:");
    for size in [50, 100, 500, 1000] {
        println!("  {} GB:", size);
        for (region, geo) in &regions {
            let r = client
                .gcp()
                .disk(DiskType::PdBalanced)
                .region(*region)
                .size_gb(size)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    Ok(())
}
