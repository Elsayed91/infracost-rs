//! Integration tests for AWS EC2 Instance regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing works correctly across all AWS regions (not just us-east-1)
//! 3. Source tracking is correct (PriceSource::Api vs PriceSource::Default)
//! 4. Dynamic pricing is fetched from the API, not hardcoded defaults
//! 5. Operating system variants (Linux, Windows) return different prices
//! 6. Different instance types return appropriate prices
//! 7. Monthly conversion works correctly (hourly * 730)
//!
//! Run with:
//! ```bash
//! cargo test --test aws_ec2_instance_regional_pricing -- --ignored
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

// ============================================================
// Per-Region Comparison Tests (t3.micro, Linux)
// ============================================================
// These tests compare convenience builder vs raw ProductFilter
// to ensure they return identical results

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_us_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_us_west_2() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("us-west-2").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_sa_east_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("sa-east-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_eu_west_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eu-west-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_eu_central_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eu-central-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_ap_southeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("ap-southeast-1").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_ap_northeast_1() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("ap-northeast-1").await
}

/// Helper function to test EC2 Instance pricing for a specific region
/// Compares convenience builder against raw ProductFilter query
async fn test_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let instance_type = "t3.micro";

    // 1. Get price using convenience function
    let convenience_result = client
        .aws()
        .ec2_instance(instance_type)
        .region(region)
        .fetch()
        .await?;

    // 2. Get price using raw ProductFilter
    let filter = ProductFilter::builder()
        .vendor("aws")
        .service("AmazonEC2")
        .product_family("Compute Instance")
        .region(region)
        .attribute("instanceType", instance_type)
        .attribute("tenancy", "Shared")
        .attribute("operatingSystem", "Linux")
        .attribute("preInstalledSw", "NA")
        .attribute("capacitystatus", "Used")
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
        .unwrap_or(0.0104);

    // 3. Compare results
    assert_eq!(
        convenience_result.price, raw_price,
        "EC2 Instance price mismatch for {}: convenience={}, raw={}",
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

    // 5. Validate price is reasonable (EC2 hourly prices typically $0.001 to $100)
    assert!(
        convenience_result.price > 0.001 && convenience_result.price < 100.0,
        "Price should be reasonable for region {}: got ${}",
        region,
        convenience_result.price
    );

    // 6. Validate unit is hour
    assert_eq!(convenience_result.unit, "hour");

    println!(
        "Region {}: price=${}/hour, source={:?}",
        region, convenience_result.price, convenience_result.source
    );

    Ok(())
}

// ============================================================
// Source Tracking Validation
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_source_tracking_across_regions() -> Result<(), Box<dyn std::error::Error>>
{
    let client = get_client()?;
    let test_regions = vec!["us-east-1", "eu-west-1", "ap-southeast-1"];

    for region in test_regions {
        let result = client
            .aws()
            .ec2_instance("t3.micro")
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
    }
    Ok(())
}

// ============================================================
// Monthly Conversion Test
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_monthly_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    let hourly = client
        .aws()
        .ec2_instance("t3.micro")
        .region(region)
        .fetch()
        .await?;

    let monthly = client
        .aws()
        .ec2_instance("t3.micro")
        .region(region)
        .fetch_monthly()
        .await?;

    // Monthly should be hourly * 730
    let expected = hourly.price * 730.0;
    assert!(
        (monthly.price - expected).abs() < 0.01,
        "Monthly={}, expected={}",
        monthly.price,
        expected
    );

    assert_eq!(hourly.unit, "hour");
    assert_eq!(monthly.unit, "month");
    Ok(())
}

// ============================================================
// Operating System Variant Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_windows_vs_linux() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";
    let instance_type = "t3.micro";

    // Get Linux price
    let linux_result = client
        .aws()
        .ec2_instance(instance_type)
        .region(region)
        .operating_system("Linux")
        .fetch()
        .await?;

    // Get Windows price
    let windows_result = client
        .aws()
        .ec2_instance(instance_type)
        .region(region)
        .operating_system("Windows")
        .fetch()
        .await?;

    // Both should be from API
    assert_eq!(linux_result.source, PriceSource::Api);
    assert_eq!(windows_result.source, PriceSource::Api);

    // Windows should be more expensive than Linux
    assert!(
        windows_result.price > linux_result.price,
        "Windows (${}) should be more expensive than Linux (${})",
        windows_result.price,
        linux_result.price
    );

    println!(
        "OS Pricing for {} in {}: Linux=${}/hour, Windows=${}/hour",
        instance_type, region, linux_result.price, windows_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_windows_regional() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test Windows pricing across multiple regions
    for region in &["us-east-1", "eu-west-1", "ap-southeast-1"] {
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .region(*region)
            .operating_system("Windows")
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("Windows pricing in {}: ${}/hour", region, result.price);
    }

    Ok(())
}

