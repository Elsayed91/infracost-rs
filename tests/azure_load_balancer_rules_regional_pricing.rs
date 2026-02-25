//! Integration tests for Azure Load Balancer Rules regional pricing.
//!
//! These tests validate that:
//! 1. Convenience functions produce the same results as raw ProductFilter queries
//! 2. Regional pricing behaves consistently across all Azure regions
//! 3. Source tracking is correct (PriceSource::Default, since Azure Load Balancer
//!    pricing is global-only in the IRS API; standard regions always fall back
//!    to the default_price)
//! 4. Monthly tiered calculation is correct for various rule counts
//!
//! Key pricing constants (defaults):
//!   - Tier1 (first 5 rules):        $0.025/rule/hr
//!   - Tier2 (additional rules >5):  $0.010/rule/hr
//!
//! Example monthly calculations:
//!   - 3  rules: 3  x $0.025 x 730 = $54.75
//!   - 5  rules: 5  x $0.025 x 730 = $91.25
//!   - 10 rules: (5 x $0.025 + 5 x $0.010) x 730 = $127.75
//!
//! Run with:
//! ```bash
//! cargo test --test azure_load_balancer_rules_regional_pricing -- --ignored
//! ```

use infracost_rs::providers::PriceSource;
use infracost_rs::{Client, ProductFilter};

// Seven Azure regions under test
const AZURE_TEST_REGIONS: &[&str] = &[
    "eastus",
    "westus2",
    "westeurope",
    "northeurope",
    "southeastasia",
    "japaneast",
    "brazilsouth",
];

const DEFAULT_TIER1_HOURLY: f64 = 0.025;
const DEFAULT_TIER2_HOURLY: f64 = 0.010;
const HOURS_PER_MONTH: f64 = 730.0;
const TIER1_MAX: u64 = 5;

/// Helper to get a client with API key from environment.
fn get_client() -> Result<Client, Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let client = Client::from_env().map_err(|e| format!("INFRACOST_API_KEY must be set: {}", e))?;
    Ok(client)
}

// ============================================================
// Per-Region Comparison Tests
// ============================================================
// Azure Load Balancer pricing is global-only in the IRS API.
// Standard region queries return no products, so the convenience
// builder falls back to default_price and PriceSource::Default.
// The raw ProductFilter query targeting the same region similarly
// returns no results, keeping both paths consistent.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_eastus() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("eastus").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_westus2() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("westus2").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_westeurope() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("westeurope").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_northeurope() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("northeurope").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_southeastasia() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("southeastasia").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_japaneast() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("japaneast").await
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_brazilsouth() -> Result<(), Box<dyn std::error::Error>> {
    test_region_pricing("brazilsouth").await
}

/// Helper function to test pricing for a specific region.
///
/// Azure Load Balancer is global-only in the IRS API, so standard region queries
/// fall back to the hardcoded default_price. Both the convenience builder and
/// a raw ProductFilter targeting the same region will miss live products and
/// use the default value, keeping them consistent.
async fn test_region_pricing(region: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 1. Get price using convenience function (returns tier1 hourly per-rule price)
    let convenience_result = client
        .azure()
        .load_balancer_rules()
        .region(region)
        .fetch()
        .await?;

    // 2. Get price using raw ProductFilter with the same attributes.
    //    The query will return no products for non-Global regions, so both
    //    paths resolve to the default tier1 price of $0.025/hr.
    let filter = ProductFilter::builder()
        .vendor("azure")
        .service("Load Balancer")
        .product_family("Networking")
        .attribute("productName", "Load Balancer")
        .attribute("skuName", "Standard")
        .attribute("meterName", "Standard Included LB Rules and Outbound Rules")
        .region(region)
        .build();

    let products = client.query_products(filter).await?;

    // For standard regions the IRS returns no results; default fallback applies.
    let raw_price = if products.is_empty() {
        DEFAULT_TIER1_HOURLY
    } else {
        products[0].first_nonzero_price_or(DEFAULT_TIER1_HOURLY)
    };

    // 3. Compare results — both should equal the default tier1 price
    assert_eq!(
        convenience_result.price, raw_price,
        "Load Balancer Rules price mismatch for {}: convenience={}, raw={}",
        region, convenience_result.price, raw_price
    );

    // 4. Validate source tracking — must be Default for standard regions
    assert_eq!(
        convenience_result.source,
        PriceSource::Default,
        "Expected Default source for region {} (global-only pricing), got {:?}",
        region,
        convenience_result.source
    );

    // 5. Validate price equals the documented tier1 default
    assert_eq!(
        convenience_result.price, DEFAULT_TIER1_HOURLY,
        "Load Balancer Rules default tier1 price should be ${}/hr for region {}",
        DEFAULT_TIER1_HOURLY, region
    );

    // 6. Unit must be "hour"
    assert_eq!(
        convenience_result.unit, "hour",
        "Unit should be 'hour' for region {}, got '{}'",
        region, convenience_result.unit
    );

    println!(
        "Region {}: price={}, source={:?}",
        region, convenience_result.price, convenience_result.source
    );

    Ok(())
}

