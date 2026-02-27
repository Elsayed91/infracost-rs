//! GCP Backend Service (Load Balancer) pricing (Premium vs Standard tier)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_backend_service
//! ```

use infracost_rs::Client;
use infracost_rs::providers::gcp::BackendServiceTier;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = if let Ok(key) = std::env::var("INFRACOST_API_KEY") {
        Client::new(key)?
    } else {
        println!("(No API key — using built-in defaults)\n");
        Client::anonymous()?
    };

    let regions = [
        ("us-central1", "Americas"),
        ("europe-west1", "Europe"),
        ("asia-southeast1", "Asia-Pacific"),
    ];

    println!("=== GCP Backend Service ===\n");

    println!("Data processing unit price (per GB):");
    for tier in [BackendServiceTier::Premium, BackendServiceTier::Standard] {
        println!("  {:?}:", tier);
        for (region, geo) in &regions {
            let r = client
                .gcp()
                .backend_service(tier)
                .region(*region)
                .fetch()
                .await?;
            println!(
                "    {:<15} ${:.4}/{}  ({:?})",
                geo, r.price, r.unit, r.source
            );
        }
    }

    println!("\nMonthly cost by data volume (Premium, us-central1):");
    for gb in [0, 100, 500, 1000] {
        let r = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .data_processed_gb(gb)
            .fetch_monthly()
            .await?;
        println!("  {:>5} GB   ${:.2}/month  ({:?})", gb, r.price, r.source);
    }

    println!("\nMonthly cost with forwarding rules included (Premium, us-central1):");
    for rules in [1, 5, 10] {
        let r = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .forwarding_rules(rules)
            .data_processed_gb(500)
            .fetch_monthly()
            .await?;
        println!(
            "  {} rule(s) + 500 GB  ${:.2}/month  ({:?})",
            rules, r.price, r.source
        );
    }

    Ok(())
}
