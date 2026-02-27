//! AWS pricing using the blocking (synchronous) API
//!
//! Demonstrates all AWS builders using `infracost_rs::blocking::Client`,
//! which is a synchronous wrapper suitable for non-async contexts.
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example aws_blocking
//! ```

use infracost_rs::blocking::Client;
use infracost_rs::providers::aws::{EbsType, RdsStorageType};

fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key)?,
        Err(_) => {
            println!("(No API key — using built-in defaults)\n");
            Client::anonymous()?
        }
    };

    println!("=== AWS Blocking API ===\n");

    // EBS
    let r = client.aws().ebs(EbsType::Gp3).region("us-east-1").fetch()?;
    println!(
        "EBS gp3         ${:.4}/{}  ({:?})",
        r.price, r.unit, r.source
    );

    let r = client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .size_gb(500)
        .iops(6000)
        .throughput_mibps(250)
        .fetch_monthly()?;
    println!(
        "EBS gp3 monthly ${:.2}/month (500 GB, 6000 IOPS, 250 MiBps)  ({:?})",
        r.price, r.source
    );

    // Snapshot
    let r = client
        .aws()
        .snapshot()
        .region("us-east-1")
        .size_gb(200)
        .fetch_monthly()?;
    println!(
        "Snapshot        ${:.2}/month (200 GB)  ({:?})",
        r.price, r.source
    );

    // Elastic IP
    let r = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch_monthly()?;
    println!("Elastic IP      ${:.2}/month  ({:?})", r.price, r.source);

    // NAT Gateway
    let r = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .data_processed_gb(500)
        .fetch_monthly()?;
    println!(
        "NAT Gateway     ${:.2}/month (500 GB)  ({:?})",
        r.price, r.source
    );

    // ALB
    let r = client
        .aws()
        .alb()
        .region("us-east-1")
        .lcu_hours(1000)
        .fetch_monthly()?;
    println!(
        "ALB             ${:.2}/month (1000 LCU-hr)  ({:?})",
        r.price, r.source
    );

    // EC2 Instance
    let r = client
        .aws()
        .ec2_instance("t3.micro")
        .region("us-east-1")
        .fetch_monthly()?;
    println!("EC2 t3.micro    ${:.2}/month  ({:?})", r.price, r.source);

    let r = client
        .aws()
        .ec2_instance("m5.xlarge")
        .region("us-east-1")
        .operating_system("Windows")
        .fetch_monthly()?;
    println!(
        "EC2 m5.xlarge   ${:.2}/month (Windows)  ({:?})",
        r.price, r.source
    );

    // RDS
    let r = client
        .aws()
        .rds("db.t3.micro")
        .region("us-east-1")
        .engine("mysql")
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .fetch_monthly()?;
    println!(
        "RDS db.t3.micro ${:.2}/month (MySQL, 100 GB gp3)  ({:?})",
        r.price, r.source
    );

    let r = client
        .aws()
        .rds("db.t3.micro")
        .region("us-east-1")
        .engine("postgres")
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .multi_az()
        .fetch_monthly()?;
    println!(
        "RDS db.t3.micro ${:.2}/month (Postgres, 100 GB gp3, Multi-AZ)  ({:?})",
        r.price, r.source
    );

    Ok(())
}
