//! GCP pricing using the blocking (synchronous) API
//!
//! Demonstrates all GCP builders using `infracost_rs::blocking::Client`,
//! which is a synchronous wrapper suitable for non-async contexts.
//!
//! ```bash
//! INFRACOST_API_KEY=xxx cargo run --example gcp_blocking
//! ```

use infracost_rs::blocking::Client;
use infracost_rs::providers::gcp::{
    BackendServiceTier, CloudSqlAvailability, CloudSqlEngine, DiskType, PurchaseOption,
    SnapshotType,
};

fn main() -> infracost_rs::Result<()> {
    let client = match std::env::var("INFRACOST_API_KEY") {
        Ok(key) => Client::new(key)?,
        Err(_) => {
            println!("(No API key — using built-in defaults)\n");
            Client::anonymous()?
        }
    };

    println!("=== GCP Blocking API ===\n");

    // Persistent Disk
    let r = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .size_gb(100)
        .fetch_monthly()?;
    println!(
        "PD SSD          ${:.2}/month (100 GB)  ({:?})",
        r.price, r.source
    );

    let r = client
        .gcp()
        .disk(DiskType::HyperdiskExtreme)
        .region("us-central1")
        .size_gb(500)
        .iops(15000)
        .fetch_monthly()?;
    println!(
        "Hyperdisk Ext   ${:.2}/month (500 GB, 15000 IOPS)  ({:?})",
        r.price, r.source
    );

    // Snapshot
    let r = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .size_gb(200)
        .fetch_monthly()?;
    println!(
        "Snapshot std    ${:.2}/month (200 GB)  ({:?})",
        r.price, r.source
    );

    let r = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region("us-central1")
        .size_gb(200)
        .retrieval_size_gb(50)
        .fetch_monthly()?;
    println!(
        "Snapshot arch   ${:.2}/month (200 GB + 50 GB retrieval)  ({:?})",
        r.price, r.source
    );

    // Static IP
    let r = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch_monthly()?;
    println!("Static IP       ${:.2}/month  ({:?})", r.price, r.source);

    // NAT Gateway
    let r = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .data_processed_gb(500)
        .fetch_monthly()?;
    println!(
        "NAT Gateway     ${:.2}/month (500 GB)  ({:?})",
        r.price, r.source
    );

    // Forwarding Rule
    let r = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .data_processed_gb(500)
        .fetch_monthly()?;
    println!(
        "Forwarding Rule ${:.2}/month (500 GB)  ({:?})",
        r.price, r.source
    );

    // Backend Service
    let r = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region("us-central1")
        .forwarding_rules(1)
        .data_processed_gb(500)
        .fetch_monthly()?;
    println!(
        "Backend Svc     ${:.2}/month (1 rule, 500 GB, Premium)  ({:?})",
        r.price, r.source
    );

    // Cloud SQL
    let r = client
        .gcp()
        .cloud_sql()
        .engine(CloudSqlEngine::PostgreSql)
        .availability(CloudSqlAvailability::Zonal)
        .region("us-central1")
        .cpu_count(4)
        .memory_gb(16)
        .storage_gb(100)
        .fetch_monthly()?;
    println!(
        "Cloud SQL PG    ${:.2}/month (4 vCPU, 16 GB, 100 GB, Zonal)  ({:?})",
        r.price, r.source
    );

    // BigQuery Storage
    let r = client
        .gcp()
        .bigquery_storage()
        .region("us-central1")
        .active_logical_storage_gb(500)
        .long_term_logical_storage_gb(200)
        .fetch_monthly()?;
    println!(
        "BigQuery        ${:.2}/month (500 GB active, 200 GB long-term)  ({:?})",
        r.price, r.source
    );

    // Compute Instance
    let r = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region("us-central1")
        .fetch_monthly()?;
    println!(
        "Compute n2-std4 ${:.2}/month (on-demand)  ({:?})",
        r.price, r.source
    );

    let r = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region("us-central1")
        .purchase_option(PurchaseOption::Preemptible)
        .fetch_monthly()?;
    println!(
        "Compute n2-std4 ${:.2}/month (preemptible)  ({:?})",
        r.price, r.source
    );

    let r = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region("us-central1")
        .purchase_option(PurchaseOption::Commit1Yr)
        .fetch_monthly()?;
    println!(
        "Compute n2-std4 ${:.2}/month (1-yr CUD)  ({:?})",
        r.price, r.source
    );

    Ok(())
}
