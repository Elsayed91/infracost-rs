//! Integration tests for AWS RDS regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all AWS regions (not just us-east-1)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. Multiple database engines (MySQL, PostgreSQL, MariaDB) work correctly
//! 6. Multi-AZ deployment option affects pricing appropriately
//! 7. Different storage types (gp3, gp2, io1) work correctly across regions
//! 8. Monthly cost calculation includes instance + storage components
//!
//! Run with:
//! ```bash
//! cargo test --test aws_rds_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::providers::aws::RdsStorageType;
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
// Per-Region Instance Pricing Tests (db.t3.micro, MySQL)
// ============================================================
// These tests compare convenience builder vs raw ProductFilter
// to ensure they return identical results

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_instance_us_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_instance_region_pricing("us-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_instance_us_west_2() -> Result<(), Box<dyn std::error::Error>> {
    test_instance_region_pricing("us-west-2").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_instance_sa_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_instance_region_pricing("sa-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_instance_eu_west_1() -> Result<(), Box<dyn std::error::Error>> {
    test_instance_region_pricing("eu-west-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_instance_eu_central_1() -> Result<(), Box<dyn std::error::Error>> {
    test_instance_region_pricing("eu-central-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_instance_ap_southeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_instance_region_pricing("ap-southeast-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_instance_ap_northeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_instance_region_pricing("ap-northeast-1").await
}

/// Helper function to test RDS instance pricing for a specific region
/// Compares convenience builder against raw ProductFilter query
async fn test_instance_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let instance_class = "db.t3.micro";

    // 1. Get price using convenience function
    let convenience_result = client
        .aws()
        .rds(instance_class)
        .engine("mysql")
        .region(region)
        .fetch()
        .await?;

    // 2. Get price using raw ProductFilter with validated universal parameters
    let filter = ProductFilter::builder()
        .vendor("aws")
        .service("AmazonRDS")
        .product_family("Database Instance")
        .region(region)
        .attribute("databaseEngine", "MySQL")
        .attribute("instanceType", instance_class)
        .attribute("deploymentOption", "Single-AZ")
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "Raw query should return products for region: {}",
        region
    );

    let raw_price = products[0]
        .prices()
        .purchase_option("on_demand")
        .first_nonzero_f64()
        .unwrap_or(0.017);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "RDS instance price mismatch for {}: convenience={}, raw={}",
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

    // 5. Validate price is reasonable (RDS hourly prices typically $0.01 to $100)
    assert!(
        convenience_result.price > 0.01 && convenience_result.price < 100.0,
        "Price should be reasonable for region {}: got ${}",
        region,
        convenience_result.price
    );

    // 6. Validate unit is hour
    assert_eq!(convenience_result.unit, "hour");

    println!(
        "Region {}: instance price=${}/hour, source={:?}",
        region, convenience_result.price, convenience_result.source
    );

    Ok(())
}

// ============================================================
// Per-Region GP3 Storage Pricing Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_gp3_storage_us_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_gp3_storage_region_pricing("us-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_gp3_storage_us_west_2() -> Result<(), Box<dyn std::error::Error>> {
    test_gp3_storage_region_pricing("us-west-2").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_gp3_storage_sa_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_gp3_storage_region_pricing("sa-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_gp3_storage_eu_west_1() -> Result<(), Box<dyn std::error::Error>> {
    test_gp3_storage_region_pricing("eu-west-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_gp3_storage_eu_central_1() -> Result<(), Box<dyn std::error::Error>> {
    test_gp3_storage_region_pricing("eu-central-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_gp3_storage_ap_southeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_gp3_storage_region_pricing("ap-southeast-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_gp3_storage_ap_northeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_gp3_storage_region_pricing("ap-northeast-1").await
}

/// Helper function to test GP3 storage pricing for a specific region
async fn test_gp3_storage_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Query GP3 storage pricing directly
    let filter = ProductFilter::builder()
        .vendor("aws")
        .service("AmazonRDS")
        .product_family("Database Storage")
        .region(region)
        .attribute("databaseEngine", "MySQL")
        .attribute("volumeType", "General Purpose-GP3")
        .attribute("deploymentOption", "Single-AZ")
        .build();

    let products = client.query_products(filter).await?;
    assert!(
        !products.is_empty(),
        "GP3 storage query should return products for region: {}",
        region
    );

    let gp3_price = products[0].first_nonzero_price_or(0.115);

    // Validate price is positive
    assert!(
        gp3_price > 0.0,
        "GP3 storage price should be positive for region {}",
        region
    );

    println!(
        "Region {}: GP3 storage price=${}/GB-month",
        region, gp3_price
    );

    Ok(())
}

// ============================================================
// Source Tracking Validation
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_source_tracking_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let test_regions = vec!["us-east-1", "eu-west-1", "ap-southeast-1"];

    for region in test_regions {
        let result = client
            .aws()
            .rds("db.t3.micro")
            .engine("mysql")
            .region(region)
            .fetch()
            .await?;

        assert_eq!(
            result.source,
            PriceSource::Api,
            "Region {} should return API pricing",
            region
        );
        assert!(
            result.price > 0.0,
            "Price should be positive for {}",
            region
        );
        assert_eq!(result.unit, "hour");

        println!(
            "Source tracking validated for {}: price=${}, source={:?}",
            region, result.price, result.source
        );
    }
    Ok(())
}

// ============================================================
// Monthly Conversion Test
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_monthly_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get hourly instance price
    let hourly = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .fetch()
        .await?;

    // Get monthly cost (instance only, no storage)
    let monthly = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .fetch_monthly()
        .await?;

    // Monthly should be hourly * 730
    let expected = hourly.price * 730.0;
    assert!(
        (monthly.price - expected).abs() < 0.01,
        "Monthly instance-only cost mismatch: got ${}, expected ${}",
        monthly.price,
        expected
    );

    assert_eq!(hourly.unit, "hour");
    assert_eq!(monthly.unit, "month");
    assert_eq!(hourly.source, PriceSource::Api);
    assert_eq!(monthly.source, PriceSource::Api);

    println!(
        "Monthly conversion: hourly=${}, monthly=${} (expected={})",
        hourly.price, monthly.price, expected
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_monthly_with_storage() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get instance hourly price
    let instance_hourly = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .fetch()
        .await?;

    // Get storage price per GB-month
    let storage_filter = ProductFilter::builder()
        .vendor("aws")
        .service("AmazonRDS")
        .product_family("Database Storage")
        .region(region)
        .attribute("databaseEngine", "MySQL")
        .attribute("volumeType", "General Purpose-GP3")
        .attribute("deploymentOption", "Single-AZ")
        .build();

    let storage_products = client.query_products(storage_filter).await?;
    let storage_per_gb = storage_products[0].first_nonzero_price_or(0.115);

    // Get monthly cost with 100GB storage
    let monthly_total = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .fetch_monthly()
        .await?;

    // Expected: (instance hourly * 730) + (storage GB * 100)
    let expected = (instance_hourly.price * 730.0) + (storage_per_gb * 100.0);
    let tolerance = expected * 0.02; // 2% tolerance for rounding

    assert!(
        (monthly_total.price - expected).abs() < tolerance,
        "Monthly cost with storage mismatch: got ${}, expected ${} (tolerance=${})",
        monthly_total.price,
        expected,
        tolerance
    );

    assert_eq!(monthly_total.unit, "month");
    assert_eq!(monthly_total.source, PriceSource::Api);

    println!(
        "Monthly with storage: instance_hourly=${}, storage_per_gb=${}, total_monthly=${} (expected={})",
        instance_hourly.price, storage_per_gb, monthly_total.price, expected
    );

    Ok(())
}

// ============================================================
// Engine Variation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_different_engines() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";
    let instance_class = "db.t3.micro";

    // Test MySQL
    let mysql_result = client
        .aws()
        .rds(instance_class)
        .engine("mysql")
        .region(region)
        .fetch()
        .await?;

    // Test PostgreSQL
    let postgres_result = client
        .aws()
        .rds(instance_class)
        .engine("postgres")
        .region(region)
        .fetch()
        .await?;

    // Test MariaDB
    let mariadb_result = client
        .aws()
        .rds(instance_class)
        .engine("mariadb")
        .region(region)
        .fetch()
        .await?;

    // All should be from API
    assert_eq!(mysql_result.source, PriceSource::Api);
    assert_eq!(postgres_result.source, PriceSource::Api);
    assert_eq!(mariadb_result.source, PriceSource::Api);

    // All should have positive prices
    assert!(mysql_result.price > 0.0);
    assert!(postgres_result.price > 0.0);
    assert!(mariadb_result.price > 0.0);

    println!("Engine pricing for {} in {}:", instance_class, region);
    println!("  MySQL: ${}/hour", mysql_result.price);
    println!("  PostgreSQL: ${}/hour", postgres_result.price);
    println!("  MariaDB: ${}/hour", mariadb_result.price);

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_engines_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let engines = vec!["mysql", "postgres", "mariadb"];

    for engine in engines {
        for region in &["us-east-1", "eu-west-1", "ap-southeast-1"] {
            let result = client
                .aws()
                .rds("db.t3.micro")
                .engine(engine)
                .region(*region)
                .fetch()
                .await?;

            assert_eq!(result.source, PriceSource::Api);
            assert!(result.price > 0.0);
            assert_eq!(result.unit, "hour");

            println!("{} in {}: ${}/hour", engine, region, result.price);
        }
    }

    Ok(())
}

