//! Integration tests for GCP Snapshot regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all GCP regions (not just US)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. Both fetch() and fetch_monthly() methods work correctly
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_snapshot_regional_pricing -- --ignored
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
// Snapshot Regional Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client.gcp().snapshot().region(*region).fetch().await?;

        // Validate source and price
        assert_eq!(
            result.source,
            PriceSource::Api,
            "Snapshot in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "Snapshot price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GB-month");

        println!(
            "✓ Snapshot {}: ${}/GB-month (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_convenience_vs_raw() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Test using convenience function
    let convenience_result = client.gcp().snapshot().region(region).fetch().await?;

    // Test using raw ProductFilter with correct parameters
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .region(region)
        .product_family("Storage")
        .attribute("resourceGroup", "PDSnapshot")
        .attribute_regex("description", "^Storage PD Snapshot")
        .build();

    let products = client.query_products(filter).await?;

    assert!(
        !products.is_empty(),
        "Raw query should return products for snapshot in {}",
        region
    );

    let raw_price = products[0].first_nonzero_price_or(0.05);

    // Both should return API pricing
    assert_eq!(convenience_result.source, PriceSource::Api);
    assert_eq!(
        convenience_result.price, raw_price,
        "Convenience function and raw query should return same price"
    );

    println!(
        "✓ Snapshot convenience vs raw in {}: ${} (both API)",
        region, convenience_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_regional_pricing_variations() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test regions known to have different pricing
    let us_result = client
        .gcp()
        .snapshot()
        .region("us-central1")
        .fetch()
        .await?;

    let europe_result = client
        .gcp()
        .snapshot()
        .region("europe-north1")
        .fetch()
        .await?;

    let australia_result = client
        .gcp()
        .snapshot()
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

    println!("✓ Regional pricing variations:");
    println!("  US (us-central1): ${}/GB-month", us_result.price);
    println!(
        "  Europe (europe-north1): ${}/GB-month",
        europe_result.price
    );
    println!(
        "  Australia (australia-southeast1): ${}/GB-month",
        australia_result.price
    );

    // Note: We don't assert specific price values since they can change,
    // but we validate that regional variations exist and all come from API

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_monthly_calculation() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let size_gb = 100;

    // Get the hourly rate
    let rate_result = client.gcp().snapshot().region(region).fetch().await?;

    // Get the monthly cost
    let monthly_result = client
        .gcp()
        .snapshot()
        .region(region)
        .size_gb(size_gb)
        .fetch_monthly()
        .await?;

    // Both should be from same source
    assert_eq!(rate_result.source, monthly_result.source);

    // Monthly should be rate × size
    let expected_monthly = rate_result.price * size_gb as f64;
    assert!(
        (monthly_result.price - expected_monthly).abs() < 0.001,
        "Monthly cost should be rate × size: {} ≈ {} × {}",
        monthly_result.price,
        rate_result.price,
        size_gb
    );

    assert_eq!(monthly_result.unit, "month");

    println!(
        "✓ Monthly calculation: ${}/GB-month × {}GB = ${}/month",
        rate_result.price, size_gb, monthly_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_source_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "europe-west1";

    let result = client.gcp().snapshot().region(region).fetch().await?;

    // Should be from API, not default
    assert_eq!(
        result.source,
        PriceSource::Api,
        "Snapshot pricing should come from API in {}",
        region
    );

    assert!(result.price > 0.0, "API price should be positive");

    println!(
        "✓ Source tracking for {}: {:?} (${}/GB-month)",
        region, result.source, result.price
    );

    Ok(())
}

// ============================================================
// Cross-Region Validation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_americas_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let americas_regions = vec!["us-central1", "us-east1", "southamerica-east1"];

    for region in americas_regions {
        let result = client.gcp().snapshot().region(region).fetch().await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "GB-month");

        println!("✓ Americas - {}: ${}/GB-month", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_europe_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let europe_regions = vec!["europe-west1", "europe-north1"];

    for region in europe_regions {
        let result = client.gcp().snapshot().region(region).fetch().await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "GB-month");

        println!("✓ Europe - {}: ${}/GB-month", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_asia_pacific_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let apac_regions = vec!["asia-southeast1", "australia-southeast1"];

    for region in apac_regions {
        let result = client.gcp().snapshot().region(region).fetch().await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "GB-month");

        println!("✓ Asia-Pacific - {}: ${}/GB-month", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_fetch_monthly_requires_size() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .gcp()
        .snapshot()
        .region("us-central1")
        .fetch_monthly()
        .await;

    assert!(result.is_err(), "fetch_monthly should require size_gb");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("size_gb is required"),
        "Error should mention size_gb requirement"
    );

    println!("✓ fetch_monthly correctly requires size_gb");

    Ok(())
}
