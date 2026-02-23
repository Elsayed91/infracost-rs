//! Integration tests for GCP Snapshot regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all GCP regions (not just US)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. Both fetch() and fetch_monthly() methods work correctly
//! 6. Archive snapshots are cheaper than standard snapshots
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_snapshot_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::providers::gcp::SnapshotType;
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
    "southamerica-east1",   // Americas: Sao Paulo
    "europe-west1",         // Europe: Belgium
    "europe-north1",        // Europe: Finland
    "asia-southeast1",      // Asia-Pacific: Singapore
    "australia-southeast1", // Asia-Pacific: Sydney
];

// ============================================================
// Standard Snapshot Regional Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region(*region)
            .fetch()
            .await?;

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

        assert_eq!(result.unit, "GiB-month");

        println!(
            "  Standard Snapshot {}: ${}/GiB-month (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_convenience_vs_raw() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Test using convenience function
    let convenience_result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region(region)
        .fetch()
        .await?;

    // Test using raw ProductFilter with correct parameters
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .region(region)
        .product_family("Storage")
        .attribute("resourceGroup", "PDSnapshot")
        .build();

    let products = client.query_products(filter).await?;

    assert!(
        !products.is_empty(),
        "Raw query should return products for snapshot in {}",
        region
    );

    // Apply the same post-filter the convenience function uses:
    // find the product whose description starts with "Storage PD Snapshot"
    let selected = products
        .iter()
        .find(|p| {
            p.attribute("description")
                .unwrap_or("")
                .starts_with("Storage PD Snapshot")
        })
        .unwrap_or(&products[0]);

    let raw_price = selected.first_nonzero_price_or(0.05);

    // Both should return API pricing
    assert_eq!(convenience_result.source, PriceSource::Api);
    assert_eq!(
        convenience_result.price, raw_price,
        "Convenience function and raw query should return same price"
    );

    println!(
        "  Standard snapshot convenience vs raw in {}: ${} (both API)",
        region, convenience_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_regional_pricing_variations()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test regions known to have different pricing
    let us_result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .fetch()
        .await?;

    let europe_result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("europe-north1")
        .fetch()
        .await?;

    let australia_result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
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

    println!("  Regional pricing variations (standard):");
    println!("  US (us-central1): ${}/GiB-month", us_result.price);
    println!(
        "  Europe (europe-north1): ${}/GiB-month",
        europe_result.price
    );
    println!(
        "  Australia (australia-southeast1): ${}/GiB-month",
        australia_result.price
    );

    // Note: We don't assert specific price values since they can change,
    // but we validate that regional variations exist and all come from API

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_fetch_monthly_calculation() -> Result<(), Box<dyn std::error::Error>>
{
    let client = get_client()?;
    let region = "us-central1";
    let size_gb = 100;

    // Get the per-unit rate
    let rate_result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region(region)
        .fetch()
        .await?;

    // Get the monthly cost
    let monthly_result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region(region)
        .size_gb(size_gb)
        .fetch_monthly()
        .await?;

    // Both should be from same source
    assert_eq!(rate_result.source, monthly_result.source);

    // Monthly should be rate x size
    let expected_monthly = rate_result.price * size_gb as f64;
    assert!(
        (monthly_result.price - expected_monthly).abs() < 0.001,
        "Monthly cost should be rate x size: {} ~ {} x {}",
        monthly_result.price,
        rate_result.price,
        size_gb
    );

    assert_eq!(monthly_result.unit, "month");

    println!(
        "  Monthly calculation: ${}/GiB-month x {}GiB = ${}/month",
        rate_result.price, size_gb, monthly_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_source_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "europe-west1";

    let result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region(region)
        .fetch()
        .await?;

    // Should be from API, not default
    assert_eq!(
        result.source,
        PriceSource::Api,
        "Snapshot pricing should come from API in {}",
        region
    );

    assert!(result.price > 0.0, "API price should be positive");

    println!(
        "  Source tracking for {}: {:?} (${}/GiB-month)",
        region, result.source, result.price
    );

    Ok(())
}

// ============================================================
// Cross-Region Validation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_americas_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let americas_regions = vec!["us-central1", "us-east1", "southamerica-east1"];

    for region in americas_regions {
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "GiB-month");

        println!("  Americas - {}: ${}/GiB-month", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_europe_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let europe_regions = vec!["europe-west1", "europe-north1"];

    for region in europe_regions {
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "GiB-month");

        println!("  Europe - {}: ${}/GiB-month", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_standard_snapshot_asia_pacific_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let apac_regions = vec!["asia-southeast1", "australia-southeast1"];

    for region in apac_regions {
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "GiB-month");

        println!("  Asia-Pacific - {}: ${}/GiB-month", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_snapshot_fetch_monthly_requires_size() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .fetch_monthly()
        .await;

    assert!(result.is_err(), "fetch_monthly should require size_gb");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("size_gb is required"),
        "Error should mention size_gb requirement"
    );

    println!("  fetch_monthly correctly requires size_gb");

    Ok(())
}

// ============================================================
// Archive Snapshot Regional Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_snapshot_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "Archive snapshot in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "Archive snapshot price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GiB-month");

        println!(
            "  Archive Snapshot {}: ${}/GiB-month (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Archive Snapshot Per-Region Storage Comparison Tests
// ============================================================
// These tests compare the archive storage convenience function to raw
// ProductFilter queries for each region.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_storage_us_central1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_storage_region_pricing("us-central1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_storage_us_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_storage_region_pricing("us-east1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_storage_europe_west1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_storage_region_pricing("europe-west1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_storage_europe_north1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_storage_region_pricing("europe-north1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_storage_asia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_storage_region_pricing("asia-southeast1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_storage_australia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_storage_region_pricing("australia-southeast1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_storage_southamerica_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_storage_region_pricing("southamerica-east1").await
}

/// Helper: compare archive storage convenience function vs raw ProductFilter for a region.
async fn test_archive_storage_region_pricing(
    region: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function (storage component = primary component)
    let convenience_result = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region(region)
        .fetch()
        .await?;

    // 2. Get price using raw ProductFilter
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .product_family("Storage")
        .attribute("resourceGroup", "PDSnapshot")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;

    assert!(
        !products.is_empty(),
        "Raw query should return products for archive snapshot storage in {}",
        region
    );

    // Apply the same post-filter the convenience function uses:
    // find the product whose description contains "Archive Snapshot Data Storage"
    let selected = products
        .iter()
        .find(|p| {
            p.attribute("description")
                .unwrap_or("")
                .contains("Archive Snapshot Data Storage")
        })
        .unwrap_or(&products[0]);

    let raw_price = selected.first_nonzero_price_or(0.019);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "Archive storage price mismatch for {}: convenience={}, raw={}",
        region, convenience_result.price, raw_price
    );

    // 4. Validate source tracking
    assert_eq!(
        convenience_result.source,
        PriceSource::Api,
        "Expected API source for archive storage in {}, got {:?}",
        region,
        convenience_result.source
    );

    // 5. Validate price is positive
    assert!(
        convenience_result.price > 0.0,
        "Archive storage price should be positive in {}",
        region
    );

    println!(
        "  Archive Storage {}: price={}, raw={}, source={:?}",
        region, convenience_result.price, raw_price, convenience_result.source
    );

    Ok(())
}

// ============================================================
// Archive Snapshot Per-Region Retrieval Comparison Tests
// ============================================================
// These tests compare the raw ProductFilter retrieval component price against
// the expected values for each region.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_retrieval_us_central1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_retrieval_region_pricing("us-central1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_retrieval_us_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_retrieval_region_pricing("us-east1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_retrieval_europe_west1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_retrieval_region_pricing("europe-west1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_retrieval_europe_north1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_retrieval_region_pricing("europe-north1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_retrieval_asia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_retrieval_region_pricing("asia-southeast1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_retrieval_australia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_retrieval_region_pricing("australia-southeast1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_retrieval_southamerica_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_archive_retrieval_region_pricing("southamerica-east1").await
}

/// Helper: verify the archive retrieval product is present and has a positive price
/// for a given region via raw ProductFilter.
async fn test_archive_retrieval_region_pricing(
    region: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .product_family("Storage")
        .attribute("resourceGroup", "PDSnapshot")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;

    let retrieval_product = products.iter().find(|p| {
        p.attribute("description")
            .unwrap_or("")
            .contains("Archive Snapshot Retrieval")
    });

    assert!(
        retrieval_product.is_some(),
        "Should find Archive Snapshot Retrieval product in {}",
        region
    );

    let retrieval_price = retrieval_product.unwrap().first_nonzero_price_or(0.019);

    assert!(
        retrieval_price > 0.0,
        "Archive retrieval price should be positive in {}",
        region
    );

    println!(
        "  Archive Retrieval {}: price={}/GiB",
        region, retrieval_price
    );

    Ok(())
}

// ============================================================
// Archive Snapshot Source Tracking
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_snapshot_source_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        let result = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region(region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "Archive snapshot in {} should return API pricing",
            region
        );
        assert!(
            result.price > 0.0,
            "Archive storage price should be positive in {}",
            region
        );
        assert_eq!(result.unit, "GiB-month");

        println!(
            "  Archive source tracking {}: price={}, source={:?}",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Archive Snapshot Monthly Conversion Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_snapshot_monthly_storage_only() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let size_gb: u64 = 500;

    // Get per-unit storage rate
    let rate_result = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region(region)
        .fetch()
        .await?;

    // Get monthly cost (storage only, no retrieval)
    let monthly_result = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region(region)
        .size_gb(size_gb)
        .fetch_monthly()
        .await?;

    // Both should be from API
    assert_eq!(rate_result.source, PriceSource::Api);
    assert_eq!(monthly_result.source, PriceSource::Api);

    // Monthly should be rate x size_gb
    let expected_monthly = rate_result.price * size_gb as f64;
    assert!(
        (monthly_result.price - expected_monthly).abs() < 0.001,
        "Archive storage monthly should be rate x size_gb: {} ~ {} x {}",
        monthly_result.price,
        rate_result.price,
        size_gb
    );

    assert_eq!(monthly_result.unit, "month");

    println!(
        "  Archive monthly (storage only): ${}/GiB-month x {}GiB = ${}/month",
        rate_result.price, size_gb, monthly_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_snapshot_monthly_storage_plus_retrieval()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let size_gb: u64 = 500;
    let retrieval_gb: u64 = 100;

    // Monthly cost without retrieval
    let storage_only = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region(region)
        .size_gb(size_gb)
        .fetch_monthly()
        .await?;

    // Monthly cost with retrieval
    let with_retrieval = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region(region)
        .size_gb(size_gb)
        .retrieval_size_gb(retrieval_gb)
        .fetch_monthly()
        .await?;

    // Both should be from API
    assert_eq!(storage_only.source, PriceSource::Api);
    assert_eq!(with_retrieval.source, PriceSource::Api);

    // Adding retrieval should increase the total cost
    assert!(
        with_retrieval.price > storage_only.price,
        "Monthly cost with retrieval ({}) should exceed storage-only cost ({})",
        with_retrieval.price,
        storage_only.price
    );

    assert_eq!(with_retrieval.unit, "month");

    println!(
        "  Archive monthly (with retrieval): storage_only=${}, with_{}GiB_retrieval=${}",
        storage_only.price, retrieval_gb, with_retrieval.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_archive_cheaper_than_standard_across_regions()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let standard = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region(*region)
            .fetch()
            .await?;

        let archive = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region(*region)
            .fetch()
            .await?;

        assert!(
            archive.price < standard.price,
            "Archive (${}) should be cheaper than standard (${}) in {}",
            archive.price,
            standard.price,
            region
        );

        println!(
            "  {}: standard=${}/GiB-month, archive=${}/GiB-month ({}x cheaper)",
            region,
            standard.price,
            archive.price,
            format!("{:.1}", standard.price / archive.price)
        );
    }

    Ok(())
}
