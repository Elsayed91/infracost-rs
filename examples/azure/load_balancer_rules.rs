//! Azure Load Balancer Rules pricing
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example azure_load_balancer_rules
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

    println!("=== Azure Load Balancer Rules ===\n");

    println!("Hourly price per rule:");
    for (region, geo) in &regions {
        let r = client
            .azure()
            .load_balancer_rules()
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/hr  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly cost by rule count (eastus):");
    for rule_count in [1, 5, 10, 25] {
        let r = client
            .azure()
            .load_balancer_rules()
            .region("eastus")
            .rule_count(rule_count)
            .fetch_monthly()
            .await?;
        println!(
            "  {:>3} rule(s)  ${:.2}/month  ({:?})",
            rule_count, r.price, r.source
        );
    }

    Ok(())
}