// ============================================================
// Global Region Direct Query Test
// ============================================================
// Querying the "Global" region directly should also return the
// default price (unless the API has live data for it).

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_global_region() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("Global")
        .fetch()
        .await?;

    // Should return a valid hourly price — either from API or default
    assert!(
        result.price > 0.0,
        "Load Balancer Rules price for Global region should be positive, got {}",
        result.price
    );
    assert_eq!(
        result.unit, "hour",
        "Unit should be 'hour' for Global region, got '{}'",
        result.unit
    );

    println!(
        "Global region: price={}, source={:?}",
        result.price, result.source
    );

    Ok(())
}

// ============================================================
// Source Tracking Validation
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_source_tracking_across_regions()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // Validate that every test region returns PriceSource::Default
    for region in AZURE_TEST_REGIONS {
        let result = client
            .azure()
            .load_balancer_rules()
            .region(*region)
            .fetch()
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to fetch Load Balancer Rules price for region {}: {}",
                    region, e
                )
            });

        assert_eq!(
            result.source,
            PriceSource::Default,
            "Region {} should return Default pricing (global-only IRS data), got {:?}",
            region,
            result.source
        );

        assert_eq!(
            result.price, DEFAULT_TIER1_HOURLY,
            "Region {} should return the default tier1 price ${}/hr, got {}",
            region, DEFAULT_TIER1_HOURLY, result.price
        );

        assert_eq!(
            result.unit, "hour",
            "Region {} unit should be 'hour', got '{}'",
            region, result.unit
        );

        println!(
            "Source tracking validated for {}: price={}, source={:?}",
            region, result.price, result.source
        );
    }

    Ok(())
}

// ============================================================
// Monthly Conversion — Zero Rules
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_zero_rules() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("eastus")
        .rule_count(0)
        .fetch_monthly()
        .await?;

    // 0 rules => $0.00/month
    assert_eq!(
        result.price, 0.0,
        "Monthly cost for 0 rules should be $0.00, got {}",
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!("Monthly (0 rules): price={}", result.price);
    Ok(())
}

