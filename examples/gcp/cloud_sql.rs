//! GCP Cloud SQL pricing (MySQL, PostgreSQL, SQL Server)
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_cloud_sql
//! ```

use infracost_rs::Client;
use infracost_rs::providers::gcp::{CloudSqlAvailability, CloudSqlEngine};

#[tokio::main]
async fn main() -> infracost_rs::Result<()> {
    let client = if let Ok(key) = std::env::var("INFRACOST_API_KEY") {
        Client::new(key)?
    } else {
        println!("(No API key — using built-in defaults)\n");
        Client::anonymous()?
    };

    let regions = [
        ("us-central1", "Americas"),
        ("europe-west1", "Europe"),
        ("asia-southeast1", "Asia-Pacific"),
    ];

    println!("=== GCP Cloud SQL ===\n");

    println!("CPU hourly rate by engine (Zonal):");
    for engine in [
        CloudSqlEngine::MySql,
        CloudSqlEngine::PostgreSql,
        CloudSqlEngine::SqlServer,
    ] {
        println!("  {:?}:", engine);
        for (region, geo) in &regions {
            let r = client
                .gcp()
                .cloud_sql()
                .engine(engine)
                .availability(CloudSqlAvailability::Zonal)
                .region(*region)
                .fetch()
                .await?;
            println!("    {:<15} ${:.4}/vCPU-hr  ({:?})", geo, r.price, r.source);
        }
    }

    println!("\nMonthly cost by size (PostgreSQL, Zonal, us-central1):");
    for (cpus, mem_gb, storage_gb, label) in [
        (2, 8, 50, "small  (2 vCPU, 8 GB, 50 GB)"),
        (4, 16, 100, "medium (4 vCPU, 16 GB, 100 GB)"),
        (8, 32, 500, "large  (8 vCPU, 32 GB, 500 GB)"),
    ] {
        let r = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .availability(CloudSqlAvailability::Zonal)
            .region("us-central1")
            .cpu_count(cpus)
            .memory_gb(mem_gb)
            .storage_gb(storage_gb)
            .fetch_monthly()
            .await?;
        println!("  {}  ${:.2}/month  ({:?})", label, r.price, r.source);
    }

    println!("\nZonal vs Regional HA (PostgreSQL, 4 vCPU, 16 GB, 100 GB, us-central1):");
    for (availability, label) in [
        (CloudSqlAvailability::Zonal, "Zonal"),
        (CloudSqlAvailability::Regional, "Regional (HA)"),
    ] {
        let r = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .availability(availability)
            .region("us-central1")
            .cpu_count(4)
            .memory_gb(16)
            .storage_gb(100)
            .fetch_monthly()
            .await?;
        println!("  {:<15}  ${:.2}/month  ({:?})", label, r.price, r.source);
    }

    Ok(())
}
