//! Integration tests for AWS ALB regional pricing.
//!
//! These tests verify that ALB pricing queries work correctly across all AWS regions
//! using universal attributes (productFamily + operation) instead of region-specific usagetype.
//!
//! Run with: cargo test --test aws_alb_regional_pricing -- --include-ignored

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
async fn test_alb_us_east_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .alb()
        .region("us-east-1")
        .fetch()
        .await
        .expect("Failed to fetch ALB price for us-east-1");

    assert_from_api(&result);
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_us_west_2() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .alb()
        .region("us-west-2")
        .fetch()
        .await
        .expect("Failed to fetch ALB price for us-west-2");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_sa_east_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .alb()
        .region("sa-east-1")
        .fetch()
        .await
        .expect("Failed to fetch ALB price for sa-east-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_eu_west_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .alb()
        .region("eu-west-1")
        .fetch()
        .await
        .expect("Failed to fetch ALB price for eu-west-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_eu_central_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .alb()
        .region("eu-central-1")
        .fetch()
        .await
        .expect("Failed to fetch ALB price for eu-central-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_ap_southeast_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .alb()
        .region("ap-southeast-1")
        .fetch()
        .await
        .expect("Failed to fetch ALB price for ap-southeast-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_ap_northeast_1() {
    let client = get_client().expect("Failed to create client");
    let result = client
        .aws()
        .alb()
        .region("ap-northeast-1")
        .fetch()
        .await
        .expect("Failed to fetch ALB price for ap-northeast-1");

    assert_from_api(&result);
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_monthly_cost_calculation() {
    let client = get_client().expect("Failed to create client");

    // Test ALB with LCU hours
    let result = client
        .aws()
        .alb()
        .region("us-east-1")
        .lcu_hours(10000)
        .fetch_monthly()
        .await
        .expect("Failed to fetch ALB monthly cost");

    assert_from_api(&result);
    assert_eq!(result.unit, "month");
    // Should be (hourly * 730) + (lcu_price * 10000)
    // Roughly ($0.0225 * 730) + ($0.008 * 10000) = $16.425 + $80 = $96.425
    assert!(
        result.price > 90.0 && result.price < 100.0,
        "Monthly cost with LCU should be around $96, got {}",
        result.price
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_source_tracking_across_regions() {
    let client = get_client().expect("Failed to create client");

    // Test multiple regions to ensure API source is correctly tracked
    let regions = vec!["us-east-1", "eu-west-1", "ap-southeast-1"];

    for region in regions {
        let result = client
            .aws()
            .alb()
            .region(region)
            .fetch()
            .await
            .unwrap_or_else(|e| panic!("Failed to fetch ALB price for {}: {}", region, e));

        assert_from_api(&result);
        assert_eq!(result.unit, "hour");
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_monthly_hourly_only() {
    let client = get_client().expect("Failed to create client");

    // Test ALB monthly cost without LCU hours (hourly only)
    let result = client
        .aws()
        .alb()
        .region("us-east-1")
        .fetch_monthly()
        .await
        .expect("Failed to fetch ALB monthly cost");

    assert_from_api(&result);
    assert_eq!(result.unit, "month");
    // Should be hourly * 730 hours
    // Roughly $0.0225 * 730 = $16.425
    assert!(
        result.price > 15.0 && result.price < 18.0,
        "Monthly cost without LCU should be around $16.43, got {}",
        result.price
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_alb_lcu_pricing_component() {
    let client = get_client().expect("Failed to create client");

    // Test minimal LCU usage (730 LCU-hours = 1 LCU for whole month)
    let result = client
        .aws()
        .alb()
        .region("us-east-1")
        .lcu_hours(730)
        .fetch_monthly()
        .await
        .expect("Failed to fetch ALB monthly cost with minimal LCU");

    assert_from_api(&result);
    assert_eq!(result.unit, "month");
    // Should be (hourly * 730) + (lcu_price * 730)
    // Roughly ($0.0225 * 730) + ($0.008 * 730) = $16.425 + $5.84 = $22.265
    assert!(
        result.price > 20.0 && result.price < 25.0,
        "Monthly cost with 730 LCU-hours should be around $22.27, got {}",
        result.price
    );
}
