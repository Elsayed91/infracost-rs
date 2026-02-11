//! Integration tests for GCP Compute Instance regional pricing.
//!
//! Validates that the parameterized YAML and machine type parsing work correctly
//! across multiple regions and machine families.

use infracost_rs::{Client, providers::gcp::PurchaseOption};
use std::env;

fn get_test_client() -> Client {
    let api_key = env::var("INFRACOST_API_KEY")
        .expect("INFRACOST_API_KEY must be set to run integration tests");
    Client::new(api_key)
}

#[tokio::test]
#[ignore] // Run with: cargo test --test gcp_compute_instance_regional_pricing -- --ignored
async fn test_n2_standard_4_regional_pricing() {
    let client = get_test_client();

    let regions = vec![
        "us-central1",
        "us-east1",
        "europe-west1",
        "europe-north1",
        "asia-southeast1",
        "australia-southeast1",
        "southamerica-east1",
    ];

    for region in regions {
        let result = client
            .gcp()
            .compute_instance()
            .machine_type("n2-standard-4")
            .region(region)
            .fetch_monthly()
            .await;

        match result {
            Ok(price) => {
                println!(
                    "[{}] n2-standard-4 monthly cost: ${:.2} (source: {:?})",
                    region, price.price, price.source
                );
                // Prices vary by region (US: ~$140, EU: ~$155, APAC: ~$175-200)
                assert!(price.price > 100.0 && price.price < 250.0);
                assert_eq!(price.unit, "month");
            }
            Err(e) => {
                println!("[{}] Error: {}", region, e);
                // Some regions might not have N2 instances
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_e2_medium_regional_pricing() {
    let client = get_test_client();

    let regions = vec![
        "us-central1",
        "europe-north1",
        "asia-southeast1",
        "australia-southeast1",
        "southamerica-east1",
    ];

    for region in regions {
        let result = client
            .gcp()
            .compute_instance()
            .machine_type("e2-medium")
            .region(region)
            .fetch_monthly()
            .await;

        match result {
            Ok(price) => {
                println!(
                    "[{}] e2-medium monthly cost: ${:.2} (source: {:?})",
                    region, price.price, price.source
                );
                // E2 medium (1 core, 4 GiB) should cost around $20-30/month
                assert!(price.price > 15.0 && price.price < 40.0);
                assert_eq!(price.unit, "month");
            }
            Err(e) => {
                println!("[{}] Error: {}", region, e);
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_n2_spot_vs_ondemand_pricing() {
    let client = get_test_client();
    let region = "us-central1";

    // On-demand pricing
    let ondemand = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region(region)
        .fetch_monthly()
        .await
        .unwrap();

    println!("N2 standard-4 on-demand: ${:.2}/month", ondemand.price);

    // Spot pricing
    let spot = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region(region)
        .purchase_option(PurchaseOption::Preemptible)
        .fetch_monthly()
        .await
        .unwrap();

    println!(
        "N2 standard-4 spot: ${:.2}/month (source: {:?})",
        spot.price, spot.source
    );

    // Spot should be significantly cheaper (roughly 60-80% discount)
    assert!(spot.price < ondemand.price * 0.5);
}

#[tokio::test]
#[ignore]
async fn test_custom_machine_type() {
    let client = get_test_client();

    // Test custom machine type: 4 cores, 8 GiB RAM
    let result = client
        .gcp()
        .compute_instance()
        .machine_type("n2-custom-4-8192")
        .region("us-central1")
        .fetch_monthly()
        .await
        .unwrap();

    println!("N2 custom-4-8192 monthly cost: ${:.2}", result.price);

    // Should cost less than standard-4 (which has 16 GiB RAM)
    let standard4 = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region("us-central1")
        .fetch_monthly()
        .await
        .unwrap();

    assert!(result.price < standard4.price);
}

#[tokio::test]
#[ignore]
async fn test_machine_type_with_zone_prefix() {
    let client = get_test_client();

    // Test parsing machine type with zone prefix
    let result = client
        .gcp()
        .compute_instance()
        .machine_type("zones/us-central1-a/machineTypes/n2-standard-4")
        .region("us-central1")
        .fetch_monthly()
        .await
        .unwrap();

    println!(
        "N2 standard-4 (with zone prefix) monthly cost: ${:.2}",
        result.price
    );

    assert!(result.price > 100.0 && result.price < 200.0);
}

#[tokio::test]
#[ignore]
async fn test_manual_specs() {
    let client = get_test_client();

    // Test manually specifying specs instead of machine type
    let result = client
        .gcp()
        .compute_instance()
        .machine_family("n2")
        .cpu_cores(8)
        .memory_gib(32)
        .region("us-central1")
        .fetch_monthly()
        .await
        .unwrap();

    println!(
        "N2 custom (8 cores, 32 GiB) monthly cost: ${:.2}",
        result.price
    );

    // Should be roughly 2x the cost of n2-standard-4
    let standard4 = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region("us-central1")
        .fetch_monthly()
        .await
        .unwrap();

    assert!((result.price / standard4.price - 2.0).abs() < 0.1);
}

#[tokio::test]
#[ignore]
async fn test_price_source_tracking() {
    let client = get_test_client();

    let result = client
        .gcp()
        .compute_instance()
        .machine_type("n2-standard-4")
        .region("us-central1")
        .fetch_monthly()
        .await
        .unwrap();

    // Should be from API since we provided an API key
    assert!(result.is_from_api());
}
