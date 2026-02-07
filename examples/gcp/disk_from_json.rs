//! GCP Disk pricing from `gcloud compute disks describe` JSON output
//!
//! ```bash
//! # With real JSON from gcloud:
//! # gcloud compute disks describe my-disk --format=json | cargo run --example gcp_disk_from_json
//!
//! # With defaults (no API key needed):
//! cargo run --example gcp_disk_from_json
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

    // Simulated `gcloud compute disks describe` JSON output (zonal disk)
    let zonal_disk = serde_json::json!({
        "type": "projects/my-project/zones/us-central1-a/diskTypes/pd-ssd",
        "zone": "projects/my-project/zones/us-central1-a",
        "sizeGb": "500",
        "status": "READY",
        "name": "my-disk"
    });

    println!("=== GCP Disk from JSON ===\n");

    println!("Zonal PD-SSD (500 GB, us-central1):");
    let r = client
        .gcp()
        .disk_from_json(&zonal_disk)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})\n", r.price, r.source);

    // Simulated regional disk (replicated across zones = 2x price)
    let regional_disk = serde_json::json!({
        "type": "projects/my-project/regions/us-central1/diskTypes/pd-ssd",
        "region": "https://www.googleapis.com/compute/v1/projects/my-project/regions/us-central1",
        "sizeGb": "500",
        "replicaZones": [
            "projects/my-project/zones/us-central1-a",
            "projects/my-project/zones/us-central1-b"
        ],
        "status": "READY",
        "name": "my-regional-disk"
    });

    println!("Regional PD-SSD (500 GB, us-central1, 2x price):");
    let r = client
        .gcp()
        .disk_from_json(&regional_disk)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})\n", r.price, r.source);

    // Hyperdisk with provisioned IOPS and throughput
    let hyperdisk = serde_json::json!({
        "type": "projects/my-project/zones/us-central1-a/diskTypes/hyperdisk-balanced",
        "zone": "projects/my-project/zones/us-central1-a",
        "sizeGb": "1000",
        "provisionedIops": "10000",
        "provisionedThroughput": "500",
        "status": "READY",
        "name": "my-hyperdisk"
    });

    println!("Hyperdisk Balanced (1000 GB, 10000 IOPS, 500 MB/s):");
    let r = client
        .gcp()
        .disk_from_json(&hyperdisk)?
        .fetch_monthly()
        .await?;
    println!("  ${:.2}/month  ({:?})", r.price, r.source);

    Ok(())
}
