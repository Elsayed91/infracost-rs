//! AWS EBS gp3 pricing (storage + baseline IOPS/throughput)
//!
//! gp3 includes 3,000 IOPS and 125 MiB/s baseline — only excess is billed.
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example aws_ebs_gp3
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

    println!("=== AWS EBS gp3 (baseline: 3000 IOPS, 125 MiB/s) ===\n");

    println!("Unit prices:");
    for (region, geo) in &regions {
        let r = client.aws().ebs("gp3").region(*region).fetch().await?;
        println!("  {:<15} ${:.4}/{}  ({:?})", geo, r.price, r.unit, r.source);
    }

    println!("\nMonthly — 500 GB, baseline only (3000 IOPS, 125 MiB/s):");
    for (region, geo) in &regions {
        let r = client
            .aws()
            .ebs("gp3")
            .region(*region)
            .size_gb(500)
            .fetch_monthly()
            .await?;
        println!("  {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly — 500 GB + extra IOPS:");
    for iops in [3000u64, 6000, 16000] {
        println!("  {} IOPS ({}+ baseline):", iops, iops.saturating_sub(3000));
        for (region, geo) in &regions {
            let r = client
                .aws()
                .ebs("gp3")
                .region(*region)
                .size_gb(500)
                .iops(iops)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    println!("\nMonthly — 500 GB + 6000 IOPS + extra throughput:");
    for throughput in [125u64, 250, 500] {
        println!(
            "  {} MiB/s ({}+ baseline):",
            throughput,
            throughput.saturating_sub(125)
        );
        for (region, geo) in &regions {
            let r = client
                .aws()
                .ebs("gp3")
                .region(*region)
                .size_gb(500)
                .iops(6000)
                .throughput_mibps(throughput)
                .fetch_monthly()
                .await?;
            println!("    {:<15} ${:.2}/month  ({:?})", geo, r.price, r.source);
        }
    }

    Ok(())
}
