//! GCP NAT Gateway pricing (uptime + data processing)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_nat_gateway
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
        ("us-central1", "Americas"),
        ("europe-west1", "Europe"),
        ("asia-southeast1", "Asia-Pacific"),
    ];

    println!("=== GCP NAT Gateway ===\n");

    println!("Hourly uptime price:");
    for (region, geo) in &regions {
        let r = client.gcp().nat_gateway().region(*region).fetch().await?;
        println!("  {:<15} ${:.4}/hr  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly (uptime + data processing):");
    for gb in [0, 100, 500, 1000] {
        println!("  {} GB processed:", gb);
        for (region, geo) in &regions {
            let r = client
                .gcp()
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
