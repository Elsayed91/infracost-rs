//! Example: AWS resource pricing with convenience API
//!
//! Run without API key (returns defaults):
//!   cargo run --example aws_pricing
//!
//! Run with API key (fetches live prices):
//!   INFRACOST_API_KEY="ico-xxx" cargo run --example aws_pricing

use infracost_rs::Client;
use infracost_rs::providers::aws::EbsType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let has_api_key = std::env::var("INFRACOST_API_KEY").is_ok();

    println!("=== AWS Pricing Example ===\n");
    println!(
        "API Key: {}\n",
        if has_api_key {
            "provided"
        } else {
            "not provided (using defaults)"
        }
    );

    let client = if has_api_key {
        Client::from_env()?
    } else {
        Client::anonymous()
    };

    // Test all EBS types
    println!("--- EBS Volume Pricing (per GB-month) ---");
    for ebs_type in [
        EbsType::Gp3,
        EbsType::Gp2,
        EbsType::Io2,
        EbsType::St1,
        EbsType::Sc1,
    ] {
        let result = client
            .aws()
            .ebs(ebs_type)
            .region("us-east-1")
            .fetch()
            .await?;
        println!(
            "{:?}: ${:.4}/{} (source: {:?})",
            ebs_type, result.price, result.unit, result.source
        );
    }

    // Test snapshot
    println!("\n--- EBS Snapshot Pricing (per GB-month) ---");
    let result = client.aws().snapshot().region("us-east-1").fetch().await?;
    println!(
        "Snapshot: ${:.4}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test snapshot monthly cost
    let result = client
        .aws()
        .snapshot()
        .region("us-east-1")
        .size_gb(100)
        .fetch_monthly()
        .await?;
    println!(
        "Snapshot 100GB: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: $0.05 × 100 GB");

    // Test Elastic IP
    println!("\n--- Elastic IP Pricing (per hour, idle) ---");
    let result = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch()
        .await?;
    println!(
        "Elastic IP: ${:.4}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test Elastic IP monthly cost
    let result = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch_monthly()
        .await?;
    println!(
        "Elastic IP: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: $0.005 × 730 hours");

    // Test NAT Gateway
    println!("\n--- NAT Gateway Pricing (per hour) ---");
    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch()
        .await?;
    println!(
        "NAT Gateway: ${:.4}/{} (~${:.2}/month) (source: {:?})",
        result.price,
        result.unit,
        result.price * 730.0,
        result.source
    );

    // Test Application Load Balancer
    println!("\n--- Application Load Balancer Pricing (per hour) ---");
    let result = client.aws().alb().region("us-east-1").fetch().await?;
    println!(
        "ALB: ${:.4}/{} (~${:.2}/month) (source: {:?})",
        result.price,
        result.unit,
        result.price * 730.0,
        result.source
    );

    // Test ALB monthly cost calculation
    println!("\n--- ALB Monthly Cost Calculation ---");

    // ALB with hourly cost only (no LCU specified)
    let result = client.aws().alb().fetch_monthly().await?;
    println!(
        "ALB (hourly only): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: $0.0225 × 730 hours");

    // ALB with LCU usage
    let result = client.aws().alb().lcu_hours(10000).fetch_monthly().await?;
    println!(
        "ALB + 10,000 LCU-hours: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: ($0.0225 × 730 hours) + ($0.008 × 10,000 LCU-hours)");

    // Test EBS monthly cost calculation
    println!("\n--- EBS Monthly Cost Calculation ---");

    // gp3 with full spec: 500 GB, 6000 IOPS, 250 MiBps
    let result = client
        .aws()
        .ebs(EbsType::Gp3)
        .size_gb(500)
        .iops(6000)
        .throughput_mibps(250)
        .fetch_monthly()
        .await?;
    println!(
        "gp3 500GB + 6000 IOPS + 250 MiBps: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: (500 × $0.08) + (3000 extra IOPS × $0.005) + (125 extra MiBps × $0.04)");

    // gp3 with baseline only
    let result = client
        .aws()
        .ebs(EbsType::Gp3)
        .size_gb(100)
        .fetch_monthly()
        .await?;
    println!(
        "gp3 100GB (baseline): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // gp2 (storage only, no provisioned IOPS)
    let result = client
        .aws()
        .ebs(EbsType::Gp2)
        .size_gb(100)
        .fetch_monthly()
        .await?;
    println!(
        "gp2 100GB: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // Test NAT Gateway monthly cost calculation
    println!("\n--- NAT Gateway Monthly Cost Calculation ---");

    // NAT Gateway with 1000 GB data processing
    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .data_processed_gb(1000)
        .fetch_monthly()
        .await?;
    println!(
        "NAT Gateway + 1000 GB data: ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: ($0.045 × 730 hours) + ($0.045 × 1000 GB) = $32.85 + $45.00");

    // NAT Gateway without data processing
    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch_monthly()
        .await?;
    println!(
        "NAT Gateway (hourly only): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );

    // io2 examples with tiered IOPS pricing
    println!("\n--- io2 Tiered IOPS Pricing ---");

    // io2 with tier 1 IOPS only
    let result = client
        .aws()
        .ebs(EbsType::Io2)
        .size_gb(100)
        .iops(10000)
        .fetch_monthly()
        .await?;
    println!(
        "io2 100GB + 10,000 IOPS (tier 1): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: (100 × $0.125) + (10,000 × $0.065)");

    // io2 with tier 1 and tier 2 IOPS
    let result = client
        .aws()
        .ebs(EbsType::Io2)
        .size_gb(100)
        .iops(50000)
        .fetch_monthly()
        .await?;
    println!(
        "io2 100GB + 50,000 IOPS (tier 1+2): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!("  Breakdown: (100 × $0.125) + (32,000 × $0.065) + (18,000 × $0.0455)");

    // io2 with all three tiers
    let result = client
        .aws()
        .ebs(EbsType::Io2)
        .size_gb(100)
        .iops(100000)
        .fetch_monthly()
        .await?;
    println!(
        "io2 100GB + 100,000 IOPS (all tiers): ${:.2}/{} (source: {:?})",
        result.price, result.unit, result.source
    );
    println!(
        "  Breakdown: (100 × $0.125) + (32,000 × $0.065) + (32,000 × $0.0455) + (36,000 × $0.03185)"
    );
    println!("  Note: io2 has NO baseline - all IOPS are billed");

    Ok(())
}
