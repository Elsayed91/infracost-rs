//! Integration tests for AWS EBS Snapshot regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all AWS regions (not just us-east-1)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. Monthly cost calculation works correctly (rate × size_gb)
//!
//! Run with:
//! ```bash
//! cargo test --test aws_snapshot_regional_pricing -- --ignored
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
async fn test_snapshot_us_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_us_west_2() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-west-2").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_sa_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("sa-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_eu_west_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eu-west-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_eu_central_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eu-central-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_ap_southeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("ap-southeast-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_ap_northeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("ap-northeast-1").await
}

/// Helper function to test pricing for a specific region
async fn test_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function
    let convenience_result = client.aws().snapshot().region(region).fetch().await?;

    // 2. Get price using raw ProductFilter with validated universal parameters
    // Using productFamily="Storage Snapshot" which works across all regions
    // (usagetype varies by region with prefixes like EU-, APS1-, etc.)
    let filter = ProductFilter::builder()
        .vendor("aws")
        .region(region)
        .product_family("Storage Snapshot")
        .attribute("servicecode", "AmazonEC2")
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query should return products for region: {}",
        region
    );

    // Filter for standard snapshot (same logic as snapshot.rs)
    let standard_snapshot = products.iter().find(|p| {
        p.attributes.iter().any(|attr| {
            attr.key == "usagetype"
                && attr
                    .value
                    .as_ref()
                    .map(|v| {
                        v.ends_with("EBS:SnapshotUsage")
                            && !v.contains("Archive")
                            && !v.contains("outposts")
                    })
                    .unwrap_or(false)
        })
    });

    assert!(
        standard_snapshot.is_some(),
        "Should find standard snapshot for region: {}",
        region
    );

    let raw_price = standard_snapshot
        .map(|p| p.first_nonzero_price_or(0.05))
        .unwrap_or(0.05);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "Snapshot price mismatch for {}: convenience={}, raw={}",
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

    // 6. Validate unit is GB-month
    assert_eq!(convenience_result.unit, "GB-month");

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
async fn test_snapshot_source_tracking_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a subset of regions to validate source tracking
    let test_regions = vec!["us-east-1", "eu-west-1", "ap-southeast-1"];

    for region in test_regions {
        let result = client.aws().snapshot().region(region).fetch().await?;

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
// Monthly Cost Calculation Test
// ============================================================
// Validate that fetch_monthly() correctly calculates total cost (rate × size_gb)

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_monthly_cost_calculation() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get per-GB rate
    let rate = client.aws().snapshot().region(region).fetch().await?;

    // Get monthly cost for 100 GB snapshot
    let monthly = client
        .aws()
        .snapshot()
        .region(region)
        .size_gb(100)
        .fetch_monthly()
        .await?;

    // Monthly should be rate × size_gb
    let expected_monthly = rate.price * 100.0;
    assert!(
        (monthly.price - expected_monthly).abs() < 0.01,
        "Monthly cost should be rate × size_gb. Got monthly={}, expected={}",
        monthly.price,
        expected_monthly
    );

    assert_eq!(rate.unit, "GB-month");
    assert_eq!(monthly.unit, "month");

    println!(
        "✓ Monthly cost calculation validated: rate={}, 100GB monthly={} (expected={})",
        rate.price, monthly.price, expected_monthly
    );

    Ok(())
}
