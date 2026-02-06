//! Integration tests for AWS NAT Gateway regional pricing.
//!
//! These tests verify that NAT Gateway pricing queries work correctly across all AWS regions
//! using universal attributes (productFamily="NAT Gateway") instead of region-specific usagetype.
//!
//! Run with: cargo test --test aws_nat_gateway_regional_pricing -- --include-ignored

use infracost_rs::Client;

/// Helper to get a client with API key from environment
fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    // Try to load from .env file
    let _ = dotenvy::dotenv();

    let client = Client::from_env().map_err(|e| format!("INFRACOST_API_KEY must be set: {}", e))?;

    Ok(client)
}

/// Test helper to verify API source
fn assert_from_api(result: &infracost_rs::PriceResult) {
    assert!(
        result.is_from_api(),
        "Expected API source, got {:?}",
        result.source
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_us_east_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch()
        .await
        .expect("Failed to fetch NAT Gateway price for us-east-1");

    assert_from_api(&result);
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_us_west_2() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .nat_gateway()
        .region("us-west-2")
        .fetch()
        .await
        .expect("Failed to fetch NAT Gateway price for us-west-2");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_sa_east_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .nat_gateway()
        .region("sa-east-1")
        .fetch()
        .await
        .expect("Failed to fetch NAT Gateway price for sa-east-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_eu_west_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .nat_gateway()
        .region("eu-west-1")
        .fetch()
        .await
        .expect("Failed to fetch NAT Gateway price for eu-west-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_eu_central_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .nat_gateway()
        .region("eu-central-1")
        .fetch()
        .await
        .expect("Failed to fetch NAT Gateway price for eu-central-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_ap_southeast_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .nat_gateway()
        .region("ap-southeast-1")
        .fetch()
        .await
        .expect("Failed to fetch NAT Gateway price for ap-southeast-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_ap_northeast_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .nat_gateway()
        .region("ap-northeast-1")
        .fetch()
        .await
        .expect("Failed to fetch NAT Gateway price for ap-northeast-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_monthly_cost_calculation() {
    let client = get_client().expect("Failed to create client");

    // Test NAT Gateway with data processing
    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .data_processed_gb(1000)
        .fetch_monthly()
        .await
        .expect("Failed to fetch NAT Gateway monthly cost");

    assert_from_api(&result);
    assert_eq!(result.unit, "month");
    // Should be (hourly * 730) + (data_price * 1000)
    // Roughly ($0.045 * 730) + ($0.045 * 1000) = $32.85 + $45.00 = $77.85
    assert!(
        result.price > 75.0 && result.price < 80.0,
        "Monthly cost with data processing should be around $77.85, got {}",
        result.price
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_source_tracking_across_regions() {
    let client = get_client().expect("Failed to create client");

    // Test multiple regions to ensure API source is correctly tracked
    let regions = vec!["us-east-1", "eu-west-1", "ap-southeast-1"];

    for region in regions {
        let result = client
            .aws()
            .nat_gateway()
            .region(region)
            .fetch()
            .await
            .unwrap_or_else(|e| panic!("Failed to fetch NAT Gateway price for {}: {}", region, e));

        assert_from_api(&result);
        assert_eq!(result.unit, "hour");
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_monthly_hourly_only() {
    let client = get_client().expect("Failed to create client");

    // Test NAT Gateway monthly cost without data processing (hourly only)
    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch_monthly()
        .await
        .expect("Failed to fetch NAT Gateway monthly cost");

    assert_from_api(&result);
    assert_eq!(result.unit, "month");
    // Should be hourly * 730 hours
    // Roughly $0.045 * 730 = $32.85
    assert!(
        result.price > 30.0 && result.price < 35.0,
        "Monthly cost without data processing should be around $32.85, got {}",
        result.price
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_nat_gateway_data_processing_component() {
    let client = get_client().expect("Failed to create client");

    // Test with zero data processing
    let result_zero = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .data_processed_gb(0)
        .fetch_monthly()
        .await
        .expect("Failed to fetch NAT Gateway monthly cost with zero data");

    assert_from_api(&result_zero);
    assert_eq!(result_zero.unit, "month");
    // Should be hourly * 730 (no data processing)
    assert!(result_zero.price > 30.0 && result_zero.price < 35.0);

    // Test with data processing
    let result_data = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .data_processed_gb(100)
        .fetch_monthly()
        .await
        .expect("Failed to fetch NAT Gateway monthly cost with data");

    assert_from_api(&result_data);
    assert_eq!(result_data.unit, "month");
    // Should be higher than hourly-only cost
    assert!(
        result_data.price > result_zero.price,
        "Cost with data processing should be higher than hourly-only"
    );
}
