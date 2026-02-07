//! Integration tests for GCP Static IP regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all GCP regions (not just US)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_static_ip_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::{Client, ProductFilter};

/// Helper to get a client with API key from environment
fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    // Try to load from .env file
    let _ = dotenvy::dotenv();

    let client = Client::from_env().map_err(|e| format!("INFRACOST_API_KEY must be set: {}", e))?;

    Ok(client)
}

// ============================================================
// Per-Region Comparison Tests
// ============================================================
// These tests compare convenience function output to raw ProductFilter queries
// for each region, validating that the query parameters work universally.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_us_central1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-central1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_us_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-east1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_southamerica_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("southamerica-east1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_europe_west1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("europe-west1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_europe_north1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("europe-north1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_asia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("asia-southeast1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_australia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("australia-southeast1").await
}

/// Helper function to test pricing for a specific region
async fn test_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function
    let convenience_result = client.gcp().static_ip().region(region).fetch().await?;

    // 2. Get price using raw ProductFilter with validated universal parameters
    // Using resourceGroup="IpAddress" which works across all regions
    // (description varies by region, but resourceGroup is universal)
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .product_family("Network")
        .attribute("resourceGroup", "IpAddress")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query should return products for region: {}",
        region
    );

    let raw_price = products[0].first_nonzero_price_or(0.01);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "Static IP price mismatch for {}: convenience={}, raw={}",
        region, convenience_result.price, raw_price
    );

    // 4. Validate source tracking (should be Api, not Default)
    assert_eq!(
        convenience_result.source,
        PriceSource::Api,
        "Expected API source for region {}, got {:?}",
        region,
        convenience_result.source
    );

    // 5. Validate price is positive (not falling back to default)
    assert!(
        convenience_result.price > 0.0,
        "Price should be positive for region {}",
        region
    );

    println!(
        "✓ Region {}: convenience={}, raw={}, source={:?}",
        region, convenience_result.price, raw_price, convenience_result.source
    );

    Ok(())
}

// ============================================================
// Source Tracking Validation Test
// ============================================================
// This test validates that we're getting real API prices (dynamic)
// and not falling back to hardcoded defaults, which is the core bug.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_source_tracking_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a subset of regions to validate source tracking
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        let result = client.gcp().static_ip().region(region).fetch().await?;

        // With valid API key, source should be Api, not Default
        assert_eq!(
            result.source,
            PriceSource::Api,
            "Region {} should return API pricing, not defaults. Got source: {:?}",
            region,
            result.source
        );

        // Price should be positive
        assert!(
            result.price > 0.0,
            "Price should be positive for region {}. Got: {}",
            region,
            result.price
        );

        println!(
            "✓ Source tracking validated for {}: price={}, source={:?}",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Monthly Conversion Test
// ============================================================
// Validate that fetch_monthly() correctly converts hourly to monthly

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_static_ip_monthly_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Get hourly price
    let hourly = client.gcp().static_ip().region(region).fetch().await?;

    // Get monthly price
    let monthly = client
        .gcp()
        .static_ip()
        .region(region)
        .fetch_monthly()
        .await?;

    // Monthly should be hourly × 730
    let expected_monthly = hourly.price * 730.0;
    assert!(
        (monthly.price - expected_monthly).abs() < 0.01,
        "Monthly price should be hourly × 730. Got monthly={}, expected={}",
        monthly.price,
        expected_monthly
    );

    assert_eq!(hourly.unit, "hour");
    assert_eq!(monthly.unit, "month");

    println!(
        "✓ Monthly conversion validated: hourly={}, monthly={} (expected={})",
        hourly.price, monthly.price, expected_monthly
    );

    Ok(())
}
