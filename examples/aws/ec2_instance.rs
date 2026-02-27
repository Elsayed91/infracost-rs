//! AWS EC2 Instance pricing (on-demand, hourly and monthly)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example aws_ec2_instance
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
        ("us-east-1", "Americas"),
        ("eu-west-1", "Europe"),
        ("ap-southeast-1", "Asia-Pacific"),
    ];

    println!("=== AWS EC2 Instance ===\n");

    println!("Hourly on-demand price (t3.micro, Linux):");
    for (region, geo) in &regions {
        let r = client
            .aws()
            .ec2_instance("t3.micro")
            .region(*region)
            .fetch()
            .await?;
        println!("  {:<15} ${:.4}/hr  ({:?})", geo, r.price, r.source);
    }

    println!("\nMonthly cost by instance type (us-east-1, Linux):");
    for instance_type in ["t3.micro", "t3.medium", "m5.xlarge", "c5.2xlarge"] {
        let r = client
            .aws()
            .ec2_instance(instance_type)
            .region("us-east-1")
            .fetch_monthly()
            .await?;
        println!(
            "  {:<15} ${:.2}/month  ({:?})",
            instance_type, r.price, r.source
        );
    }

    println!("\nOS comparison (m5.xlarge, us-east-1):");
    for os in ["Linux", "Windows", "RHEL", "SUSE"] {
        let r = client
            .aws()
            .ec2_instance("m5.xlarge")
            .region("us-east-1")
            .operating_system(os)
            .fetch()
            .await?;
        println!("  {:<10} ${:.4}/hr  ({:?})", os, r.price, r.source);
    }

    Ok(())
}
