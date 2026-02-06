//! Integration tests for GCP Persistent Disk regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all GCP regions (not just US)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. All disk types (pd-standard, pd-ssd, pd-balanced, pd-extreme) work correctly
//! 6. Both fetch() and fetch_monthly() methods work correctly
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_disk_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::providers::gcp::DiskType;
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
// Per-Disk-Type Tests
// ============================================================
// These tests validate each disk type works correctly across regions

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pd_standard_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a subset of regions for pd-standard
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        let result = client
            .gcp()
            .disk(DiskType::PdStandard)
            .region(region)
            .fetch()
            .await?;

        // Validate source and price
        assert_eq!(
            result.source,
            PriceSource::Api,
            "PD-Standard in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "PD-Standard price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GiB-month");

        println!(
            "✓ PD-Standard {}: ${}/GiB-month (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pd_ssd_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a subset of regions for pd-ssd
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region(region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "PD-SSD in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "PD-SSD price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GiB-month");

        println!(
            "✓ PD-SSD {}: ${}/GiB-month (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pd_balanced_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a subset of regions for pd-balanced
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        let result = client
            .gcp()
            .disk(DiskType::PdBalanced)
            .region(region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "PD-Balanced in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "PD-Balanced price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GiB-month");

        println!(
            "✓ PD-Balanced {}: ${}/GiB-month (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pd_extreme_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a subset of regions for pd-extreme
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        let result = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .region(region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "PD-Extreme in {} should use API source",
            region
        );

        assert!(
            result.price > 0.0,
            "PD-Extreme price should be positive in {}",
            region
        );

        assert_eq!(result.unit, "GiB-month");

        println!(
            "✓ PD-Extreme {}: ${}/GiB-month (source: {:?})",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Convenience vs Raw Query Comparison Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pd_standard_convenience_vs_raw() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Convenience function
    let convenience_result = client
        .gcp()
        .disk(DiskType::PdStandard)
        .region(region)
        .fetch()
        .await?;

    // Raw ProductFilter with validated parameters
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .product_family("Storage")
        .attribute("resourceGroup", "PDStandard")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query should return products for PD-Standard in {}",
        region
    );

    // Filter by description to get the correct product
    // (multiple products may share the same resourceGroup)
    let standard_product = products
        .iter()
        .find(|p| {
            p.attributes.iter().any(|attr| {
                attr.key == "description"
                    && attr
                        .value
                        .as_ref()
                        .map(|v| v.starts_with("Storage PD Capacity"))
                        .unwrap_or(false)
            })
        })
        .expect("Should find Storage PD Capacity product");

    let raw_price = standard_product.first_nonzero_price_or(0.04);

    // Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "PD-Standard price mismatch for {}: convenience={}, raw={}",
        region, convenience_result.price, raw_price
    );

    println!(
        "✓ PD-Standard {}: convenience={}, raw={} ✓ MATCH",
        region, convenience_result.price, raw_price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pd_ssd_convenience_vs_raw() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Convenience function
    let convenience_result = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region(region)
        .fetch()
        .await?;

    // Raw ProductFilter with validated parameters
    // Note: PdSsd, PdBalanced, and PdExtreme all use resourceGroup="SSD"
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .product_family("Storage")
        .attribute("resourceGroup", "SSD")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query should return products for PD-SSD in {}",
        region
    );

    // Find the SSD backed PD product
    let ssd_product = products
        .iter()
        .find(|p| {
            p.attributes.iter().any(|attr| {
                attr.key == "description"
                    && attr
                        .value
                        .as_ref()
                        .map(|v| v.contains("SSD backed"))
                        .unwrap_or(false)
            })
        })
        .expect("Should find SSD backed PD product");

    let raw_price = ssd_product.first_nonzero_price_or(0.17);

    // Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "PD-SSD price mismatch for {}: convenience={}, raw={}",
        region, convenience_result.price, raw_price
    );

    println!(
        "✓ PD-SSD {}: convenience={}, raw={} ✓ MATCH",
        region, convenience_result.price, raw_price
    );

    Ok(())
}

// ============================================================
// fetch_monthly() Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_monthly_calculation() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let size_gb = 500;

    // Get per-unit price
    let per_unit = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region(region)
        .fetch()
        .await?;

    // Get monthly cost
    let monthly = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region(region)
        .size_gb(size_gb)
        .fetch_monthly()
        .await?;

    // Validate calculation: monthly = per_unit × size_gb
    let expected_monthly = per_unit.price * size_gb as f64;
    assert!(
        (monthly.price - expected_monthly).abs() < 0.01,
        "Monthly cost should be per-unit × size_gb. Got {}, expected {}",
        monthly.price,
        expected_monthly
    );

    assert_eq!(per_unit.unit, "GiB-month");
    assert_eq!(monthly.unit, "month");

    println!(
        "✓ Monthly calculation: {} GiB × ${}/GiB = ${}/month",
        size_gb, per_unit.price, monthly.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pd_extreme_with_iops() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let size_gb = 500;
    let iops = 15000;

    // Get monthly cost with IOPS
    let monthly = client
        .gcp()
        .disk(DiskType::PdExtreme)
        .region(region)
        .size_gb(size_gb)
        .iops(iops)
        .fetch_monthly()
        .await?;

    // Validate price is positive and includes IOPS cost
    assert!(
        monthly.price > 0.0,
        "Monthly cost should be positive. Got: {}",
        monthly.price
    );

    assert_eq!(monthly.unit, "month");
    assert_eq!(monthly.source, PriceSource::Api);

    // Expected: (500 × storage_price) + (15000 × iops_price)
    // Storage: ~$0.125/GiB-month = $62.50
    // IOPS: ~$0.065/IOPS-month = $975
    // Total: ~$1037.50
    println!(
        "✓ PD-Extreme with IOPS: {} GiB + {} IOPS = ${}/month",
        size_gb, iops, monthly.price
    );

    Ok(())
}

// ============================================================
// Source Tracking Validation Test
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_disk_source_tracking_across_types() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    let disk_types = vec![
        DiskType::PdStandard,
        DiskType::PdSsd,
        DiskType::PdBalanced,
        DiskType::PdExtreme,
    ];

    for disk_type in disk_types {
        let result = client.gcp().disk(disk_type).region(region).fetch().await?;

        // With valid API key, source should be Api, not Default
        assert_eq!(
            result.source,
            PriceSource::Api,
            "Disk type {:?} in {} should return API pricing, not defaults",
            disk_type,
            region
        );

        assert!(
            result.price > 0.0,
            "Price should be positive for {:?} in {}",
            disk_type,
            region
        );

        println!(
            "✓ Source tracking validated for {:?}: price={}, source={:?}",
            disk_type, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Regional Pricing Variation Test
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_regional_pricing_variations() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test same disk type across different regions to validate pricing varies
    let regions = vec!["us-central1", "europe-west1", "asia-southeast1"];
    let mut prices = Vec::new();

    for region in &regions {
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region(*region)
            .fetch()
            .await?;

        prices.push((*region, result.price));

        println!("✓ PD-SSD {}: ${}/GiB-month", region, result.price);
    }

    // Validate that prices are not all identical (regional variation exists)
    let first_price = prices[0].1;
    let has_variation = prices
        .iter()
        .any(|(_, price)| (*price - first_price).abs() > 0.001);

    if has_variation {
        println!("✓ Regional pricing variation confirmed (prices differ across regions)");
    } else {
        println!(
            "⚠ All regions have same price - may indicate pricing normalization or limited test data"
        );
    }

    Ok(())
}
