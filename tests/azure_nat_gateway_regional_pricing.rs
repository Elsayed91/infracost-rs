//! Integration tests for Azure NAT Gateway regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing behaves consistently across all Azure regions
//! 3. Source tracking is correct (PriceSource::Default, since Azure NAT Gateway
//!    pricing is global-only in the IRS API; standard regions always fall back
//!    to the default_price)
//! 4. Monthly conversion works correctly with and without data_processed_gb
//!
//! Key pricing constants (defaults):
//!   - Uptime:          $0.045/hr  -> $32.85/month (0.045 * 730)
//!   - Data processing: $0.045/GB
//!
//! Run with:
//! ```bash
//! cargo test --test azure_nat_gateway_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::{Client, ProductFilter};

// Seven Azure regions under test
const AZURE_TEST_REGIONS: &[&str] = &[
    "eastus",
    "westus2",
    "westeurope",
    "northeurope",
    "southeastasia",
    "japaneast",
    "brazilsouth",
];

const DEFAULT_UPTIME_HOURLY: f64 = 0.045;
const DEFAULT_DATA_PRICE_PER_GB: f64 = 0.045;
const HOURS_PER_MONTH: f64 = 730.0;

/// Helper to get a client with API key from environment.
fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let client = Client::from_env().map_err(|e| format!("INFRACOST_API_KEY must be set: {}", e))?;
    Ok(client)
}

// ============================================================
// Per-Region Comparison Tests
// ============================================================
// Azure NAT Gateway pricing is global-only in the IRS API.
// Standard region queries return no products, so the convenience
// builder falls back to default_price and PriceSource::Default.
// The raw ProductFilter query similarly returns no results for
// these regions, making both paths consistent.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_eastus() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eastus").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_westus2() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("westus2").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_westeurope() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("westeurope").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_northeurope() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("northeurope").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_southeastasia() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("southeastasia").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_japaneast() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("japaneast").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_brazilsouth() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("brazilsouth").await
}

/// Helper function to test pricing for a specific region.
///
/// Azure NAT Gateway is global-only in the IRS API, so standard region queries
/// fall back to the hardcoded default_price. Both the convenience builder and
/// a raw ProductFilter targeting the same region will miss live products and
/// use the default value, keeping them consistent.
async fn test_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function
    let convenience_result = client.azure().nat_gateway().region(region).fetch().await?;

    // 2. Get price using raw ProductFilter with the same attributes
    //    The query will return no products for non-Global regions, so both
    //    paths resolve to the default price of $0.045/hr.
    let filter = ProductFilter::builder()
        .vendor("azure")
        .service("NAT Gateway")
        .product_family("Networking")
        .attribute("productName", "NAT Gateway")
        .attribute("skuName", "Standard")
        .attribute("meterName", "Standard Gateway")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;

    // For standard regions the IRS returns no results; default fallback applies.
    let raw_price = if products.is_empty() {
        DEFAULT_UPTIME_HOURLY
    } else {
        products[0].first_nonzero_price_or(DEFAULT_UPTIME_HOURLY)
    };

    // 3. Compare results - both should equal the default uptime price
    assert_eq!(
        convenience_result.price, raw_price,
        "NAT Gateway price mismatch for {}: convenience={}, raw={}",
        region, convenience_result.price, raw_price
    );

    // 4. Validate source tracking - must be Default for standard regions
    assert_eq!(
        convenience_result.source,
        PriceSource::Default,
        "Expected Default source for region {} (global-only pricing), got {:?}",
        region,
        convenience_result.source
    );

    // 5. Validate price equals the documented default
    assert_eq!(
        convenience_result.price, DEFAULT_UPTIME_HOURLY,
        "NAT Gateway default uptime price should be ${}/hr for region {}",
        DEFAULT_UPTIME_HOURLY, region
    );

    // 6. Unit must be "hour"
    assert_eq!(
        convenience_result.unit, "hour",
        "Unit should be 'hour' for region {}, got '{}'",
        region, convenience_result.unit
    );

    println!(
        "Region {}: price={}, source={:?}",
        region, convenience_result.price, convenience_result.source
    );

    Ok(())
}

