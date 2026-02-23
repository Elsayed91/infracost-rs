//! Integration tests for GCP BigQuery Storage regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all GCP regions (not just US)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. All four cost components work: active logical, long-term logical,
//!    active physical, and long-term physical
//! 6. fetch_monthly() correctly multiplies unit prices by storage quantities
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_bigquery_storage_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::{Client, ProductFilter};

/// Helper to get a client with API key from environment.
fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
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
// Per-Region Comparison Tests: Active Logical Storage
// ============================================================
// Compare convenience function vs raw ProductFilter query for
// active logical storage (the primary component) across all 7 regions.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_logical_storage_us_central1() -> Result<(), Box<dyn std::error::Error>> {
    test_active_logical_region("us-central1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_logical_storage_us_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_active_logical_region("us-east1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_logical_storage_southamerica_east1() -> Result<(), Box<dyn std::error::Error>>
{
    test_active_logical_region("southamerica-east1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_logical_storage_europe_west1() -> Result<(), Box<dyn std::error::Error>> {
    test_active_logical_region("europe-west1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_logical_storage_europe_north1() -> Result<(), Box<dyn std::error::Error>> {
    test_active_logical_region("europe-north1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_logical_storage_asia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_active_logical_region("asia-southeast1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_logical_storage_australia_southeast1() -> Result<(), Box<dyn std::error::Error>>
{
    test_active_logical_region("australia-southeast1").await
}

/// Helper: compare convenience function vs raw ProductFilter for active logical storage.
///
/// The convenience function uses `fetch()` which returns the primary component
/// (active logical storage). The raw query uses resourceGroup=ActiveStorage.
async fn test_active_logical_region(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function (primary component = active logical storage)
    let convenience_result = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .fetch()
        .await?;

    // 2. Get price using raw ProductFilter
    // resourceGroup=ActiveStorage works universally across all regions
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("BigQuery")
        .attribute("resourceGroup", "ActiveStorage")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query for ActiveStorage should return products for region: {}",
        region
    );

    let raw_price = products[0].first_nonzero_price_or(0.023);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "Active logical storage price mismatch for {}: convenience={}, raw={}",
        region, convenience_result.price, raw_price
    );

    // 4. Validate source tracking
    assert_eq!(
        convenience_result.source,
        PriceSource::Api,
        "Expected API source for region {}, got {:?}",
        region,
        convenience_result.source
    );

    // 5. Validate price is positive
    assert!(
        convenience_result.price > 0.0,
        "Active logical storage price should be positive for region {}",
        region
    );

    assert_eq!(
        convenience_result.unit, "gibibyte month",
        "Unit should be 'gibibyte month' for region {}",
        region
    );

    println!(
        "Active logical storage {}: price=${}/GiB-month, source={:?}",
        region, convenience_result.price, convenience_result.source
    );

    Ok(())
}

// ============================================================
// Per-Region Comparison Tests: Long-Term Logical Storage
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_long_term_logical_storage_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        // Raw ProductFilter for long-term logical storage
        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("BigQuery")
            .attribute("resourceGroup", "LongTermStorage")
            .region(*region)
            .build();

        let products = client.query_products(filter).await?;
        assert!(
            !products.is_empty(),
            "Raw query for LongTermStorage should return products for region: {}",
            region
        );

        let price = products[0].first_nonzero_price_or(0.016);

        assert!(
            price > 0.0,
            "Long-term logical storage price should be positive for region {}",
            region
        );

        println!(
            "Long-term logical storage {}: price=${}/GiB-month",
            region, price
        );
    }

    Ok(())
}

// ============================================================
// Per-Region Comparison Tests: Physical Storage Components
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_active_physical_storage_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        // Raw ProductFilter for physical storage (requires description filtering)
        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("BigQuery")
            .attribute("resourceGroup", "PhysicalStorage")
            .region(*region)
            .build();

        let products = client.query_products(filter).await?;
        assert!(
            !products.is_empty(),
            "Raw query for PhysicalStorage should return products for region: {}",
            region
        );

        // The API returns multiple PhysicalStorage products; find active physical by description
        let active_physical = products.iter().find(|product| {
            product
                .attribute("description")
                .map(|d| d.starts_with("Active Physical Storage"))
                .unwrap_or(false)
        });

        assert!(
            active_physical.is_some(),
            "Should find 'Active Physical Storage' product in region {}",
            region
        );

        let price = active_physical.unwrap().first_nonzero_price_or(0.04);

        assert!(
            price > 0.0,
            "Active physical storage price should be positive for region {}",
            region
        );

        println!(
            "Active physical storage {}: price=${}/GiB-month",
            region, price
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_long_term_physical_storage_across_regions() -> Result<(), Box<dyn std::error::Error>>
{
    let client = get_client()?;

    for region in TEST_REGIONS {
        // Raw ProductFilter for physical storage (requires description filtering)
        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("BigQuery")
            .attribute("resourceGroup", "PhysicalStorage")
            .region(*region)
            .build();

        let products = client.query_products(filter).await?;
        assert!(
            !products.is_empty(),
            "Raw query for PhysicalStorage should return products for region: {}",
            region
        );

        // Find long-term physical by description prefix
        let long_term_physical = products.iter().find(|product| {
            product
                .attribute("description")
                .map(|d| d.starts_with("Long-Term Physical Storage"))
                .unwrap_or(false)
        });

        assert!(
            long_term_physical.is_some(),
            "Should find 'Long-Term Physical Storage' product in region {}",
            region
        );

        let price = long_term_physical.unwrap().first_nonzero_price_or(0.02);

        assert!(
            price > 0.0,
            "Long-term physical storage price should be positive for region {}",
            region
        );

        println!(
            "Long-term physical storage {}: price=${}/GiB-month",
            region, price
        );
    }

    Ok(())
}

// ============================================================
// Source Tracking Validation Test
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_bigquery_storage_source_tracking_across_regions()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test a representative subset of regions for source tracking
    let test_regions = vec!["us-central1", "europe-west1", "asia-southeast1"];

    for region in test_regions {
        let result = client
            .gcp()
            .bigquery_storage()
            .region(region)
            .fetch()
            .await?;

        // With valid API key, source should be Api, not Default
        assert_eq!(
            result.source,
            PriceSource::Api,
            "Region {} should return API pricing, not defaults. Got source: {:?}",
            region,
            result.source
        );

        assert!(
            result.price > 0.0,
            "Price should be positive for region {}. Got: {}",
            region,
            result.price
        );

        println!(
            "Source tracking validated for {}: price={}, source={:?}",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Monthly Conversion Test: Active Logical Storage Only
// ============================================================
// Since pricing is per GiB-month (not hourly), fetch_monthly() with
// a quantity uses: total_cost = unit_price * quantity_gb (linear model).

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_bigquery_storage_monthly_active_logical_only()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let storage_gb: u64 = 100;

    // Get unit price via fetch()
    let unit_price_result = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .fetch()
        .await?;

    // Get monthly cost via fetch_monthly() with a quantity
    let monthly_result = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .active_logical_storage_gb(storage_gb)
        .fetch_monthly()
        .await?;

    // Both should come from the API
    assert_eq!(
        unit_price_result.source,
        PriceSource::Api,
        "Unit price should come from API for {}",
        region
    );
    assert_eq!(
        monthly_result.source,
        PriceSource::Api,
        "Monthly result should come from API for {}",
        region
    );

    // Monthly cost = unit_price * quantity (linear model, no free tier)
    let expected_monthly = unit_price_result.price * storage_gb as f64;
    assert!(
        (monthly_result.price - expected_monthly).abs() < 0.001,
        "Monthly cost should be unit_price * quantity. Got monthly={}, expected={}",
        monthly_result.price,
        expected_monthly
    );

    assert_eq!(unit_price_result.unit, "gibibyte month");
    assert_eq!(monthly_result.unit, "month");

    println!(
        "Monthly (active logical) {}: unit_price=${}/GiB-month, {}GiB => ${}/month",
        region, unit_price_result.price, storage_gb, monthly_result.price
    );

    Ok(())
}

// ============================================================
// Monthly Conversion Test: All Four Components
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_bigquery_storage_monthly_all_components() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    let active_logical_gb: u64 = 500;
    let long_term_logical_gb: u64 = 200;
    let active_physical_gb: u64 = 100;
    let long_term_physical_gb: u64 = 50;

    // Fetch monthly cost with all four components set
    let monthly_result = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .active_logical_storage_gb(active_logical_gb)
        .long_term_logical_storage_gb(long_term_logical_gb)
        .active_physical_storage_gb(active_physical_gb)
        .long_term_physical_storage_gb(long_term_physical_gb)
        .fetch_monthly()
        .await?;

    // Should be from API with all components contributing
    assert_eq!(
        monthly_result.source,
        PriceSource::Api,
        "Monthly result with all components should come from API for {}",
        region
    );

    assert!(
        monthly_result.price > 0.0,
        "Total monthly cost with all components should be positive in {}",
        region
    );

    assert_eq!(monthly_result.unit, "month");

    // Fetch individual component unit prices via raw queries to verify total
    // Active logical: resourceGroup=ActiveStorage
    let active_logical_filter = ProductFilter::builder()
        .vendor("gcp")
        .service("BigQuery")
        .attribute("resourceGroup", "ActiveStorage")
        .region(region)
        .build();
    let active_logical_products = client.query_products(active_logical_filter).await?;
    let active_logical_price = active_logical_products[0].first_nonzero_price_or(0.023);

    // Long-term logical: resourceGroup=LongTermStorage
    let long_term_logical_filter = ProductFilter::builder()
        .vendor("gcp")
        .service("BigQuery")
        .attribute("resourceGroup", "LongTermStorage")
        .region(region)
        .build();
    let long_term_logical_products = client.query_products(long_term_logical_filter).await?;
    let long_term_logical_price = long_term_logical_products[0].first_nonzero_price_or(0.016);

    // Physical storage: filter by description
    let physical_filter = ProductFilter::builder()
        .vendor("gcp")
        .service("BigQuery")
        .attribute("resourceGroup", "PhysicalStorage")
        .region(region)
        .build();
    let physical_products = client.query_products(physical_filter).await?;

    let active_physical_price = physical_products
        .iter()
        .find(|p| {
            p.attribute("description")
                .map(|d| d.starts_with("Active Physical Storage"))
                .unwrap_or(false)
        })
        .map(|p| p.first_nonzero_price_or(0.04))
        .unwrap_or(0.04);

    let long_term_physical_price = physical_products
        .iter()
        .find(|p| {
            p.attribute("description")
                .map(|d| d.starts_with("Long-Term Physical Storage"))
                .unwrap_or(false)
        })
        .map(|p| p.first_nonzero_price_or(0.02))
        .unwrap_or(0.02);

    // Expected total = sum of (price * quantity) for each component
    let expected_total = (active_logical_price * active_logical_gb as f64)
        + (long_term_logical_price * long_term_logical_gb as f64)
        + (active_physical_price * active_physical_gb as f64)
        + (long_term_physical_price * long_term_physical_gb as f64);

    assert!(
        (monthly_result.price - expected_total).abs() < 0.01,
        "All-component monthly cost mismatch. Got monthly={}, expected={}",
        monthly_result.price,
        expected_total
    );

    println!(
        "Monthly (all components) {}: active_logical={}x${}, long_term_logical={}x${}, active_physical={}x${}, long_term_physical={}x${} => ${}/month",
        region,
        active_logical_gb,
        active_logical_price,
        long_term_logical_gb,
        long_term_logical_price,
        active_physical_gb,
        active_physical_price,
        long_term_physical_gb,
        long_term_physical_price,
        monthly_result.price
    );

    Ok(())
}

// ============================================================
// Monthly Test: Zero Quantities Returns Zero Cost
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_bigquery_storage_monthly_zero_quantities() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // fetch_monthly() with no quantities set should return $0
    let monthly_result = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .fetch_monthly()
        .await?;

    assert_eq!(
        monthly_result.price, 0.0,
        "Monthly cost with no quantities should be $0, got {}",
        monthly_result.price
    );
    assert_eq!(monthly_result.unit, "month");

    println!(
        "Monthly (zero quantities) {}: ${}/month",
        region, monthly_result.price
    );

    Ok(())
}

// ============================================================
// Regional Pricing Variation Test
// ============================================================
// Validate that European and APAC regions return different prices
// from US, confirming region-specific pricing is working.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_bigquery_storage_regional_pricing_variations()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Fetch active logical storage prices across regions with known pricing differences
    let us_result = client
        .gcp()
        .bigquery_storage()
        .region("us-central1")
        .fetch()
        .await?;

    let europe_result = client
        .gcp()
        .bigquery_storage()
        .region("europe-west1")
        .fetch()
        .await?;

    let australia_result = client
        .gcp()
        .bigquery_storage()
        .region("australia-southeast1")
        .fetch()
        .await?;

    let sa_result = client
        .gcp()
        .bigquery_storage()
        .region("southamerica-east1")
        .fetch()
        .await?;

    // All should be from API
    assert_eq!(us_result.source, PriceSource::Api);
    assert_eq!(europe_result.source, PriceSource::Api);
    assert_eq!(australia_result.source, PriceSource::Api);
    assert_eq!(sa_result.source, PriceSource::Api);

    // All prices should be positive
    assert!(us_result.price > 0.0);
    assert!(europe_result.price > 0.0);
    assert!(australia_result.price > 0.0);
    assert!(sa_result.price > 0.0);

    // europe-west1 has a lower active logical price than us-central1 ($0.020 vs $0.023)
    assert!(
        europe_result.price < us_result.price,
        "europe-west1 active logical price ({}) should be less than us-central1 ({})",
        europe_result.price,
        us_result.price
    );

    // southamerica-east1 should have same or higher price than us-central1
    assert!(
        sa_result.price >= us_result.price,
        "southamerica-east1 active logical price ({}) should be >= us-central1 ({})",
        sa_result.price,
        us_result.price
    );

    println!("Regional pricing variations (active logical storage):");
    println!("  us-central1: ${}/GiB-month", us_result.price);
    println!("  europe-west1: ${}/GiB-month", europe_result.price);
    println!(
        "  australia-southeast1: ${}/GiB-month",
        australia_result.price
    );
    println!("  southamerica-east1: ${}/GiB-month", sa_result.price);

    Ok(())
}