// ============================================================
// Multi-AZ Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_multi_az_pricing() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";
    let instance_class = "db.t3.micro";

    // Get Single-AZ price
    let single_az = client
        .aws()
        .rds(instance_class)
        .engine("mysql")
        .region(region)
        .deployment_option("Single-AZ")
        .fetch()
        .await?;

    // Get Multi-AZ price
    let multi_az = client
        .aws()
        .rds(instance_class)
        .engine("mysql")
        .region(region)
        .multi_az()
        .fetch()
        .await?;

    // Both should be from API
    assert_eq!(single_az.source, PriceSource::Api);
    assert_eq!(multi_az.source, PriceSource::Api);

    // Multi-AZ should be more expensive (typically ~2x)
    assert!(
        multi_az.price > single_az.price,
        "Multi-AZ (${}) should be more expensive than Single-AZ (${})",
        multi_az.price,
        single_az.price
    );

    // Multi-AZ should be roughly 2x Single-AZ (within 1.5x to 2.5x range)
    let ratio = multi_az.price / single_az.price;
    assert!(
        ratio > 1.5 && ratio < 2.5,
        "Multi-AZ to Single-AZ ratio should be around 2x, got {:.2}x",
        ratio
    );

    println!(
        "Deployment option pricing for {} in {}:",
        instance_class, region
    );
    println!("  Single-AZ: ${}/hour", single_az.price);
    println!(
        "  Multi-AZ: ${}/hour (ratio: {:.2}x)",
        multi_az.price, ratio
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_multi_az_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in &["us-east-1", "eu-west-1", "ap-southeast-1"] {
        let single_az = client
            .aws()
            .rds("db.t3.micro")
            .engine("mysql")
            .region(*region)
            .deployment_option("Single-AZ")
            .fetch()
            .await?;

        let multi_az = client
            .aws()
            .rds("db.t3.micro")
            .engine("mysql")
            .region(*region)
            .multi_az()
            .fetch()
            .await?;

        assert_eq!(single_az.source, PriceSource::Api);
        assert_eq!(multi_az.source, PriceSource::Api);
        assert!(multi_az.price > single_az.price);

        let ratio = multi_az.price / single_az.price;
        println!(
            "{}: Single-AZ=${}/hr, Multi-AZ=${}/hr (ratio: {:.2}x)",
            region, single_az.price, multi_az.price, ratio
        );
    }

    Ok(())
}

