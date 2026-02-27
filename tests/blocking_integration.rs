//! Integration tests for the blocking provider convenience API.
//!
//! These tests require a valid API key set in the INFRACOST_API_KEY environment variable
//! or in a .env file. They make real API calls and should be run with:
//!
//! ```bash
//! cargo test --features blocking,cache-memory --test blocking_integration -- --ignored
//! ```

#![cfg(feature = "blocking")]

use infracost_rs::blocking::Client;
use infracost_rs::providers::aws::EbsType;
use infracost_rs::providers::azure::{ManagedDiskSize, ManagedDiskType};
use infracost_rs::providers::gcp::{DiskType, SnapshotType};

fn get_client() -> Option<Client> {
    // Try to load from .env file
    let _ = dotenvy::dotenv();
    Client::from_env().ok()
}

// ============================================================
// GCP Provider Integration Tests
// ============================================================

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_disk_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test all disk types
    for disk_type in [
        DiskType::PdStandard,
        DiskType::PdSsd,
        DiskType::PdBalanced,
        DiskType::PdExtreme,
    ] {
        let result = client
            .gcp()
            .disk(disk_type)
            .region("us-central1")
            .fetch()
            .expect("Query should succeed");

        assert!(
            result.is_from_api(),
            "Should get price from API for {:?}",
            disk_type
        );
        assert!(
            result.price > 0.0,
            "Price should be positive for {:?}",
            disk_type
        );
        assert_eq!(result.unit, "GiB-month");
    }
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_disk_fetch_price() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let price = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_disk_fetch_monthly() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .size_gb(100)
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Monthly price should be positive");
    assert_eq!(result.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_snapshot_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "GiB-month");

    // Test fetch_price
    let price = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .size_gb(50)
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_archive_snapshot_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch (storage rate)
    let result = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region("us-central1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "GiB-month");

    // Archive should be cheaper than standard
    let standard = client
        .gcp()
        .snapshot(SnapshotType::Standard)
        .region("us-central1")
        .fetch()
        .expect("Standard snapshot query should succeed");

    assert!(
        result.price < standard.price,
        "Archive (${}) should be cheaper than standard (${})",
        result.price,
        standard.price
    );

    // Test fetch_price
    let price = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region("us-central1")
        .fetch_price()
        .expect("fetch_price should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly (storage only)
    let monthly_storage = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region("us-central1")
        .size_gb(500)
        .fetch_monthly()
        .expect("Monthly storage query should succeed");

    assert!(
        monthly_storage.price > 0.0,
        "Monthly storage price should be positive"
    );
    assert_eq!(monthly_storage.unit, "month");

    // Test fetch_monthly (storage + retrieval)
    let monthly_with_retrieval = client
        .gcp()
        .snapshot(SnapshotType::Archive)
        .region("us-central1")
        .size_gb(500)
        .retrieval_size_gb(100)
        .fetch_monthly()
        .expect("Monthly with retrieval query should succeed");

    assert!(
        monthly_with_retrieval.price > monthly_storage.price,
        "Adding retrieval should increase monthly cost"
    );
    assert_eq!(monthly_with_retrieval.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_static_ip_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");

    // Test fetch_price
    let price = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_nat_gateway_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");

    // Test fetch_price
    let price = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_gcp_forwarding_rule_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");

    // Test fetch_price
    let price = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

// ============================================================
// AWS Provider Integration Tests
// ============================================================

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_ebs_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test all EBS types
    for ebs_type in [
        EbsType::Gp3,
        EbsType::Gp2,
        EbsType::Io2,
        EbsType::St1,
        EbsType::Sc1,
    ] {
        let result = client
            .aws()
            .ebs(ebs_type)
            .region("us-east-1")
            .fetch()
            .expect("Query should succeed");

        assert!(
            result.is_from_api(),
            "Should get price from API for {:?}",
            ebs_type
        );
        assert!(
            result.price > 0.0,
            "Price should be positive for {:?}",
            ebs_type
        );
        assert_eq!(result.unit, "GB-month");
    }
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_ebs_fetch_price() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let price = client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_ebs_fetch_monthly() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .size_gb(100)
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Monthly price should be positive");
    assert_eq!(result.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_snapshot_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .aws()
        .snapshot()
        .region("us-east-1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "GB-month");

    // Test fetch_price
    let price = client
        .aws()
        .snapshot()
        .region("us-east-1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .aws()
        .snapshot()
        .region("us-east-1")
        .size_gb(50)
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_elastic_ip_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");

    // Test fetch_price
    let price = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_nat_gateway_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");

    // Test fetch_price
    let price = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_alb_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .aws()
        .alb()
        .region("us-east-1")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");

    // Test fetch_price
    let price = client
        .aws()
        .alb()
        .region("us-east-1")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .aws()
        .alb()
        .region("us-east-1")
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

// ============================================================
// Azure Provider Integration Tests
// ============================================================

#[test]
#[ignore = "Requires API key"]
fn test_blocking_azure_managed_disk_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test Premium SSD P10
    let result = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "month");

    // Test Standard SSD E10
    let result = client
        .azure()
        .managed_disk(ManagedDiskType::StandardSsd, ManagedDiskSize::E10)
        .region("eastus")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");

    // Test Standard HDD S10
    let result = client
        .azure()
        .managed_disk(ManagedDiskType::StandardHdd, ManagedDiskSize::S10)
        .region("eastus")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_azure_managed_disk_fetch_price() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let price = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_azure_snapshot_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .azure()
        .snapshot()
        .region("eastus")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "GB-month");

    // Test fetch_price
    let price = client
        .azure()
        .snapshot()
        .region("eastus")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .azure()
        .snapshot()
        .region("eastus")
        .size_gb(50)
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_azure_public_ip_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test fetch
    let result = client
        .azure()
        .public_ip()
        .region("eastus")
        .fetch()
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");

    // Test fetch_price
    let price = client
        .azure()
        .public_ip()
        .region("eastus")
        .fetch_price()
        .expect("Query should succeed");

    assert!(price > 0.0, "Price should be positive");

    // Test fetch_monthly
    let monthly = client
        .azure()
        .public_ip()
        .region("eastus")
        .fetch_monthly()
        .expect("Query should succeed");

    assert!(monthly.price > 0.0, "Monthly price should be positive");
    assert_eq!(monthly.unit, "month");
}

// ============================================================
// Blocking vs Async Parity Tests
// ============================================================

#[test]
#[ignore = "Requires API key"]
fn test_blocking_vs_async_parity_gcp_disk() {
    use tokio::runtime::Runtime;

    let blocking_client = get_client().expect("INFRACOST_API_KEY must be set");

    // Blocking call
    let blocking_result = blocking_client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch()
        .expect("Blocking query should succeed");

    // Async call
    let rt = Runtime::new().unwrap();
    let async_client = infracost_rs::Client::from_env().expect("INFRACOST_API_KEY must be set");
    let async_result = rt
        .block_on(async {
            async_client
                .gcp()
                .disk(DiskType::PdSsd)
                .region("us-central1")
                .fetch()
                .await
        })
        .expect("Async query should succeed");

    // Both should return the same price and unit
    assert_eq!(
        blocking_result.price, async_result.price,
        "Blocking and async should return same price"
    );
    assert_eq!(
        blocking_result.unit, async_result.unit,
        "Blocking and async should return same unit"
    );
    assert_eq!(
        blocking_result.is_from_api(),
        async_result.is_from_api(),
        "Blocking and async should have same source"
    );
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_vs_async_parity_aws_ebs() {
    use tokio::runtime::Runtime;

    let blocking_client = get_client().expect("INFRACOST_API_KEY must be set");

    // Blocking call
    let blocking_result = blocking_client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .fetch()
        .expect("Blocking query should succeed");

    // Async call
    let rt = Runtime::new().unwrap();
    let async_client = infracost_rs::Client::from_env().expect("INFRACOST_API_KEY must be set");
    let async_result = rt
        .block_on(async {
            async_client
                .aws()
                .ebs(EbsType::Gp3)
                .region("us-east-1")
                .fetch()
                .await
        })
        .expect("Async query should succeed");

    // Both should return the same price and unit
    assert_eq!(
        blocking_result.price, async_result.price,
        "Blocking and async should return same price"
    );
    assert_eq!(
        blocking_result.unit, async_result.unit,
        "Blocking and async should return same unit"
    );
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_vs_async_parity_azure_managed_disk() {
    use tokio::runtime::Runtime;

    let blocking_client = get_client().expect("INFRACOST_API_KEY must be set");

    // Blocking call
    let blocking_result = blocking_client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch()
        .expect("Blocking query should succeed");

    // Async call
    let rt = Runtime::new().unwrap();
    let async_client = infracost_rs::Client::from_env().expect("INFRACOST_API_KEY must be set");
    let async_result = rt
        .block_on(async {
            async_client
                .azure()
                .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
                .region("eastus")
                .fetch()
                .await
        })
        .expect("Async query should succeed");

    // Both should return the same price and unit
    assert_eq!(
        blocking_result.price, async_result.price,
        "Blocking and async should return same price"
    );
    assert_eq!(
        blocking_result.unit, async_result.unit,
        "Blocking and async should return same unit"
    );
}

// ============================================================
// Cache Support Tests
// ============================================================

#[cfg(feature = "cache-memory")]
#[test]
#[ignore = "Requires API key"]
fn test_blocking_client_with_memory_cache() {
    use infracost_rs::cache::MemoryCache;
    use std::time::Duration;

    let _ = dotenvy::dotenv();
    let api_key = std::env::var("INFRACOST_API_KEY").expect("INFRACOST_API_KEY must be set");

    let client = Client::builder()
        .api_key(api_key)
        .with_cache(MemoryCache::new())
        .cache_ttl(Duration::from_secs(300))
        .build()
        .expect("Should build client with cache");

    // First call - should hit API
    let result1 = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch()
        .expect("First query should succeed");

    assert!(result1.is_from_api(), "First call should be from API");
    assert!(result1.price > 0.0, "Should have a price");

    // Second call with same parameters - should succeed
    let result2 = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch()
        .expect("Second query should succeed");

    // Results should match
    assert_eq!(result1.price, result2.price, "Prices should match");
    assert_eq!(result1.unit, result2.unit, "Units should match");
}

#[cfg(feature = "cache-memory")]
#[test]
#[ignore = "Requires API key"]
fn test_blocking_cache_different_queries() {
    use infracost_rs::cache::MemoryCache;

    let _ = dotenvy::dotenv();
    let api_key = std::env::var("INFRACOST_API_KEY").expect("INFRACOST_API_KEY must be set");

    let client = Client::builder()
        .api_key(api_key)
        .with_cache(MemoryCache::new())
        .build()
        .expect("Should build client with cache");

    // Query 1: GCP SSD disk
    let result1 = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch()
        .expect("Query 1 should succeed");

    // Query 2: GCP Standard disk (different cache key)
    let result2 = client
        .gcp()
        .disk(DiskType::PdStandard)
        .region("us-central1")
        .fetch()
        .expect("Query 2 should succeed");

    // Prices should be different
    assert_ne!(
        result1.price, result2.price,
        "Different disk types should have different prices"
    );
}

#[cfg(feature = "cache-memory")]
#[test]
#[ignore = "Requires API key"]
fn test_blocking_cache_multi_provider() {
    use infracost_rs::cache::MemoryCache;

    let _ = dotenvy::dotenv();
    let api_key = std::env::var("INFRACOST_API_KEY").expect("INFRACOST_API_KEY must be set");

    let client = Client::builder()
        .api_key(api_key)
        .with_cache(MemoryCache::new())
        .build()
        .expect("Should build client with cache");

    // Test caching works across different providers
    let gcp_result = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch()
        .expect("GCP query should succeed");

    let aws_result = client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .fetch()
        .expect("AWS query should succeed");

    let azure_result = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch()
        .expect("Azure query should succeed");

    assert!(gcp_result.price > 0.0, "GCP should have price");
    assert!(aws_result.price > 0.0, "AWS should have price");
    assert!(azure_result.price > 0.0, "Azure should have price");
}

// ============================================================
// Error Condition Tests
// ============================================================

#[test]
#[ignore = "Requires API key"]
fn test_blocking_error_on_fallback() {
    // Create an anonymous client with error_on_fallback enabled
    let client = Client::builder()
        .error_on_fallback(true)
        .build()
        .expect("Should build client");

    // This should error because we don't have an API key and fallback is disabled
    let result = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch();

    assert!(
        result.is_err(),
        "Should error when no API key and error_on_fallback is true"
    );
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_fetch_monthly_without_size_gb() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // This should error because we didn't set size_gb
    let result = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .fetch_monthly();

    assert!(
        result.is_err(),
        "Should error when fetch_monthly called without size_gb"
    );
    assert!(
        result.unwrap_err().to_string().contains("size_gb"),
        "Error should mention size_gb"
    );
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_aws_fetch_monthly_without_size_gb() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // This should error because we didn't set size_gb
    let result = client
        .aws()
        .ebs(EbsType::Gp3)
        .region("us-east-1")
        .fetch_monthly();

    assert!(
        result.is_err(),
        "Should error when fetch_monthly called without size_gb"
    );
}

#[test]
#[ignore = "Requires API key"]
fn test_blocking_azure_fetch_monthly_without_size_gb() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // This should error because we didn't set size_gb (for snapshot)
    let result = client.azure().snapshot().region("eastus").fetch_monthly();

    assert!(
        result.is_err(),
        "Should error when fetch_monthly called without size_gb"
    );
}

// ============================================================
// API Key Override Tests
// ============================================================

#[test]
#[ignore = "Requires API key"]
fn test_blocking_api_key_override() {
    let _ = dotenvy::dotenv();
    let api_key = std::env::var("INFRACOST_API_KEY").expect("INFRACOST_API_KEY must be set");

    // Create an anonymous client
    let client = Client::anonymous().unwrap();

    // Override the API key for this request
    let result = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .api_key(api_key)
        .fetch()
        .expect("Query with API key override should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
}

// ============================================================
// Override Default Tests
// ============================================================

#[test]
fn test_blocking_override_default_fallback() {
    // Create an anonymous client
    let client = Client::anonymous().unwrap();

    // Override the default fallback price
    let result = client
        .gcp()
        .disk(DiskType::PdSsd)
        .region("us-central1")
        .override_default(0.25)
        .fetch()
        .expect("Query with override_default should succeed");

    assert!(!result.is_from_api(), "Should use fallback");
    assert_eq!(result.price, 0.25, "Should use overridden default");
}
