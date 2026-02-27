//! AWS EBS io2 pricing (storage + tiered IOPS)
//!
//! io2 uses tiered IOPS pricing:
//!   Tier 1: 1–32,000 IOPS at $0.065/IOPS
//!   Tier 2: 32,001–64,000 IOPS at $0.0455/IOPS
//!   Tier 3: 64,001+ IOPS at $0.03185/IOPS
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example aws_ebs_io2
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

    println!("=== AWS EBS io2 (tiered IOPS pricing) ===\n");

    println!("Unit prices:");
    for (region, geo) in &regions {
        let r = client.aws().ebs("io2").region(*region).fetch().await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly — 500 GB, storage only:");
    for (region, geo) in &regions {
        let r = client
            .aws()
            .ebs("io2")
            .region(*region)
            .size_gb(500)
            .fetch_monthly()
            .await?;
        println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly — 500 GB + IOPS (observe tiered pricing):");
    for iops in [1_000, 32_000, 50_000, 100_000] {
        println!("  {} IOPS:", iops);
        for (region, geo) in &regions {
            let r = client
                .aws()
                .ebs("io2")
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
