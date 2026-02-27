//! AWS Application Load Balancer pricing (uptime + LCU)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example aws_alb
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
        ("us-east-1", "Americas"),
        ("eu-west-1", "Europe"),
        ("ap-southeast-1", "Asia-Pacific"),
    ];

    println!("=== AWS Application Load Balancer ===\n");

    println!("Hourly uptime price:");
    for (region, geo) in &regions {
        let r = client.aws().alb().region(*region).fetch().await?;
        println!("  {:<15} ${:.4}/hr  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly (uptime + LCU hours):");
    for lcu in [0, 100, 500, 1000] {
        println!("  {} LCU-hours:", lcu);
        for (region, geo) in &regions {
            let r = client
                .aws()
                .alb()
                .region(*region)
                .lcu_hours(lcu)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    Ok(())
}