// ============================================================
// Source Tracking Validation
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_source_tracking_across_regions() -> Result<(), Box<dyn std::error::Error>>
{
    let client = get_client()?;

    // Validate that every test region returns PriceSource::Default
    for region in AZURE_TEST_REGIONS {
        let result = client
            .azure()
            .nat_gateway()
            .region(*region)
            .fetch()
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to fetch NAT Gateway price for region {}: {}",
                    region, e
                )
            });

        assert_eq!(
            result.source,
            PriceSource::Default,
            "Region {} should return Default pricing (global-only IRS data), got {:?}",
            region,
            result.source
        );

        assert_eq!(
            result.price, DEFAULT_UPTIME_HOURLY,
            "Region {} should return the default uptime price ${}/hr, got {}",
            region, DEFAULT_UPTIME_HOURLY, result.price
        );

        assert_eq!(
            result.unit, "hour",
            "Region {} unit should be 'hour', got '{}'",
            region, result.unit
        );

        println!(
            "Source tracking validated for {}: price={}, source={:?}",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Monthly Conversion - Uptime Only (no data_processed_gb)
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_monthly_uptime_only() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "eastus";

    // fetch() returns hourly price
    let hourly = client.azure().nat_gateway().region(region).fetch().await?;

    // fetch_monthly() without data_processed_gb => uptime only
    let monthly = client
        .azure()
        .nat_gateway()
        .region(region)
        .fetch_monthly()
        .await?;

    let expected_monthly = DEFAULT_UPTIME_HOURLY * HOURS_PER_MONTH; // 0.045 * 730 = 32.85

    assert_eq!(
        hourly.price, DEFAULT_UPTIME_HOURLY,
        "Hourly price should be ${}",
        DEFAULT_UPTIME_HOURLY
    );
    assert_eq!(hourly.unit, "hour");
    assert_eq!(hourly.source, PriceSource::Default);

    assert!(
        (monthly.price - expected_monthly).abs() < 0.001,
        "Monthly uptime-only price should be {}, got {}",
        expected_monthly,
        monthly.price
    );
    assert_eq!(monthly.unit, "month");
    assert_eq!(monthly.source, PriceSource::Default);

    println!(
        "Monthly (uptime only): hourly={}, monthly={}, expected={}",
        hourly.price, monthly.price, expected_monthly
    );

    Ok(())
}

// ============================================================
// Monthly Conversion - With data_processed_gb
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_monthly_with_data_processing() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "eastus";
    let data_gb: u64 = 1000;

    // Expected: ($0.045 * 730) + ($0.045 * 1000) = $32.85 + $45.00 = $77.85
    let expected_monthly =
        (DEFAULT_UPTIME_HOURLY * HOURS_PER_MONTH) + (DEFAULT_DATA_PRICE_PER_GB * data_gb as f64);

    let result = client
        .azure()
        .nat_gateway()
        .region(region)
        .data_processed_gb(data_gb)
        .fetch_monthly()
        .await?;

    assert!(
        (result.price - expected_monthly).abs() < 0.001,
        "Monthly cost with {} GB data should be {}, got {}",
        data_gb,
        expected_monthly,
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!(
        "Monthly (1000 GB data): price={}, expected={}",
        result.price, expected_monthly
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_monthly_with_zero_data_processing()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "westeurope";

    // Explicitly passing 0 GB should produce same result as uptime-only
    let result = client
        .azure()
        .nat_gateway()
        .region(region)
        .data_processed_gb(0)
        .fetch_monthly()
        .await?;

    let expected = DEFAULT_UPTIME_HOURLY * HOURS_PER_MONTH; // 32.85

    assert!(
        (result.price - expected).abs() < 0.001,
        "Monthly with 0 GB data should be {}, got {}",
        expected,
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!(
        "Monthly (0 GB data): price={}, expected={}",
        result.price, expected
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_monthly_data_adds_cost() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "southeastasia";

    // Cost without data processing
    let uptime_only = client
        .azure()
        .nat_gateway()
        .region(region)
        .fetch_monthly()
        .await?;

    // Cost with 500 GB data processing
    let with_data = client
        .azure()
        .nat_gateway()
        .region(region)
        .data_processed_gb(500)
        .fetch_monthly()
        .await?;

    // Cost with data should exceed uptime-only cost
    assert!(
        with_data.price > uptime_only.price,
        "Cost with data processing ({}) should exceed uptime-only cost ({})",
        with_data.price,
        uptime_only.price
    );

    let expected_data_component = DEFAULT_DATA_PRICE_PER_GB * 500.0; // 22.50
    let actual_data_component = with_data.price - uptime_only.price;

    assert!(
        (actual_data_component - expected_data_component).abs() < 0.001,
        "Data processing component should be {}, got {}",
        expected_data_component,
        actual_data_component
    );

    assert_eq!(uptime_only.unit, "month");
    assert_eq!(with_data.unit, "month");

    println!(
        "Data component validated: uptime_only={}, with_500gb={}, data_cost={}",
        uptime_only.price, with_data.price, actual_data_component
    );

    Ok(())
}

// ============================================================
// Cross-Region Monthly Consistency
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_monthly_consistent_across_regions()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let expected_monthly = DEFAULT_UPTIME_HOURLY * HOURS_PER_MONTH; // 32.85

    for region in AZURE_TEST_REGIONS {
        let result = client
            .azure()
            .nat_gateway()
            .region(*region)
            .fetch_monthly()
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to fetch monthly NAT Gateway price for region {}: {}",
                    region, e
                )
            });

        assert!(
            (result.price - expected_monthly).abs() < 0.001,
            "Region {} monthly price should be {}, got {}",
            region,
            expected_monthly,
            result.price
        );
        assert_eq!(
            result.unit, "month",
            "Region {} unit should be 'month'",
            region
        );
        assert_eq!(
            result.source,
            PriceSource::Default,
            "Region {} should have Default source",
            region
        );

        println!(
            "Monthly consistency validated for {}: price={}",
            region, result.price
        );
    }

    Ok(())
}