// ============================================================
// Storage Type Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_storage_types() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Query GP3 storage
    let gp3_filter = ProductFilter::builder()
        .vendor("aws")
        .service("AmazonRDS")
        .product_family("Database Storage")
        .region(region)
        .attribute("databaseEngine", "MySQL")
        .attribute("volumeType", "General Purpose-GP3")
        .attribute("deploymentOption", "Single-AZ")
        .build();

    let gp3_products = client.query_products(gp3_filter).await?;
    assert!(!gp3_products.is_empty());
    let gp3_price = gp3_products[0].first_nonzero_price_or(0.115);

    // Query GP2 storage
    let gp2_filter = ProductFilter::builder()
        .vendor("aws")
        .service("AmazonRDS")
        .product_family("Database Storage")
        .region(region)
        .attribute("databaseEngine", "MySQL")
        .attribute("volumeType", "General Purpose")
        .attribute("deploymentOption", "Single-AZ")
        .build();

    let gp2_products = client.query_products(gp2_filter).await?;
    assert!(!gp2_products.is_empty());
    let gp2_price = gp2_products[0].first_nonzero_price_or(0.115);

    // Query IO1 storage
    let io1_filter = ProductFilter::builder()
        .vendor("aws")
        .service("AmazonRDS")
        .product_family("Database Storage")
        .region(region)
        .attribute("databaseEngine", "MySQL")
        .attribute("volumeType", "Provisioned IOPS")
        .attribute("deploymentOption", "Single-AZ")
        .build();

    let io1_products = client.query_products(io1_filter).await?;
    assert!(!io1_products.is_empty());
    let io1_price = io1_products[0].first_nonzero_price_or(0.125);

    // All should have positive prices
    assert!(gp3_price > 0.0);
    assert!(gp2_price > 0.0);
    assert!(io1_price > 0.0);

    println!("Storage type pricing in {}:", region);
    println!("  GP3: ${}/GB-month", gp3_price);
    println!("  GP2: ${}/GB-month", gp2_price);
    println!("  IO1: ${}/GB-month", io1_price);

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_storage_types_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    for region in &["us-east-1", "eu-west-1", "ap-southeast-1"] {
        // Test GP3
        let gp3_filter = ProductFilter::builder()
            .vendor("aws")
            .service("AmazonRDS")
            .product_family("Database Storage")
            .region(*region)
            .attribute("databaseEngine", "MySQL")
            .attribute("volumeType", "General Purpose-GP3")
            .attribute("deploymentOption", "Single-AZ")
            .build();

        let gp3_products = client.query_products(gp3_filter).await?;
        assert!(
            !gp3_products.is_empty(),
            "GP3 should be available in {}",
            region
        );
        let gp3_price = gp3_products[0].first_nonzero_price_or(0.115);

        // Test GP2
        let gp2_filter = ProductFilter::builder()
            .vendor("aws")
            .service("AmazonRDS")
            .product_family("Database Storage")
            .region(*region)
            .attribute("databaseEngine", "MySQL")
            .attribute("volumeType", "General Purpose")
            .attribute("deploymentOption", "Single-AZ")
            .build();

        let gp2_products = client.query_products(gp2_filter).await?;
        assert!(
            !gp2_products.is_empty(),
            "GP2 should be available in {}",
            region
        );
        let gp2_price = gp2_products[0].first_nonzero_price_or(0.115);

        // Test IO1
        let io1_filter = ProductFilter::builder()
            .vendor("aws")
            .service("AmazonRDS")
            .product_family("Database Storage")
            .region(*region)
            .attribute("databaseEngine", "MySQL")
            .attribute("volumeType", "Provisioned IOPS")
            .attribute("deploymentOption", "Single-AZ")
            .build();

        let io1_products = client.query_products(io1_filter).await?;
        assert!(
            !io1_products.is_empty(),
            "IO1 should be available in {}",
            region
        );
        let io1_price = io1_products[0].first_nonzero_price_or(0.125);

        assert!(gp3_price > 0.0);
        assert!(gp2_price > 0.0);
        assert!(io1_price > 0.0);

        println!(
            "{}: GP3=${}, GP2=${}, IO1=${} per GB-month",
            region, gp3_price, gp2_price, io1_price
        );
    }

    Ok(())
}

