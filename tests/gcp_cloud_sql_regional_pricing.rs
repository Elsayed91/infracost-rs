//! Integration tests for GCP Cloud SQL regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all GCP regions (not just US)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. All 5 cost components work (CPU, memory, storage, backup, IP address)
//! 6. fetch_monthly() correctly combines all components
//! 7. Different database engines (MySQL, PostgreSQL, SQL Server) return prices
//! 8. Different availability types (Zonal, Regional) affect pricing
//!
//! Run with:
//! ```bash
//! cargo test --test gcp_cloud_sql_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::providers::gcp::{CloudSqlAvailability, CloudSqlEngine};
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
    "us-central1",
    "us-east1",
    "southamerica-east1",
    "europe-west1",
    "europe-north1",
    "asia-southeast1",
    "australia-southeast1",
];

// ============================================================
// Per-Region Comparison Tests (CPU Component)
// ============================================================
// These tests compare convenience function output to raw ProductFilter queries
// for each region, validating that the query parameters work universally.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_cpu_us_central1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing_cpu("us-central1", CloudSqlEngine::PostgreSql).await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_cpu_us_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing_cpu("us-east1", CloudSqlEngine::PostgreSql).await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_cpu_southamerica_east1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing_cpu("southamerica-east1", CloudSqlEngine::PostgreSql).await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_cpu_europe_west1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing_cpu("europe-west1", CloudSqlEngine::PostgreSql).await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_cpu_europe_north1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing_cpu("europe-north1", CloudSqlEngine::PostgreSql).await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_cpu_asia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing_cpu("asia-southeast1", CloudSqlEngine::PostgreSql).await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_cpu_australia_southeast1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing_cpu("australia-southeast1", CloudSqlEngine::PostgreSql).await
}

/// Helper function to test CPU pricing for a specific region.
/// Compares convenience function vs raw ProductFilter query.
async fn test_region_pricing_cpu(
    region: &str,
    engine: CloudSqlEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function
    let convenience_result = client
        .gcp()
        .cloud_sql()
        .engine(engine)
        .availability(CloudSqlAvailability::Zonal)
        .region(region)
        .fetch()
        .await?;

    // 2. Get price using raw ProductFilter with validated universal parameters
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Cloud SQL")
        .attribute("resourceGroup", "SQLGen2InstancesCPU")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query should return products for region: {}",
        region
    );

    // Filter for the specific engine and availability (Zonal, vCPU, not Enterprise Plus)
    let engine_name = match engine {
        CloudSqlEngine::MySql => "MySQL",
        CloudSqlEngine::PostgreSql => "PostgreSQL",
        CloudSqlEngine::SqlServer => "SQL Server",
    };

    let matching_product = products.iter().find(|product| {
        let desc = product.attribute("description").unwrap_or("");
        desc.contains("Cloud SQL for")
            && desc.contains(engine_name)
            && desc.contains("Zonal")
            && desc.contains("vCPU")
            && !desc.contains("Enterprise Plus")
    });

    assert!(
        matching_product.is_some(),
        "Should find Cloud SQL CPU product for {} in {}",
        engine_name,
        region
    );

    let raw_price = matching_product.unwrap().first_nonzero_price_or(0.0413);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "Cloud SQL CPU price mismatch for {} in {}: convenience={}, raw={}",
        engine_name, region, convenience_result.price, raw_price
    );

    // 4. Validate source tracking (should be Api, not Default)
    assert_eq!(
        convenience_result.source,
        PriceSource::Api,
        "Expected API source for {} in {}, got {:?}",
        engine_name,
        region,
        convenience_result.source
    );

    // 5. Validate price is positive
    assert!(
        convenience_result.price > 0.0,
        "Price should be positive for {} in {}",
        engine_name,
        region
    );

    println!(
        "✓ Cloud SQL CPU ({}) {}: ${}/hour (convenience={}, raw={}, source={:?})",
        engine_name,
        region,
        convenience_result.price,
        convenience_result.price,
        raw_price,
        convenience_result.source
    );

    Ok(())
}

