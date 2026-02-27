//! GCP Hyperdisk Throughput pricing (storage + throughput)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_hyperdisk_throughput
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

    println!("=== GCP Hyperdisk Throughput (storage + throughput) ===\n");

    println!("Unit prices:");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .disk(DiskType::HyperdiskThroughput)
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly — 100 GB, storage only:");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .disk(DiskType::HyperdiskThroughput)
            .region(*region)
            .size_gb(100)
            .fetch_monthly()
            .await?;
        println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly — 1000 GB + throughput:");
    for throughput in [100, 250, 500] {
        println!("  {} MiB/s:", throughput);
        for (region, geo) in &regions {
            let r = client
                .gcp()
                .disk(DiskType::HyperdiskThroughput)
                .region(*region)
                .size_gb(1000)
                .throughput(throughput)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    Ok(())
}
