//! GCP Static IP pricing
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_static_ip
//! ```

use infracost_rs::Client;

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

    println!("=== GCP Static IP ===\n");

    println!("Hourly and monthly prices:");
    for (region, geo) in &regions {
        let hourly = client.gcp().static_ip().region(*region).fetch().await?;
        let monthly = client
            .gcp()
            .static_ip()
            .region(*region)
            .fetch_monthly()
            .await?;
        println!(
            "  {:<15} ${:.4}/hr -> ${:.2}/month  ({:?})",
            geo, hourly.price, monthly.price, hourly.source
        );
    }

    Ok(())
}
