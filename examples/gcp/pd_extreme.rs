//! GCP PD Extreme pricing (storage + provisioned IOPS)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_pd_extreme
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

    println!("=== GCP PD Extreme (storage + IOPS) ===\n");

    println!("Unit prices:");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly — 500 GB, storage only:");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .region(*region)
            .size_gb(500)
            .fetch_monthly()
            .await?;
        println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly — 500 GB + IOPS:");
    for iops in [1_000, 10_000, 50_000] {
        println!("  {} IOPS:", iops);
        for (region, geo) in &regions {
            let r = client
                .gcp()
                .disk(DiskType::PdExtreme)
                .region(*region)
                .size_gb(500)
                .iops(iops)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    Ok(())
}