// ============================================================
// Monthly Test: Logical Billing Model
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_bigquery_storage_logical_billing_model() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Test with logical billing (active + long-term logical, no physical)
    let logical_monthly = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .active_logical_storage_gb(500)
        .long_term_logical_storage_gb(200)
        .fetch_monthly()
        .await?;

    assert_eq!(logical_monthly.source, PriceSource::Api);
    assert!(logical_monthly.price > 0.0);
    assert_eq!(logical_monthly.unit, "month");

    println!(
        "Logical billing model {}: 500GiB active + 200GiB long-term => ${}/month",
        region, logical_monthly.price
    );

    Ok(())
}

// ============================================================
// Monthly Test: Physical Billing Model
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_bigquery_storage_physical_billing_model() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Test with physical billing (active + long-term physical, no logical)
    let physical_monthly = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .active_physical_storage_gb(100)
        .long_term_physical_storage_gb(50)
        .fetch_monthly()
        .await?;

    assert_eq!(physical_monthly.source, PriceSource::Api);
    assert!(physical_monthly.price > 0.0);
    assert_eq!(physical_monthly.unit, "month");

    println!(
        "Physical billing model {}: 100GiB active + 50GiB long-term => ${}/month",
        region, physical_monthly.price
    );

    Ok(())
}