// ============================================================
// Source Tracking Validation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_source_tracking_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .region(*region)
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

        // Price should be positive
        assert!(
            result.price > 0.0,
            "Price should be positive for region {}. Got: {}",
            region,
            result.price
        );

        println!(
            "✓ Source tracking validated for {}: price=${}/hour, source={:?}",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Monthly Conversion Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_monthly_with_all_components() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    let result = client
        .gcp()
        .cloud_sql()
        .engine(CloudSqlEngine::PostgreSql)
        .availability(CloudSqlAvailability::Zonal)
        .region(region)
        .cpu_count(4)
        .memory_gb(16)
        .storage_gb(100)
        .backup_storage_gb(50)
        .fetch_monthly()
        .await?;

    // Should be from API
    assert_eq!(result.source, PriceSource::Api);

    // Monthly cost should be positive and reasonable
    // Rough estimate: CPU (4 * $0.04 * 730) + RAM (16 * $0.007 * 730) + Storage (100 * $0.17) + Backup (50 * $0.08) + IP ($0.01 * 730)
    // ~= $116.8 + $81.76 + $17 + $4 + $7.3 = ~$227
    // Allow wide range since API prices vary by region
    assert!(
        result.price > 100.0 && result.price < 500.0,
        "Monthly cost should be reasonable: got ${}",
        result.price
    );

    assert_eq!(result.unit, "month");

    println!(
        "✓ Monthly cost with all components (PostgreSQL, Zonal, 4 vCPU, 16 GB RAM, 100 GB storage, 50 GB backup): ${}/month",
        result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_monthly_cpu_and_ram_only() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    let result = client
        .gcp()
        .cloud_sql()
        .engine(CloudSqlEngine::PostgreSql)
        .region(region)
        .cpu_count(2)
        .memory_gb(8)
        .fetch_monthly()
        .await?;

    // Should be from API
    assert_eq!(result.source, PriceSource::Api);

    // Monthly cost should be positive (CPU + RAM + IP only)
    // Rough estimate: CPU (2 * $0.04 * 730) + RAM (8 * $0.007 * 730) + IP ($0.01 * 730)
    // ~= $58.4 + $40.88 + $7.3 = ~$107
    assert!(
        result.price > 50.0 && result.price < 300.0,
        "Monthly cost should be reasonable: got ${}",
        result.price
    );

    assert_eq!(result.unit, "month");

    println!(
        "✓ Monthly cost with CPU and RAM only (2 vCPU, 8 GB RAM): ${}/month",
        result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_monthly_regional_vs_zonal() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    let zonal = client
        .gcp()
        .cloud_sql()
        .engine(CloudSqlEngine::PostgreSql)
        .availability(CloudSqlAvailability::Zonal)
        .region(region)
        .cpu_count(2)
        .memory_gb(8)
        .fetch_monthly()
        .await?;

    let regional = client
        .gcp()
        .cloud_sql()
        .engine(CloudSqlEngine::PostgreSql)
        .availability(CloudSqlAvailability::Regional)
        .region(region)
        .cpu_count(2)
        .memory_gb(8)
        .fetch_monthly()
        .await?;

    // Both should be from API
    assert_eq!(zonal.source, PriceSource::Api);
    assert_eq!(regional.source, PriceSource::Api);

    // Regional should be approximately 2x the cost of Zonal
    // Allow for some variance (1.8x to 2.2x)
    let ratio = regional.price / zonal.price;
    assert!(
        ratio > 1.8 && ratio < 2.2,
        "Regional should be ~2x Zonal cost: ratio={} (regional=${}, zonal=${})",
        ratio,
        regional.price,
        zonal.price
    );

    println!(
        "✓ Regional vs Zonal pricing validated: Zonal=${}/month, Regional=${}/month (ratio={:.2}x)",
        zonal.price, regional.price, ratio
    );

    Ok(())
}

// ============================================================
// Engine Variant Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_mysql_engine() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .cloud_sql()
            .engine("mysql")
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("✓ MySQL engine {}: ${}/hour per vCPU", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_postgresql_engine() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .cloud_sql()
            .engine("postgresql")
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!(
            "✓ PostgreSQL engine {}: ${}/hour per vCPU",
            region, result.price
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_sqlserver_engine() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .cloud_sql()
            .engine("sqlserver")
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!(
            "✓ SQL Server engine {}: ${}/hour per vCPU",
            region, result.price
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_engine_price_differences() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    let mysql = client
        .gcp()
        .cloud_sql()
        .engine("mysql")
        .region(region)
        .fetch()
        .await?;

    let postgresql = client
        .gcp()
        .cloud_sql()
        .engine("postgresql")
        .region(region)
        .fetch()
        .await?;

    let sqlserver = client
        .gcp()
        .cloud_sql()
        .engine("sqlserver")
        .region(region)
        .fetch()
        .await?;

    // All should be from API
    assert_eq!(mysql.source, PriceSource::Api);
    assert_eq!(postgresql.source, PriceSource::Api);
    assert_eq!(sqlserver.source, PriceSource::Api);

    // All should be positive
    assert!(mysql.price > 0.0);
    assert!(postgresql.price > 0.0);
    assert!(sqlserver.price > 0.0);

    println!("✓ Engine pricing comparison in {}:", region);
    println!("  MySQL: ${}/hour per vCPU", mysql.price);
    println!("  PostgreSQL: ${}/hour per vCPU", postgresql.price);
    println!("  SQL Server: ${}/hour per vCPU", sqlserver.price);

    Ok(())
}

// ============================================================
// Availability Type Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_zonal_availability() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .cloud_sql()
            .engine("postgresql")
            .availability("zonal")
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);

        println!(
            "✓ Zonal availability {}: ${}/hour per vCPU",
            region, result.price
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_regional_availability() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in TEST_REGIONS {
        let result = client
            .gcp()
            .cloud_sql()
            .engine("postgresql")
            .availability("regional")
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);

        println!(
            "✓ Regional (HA) availability {}: ${}/hour per vCPU",
            region, result.price
        );
    }

    Ok(())
}

// ============================================================
// Component Integration Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_storage_adds_to_monthly_cost() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Get monthly cost without storage
    let without_storage = client
        .gcp()
        .cloud_sql()
        .engine("postgresql")
        .region(region)
        .cpu_count(2)
        .memory_gb(8)
        .storage_gb(0)
        .fetch_monthly()
        .await?;

    // Get monthly cost with storage
    let with_storage = client
        .gcp()
        .cloud_sql()
        .engine("postgresql")
        .region(region)
        .cpu_count(2)
        .memory_gb(8)
        .storage_gb(100)
        .fetch_monthly()
        .await?;

    // Both should be from API
    assert_eq!(without_storage.source, PriceSource::Api);
    assert_eq!(with_storage.source, PriceSource::Api);

    // With storage should be more expensive
    assert!(
        with_storage.price > without_storage.price,
        "Cost with storage (${}) should be greater than without (${})",
        with_storage.price,
        without_storage.price
    );

    let storage_cost = with_storage.price - without_storage.price;
    println!(
        "✓ Storage component validated: without=${}/month, with 100GB=${}/month (storage cost=${:.2})",
        without_storage.price, with_storage.price, storage_cost
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cloud_sql_backup_adds_to_monthly_cost() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-central1";

    // Get monthly cost without backup
    let without_backup = client
        .gcp()
        .cloud_sql()
        .engine("postgresql")
        .region(region)
        .cpu_count(2)
        .memory_gb(8)
        .backup_storage_gb(0)
        .fetch_monthly()
        .await?;

    // Get monthly cost with backup
    let with_backup = client
        .gcp()
        .cloud_sql()
        .engine("postgresql")
        .region(region)
        .cpu_count(2)
        .memory_gb(8)
        .backup_storage_gb(50)
        .fetch_monthly()
        .await?;

    // Both should be from API
    assert_eq!(without_backup.source, PriceSource::Api);
    assert_eq!(with_backup.source, PriceSource::Api);

    // With backup should be more expensive
    assert!(
        with_backup.price > without_backup.price,
        "Cost with backup (${}) should be greater than without (${})",
        with_backup.price,
        without_backup.price
    );

    let backup_cost = with_backup.price - without_backup.price;
    println!(
        "✓ Backup component validated: without=${}/month, with 50GB=${}/month (backup cost=${:.2})",
        without_backup.price, with_backup.price, backup_cost
    );

    Ok(())
}
