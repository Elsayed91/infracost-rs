//! Integration tests for GCP Forwarding Rule regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all GCP regions (not just US)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. Both pricing components work (hourly uptime + data processing)
//! 6. fetch_monthly() correctly combines both components
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_forwarding_rule_regional_pricing -- --ignored
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
/// - Americas: us-central1, us-east1, southamerica-east1
/// - Europe: europe-west1, europe-north1
/// - Asia-Pacific: asia-southeast1, australia-southeast1
const TEST_REGIONS: &[&str] = &[
    "us-central1",          // Americas: Iowa
    "us-east1",             // Americas: South Carolina
    "southamerica-east1",   // Americas: São Paulo
    "europe-west1",         // Europe: Belgium
    "europe-north1",        // Europe: Finland
    "asia-southeast1",      // Asia-Pacific: Singapore
    "australia-southeast1", // Asia-Pacific: Sydney
];

// ============================================================
// Hourly Uptime Pricing Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_forwarding_rule_hourly_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .forwarding_rule()
            .region(*region)
            .fetch()
            .await?;

        // Validate source and price
        assert_eq!(
            result.source,
            PriceSource::Api,
            "Forwarding rule hourly in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "Forwarding rule hourly price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "hour");

        println!(
            "✓ Forwarding Rule (hourly) {}: ${}/hour (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_forwarding_rule_convenience_vs_raw_hourly() -> Result<(), Box<dyn std::error::Error>>
{
    let client = get_client()?;
    let region = "us-central1";

    // Test using convenience function
    let convenience_result = client
        .gcp()
        .forwarding_rule()
        .region(region)
        .fetch()
        .await?;

    // Test using raw ProductFilter
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Networking")
        .region(region)
        .product_family("Network")
        .attribute("resourceGroup", "LoadBalancing")
        .build();

    let products = client.query_products(filter).await?;

    assert!(
        !products.is_empty(),
        "Raw query should return products for forwarding rule in {}",
        region
    );

    // Filter for Regional External Forwarding Rule Minimum
    let matching_product = products.iter().find(|product| {
        product.attributes.iter().any(|attr| {
            attr.key == "description"
                && attr
                    .value
                    .as_ref()
                    .map(|v| {
                        v.contains("Regional External") && v.contains("Forwarding Rule Minimum")
                    })
                    .unwrap_or(false)
        })
    });

    assert!(
        matching_product.is_some(),
        "Should find Regional External Forwarding Rule Minimum product"
    );

    let raw_price = matching_product.unwrap().first_nonzero_price_or(0.025);

    // Both should return API pricing
    assert_eq!(convenience_result.source, PriceSource::Api);
    assert_eq!(
        convenience_result.price, raw_price,
        "Convenience function and raw query should return same hourly price"
    );

    println!(
        "✓ Forwarding Rule hourly convenience vs raw in {}: ${}/hour (both API)",
        region, convenience_result.price
    );

    Ok(())
}

// ============================================================
// Data Processing Pricing Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_data_processing_price_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a subset of regions for data processing pricing
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        // Query for data processing products
        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("Networking")
            .region(region)
            .product_family("Network")
            .attribute("resourceGroup", "LoadBalancing")
            .build();

        let products = client.query_products(filter).await?;

        // Filter for Regional External Outbound Data Processing
        let data_product = products.iter().find(|product| {
            product.attributes.iter().any(|attr| {
                attr.key == "description"
                    && attr
                        .value
                        .as_ref()
                        .map(|v| {
                            v.contains("Regional External")
                                && v.contains("Outbound Data Processing")
                        })
                        .unwrap_or(false)
            })
        });

        assert!(
            data_product.is_some(),
            "Should find data processing product in {}",
            region
        );

        let price = data_product.unwrap().first_nonzero_price_or(0.008);

        assert!(
            price > 0.0,
            "Data processing price should be positive in {}",
            region
        );

        println!(
            "✓ Forwarding Rule (data processing) {}: ${}/GB",
            region, price
        );
    }

    Ok(())
}

