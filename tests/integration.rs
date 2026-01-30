//! Integration tests for the Infracost client.
//!
//! These tests require a valid API key set in the INFRACOST_API_KEY environment variable
//! or in a .env file. They make real API calls and should be run with:
//!
//! ```bash
//! cargo test --test integration -- --ignored
//! ```

use infracost::{Client, ProductFilter};

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
