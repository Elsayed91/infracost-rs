//! AWS RDS pricing (instance + storage, single-AZ and multi-AZ)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example aws_rds
//! ```

use infracost_rs::Client;
use infracost_rs::providers::aws::RdsStorageType;

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = if let Ok(key) = std::env::var("INFRACOST_API_KEY") {
        Client::new(key)?
    } else {
        println!("(No API key — using built-in defaults)\n");
        Client::anonymous()?
    };

    let regions = [
        ("us-east-1", "Americas"),
        ("eu-west-1", "Europe"),
        ("ap-southeast-1", "Asia-Pacific"),
    ];

    println!("=== AWS RDS ===\n");

    println!("Hourly instance price (db.t3.micro, MySQL, Single-AZ):");
    for (region, geo) in &regions {
        let r = client
            .aws()
            .rds("db.t3.micro")
            .engine("mysql")
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/hr  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly cost by engine (db.t3.micro, 100 GB gp3, us-east-1):");
    for engine in ["mysql", "postgres", "mariadb"] {
        let r = client
            .aws()
            .rds("db.t3.micro")
            .engine(engine)
            .region("us-east-1")
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .fetch_monthly()
            .await?;
        println!("  {:<12} ${:.2}/month  ({:?})", engine, r.price, r.source);
    }

    println!("\nMonthly cost by storage type (db.t3.micro, 100 GB, us-east-1):");
    for (storage_type, label) in [
        (RdsStorageType::Gp2, "gp2"),
        (RdsStorageType::Gp3, "gp3"),
        (RdsStorageType::Io1, "io1 (1000 IOPS)"),
        (RdsStorageType::Magnetic, "magnetic"),
    ] {
        let mut builder = client
            .aws()
            .rds("db.t3.micro")
            .region("us-east-1")
            .storage_type(storage_type)
            .allocated_storage_gb(100);
        if storage_type == RdsStorageType::Io1 {
            builder = builder.iops(1000);
        }
        let r = builder.fetch_monthly().await?;
        println!("  {:<20} ${:.2}/month  ({:?})", label, r.price, r.source);
    }

    println!("\nSingle-AZ vs Multi-AZ (db.t3.micro, 100 GB gp3, us-east-1):");
    let single = client
        .aws()
        .rds("db.t3.micro")
        .region("us-east-1")
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .fetch_monthly()
        .await?;
    let multi = client
        .aws()
        .rds("db.t3.micro")
        .region("us-east-1")
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .multi_az()
        .fetch_monthly()
        .await?;
    println!("  Single-AZ  ${:.2}/month", single.price);
    println!("  Multi-AZ   ${:.2}/month", multi.price);

    Ok(())
}