// ============================================================
// Monthly Conversion — Tier1 Only (rules <= 5)
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_three_rules() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("eastus")
        .rule_count(3)
        .fetch_monthly()
        .await?;

    // 3 x $0.025 x 730 = $54.75
    let expected = 3.0 * DEFAULT_TIER1_HOURLY * HOURS_PER_MONTH;
    assert!(
        (result.price - expected).abs() < 0.001,
        "Monthly cost for 3 rules should be {}, got {}",
        expected,
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!(
        "Monthly (3 rules): price={}, expected={}",
        result.price, expected
    );
    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_five_rules() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("eastus")
        .rule_count(5)
        .fetch_monthly()
        .await?;

    // 5 x $0.025 x 730 = $91.25
    let expected = TIER1_MAX as f64 * DEFAULT_TIER1_HOURLY * HOURS_PER_MONTH;
    assert!(
        (result.price - expected).abs() < 0.001,
        "Monthly cost for 5 rules should be {}, got {}",
        expected,
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!(
        "Monthly (5 rules): price={}, expected={}",
        result.price, expected
    );
    Ok(())
}

// ============================================================
// Monthly Conversion — Cross-Tier (rules > 5)
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_ten_rules() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("eastus")
        .rule_count(10)
        .fetch_monthly()
        .await?;

    // first 5: 5 x $0.025 x 730 = $91.25
    // next  5: 5 x $0.010 x 730 = $36.50
    // total  : $127.75
    let tier1_cost = TIER1_MAX as f64 * DEFAULT_TIER1_HOURLY * HOURS_PER_MONTH;
    let tier2_cost = 5.0 * DEFAULT_TIER2_HOURLY * HOURS_PER_MONTH;
    let expected = tier1_cost + tier2_cost; // 127.75

    assert!(
        (result.price - expected).abs() < 0.001,
        "Monthly cost for 10 rules should be {}, got {}",
        expected,
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!(
        "Monthly (10 rules): price={}, expected={}",
        result.price, expected
    );
    Ok(())
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_twenty_rules() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("westeurope")
        .rule_count(20)
        .fetch_monthly()
        .await?;

    // first 5:  5 x $0.025 x 730 = $91.25
    // next  15: 15 x $0.010 x 730 = $109.50
    // total   : $200.75
    let tier1_cost = TIER1_MAX as f64 * DEFAULT_TIER1_HOURLY * HOURS_PER_MONTH;
    let tier2_cost = 15.0 * DEFAULT_TIER2_HOURLY * HOURS_PER_MONTH;
    let expected = tier1_cost + tier2_cost;

    assert!(
        (result.price - expected).abs() < 0.001,
        "Monthly cost for 20 rules should be {}, got {}",
        expected,
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!(
        "Monthly (20 rules): price={}, expected={}",
        result.price, expected
    );
    Ok(())
}

// ============================================================
// Tier Boundary: 6 Rules (just over tier1 threshold)
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_six_rules() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("southeastasia")
        .rule_count(6)
        .fetch_monthly()
        .await?;

    // first 5: 5 x $0.025 x 730 = $91.25
    // next  1: 1 x $0.010 x 730 = $7.30
    // total  : $98.55
    let tier1_cost = TIER1_MAX as f64 * DEFAULT_TIER1_HOURLY * HOURS_PER_MONTH;
    let tier2_cost = 1.0 * DEFAULT_TIER2_HOURLY * HOURS_PER_MONTH;
    let expected = tier1_cost + tier2_cost;

    assert!(
        (result.price - expected).abs() < 0.001,
        "Monthly cost for 6 rules should be {}, got {}",
        expected,
        result.price
    );
    assert_eq!(result.unit, "month");
    assert_eq!(result.source, PriceSource::Default);

    println!(
        "Monthly (6 rules): price={}, expected={}",
        result.price, expected
    );
    Ok(())
}

// ============================================================
// Tier Additive Cost Validation
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_tier2_adds_cost() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;
    let region = "japaneast";

    // Cost for exactly tier1 max
    let tier1_only = client
        .azure()
        .load_balancer_rules()
        .region(region)
        .rule_count(5)
        .fetch_monthly()
        .await?;

    // Cost with one rule in tier2
    let with_tier2 = client
        .azure()
        .load_balancer_rules()
        .region(region)
        .rule_count(6)
        .fetch_monthly()
        .await?;

    // The extra rule should cost $0.010 x 730 = $7.30
    assert!(
        with_tier2.price > tier1_only.price,
        "6 rules ({}) should cost more than 5 rules ({})",
        with_tier2.price,
        tier1_only.price
    );

    let extra_rule_cost = with_tier2.price - tier1_only.price;
    let expected_extra = DEFAULT_TIER2_HOURLY * HOURS_PER_MONTH; // 7.30

    assert!(
        (extra_rule_cost - expected_extra).abs() < 0.001,
        "Extra tier2 rule cost should be {}, got {}",
        expected_extra,
        extra_rule_cost
    );

    assert_eq!(tier1_only.unit, "month");
    assert_eq!(with_tier2.unit, "month");

    println!(
        "Tier2 boundary validated: 5_rules={}, 6_rules={}, extra_rule_cost={}",
        tier1_only.price, with_tier2.price, extra_rule_cost
    );

    Ok(())
}

// ============================================================
// Monthly Consistency Across Regions
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_consistent_across_regions()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    // 10 rules should yield the same $127.75 in every region
    // (since all fall back to global defaults)
    let tier1_cost = TIER1_MAX as f64 * DEFAULT_TIER1_HOURLY * HOURS_PER_MONTH; // 91.25
    let tier2_cost = 5.0 * DEFAULT_TIER2_HOURLY * HOURS_PER_MONTH; // 36.50
    let expected_monthly = tier1_cost + tier2_cost; // 127.75

    for region in AZURE_TEST_REGIONS {
        let result = client
            .azure()
            .load_balancer_rules()
            .region(*region)
            .rule_count(10)
            .fetch_monthly()
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to fetch monthly Load Balancer Rules price for region {}: {}",
                    region, e
                )
            });

        assert!(
            (result.price - expected_monthly).abs() < 0.001,
            "Region {} monthly price for 10 rules should be {}, got {}",
            region,
            expected_monthly,
            result.price
        );
        assert_eq!(
            result.unit, "month",
            "Region {} unit should be 'month'",
            region
        );
        assert_eq!(
            result.source,
            PriceSource::Default,
            "Region {} should have Default source",
            region
        );

        println!(
            "Monthly consistency validated for {}: price={}",
            region, result.price
        );
    }

    Ok(())
}

// ============================================================
// fetch_monthly Without rule_count Should Fail
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_load_balancer_rules_monthly_requires_rule_count()
-> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    let result = client
        .azure()
        .load_balancer_rules()
        .region("eastus")
        .fetch_monthly()
        .await;

    assert!(
        result.is_err(),
        "fetch_monthly() without rule_count should return an error"
    );

    println!("Correctly returned error: {:?}", result.err());
    Ok(())
}
