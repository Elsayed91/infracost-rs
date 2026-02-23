//! Regression baseline test: captures actual API prices for comparison.
//!
//! Run with: cargo test --test regression_baseline -- --ignored
//!
//! This records prices from the API to verify no regressions during refactoring.

use infracost_rs::Client;
use infracost_rs::providers::aws::EbsType;
use infracost_rs::providers::azure::{ManagedDiskSize, ManagedDiskType};
use infracost_rs::providers::gcp::{BackendServiceTier, DiskType, SnapshotType};

fn get_client() -> Client {
    let _ = dotenvy::dotenv();
    Client::from_env().expect("INFRACOST_API_KEY must be set")
}

/// Captures all prices and prints them for comparison.
/// Run before and after refactoring to verify no regressions.
#[tokio::test]
#[ignore = "Requires API key"]
async fn regression_snapshot_all_prices() {
    let client = get_client();
    let mut results = Vec::new();

    // === GCP ===

    // GCP Disks
    for disk_type in [
        DiskType::PdStandard,
        DiskType::PdSsd,
        DiskType::PdBalanced,
        DiskType::PdExtreme,
    ] {
        let r = client
            .gcp()
            .disk(disk_type)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();
        results.push(format!(
            "gcp/disk/{:?}: price={}, unit={}, source={:?}",
            disk_type, r.price, r.unit, r.source
        ));
    }

    // GCP Disk monthly (pd-ssd 500GB)
    let r = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .size_gb(500)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "gcp/disk/pd-ssd/monthly/500gb: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Snapshot (standard)
    let r = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "gcp/snapshot/standard: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Snapshot standard monthly (100GB)
    let r = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .size_gb(100)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "gcp/snapshot/standard/monthly/100gb: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Snapshot (archive)
    let r = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region("us-central1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "gcp/snapshot/archive: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Snapshot archive monthly (100GB with 50GB retrieval)
    let r = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region("us-central1")
        .size_gb(100)
        .retrieval_size_gb(50)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "gcp/snapshot/archive/monthly/100gb-50gb-retrieval: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Static IP
    let r = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "gcp/static-ip: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Static IP monthly
    let r = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "gcp/static-ip/monthly: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP NAT Gateway
    let r = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "gcp/nat-gateway: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP NAT Gateway monthly (1000GB data)
    let r = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .data_processed_gb(1000)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "gcp/nat-gateway/monthly/1000gb: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Forwarding Rule
    let r = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "gcp/forwarding-rule: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // GCP Backend Service (Premium + Standard)
    let r = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region("us-central1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "gcp/backend-service/premium: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    let r = client
        .gcp()
        .backend_service(BackendServiceTier::Standard)
        .region("us-central1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "gcp/backend-service/standard: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // === AWS ===

    // AWS EBS types
    for ebs_type in [
        EbsType::Gp3,
        EbsType::Gp2,
        EbsType::Io2,
        EbsType::St1,
        EbsType::Sc1,
    ] {
        let r = client
            .aws()
            .ebs(ebs_type)
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();
        results.push(format!(
            "aws/ebs/{:?}: price={}, unit={}, source={:?}",
            ebs_type, r.price, r.unit, r.source
        ));
    }

    // AWS EBS gp3 monthly (500GB, 6000 IOPS, 250 MiBps)
    let r = client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .size_gb(500)
        .iops(6000)
        .throughput_mibps(250)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "aws/ebs/gp3/monthly/500gb-6000iops-250mibps: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // AWS EBS io2 monthly (100GB, 50000 IOPS)
    let r = client
        .aws()
        .ebs(EbsType::Io2)
        .region("us-east-1")
        .size_gb(100)
        .iops(50000)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "aws/ebs/io2/monthly/100gb-50000iops: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // AWS Snapshot
    let r = client
        .aws()
        .snapshot()
        .region("us-east-1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "aws/snapshot: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // AWS Elastic IP
    let r = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "aws/elastic-ip: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // AWS NAT Gateway
    let r = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "aws/nat-gateway: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // AWS NAT Gateway monthly (1000GB)
    let r = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .data_processed_gb(1000)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "aws/nat-gateway/monthly/1000gb: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // AWS ALB
    let r = client
        .aws()
        .alb()
        .region("us-east-1")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "aws/alb: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // AWS ALB monthly (10000 LCU-hours)
    let r = client
        .aws()
        .alb()
        .region("us-east-1")
        .lcu_hours(10000)
        .fetch_monthly()
        .await
        .unwrap();
    results.push(format!(
        "aws/alb/monthly/10000lcu: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // === Azure ===

    // Azure Managed Disk
    let r = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "azure/managed-disk/premium-ssd/p10: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    let r = client
        .azure()
        .managed_disk(ManagedDiskType::StandardSsd, ManagedDiskSize::E10)
        .region("eastus")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "azure/managed-disk/standard-ssd/e10: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    let r = client
        .azure()
        .managed_disk(ManagedDiskType::StandardHdd, ManagedDiskSize::S10)
        .region("eastus")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "azure/managed-disk/standard-hdd/s10: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // Azure Snapshot
    let r = client
        .azure()
        .snapshot()
        .region("eastus")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "azure/snapshot: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // Azure Public IP
    let r = client
        .azure()
        .public_ip()
        .region("eastus")
        .fetch()
        .await
        .unwrap();
    results.push(format!(
        "azure/public-ip: price={}, unit={}, source={:?}",
        r.price, r.unit, r.source
    ));

    // Print all results
    println!("\n=== REGRESSION BASELINE ===");
    for line in &results {
        println!("{}", line);
    }
    println!("=== {} total price points ===\n", results.len());
}