// ============================================================
// Monthly Test: Physical billing is more expensive per GiB than logical
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_physical_storage_more_expensive_than_logical()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";
    let storage_gb: u64 = 100;

    // Active logical: uses fetch() unit price * quantity
    let active_logical_unit = client
        .gcp()
        .bigquery_storage()
        .region(region)
        .fetch()
        .await?;
    let active_logical_cost = active_logical_unit.price * storage_gb as f64;

    // Active physical: raw query with description filter
    let physical_filter = ProductFilter::builder()
        .vendor("gcp")
        .service("BigQuery")
        .attribute("resourceGroup", "PhysicalStorage")
        .region(region)
        .build();

    let physical_products = client.query_products(physical_filter).await?;
    let active_physical_product = physical_products
        .iter()
        .find(|p| {
            p.attribute("description")
                .map(|d| d.starts_with("Active Physical Storage"))
                .unwrap_or(false)
        })
        .expect("Should find active physical storage product");

    let active_physical_price = active_physical_product.first_nonzero_price_or(0.04);
    let active_physical_cost = active_physical_price * storage_gb as f64;

    // Active physical storage is more expensive per GiB than active logical
    // ($0.04 vs $0.023 in us-central1)
    assert!(
        active_physical_cost > active_logical_cost,
        "Active physical storage (${}/GiB) should be more expensive than active logical (${}/GiB) in {}",
        active_physical_price,
        active_logical_unit.price,
        region
    );

    println!(
        "Storage cost comparison for {}GiB in {}: logical=${}, physical=${}",
        storage_gb, region, active_logical_cost, active_physical_cost
    );

    Ok(())
}