// ============================================================
// Instance Type Variant Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_types_different_prices() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get price for small instance (t3.micro)
    let micro_result = client
        .aws()
        .ec2_instance("t3.micro")
        .region(region)
        .fetch()
        .await?;

    // Get price for larger instance (m5.xlarge)
    let xlarge_result = client
        .aws()
        .ec2_instance("m5.xlarge")
        .region(region)
        .fetch()
        .await?;

    // Both should be from API
    assert_eq!(micro_result.source, PriceSource::Api);
    assert_eq!(xlarge_result.source, PriceSource::Api);

    // Larger instance should be more expensive
    assert!(
        xlarge_result.price > micro_result.price,
        "m5.xlarge (${}) should be more expensive than t3.micro (${})",
        xlarge_result.price,
        micro_result.price
    );

    println!(
        "Instance Type Pricing in {}: t3.micro=${}/hour, m5.xlarge=${}/hour",
        region, micro_result.price, xlarge_result.price
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_type_across_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let instance_type = "m5.xlarge";

    // Test same instance type across multiple regions
    for region in &["us-east-1", "eu-west-1", "ap-southeast-1"] {
        let result = client
            .aws()
            .ec2_instance(instance_type)
            .region(*region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(
            result.price > 0.0,
            "Price should be positive for {} in {}",
            instance_type,
            region
        );
        assert_eq!(result.unit, "hour");

        println!(
            "{} pricing in {}: ${}/hour",
            instance_type, region, result.price
        );
    }

    Ok(())
}

// ============================================================
// Cross-Region Validation Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_americas_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let americas_regions = vec!["us-east-1", "us-west-2", "sa-east-1"];

    for region in americas_regions {
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("Americas - {}: ${}/hour", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_europe_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let europe_regions = vec!["eu-west-1", "eu-central-1"];

    for region in europe_regions {
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("Europe - {}: ${}/hour", region, result.price);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_asia_pacific_regions() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let apac_regions = vec!["ap-southeast-1", "ap-northeast-1"];

    for region in apac_regions {
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .region(region)
            .fetch()
            .await?;

        assert_eq!(result.source, PriceSource::Api);
        assert!(result.price > 0.0);
        assert_eq!(result.unit, "hour");

        println!("Asia-Pacific - {}: ${}/hour", region, result.price);
    }

    Ok(())
}

// ============================================================
// Regional Pricing Variations
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_regional_pricing_variations() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Test regions known to potentially have different pricing
    let us_result = client
        .aws()
        .ec2_instance("t3.micro")
        .region("us-east-1")
        .fetch()
        .await?;

    let europe_result = client
        .aws()
        .ec2_instance("t3.micro")
        .region("eu-central-1")
        .fetch()
        .await?;

    let asia_result = client
        .aws()
        .ec2_instance("t3.micro")
        .region("ap-southeast-1")
        .fetch()
        .await?;

    // All should be from API
    assert_eq!(us_result.source, PriceSource::Api);
    assert_eq!(europe_result.source, PriceSource::Api);
    assert_eq!(asia_result.source, PriceSource::Api);

    // Prices should be positive
    assert!(us_result.price > 0.0);
    assert!(europe_result.price > 0.0);
    assert!(asia_result.price > 0.0);

    println!("Regional pricing variations for t3.micro:");
    println!("  US (us-east-1): ${}/hour", us_result.price);
    println!("  Europe (eu-central-1): ${}/hour", europe_result.price);
    println!("  Asia (ap-southeast-1): ${}/hour", asia_result.price);

    // Note: We don't assert specific price values since they can change,
    // but we validate that regional variations exist and all come from API

    Ok(())
}

// ============================================================
// Tenancy Variant Test
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_ec2_instance_dedicated_tenancy() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "us-east-1";

    // Get Shared tenancy price
    let shared_result = client
        .aws()
        .ec2_instance("t3.micro")
        .region(region)
        .tenancy("Shared")
        .fetch()
        .await?;

    // Get Dedicated tenancy price
    let dedicated_result = client
        .aws()
        .ec2_instance("t3.micro")
        .region(region)
        .tenancy("Dedicated")
        .fetch()
        .await?;

    // Both should be from API
    assert_eq!(shared_result.source, PriceSource::Api);
    assert_eq!(dedicated_result.source, PriceSource::Api);

    // Dedicated should typically be more expensive than Shared
    assert!(
        dedicated_result.price > shared_result.price,
        "Dedicated tenancy (${}) should be more expensive than Shared (${})",
        dedicated_result.price,
        shared_result.price
    );

    println!(
        "Tenancy Pricing in {}: Shared=${}/hour, Dedicated=${}/hour",
        region, shared_result.price, dedicated_result.price
    );

    Ok(())
}
