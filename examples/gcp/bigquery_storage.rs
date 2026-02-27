//! GCP BigQuery Storage pricing (logical and physical billing models)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_bigquery_storage
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
        ("us-central1", "Americas"),
        ("europe-west1", "Europe"),
        ("asia-southeast1", "Asia-Pacific"),
    ];

    println!("=== GCP BigQuery Storage ===\n");

    println!("Active logical storage unit price (per GB-month):");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .bigquery_storage()
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly cost — logical billing (us-central1):");
    for (active_gb, longterm_gb) in [(100, 0), (500, 200), (1000, 500), (5000, 2000)] {
        let r = client
            .gcp()
            .bigquery_storage()
            .region("us-central1")
            .active_logical_storage_gb(active_gb)
            .long_term_logical_storage_gb(longterm_gb)
            .fetch_monthly()
            .await?;
        println!(
            "  {:>5} GB active + {:>5} GB long-term  ${:.2}/month  ({:?})",
            active_gb, longterm_gb, r.price, r.source
        );
    }

    println!("\nMonthly cost — physical billing (us-central1):");
    for (active_gb, longterm_gb) in [(100, 0), (500, 200), (1000, 500)] {
        let r = client
            .gcp()
            .bigquery_storage()
            .region("us-central1")
            .active_physical_storage_gb(active_gb)
            .long_term_physical_storage_gb(longterm_gb)
            .fetch_monthly()
            .await?;
        println!(
            "  {:>5} GB active + {:>5} GB long-term  ${:.2}/month  ({:?})",
            active_gb, longterm_gb, r.price, r.source
        );
    }

    Ok(())
}
