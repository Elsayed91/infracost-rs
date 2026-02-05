//! Test io2 tiered IOPS pricing with API key
//!
//! Run with API key:
//!   INFRACOST_API_KEY="ico-xxx" cargo run --example test_io2_api

use infracost_rs::Client;
use infracost_rs::providers::aws::EbsType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    println!("Testing io2 with API key...\n");

    // Test with tier 1 only
    let result = client
        .aws()
        .ebs(EbsType::Io2)
        .region("us-east-1")
        .size_gb(100)
        .iops(10000)
        .fetch_monthly()
        .await?;

    println!("io2 100GB + 10,000 IOPS:");
    println!("  Price: ${:.2}/month", result.price);
    println!("  Source: {:?}", result.source);
    println!("  Expected: ~$662.50 (100*0.125 + 10000*0.065)");

    // Test with tier 1 and tier 2
    let result = client
        .aws()
        .ebs(EbsType::Io2)
        .region("us-east-1")
        .size_gb(100)
        .iops(50000)
        .fetch_monthly()
        .await?;

    println!("\nio2 100GB + 50,000 IOPS:");
    println!("  Price: ${:.2}/month", result.price);
    println!("  Source: {:?}", result.source);
    println!("  Expected: ~$2,911.50");

    // Test with all tiers
    let result = client
        .aws()
        .ebs(EbsType::Io2)
        .region("us-east-1")
        .size_gb(100)
        .iops(100000)
        .fetch_monthly()
        .await?;

    println!("\nio2 100GB + 100,000 IOPS:");
    println!("  Price: ${:.2}/month", result.price);
    println!("  Source: {:?}", result.source);
    println!("  Expected: ~$4,695.10");

    Ok(())
}
