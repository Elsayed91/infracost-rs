//! AWS EBS pricing from `aws ec2 describe-volumes` JSON output
//!
//! ```bash
//! # With real JSON from AWS CLI:
//! # aws ec2 describe-volumes --volume-ids vol-xxx --query 'Volumes[0]' | \
//! #   cargo run --example aws_ebs_from_json
//!
//! # With defaults (no API key needed):
//! cargo run --example aws_ebs_from_json
//! ```

use infracost_rs::Client;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key),
        Err(_) => {
            println!("(No API key — using built-in defaults)\n");
            Client::anonymous()
        }
    };

    // Simulated `aws ec2 describe-volumes` JSON output (gp3 with extras)
    let gp3_volume = serde_json::json!({
        "VolumeId": "vol-0123456789abcdef0",
        "VolumeType": "gp3",
        "AvailabilityZone": "us-east-1a",
        "Size": 500,
        "Iops": 6000,
        "Throughput": 250,
        "State": "in-use"
    });

    println!("=== AWS EBS from JSON ===\n");

    println!("gp3 (500 GB, 6000 IOPS, 250 MiB/s, us-east-1):");
    let r = client
        .aws()
        .ebs_from_json(&gp3_volume)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})\n", r.price, r.source);

    // Simulated io2 volume
    let io2_volume = serde_json::json!({
        "VolumeId": "vol-abcdef0123456789a",
        "VolumeType": "io2",
        "AvailabilityZone": "eu-west-1b",
        "Size": 200,
        "Iops": 10000,
        "State": "in-use"
    });

    println!("io2 (200 GB, 10000 IOPS, eu-west-1):");
    let r = client
        .aws()
        .ebs_from_json(&io2_volume)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})\n", r.price, r.source);

    // Simulated gp2 volume (baseline only)
    let gp2_volume = serde_json::json!({
        "VolumeId": "vol-aabbccdd11223344",
        "VolumeType": "gp2",
        "AvailabilityZone": "ap-southeast-1c",
        "Size": 100,
        "State": "in-use"
    });

    println!("gp2 (100 GB, ap-southeast-1):");
    let r = client
        .aws()
        .ebs_from_json(&gp2_volume)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})", r.price, r.source);

    Ok(())
}