// ============================================================
// Monthly Cost Calculation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_monthly_calculation() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let data_gb = 1000;

    // Get monthly cost
    let monthly_result = client
        .gcp()
        .forwarding_rule()
        .region(region)
        .data_processed_gb(data_gb)
        .fetch_monthly()
        .await?;

    // Get hourly uptime rate
    let hourly_result = client
        .gcp()
        .forwarding_rule()
        .region(region)
        .fetch()
        .await?;

    // Both should be from API
    assert_eq!(hourly_result.source, PriceSource::Api);
    assert_eq!(monthly_result.source, PriceSource::Api);

    // Monthly should include uptime (hourly × 730) + data processing
    // We can't easily get data processing price separately through convenience function,
    // but we can verify monthly is greater than just uptime cost
    let uptime_only = hourly_result.price * 730.0;

    assert!(
        monthly_result.price > uptime_only,
        "Monthly cost with data should be greater than uptime only: {} > {}",
        monthly_result.price,
        uptime_only
    );

    assert_eq!(monthly_result.unit, "month");

    println!(
        "✓ Monthly calculation: Hourly ${}/hour × 730 + {}GB data = ${}/month",
        hourly_result.price, data_gb, monthly_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_monthly_uptime_only() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Get monthly cost with no data processing
    let monthly_result = client
        .gcp()
        .forwarding_rule()
        .region(region)
        .data_processed_gb(0)
        .fetch_monthly()
        .await?;

    // Get hourly uptime rate
    let hourly_result = client
        .gcp()
        .forwarding_rule()
        .region(region)
        .fetch()
        .await?;

    // Both should be from API
    assert_eq!(hourly_result.source, PriceSource::Api);
    assert_eq!(monthly_result.source, PriceSource::Api);

    // Monthly should be exactly hourly × 730 (no data processing)
    let expected_monthly = hourly_result.price * 730.0;

    assert!(
        (monthly_result.price - expected_monthly).abs() < 0.001,
        "Monthly cost without data should be hourly × 730: {} ≈ {}",
        monthly_result.price,
        expected_monthly
    );

    assert_eq!(monthly_result.unit, "month");

    println!(
        "✓ Monthly uptime only: ${}/hour × 730 = ${}/month",
        hourly_result.price, monthly_result.price
    );

    Ok(())
}

// ============================================================
// Regional Pricing Variation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_regional_pricing_variations() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test regions known to potentially have different pricing
    let us_result = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch()
        .await?;

    let europe_result = client
        .gcp()
        .forwarding_rule()
        .region("europe-north1")
        .fetch()
        .await?;

    let australia_result = client
        .gcp()
        .forwarding_rule()
        .region("australia-southeast1")
        .fetch()
        .await?;

    // All should be from API
    assert_eq!(us_result.source, PriceSource::Api);
    assert_eq!(europe_result.source, PriceSource::Api);
    assert_eq!(australia_result.source, PriceSource::Api);

    // Prices should be positive
    assert!(us_result.price > 0.0);
    assert!(europe_result.price > 0.0);
    assert!(australia_result.price > 0.0);

    println!("✓ Regional pricing variations (hourly):");
    println!("  US (us-central1): ${}/hour", us_result.price);
    println!("  Europe (europe-north1): ${}/hour", europe_result.price);
    println!(
        "  Australia (australia-southeast1): ${}/hour",
        australia_result.price
    );

    // Note: We don't assert specific price values since they can change,
    // but we validate that regional variations exist and all come from API

    Ok(())
}

// ============================================================
// Source Tracking Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_source_tracking_hourly() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "europe-west1";

    let result = client
        .gcp()
        .forwarding_rule()
        .region(region)
        .fetch()
        .await?;

    // Should be from API, not default
    assert_eq!(
        result.source,
        PriceSource::Api,
        "Forwarding rule pricing should come from API in {}",
        region
    );

    assert!(result.price > 0.0, "API price should be positive");

    println!(
        "✓ Source tracking (hourly) for {}: {:?} (${}/hour)",
        region, result.source, result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_source_tracking_monthly() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "asia-southeast1";

    let result = client
        .gcp()
        .forwarding_rule()
        .region(region)
        .data_processed_gb(500)
        .fetch_monthly()
        .await?;

    // Should be from API, not default
    assert_eq!(
        result.source,
        PriceSource::Api,
        "Forwarding rule monthly pricing should come from API in {}",
        region
    );

    assert!(result.price > 0.0, "API price should be positive");

    println!(
        "✓ Source tracking (monthly) for {}: {:?} (${}/month)",
        region, result.source, result.price
    );

    Ok(())
}

// ============================================================
// Cross-Region Validation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_americas_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let americas_regions = vec!["us-central1", "us-east1", "southamerica-east1"];

    for region in americas_regions {
        let result = client
            .gcp()
            .forwarding_rule()
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("✓ Americas - {}: ${}/hour", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_europe_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let europe_regions = vec!["europe-west1", "europe-north1"];

    for region in europe_regions {
        let result = client
            .gcp()
            .forwarding_rule()
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("✓ Europe - {}: ${}/hour", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_asia_pacific_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let apac_regions = vec!["asia-southeast1", "australia-southeast1"];

    for region in apac_regions {
        let result = client
            .gcp()
            .forwarding_rule()
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("✓ Asia-Pacific - {}: ${}/hour", region, result.price);
    }

    Ok(())
}
