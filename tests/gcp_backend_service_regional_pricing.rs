//! Integration tests for GCP Backend Service regional pricing.
//!
//! These tests validate that:
//! 1. Backend service pricing works across all GCP regions (not just US)
//! 2. Both Premium and Standard tiers return valid prices
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. fetch_monthly() correctly calculates data processing costs
//! 5. Regional pricing variations exist and return API-sourced prices
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_backend_service_regional_pricing -- --ignored
//! ```

use infracost_rs::Client;
use infracost_rs::providers::PriceSource;
use infracost_rs::providers::gcp::BackendServiceTier;

/// Helper to get a client with API key from environment
fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let client = Client::from_env().map_err(|e| format!("INFRACOST_API_KEY must be set: {}", e))?;
    Ok(client)
}

/// Test regions covering all major geographic areas:
/// - Americas: us-central1, us-east1, southamerica-east1
/// - Europe: europe-west1, europe-north1
/// - Asia-Pacific: asia-southeast1, australia-southeast1
const TEST_REGIONS: &[&str] = &[
    "us-central1",
    "us-east1",
    "southamerica-east1",
    "europe-west1",
    "europe-north1",
    "asia-southeast1",
    "australia-southeast1",
];

// ============================================================
// Premium Tier Pricing Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_backend_service_premium_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "Backend service premium in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "Backend service premium price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GiB");

        println!(
            "Premium {}: ${}/GiB (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Standard Tier Pricing Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_backend_service_standard_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Standard)
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "Backend service standard in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "Backend service standard price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GiB");

        println!(
            "Standard {}: ${}/GiB (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Monthly Cost Calculation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_monthly_premium_calculation() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let data_gb: u64 = 1000;

    // Get per-GiB rate
    let unit_result = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region(region)
        .fetch()
        .await?;

    // Get monthly cost
    let monthly_result = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region(region)
        .data_processed_gb(data_gb)
        .fetch_monthly()
        .await?;

    // Both should be from API
    assert_eq!(unit_result.source, PriceSource::Api);
    assert_eq!(monthly_result.source, PriceSource::Api);

    // Monthly cost should be unit_price * data_gb
    let expected_monthly = unit_result.price * data_gb as f64;
    assert!(
        (monthly_result.price - expected_monthly).abs() < 0.001,
        "Monthly cost should be rate * GB: {} ~ {}",
        monthly_result.price,
        expected_monthly
    );

    assert_eq!(monthly_result.unit, "month");

    println!(
        "Monthly premium: ${}/GiB x {} GB = ${}/month",
        unit_result.price, data_gb, monthly_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_monthly_standard_calculation() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let data_gb: u64 = 500;

    // Get per-GiB rate
    let unit_result = client
        .gcp()
        .backend_service(BackendServiceTier::Standard)
        .region(region)
        .fetch()
        .await?;

    // Get monthly cost
    let monthly_result = client
        .gcp()
        .backend_service(BackendServiceTier::Standard)
        .region(region)
        .data_processed_gb(data_gb)
        .fetch_monthly()
        .await?;

    // Both should be from API
    assert_eq!(unit_result.source, PriceSource::Api);
    assert_eq!(monthly_result.source, PriceSource::Api);

    // Monthly cost should be unit_price * data_gb
    let expected_monthly = unit_result.price * data_gb as f64;
    assert!(
        (monthly_result.price - expected_monthly).abs() < 0.001,
        "Monthly cost should be rate * GB: {} ~ {}",
        monthly_result.price,
        expected_monthly
    );

    assert_eq!(monthly_result.unit, "month");

    println!(
        "Monthly standard: ${}/GiB x {} GB = ${}/month",
        unit_result.price, data_gb, monthly_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_monthly_zero_data() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region("us-central1")
        .data_processed_gb(0)
        .fetch_monthly()
        .await?;

    assert_eq!(result.price, 0.0, "Zero data should result in zero cost");
    assert_eq!(result.unit, "month");

    println!("Zero data: ${}/month", result.price);

    Ok(())
}

// ============================================================
// Tier Comparison Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_premium_vs_standard_pricing() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    let premium = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region(region)
        .fetch()
        .await?;

    let standard = client
        .gcp()
        .backend_service(BackendServiceTier::Standard)
        .region(region)
        .fetch()
        .await?;

    assert_eq!(premium.source, PriceSource::Api);
    assert_eq!(standard.source, PriceSource::Api);

    // Both should be positive
    assert!(premium.price > 0.0);
    assert!(standard.price > 0.0);

    // Both tiers have identical data processing pricing per region
    // The distinction is network routing (global vs regional), not cost
    assert!(
        (premium.price - standard.price).abs() < 0.001,
        "Premium and Standard tier prices should be identical in {}: {} vs {}",
        region,
        premium.price,
        standard.price
    );

    println!("Tier comparison in {}:", region);
    println!("  Premium: ${}/GiB", premium.price);
    println!("  Standard: ${}/GiB", standard.price);

    Ok(())
}

// ============================================================
// Regional Pricing Variation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_regional_pricing_variations() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let us_result = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region("us-central1")
        .fetch()
        .await?;

    let europe_result = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region("europe-north1")
        .fetch()
        .await?;

    let australia_result = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region("australia-southeast1")
        .fetch()
        .await?;

    // All should be from API
    assert_eq!(us_result.source, PriceSource::Api);
    assert_eq!(europe_result.source, PriceSource::Api);
    assert_eq!(australia_result.source, PriceSource::Api);

    // All should be positive
    assert!(us_result.price > 0.0);
    assert!(europe_result.price > 0.0);
    assert!(australia_result.price > 0.0);

    println!("Regional pricing variations (premium):");
    println!("  US (us-central1): ${}/GiB", us_result.price);
    println!("  Europe (europe-north1): ${}/GiB", europe_result.price);
    println!(
        "  Australia (australia-southeast1): ${}/GiB",
        australia_result.price
    );

    Ok(())
}

// ============================================================
// Source Tracking Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_source_tracking_premium() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .gcp()
        .backend_service(BackendServiceTier::Premium)
        .region("europe-west1")
        .fetch()
        .await?;

    assert_eq!(
        result.source,
        PriceSource::Api,
        "Backend service premium should come from API"
    );
    assert!(result.price > 0.0);

    println!(
        "Source tracking premium: {:?} (${}/GiB)",
        result.source, result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_source_tracking_monthly() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .gcp()
        .backend_service(BackendServiceTier::Standard)
        .region("asia-southeast1")
        .data_processed_gb(500)
        .fetch_monthly()
        .await?;

    assert_eq!(
        result.source,
        PriceSource::Api,
        "Backend service monthly should come from API"
    );
    assert!(result.price > 0.0);

    println!(
        "Source tracking monthly: {:?} (${}/month)",
        result.source, result.price
    );

    Ok(())
}