// ============================================================
// Monthly Test: Long-term storage cheaper than active storage
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_long_term_cheaper_than_active() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let storage_gb: u64 = 100;

    let test_regions = vec!["us-central1", "europe-west1", "southamerica-east1"];

    for region in test_regions {
        // Active logical price (from convenience function)
        let active_result = client
            .gcp()
            .bigquery_storage()
            .region(region)
            .fetch()
            .await?;

        // Long-term logical price (from raw query)
        let long_term_filter = ProductFilter::builder()
            .vendor("gcp")
            .service("BigQuery")
            .attribute("resourceGroup", "LongTermStorage")
            .region(region)
            .build();

        let long_term_products = client.query_products(long_term_filter).await?;
        let long_term_price = long_term_products[0].first_nonzero_price_or(0.016);

        // Long-term storage should always be cheaper than active storage
        assert!(
            long_term_price < active_result.price,
            "Long-term logical (${}/GiB) should be cheaper than active logical (${}/GiB) in {}",
            long_term_price,
            active_result.price,
            region
        );

        println!(
            "{}GiB in {}: active_logical=${}, long_term_logical=${} (long-term is {:.0}% of active)",
            storage_gb,
            region,
            active_result.price * storage_gb as f64,
            long_term_price * storage_gb as f64,
            (long_term_price / active_result.price) * 100.0
        );
    }

    Ok(())
}
