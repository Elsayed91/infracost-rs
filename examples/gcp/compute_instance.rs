//! GCP Compute Instance pricing (machine types and purchase options)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_compute_instance
//! ```

use infracost_rs::Client;
use infracost_rs::providers::gcp::PurchaseOption;

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

    println!("=== GCP Compute Instance ===\n");

    println!("Hourly on-demand price (n2-standard-4):");
    for (region, geo) in &regions {
        let r = client
            .gcp()
            .compute_instance()
            .machine_type("n2-standard-4")
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/hr  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly cost by machine type (us-central1, on-demand):");
    for machine_type in [
        "e2-micro",
        "e2-medium",
        "n2-standard-4",
        "n2-standard-8",
        "c2-standard-4",
    ] {
        let r = client
            .gcp()
            .compute_instance()
            .machine_type(machine_type)
            .region("us-central1")
            .fetch_monthly()
            .await?;
        println!(
            "  {:<20} ${:.2}/month  ({:?})",
            machine_type, r.price, r.source
        );
    }

    println!("\nPurchase options (n2-standard-4, us-central1):");
    for (option, label) in [
        (PurchaseOption::OnDemand, "On-demand"),
        (PurchaseOption::Preemptible, "Preemptible (spot)"),
        (PurchaseOption::Commit1Yr, "1-year CUD"),
        (PurchaseOption::Commit3Yr, "3-year CUD"),
    ] {
        let r = client
            .gcp()
            .compute_instance()
            .machine_type("n2-standard-4")
            .region("us-central1")
            .purchase_option(option)
            .fetch_monthly()
            .await?;
        println!("  {:<22} ${:.2}/month  ({:?})", label, r.price, r.source);
    }

    println!("\nCustom instance (N2 family, 6 vCPU, 24 GiB, us-central1):");
    let r = client
        .gcp()
        .compute_instance()
        .machine_family("n2")
        .cpu_cores(6)
        .memory_gib(24)
        .region("us-central1")
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})", r.price, r.source);

    Ok(())
}
