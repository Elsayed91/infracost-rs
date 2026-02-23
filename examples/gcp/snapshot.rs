//! GCP Snapshot pricing (standard and archive)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_snapshot
//! ```

use infracost_rs::Client;
use infracost_rs::providers::gcp::SnapshotType;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key),
        Err(_) => {
            println!("(No API key -- using built-in defaults)\n");
            Client::anonymous()
        }
    };

    let regions = [
        ("us-central1", "Americas"),
        ("europe-west1", "Europe"),
        ("asia-southeast1", "Asia-Pacific"),
    ];

    println!("=== GCP Standard Snapshot ===\n");

    println!("Unit prices:");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly cost by size:");
    for size in [50, 100, 500, 1000] {
        println!("  {} GiB:", size);
        for (region, geo) in &regions {
            let r = client
                .gcp()
                .snapshot(SnapshotType::Standard)
                .region(*region)
                .size_gb(size)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    println!("\n=== GCP Archive Snapshot ===\n");

    println!("Unit prices (storage):");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly cost by size (500 GiB storage + 100 GiB retrieval):");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region(*region)
            .size_gb(500)
            .retrieval_size_gb(100)
            .fetch_monthly()
            .await?;
        println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
    }

    Ok(())
}