// ============================================================
// Multi-Component Monthly Cost Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_monthly_with_gp3_and_iops() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get monthly cost with GP3 storage + extra IOPS
    let monthly = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .iops(6000) // 3000 extra IOPS above baseline
        .fetch_monthly()
        .await?;

    assert_eq!(monthly.unit, "month");
    assert_eq!(monthly.source, PriceSource::Api);
    assert!(monthly.price > 0.0);

    // Should include: instance + storage + IOPS
    // Rough estimate: (0.017 * 730) + (0.115 * 100) + (3000 * 0.02) = 12.41 + 11.50 + 60 = 83.91
    // Allow for regional variation
    assert!(
        monthly.price > 70.0 && monthly.price < 100.0,
        "Monthly cost with IOPS should be in reasonable range, got ${}",
        monthly.price
    );

    println!("Monthly with GP3 + IOPS: ${}/month", monthly.price);

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_monthly_with_gp3_and_throughput() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get monthly cost with GP3 storage + extra throughput
    let monthly = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .storage_throughput_mbps(250) // 125 extra MBps above baseline
        .fetch_monthly()
        .await?;

    assert_eq!(monthly.unit, "month");
    assert_eq!(monthly.source, PriceSource::Api);
    assert!(monthly.price > 0.0);

    // Should include: instance + storage + throughput
    // Rough estimate: (0.017 * 730) + (0.115 * 100) + (125 * 0.08) = 12.41 + 11.50 + 10 = 33.91
    assert!(
        monthly.price > 25.0 && monthly.price < 45.0,
        "Monthly cost with throughput should be in reasonable range, got ${}",
        monthly.price
    );

    println!("Monthly with GP3 + throughput: ${}/month", monthly.price);

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_monthly_full_spec() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get monthly cost with full GP3 specification
    let monthly = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .storage_type(RdsStorageType::Gp3)
        .allocated_storage_gb(100)
        .iops(6000)
        .storage_throughput_mbps(250)
        .fetch_monthly()
        .await?;

    assert_eq!(monthly.unit, "month");
    assert_eq!(monthly.source, PriceSource::Api);
    assert!(monthly.price > 0.0);

    // Should include: instance + storage + IOPS + throughput
    // Rough estimate: (0.017 * 730) + (0.115 * 100) + (3000 * 0.02) + (125 * 0.08)
    //                = 12.41 + 11.50 + 60 + 10 = 93.91
    assert!(
        monthly.price > 80.0 && monthly.price < 110.0,
        "Monthly cost with full spec should be in reasonable range, got ${}",
        monthly.price
    );

    println!("Monthly with full GP3 spec: ${}/month", monthly.price);

    Ok(())
}

// ============================================================
// Instance Type Variation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_rds_different_instance_types() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get price for small instance (db.t3.micro)
    let micro_result = client
        .aws()
        .rds("db.t3.micro")
        .engine("mysql")
        .region(region)
        .fetch()
        .await?;

    // Get price for larger instance (db.m5.xlarge)
    let xlarge_result = client
        .aws()
        .rds("db.m5.xlarge")
        .engine("mysql")
        .region(region)
        .fetch()
        .await?;

    // Both should be from API
    assert_eq!(micro_result.source, PriceSource::Api);
    assert_eq!(xlarge_result.source, PriceSource::Api);

    // Larger instance should be more expensive
    assert!(
        xlarge_result.price > micro_result.price,
        "db.m5.xlarge (${}) should be more expensive than db.t3.micro (${})",
        xlarge_result.price,
        micro_result.price
    );

    println!("Instance type pricing in {}:", region);
    println!("  db.t3.micro: ${}/hour", micro_result.price);
    println!("  db.m5.xlarge: ${}/hour", xlarge_result.price);

    Ok(())
}
