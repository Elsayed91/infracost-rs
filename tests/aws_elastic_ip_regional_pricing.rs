//! Integration tests for AWS Elastic IP regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all AWS regions (not just us-east-1)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//!
//! Run with:
//! ```bash
//! cargo test --test aws_elastic_ip_regional_pricing -- --ignored
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

/// Test regions covering all major geographic areas:
/// - Americas: us-east-1, us-west-2, sa-east-1
/// - Europe: eu-west-1, eu-central-1
/// - Asia-Pacific: ap-southeast-1, ap-northeast-1
#[allow(dead_code)]
const TEST_REGIONS: &[&str] = &[
    "us-east-1",      // Americas: US East (N. Virginia)
    "us-west-2",      // Americas: US West (Oregon)
    "sa-east-1",      // Americas: South America (São Paulo)
    "eu-west-1",      // Europe: Ireland
    "eu-central-1",   // Europe: Frankfurt
    "ap-southeast-1", // Asia-Pacific: Singapore
    "ap-northeast-1", // Asia-Pacific: Tokyo
];

// ============================================================
// Per-Region Comparison Tests
// ============================================================
// These tests compare convenience function output to raw ProductFilter queries
// for each region, validating that the query parameters work universally.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_elastic_ip_us_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_elastic_ip_us_west_2() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-west-2").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_elastic_ip_sa_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("sa-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_elastic_ip_eu_west_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eu-west-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_elastic_ip_eu_central_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eu-central-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_elastic_ip_ap_southeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("ap-southeast-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_elastic_ip_ap_northeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("ap-northeast-1").await
}

/// Helper function to test pricing for a specific region
async fn test_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function
    let convenience_result = client.aws().elastic_ip().region(region).fetch().await?;

    // 2. Get price using raw ProductFilter with validated universal parameters
    // Using group="ElasticIP:Address" which works across all regions
    // (usagetype varies by region with prefixes like EU-, APS1-, etc.)
    let filter = ProductFilter::builder()
        .vendor("aws")
        .region(region)
        .product_family("IP Address")
        .attribute("group", "ElasticIP:Address")
        .attribute("servicecode", "AmazonEC2")
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query should return products for region: {}",
        region
    );

    let raw_price = products[0].first_nonzero_price_or(0.005);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "Elastic IP price mismatch for {}: convenience={}, raw={}",
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
async fn test_elastic_ip_source_tracking_across_regions() -> Result<(), Box<dyn std::error::Error>>
{
    let client = get_client()?;

    // Test a subset of regions to validate source tracking
    let test_regions = vec!["us-east-1", "eu-west-1", "ap-southeast-1"];

    for region in test_regions {
        let result = client.aws().elastic_ip().region(region).fetch().await?;

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
async fn test_elastic_ip_monthly_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get hourly price
    let hourly = client.aws().elastic_ip().region(region).fetch().await?;

    // Get monthly price
    let monthly = client
        .aws()
        .elastic_ip()
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
