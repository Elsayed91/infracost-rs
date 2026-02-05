//! Integration tests for the Infracost client.
//!
//! These tests require a valid API key set in the INFRACOST_API_KEY environment variable
//! or in a .env file. They make real API calls and should be run with:
//!
//! ```bash
//! cargo test --test integration -- --ignored
//! ```

use infracost_rs::providers::aws::EbsType;
use infracost_rs::providers::azure::{ManagedDiskSize, ManagedDiskType};
use infracost_rs::providers::gcp::DiskType;
use infracost_rs::{Client, ProductFilter};

fn get_client() -> Option<Client> {
    // Try to load from .env file
    let _ = dotenvy::dotenv();
    Client::from_env().ok()
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_query_gcp_compute_engine() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let products = client
        .products()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(!products.is_empty(), "Should return products");

    for product in &products {
        assert_eq!(product.vendor_name.to_lowercase(), "gcp");
        assert_eq!(product.service, "Compute Engine");
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_query_gcp_disk_pricing() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let products = client
        .products()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .attribute("description", "SSD backed PD Capacity")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(!products.is_empty(), "Should find SSD disk pricing");

    let product = &products[0];
    let price = product.price_f64().expect("Should have a price");
    assert!(price > 0.0, "Price should be positive");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_query_aws_s3() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Query AWS S3 pricing (simpler than EC2)
    let products = client
        .products()
        .vendor("aws")
        .service("AmazonS3")
        .region("us-east-1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(!products.is_empty(), "Should return S3 products");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_query_with_regex() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let products = client
        .products()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .attribute_regex("description", ".*SSD.*Capacity.*")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(
        !products.is_empty(),
        "Should find products with regex match"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_query_products_direct() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .build();

    let products = client
        .query_products(filter)
        .await
        .expect("Query should succeed");

    assert!(!products.is_empty(), "Should return products");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_fetch_one() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let product = client
        .products()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .attribute("description", "SSD backed PD Capacity")
        .fetch_one()
        .await
        .expect("Query should succeed");

    assert!(product.is_some(), "Should find at least one product");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_price_filtering() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let products = client
        .products()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .fetch()
        .await
        .expect("Query should succeed");

    if let Some(product) = products.first() {
        // Test price filtering
        let prices = product.prices().collect();
        assert!(!prices.is_empty() || product.prices.is_empty());
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_product_attributes() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let products = client
        .products()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .attribute("description", "SSD backed PD Capacity")
        .fetch()
        .await
        .expect("Query should succeed");

    if let Some(product) = products.first() {
        let desc = product.attribute("description");
        assert!(desc.is_some(), "Should have description attribute");
        assert!(
            desc.unwrap().contains("SSD"),
            "Description should contain SSD"
        );
    }
}

// ============================================================
// GCP Provider Integration Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_gcp_disk_provider() {
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
            .await
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
        assert_eq!(result.unit, "GB-month");
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_gcp_snapshot_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .gcp()
        .snapshot()
        .region("us-central1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "GB-month");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_gcp_static_ip_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .gcp()
        .static_ip()
        .region("us-central1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_gcp_nat_gateway_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .gcp()
        .nat_gateway()
        .region("us-central1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_gcp_forwarding_rule_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .gcp()
        .forwarding_rule()
        .region("us-central1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

// ============================================================
// AWS Provider Integration Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_aws_ebs_provider() {
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
            .await
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

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_aws_snapshot_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .aws()
        .snapshot()
        .region("us-east-1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "GB-month");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_aws_elastic_ip_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .aws()
        .elastic_ip()
        .region("us-east-1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_aws_nat_gateway_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .aws()
        .nat_gateway()
        .region("us-east-1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_aws_alb_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .aws()
        .alb()
        .region("us-east-1")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

// ============================================================
// Azure Provider Integration Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_azure_managed_disk_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    // Test Premium SSD P10
    let result = client
        .azure()
        .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
        .region("eastus")
        .fetch()
        .await
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
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");

    // Test Standard HDD S10
    let result = client
        .azure()
        .managed_disk(ManagedDiskType::StandardHdd, ManagedDiskSize::S10)
        .region("eastus")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_azure_snapshot_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .azure()
        .snapshot()
        .region("eastus")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "GB-month");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_azure_public_ip_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");

    let result = client
        .azure()
        .public_ip()
        .region("eastus")
        .fetch()
        .await
        .expect("Query should succeed");

    assert!(result.is_from_api(), "Should get price from API");
    assert!(result.price > 0.0, "Price should be positive");
    assert_eq!(result.unit, "hour");
}

// ============================================================
// Redis Cache Integration Tests
// ============================================================

#[cfg(feature = "cache-redis")]
mod redis_cache_tests {
    use infracost_rs::cache::RedisCache;
    use infracost_rs::{Client, PriceCache};
    use std::time::Duration;

    const REDIS_URL: &str = "redis://localhost:6379";

    fn get_client_with_cache() -> Option<Client> {
        let _ = dotenvy::dotenv();

        let cache = RedisCache::new(REDIS_URL).ok()?;

        Client::builder()
            .api_key(std::env::var("INFRACOST_API_KEY").ok()?)
            .with_cache(cache)
            .cache_ttl(Duration::from_secs(300)) // 5 minute TTL for tests
            .build()
            .ok()
    }

    #[tokio::test]
    #[ignore = "Requires Redis and API key"]
    async fn test_redis_cache_basic_operations() {
        let cache = RedisCache::new(REDIS_URL).expect("Should connect to Redis");

        // Test set and get
        let products = vec![infracost_rs::Product {
            product_hash: "test-hash".to_string(),
            vendor_name: "test-vendor".to_string(),
            service: "test-service".to_string(),
            product_family: Some("test-family".to_string()),
            region: Some("us-east-1".to_string()),
            sku: "test-sku".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        cache
            .set("test-key", &products, Duration::from_secs(60))
            .await;

        let cached = cache.get("test-key").await;
        assert!(cached.is_some(), "Should retrieve cached products");
        assert_eq!(cached.unwrap().len(), 1);

        // Clean up
        cache.clear().await;
    }

    #[tokio::test]
    #[ignore = "Requires Redis and API key"]
    async fn test_redis_cache_miss() {
        let cache = RedisCache::new(REDIS_URL).expect("Should connect to Redis");

        let cached = cache.get("nonexistent-key-12345").await;
        assert!(cached.is_none(), "Should return None for cache miss");
    }

    #[tokio::test]
    #[ignore = "Requires Redis and API key"]
    async fn test_redis_cache_with_client() {
        let client = get_client_with_cache().expect("Requires API key and Redis");

        // First call - should hit API and cache the result
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("First query should succeed");

        assert!(result1.is_from_api(), "First call should be from API");
        assert!(result1.price > 0.0, "Should have a price");

        // Second call with same parameters - should be a cache hit
        // Note: We can't directly verify cache hit, but the call should succeed
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Second query should succeed");

        // Results should match
        assert_eq!(result1.price, result2.price, "Prices should match");
        assert_eq!(result1.unit, result2.unit, "Units should match");
    }

    #[tokio::test]
    #[ignore = "Requires Redis and API key"]
    async fn test_redis_cache_different_queries() {
        let client = get_client_with_cache().expect("Requires API key and Redis");

        // Query 1: GCP SSD disk
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 1 should succeed");

        // Query 2: GCP Standard disk (different cache key)
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdStandard)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 2 should succeed");

        // Prices should be different
        assert_ne!(
            result1.price, result2.price,
            "Different disk types should have different prices"
        );
    }

    #[tokio::test]
    #[ignore = "Requires Redis and API key"]
    async fn test_redis_cache_multi_provider() {
        let client = get_client_with_cache().expect("Requires API key and Redis");

        // Test caching works across different providers
        let gcp_result = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("GCP query should succeed");

        let aws_result = client
            .aws()
            .ebs(infracost_rs::providers::aws::EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .await
            .expect("AWS query should succeed");

        assert!(gcp_result.price > 0.0, "GCP should have price");
        assert!(aws_result.price > 0.0, "AWS should have price");
    }

    #[tokio::test]
    #[ignore = "Requires Redis and API key"]
    async fn test_redis_cache_clear() {
        let cache = RedisCache::new(REDIS_URL).expect("Should connect to Redis");

        // Set some test data
        let products = vec![];
        cache
            .set("infracost:v1:test1", &products, Duration::from_secs(60))
            .await;
        cache
            .set("infracost:v1:test2", &products, Duration::from_secs(60))
            .await;

        // Verify data was set
        assert!(cache.get("infracost:v1:test1").await.is_some());

        // Clear all infracost keys
        cache.clear().await;

        // Give Redis a moment to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify data was cleared
        assert!(
            cache.get("infracost:v1:test1").await.is_none(),
            "Cache should be cleared"
        );
        assert!(
            cache.get("infracost:v1:test2").await.is_none(),
            "Cache should be cleared"
        );
    }
}

// ============================================================
// SQLite Cache Integration Tests
// ============================================================

#[cfg(feature = "cache-sqlite")]
mod sqlite_cache_tests {
    use infracost_rs::cache::SqliteCache;
    use infracost_rs::{Client, PriceCache};
    use std::time::Duration;

    fn get_client_with_cache() -> Option<Client> {
        let _ = dotenvy::dotenv();

        // Use temp file for integration tests
        let cache = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(SqliteCache::new("/tmp/infracost_test_cache.db"))
            .ok()?;

        Client::builder()
            .api_key(std::env::var("INFRACOST_API_KEY").ok()?)
            .with_cache(cache)
            .cache_ttl(Duration::from_secs(300))
            .build()
            .ok()
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_basic_operations() {
        let cache = SqliteCache::new("/tmp/infracost_sqlite_test.db")
            .await
            .expect("Should create SQLite cache");

        // Test set and get
        let products = vec![infracost_rs::Product {
            product_hash: "test-hash".to_string(),
            vendor_name: "test-vendor".to_string(),
            service: "test-service".to_string(),
            product_family: Some("test-family".to_string()),
            region: Some("us-east-1".to_string()),
            sku: "test-sku".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        cache
            .set("sqlite-test-key", &products, Duration::from_secs(60))
            .await;

        let cached = cache.get("sqlite-test-key").await;
        assert!(cached.is_some(), "Should retrieve cached products");
        assert_eq!(cached.unwrap().len(), 1);

        // Clean up
        cache.clear().await;
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_miss() {
        let cache = SqliteCache::new("/tmp/infracost_sqlite_test.db")
            .await
            .expect("Should create SQLite cache");

        let cached = cache.get("nonexistent-key-sqlite-12345").await;
        assert!(cached.is_none(), "Should return None for cache miss");
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_with_client() {
        let client = get_client_with_cache().expect("Requires API key");

        // First call - should hit API and cache the result
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("First query should succeed");

        assert!(result1.is_from_api(), "First call should be from API");
        assert!(result1.price > 0.0, "Should have a price");

        // Second call with same parameters - should be a cache hit
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Second query should succeed");

        // Results should match
        assert_eq!(result1.price, result2.price, "Prices should match");
        assert_eq!(result1.unit, result2.unit, "Units should match");
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_different_queries() {
        let client = get_client_with_cache().expect("Requires API key");

        // Query 1: GCP SSD disk
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 1 should succeed");

        // Query 2: GCP Standard disk (different cache key)
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdStandard)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 2 should succeed");

        // Prices should be different
        assert_ne!(
            result1.price, result2.price,
            "Different disk types should have different prices"
        );
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_multi_provider() {
        let client = get_client_with_cache().expect("Requires API key");

        // Test caching works across different providers
        let gcp_result = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("GCP query should succeed");

        let aws_result = client
            .aws()
            .ebs(infracost_rs::providers::aws::EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .await
            .expect("AWS query should succeed");

        assert!(gcp_result.price > 0.0, "GCP should have price");
        assert!(aws_result.price > 0.0, "AWS should have price");
    }

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_sqlite_cache_clear() {
        let cache = SqliteCache::new("/tmp/infracost_sqlite_clear_test.db")
            .await
            .expect("Should create SQLite cache");

        // Set some test data
        let products = vec![];
        cache
            .set(
                "infracost:v1:sqlite-test1",
                &products,
                Duration::from_secs(60),
            )
            .await;
        cache
            .set(
                "infracost:v1:sqlite-test2",
                &products,
                Duration::from_secs(60),
            )
            .await;

        // Verify data was set
        assert!(cache.get("infracost:v1:sqlite-test1").await.is_some());

        // Clear all entries
        cache.clear().await;

        // Give it a moment
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify data was cleared
        assert!(
            cache.get("infracost:v1:sqlite-test1").await.is_none(),
            "Cache should be cleared"
        );
        assert!(
            cache.get("infracost:v1:sqlite-test2").await.is_none(),
            "Cache should be cleared"
        );
    }
}

// ============================================================
// PostgreSQL Cache Integration Tests
// ============================================================

#[cfg(feature = "cache-postgres")]
mod postgres_cache_tests {
    use infracost_rs::cache::PostgresCache;
    use infracost_rs::{Client, PriceCache};
    use std::time::Duration;

    fn get_postgres_url() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://infracost:infracost@localhost/infracost_cache".to_string()
        })
    }

    fn get_client_with_cache() -> Option<Client> {
        let _ = dotenvy::dotenv();

        let url = get_postgres_url();
        let cache = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(PostgresCache::new(&url))
            .ok()?;

        Client::builder()
            .api_key(std::env::var("INFRACOST_API_KEY").ok()?)
            .with_cache(cache)
            .cache_ttl(Duration::from_secs(300))
            .build()
            .ok()
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_basic_operations() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        // Test set and get
        let products = vec![infracost_rs::Product {
            product_hash: "test-hash".to_string(),
            vendor_name: "test-vendor".to_string(),
            service: "test-service".to_string(),
            product_family: Some("test-family".to_string()),
            region: Some("us-east-1".to_string()),
            sku: "test-sku".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        cache
            .set("postgres-test-key", &products, Duration::from_secs(60))
            .await;

        let cached = cache.get("postgres-test-key").await;
        assert!(cached.is_some(), "Should retrieve cached products");
        assert_eq!(cached.unwrap().len(), 1);

        // Clean up
        cache.clear().await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_miss() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        let cached = cache.get("nonexistent-key-postgres-12345").await;
        assert!(cached.is_none(), "Should return None for cache miss");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_with_client() {
        let client = get_client_with_cache().expect("Requires API key and PostgreSQL");

        // First call - should hit API and cache the result
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("First query should succeed");

        assert!(result1.is_from_api(), "First call should be from API");
        assert!(result1.price > 0.0, "Should have a price");

        // Second call with same parameters - should be a cache hit
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Second query should succeed");

        // Results should match
        assert_eq!(result1.price, result2.price, "Prices should match");
        assert_eq!(result1.unit, result2.unit, "Units should match");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_different_queries() {
        let client = get_client_with_cache().expect("Requires API key and PostgreSQL");

        // Query 1: GCP SSD disk
        let result1 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 1 should succeed");

        // Query 2: GCP Standard disk (different cache key)
        let result2 = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdStandard)
            .region("us-central1")
            .fetch()
            .await
            .expect("Query 2 should succeed");

        // Prices should be different
        assert_ne!(
            result1.price, result2.price,
            "Different disk types should have different prices"
        );
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_multi_provider() {
        let client = get_client_with_cache().expect("Requires API key and PostgreSQL");

        // Test caching works across different providers
        let gcp_result = client
            .gcp()
            .disk(infracost_rs::providers::gcp::DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .expect("GCP query should succeed");

        let aws_result = client
            .aws()
            .ebs(infracost_rs::providers::aws::EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .await
            .expect("AWS query should succeed");

        assert!(gcp_result.price > 0.0, "GCP should have price");
        assert!(aws_result.price > 0.0, "AWS should have price");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_clear() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        // Set some test data
        let products = vec![];
        cache
            .set("infracost:v1:pg-test1", &products, Duration::from_secs(60))
            .await;
        cache
            .set("infracost:v1:pg-test2", &products, Duration::from_secs(60))
            .await;

        // Verify data was set
        assert!(cache.get("infracost:v1:pg-test1").await.is_some());

        // Clear all entries
        cache.clear().await;

        // Give it a moment
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify data was cleared
        assert!(
            cache.get("infracost:v1:pg-test1").await.is_none(),
            "Cache should be cleared"
        );
        assert!(
            cache.get("infracost:v1:pg-test2").await.is_none(),
            "Cache should be cleared"
        );
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL and API key"]
    async fn test_postgres_cache_expiration() {
        let url = get_postgres_url();
        let cache = PostgresCache::new(&url)
            .await
            .expect("Should connect to PostgreSQL");

        let products = vec![infracost_rs::Product {
            product_hash: "expire-hash".to_string(),
            vendor_name: "test-vendor".to_string(),
            service: "test-service".to_string(),
            product_family: None,
            region: None,
            sku: "expire-sku".to_string(),
            attributes: vec![],
            prices: vec![],
        }];

        // Set with 1 second TTL
        cache
            .set("pg-expire-key", &products, Duration::from_secs(1))
            .await;

        // Should exist immediately
        assert!(cache.get("pg-expire-key").await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get("pg-expire-key").await.is_none());
    }
}
