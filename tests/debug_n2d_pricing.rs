//! Debug N2D pricing issue

use infracost_rs::{Client, providers::gcp::PurchaseOption};
use std::env;

#[tokio::test]
#[ignore]
async fn debug_n2d_description_filters() {
    let api_key = env::var("INFRACOST_API_KEY").expect("INFRACOST_API_KEY must be set");
    let client = Client::new(api_key).unwrap();

    println!("\n=== Testing N2D On-Demand ===");
    let n2d_ondemand_cpu = client
        .gcp()
        .compute_instance()
        .machine_type("n2d-standard-4")
        .region("us-central1")
        .fetch()
        .await;

    match n2d_ondemand_cpu {
        Ok(result) => println!(
            "N2D OnDemand CPU: ${:.6}/hour (source: {:?})",
            result.price, result.source
        ),
        Err(e) => println!("N2D OnDemand ERROR: {}", e),
    }

    println!("\n=== Testing N2D Preemptible ===");
    let n2d_spot_cpu = client
        .gcp()
        .compute_instance()
        .machine_type("n2d-standard-4")
        .region("us-central1")
        .purchase_option(PurchaseOption::Preemptible)
        .fetch()
        .await;

    match n2d_spot_cpu {
        Ok(result) => println!(
            "N2D Preemptible CPU: ${:.6}/hour (source: {:?})",
            result.price, result.source
        ),
        Err(e) => println!("N2D Preemptible ERROR: {}", e),
    }

    println!("\n=== Testing C2D On-Demand ===");
    let c2d_ondemand = client
        .gcp()
        .compute_instance()
        .machine_type("c2d-standard-4")
        .region("us-central1")
        .fetch()
        .await;

    match c2d_ondemand {
        Ok(result) => println!(
            "C2D OnDemand CPU: ${:.6}/hour (source: {:?})",
            result.price, result.source
        ),
        Err(e) => println!("C2D OnDemand ERROR: {}", e),
    }

    println!("\n=== Testing N1 Preemptible ===");
    let n1_spot = client
        .gcp()
        .compute_instance()
        .machine_type("n1-standard-4")
        .region("us-central1")
        .purchase_option(PurchaseOption::Preemptible)
        .fetch()
        .await;

    match n1_spot {
        Ok(result) => println!(
            "N1 Preemptible CPU: ${:.6}/hour (source: {:?})",
            result.price, result.source
        ),
        Err(e) => println!("N1 Preemptible ERROR: {}", e),
    }
}
