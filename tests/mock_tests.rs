//! Tests for the mock client.

use infracost_rs::mock::{MockClient, MockProduct};
use infracost_rs::{Error, PricingClient, ProductFilter};

#[tokio::test]
async fn test_mock_client_basic() {
    let client = MockClient::builder()
        .with_product(
            MockProduct::new("gcp", "Compute Engine", "us-central1")
                .sku("pd-ssd")
                .price(0.170, "GB-month")
                .attribute("description", "SSD backed PD Capacity"),
        )
        .with_product(
            MockProduct::new("gcp", "Compute Engine", "us-central1")
                .sku("pd-standard")
                .price(0.040, "GB-month")
                .attribute("description", "Standard PD Capacity"),
        )
        .build();

    let products = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();

    assert_eq!(products.len(), 2);
}

#[tokio::test]
async fn test_mock_client_filtering() {
    let client = MockClient::builder()
        .with_product(
            MockProduct::new("gcp", "Compute Engine", "us-central1")
                .sku("pd-ssd")
                .price(0.170, "GB-month"),
        )
        .with_product(
            MockProduct::new("gcp", "Compute Engine", "us-east1")
                .sku("pd-ssd")
                .price(0.170, "GB-month"),
        )
        .with_product(
            MockProduct::new("aws", "AmazonEC2", "us-east-1")
                .sku("t3.micro")
                .price(0.0104, "Hrs"),
        )
        .build();

    // Filter by vendor
    let gcp = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();
    assert_eq!(gcp.len(), 2);

    // Filter by region
    let us_central = client
        .query_products(
            ProductFilter::builder()
                .vendor("gcp")
                .region("us-central1")
                .build(),
        )
        .await
        .unwrap();
    assert_eq!(us_central.len(), 1);

    // Filter by vendor and service
    let aws_ec2 = client
        .query_products(
            ProductFilter::builder()
                .vendor("aws")
                .service("AmazonEC2")
                .build(),
        )
        .await
        .unwrap();
    assert_eq!(aws_ec2.len(), 1);
}

#[tokio::test]
async fn test_mock_client_from_prices() {
    let client = MockClient::from_prices(&[
        (
            "gcp",
            "Compute Engine",
            "us-central1",
            "pd-ssd",
            0.170,
            "GB-month",
        ),
        (
            "gcp",
            "Compute Engine",
            "us-east1",
            "pd-ssd",
            0.170,
            "GB-month",
        ),
        (
            "gcp",
            "Compute Engine",
            "europe-west1",
            "pd-ssd",
            0.187,
            "GB-month",
        ),
        ("aws", "AmazonEC2", "us-east-1", "t3.micro", 0.0104, "Hrs"),
    ]);

    let products = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();
    assert_eq!(products.len(), 3);

    let products = client
        .query_products(ProductFilter::builder().vendor("aws").build())
        .await
        .unwrap();
    assert_eq!(products.len(), 1);
}

#[tokio::test]
async fn test_mock_client_price_extraction() {
    let client = MockClient::from_prices(&[(
        "gcp",
        "Compute Engine",
        "us-central1",
        "pd-ssd",
        0.170,
        "GB-month",
    )]);

    let products = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();

    let product = &products[0];
    let price = product.price_f64().unwrap();
    assert!((price - 0.170).abs() < 0.001);
}

#[tokio::test]
async fn test_mock_client_error() {
    let client = MockClient::builder()
        .with_error(Error::Api {
            status: 429,
            message: "Rate limited".into(),
        })
        .build();

    let result = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await;

    assert!(matches!(result, Err(Error::Api { status: 429, .. })));
}

#[tokio::test]
async fn test_mock_client_empty() {
    let client = MockClient::empty();

    let products = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();

    assert!(products.is_empty());
}

#[tokio::test]
async fn test_mock_client_json() {
    let json = r#"{
        "products": [
            {
                "vendor": "gcp",
                "service": "Compute Engine",
                "region": "us-central1",
                "sku": "pd-ssd",
                "prices": [{"usd": "0.170", "unit": "GB-month"}],
                "attributes": {"description": "SSD backed PD Capacity"}
            },
            {
                "vendor": "gcp",
                "service": "Cloud Storage",
                "region": "us-central1",
                "sku": "storage-standard",
                "prices": [{"usd": "0.020", "unit": "GB-month"}],
                "attributes": {"storageClass": "standard"}
            }
        ]
    }"#;

    let client = MockClient::from_json(json).unwrap();

    let products = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();
    assert_eq!(products.len(), 2);

    let storage = client
        .query_products(
            ProductFilter::builder()
                .vendor("gcp")
                .service("Cloud Storage")
                .build(),
        )
        .await
        .unwrap();
    assert_eq!(storage.len(), 1);
    assert_eq!(storage[0].attribute("storageClass"), Some("standard"));
}

#[tokio::test]
async fn test_mock_client_attributes() {
    let client = MockClient::builder()
        .with_product(
            MockProduct::new("gcp", "Compute Engine", "us-central1")
                .sku("pd-ssd")
                .price(0.170, "GB-month")
                .attribute("description", "SSD backed PD Capacity")
                .attribute("storageType", "pd-ssd"),
        )
        .build();

    let products = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();

    let product = &products[0];
    assert_eq!(
        product.attribute("description"),
        Some("SSD backed PD Capacity")
    );
    assert_eq!(product.attribute("storageType"), Some("pd-ssd"));
    assert_eq!(product.attribute("nonexistent"), None);
}

#[tokio::test]
async fn test_product_filter_builder() {
    let filter = ProductFilter::builder()
        .vendor("gcp")
        .service("Compute Engine")
        .region("us-central1")
        .attribute("description", "SSD")
        .attribute_regex("storageType", "pd-.*")
        .build();

    assert_eq!(filter.vendor_name.as_deref(), Some("gcp"));
    assert_eq!(filter.service.as_deref(), Some("Compute Engine"));
    assert_eq!(filter.region.as_deref(), Some("us-central1"));
    assert_eq!(filter.attribute_filters.len(), 2);
}

#[tokio::test]
async fn test_price_filter() {
    let client = MockClient::builder()
        .with_product(
            MockProduct::new("gcp", "Compute Engine", "us-central1")
                .sku("n1-standard-1")
                .price_full(
                    0.0475,
                    "Hrs",
                    Some("On-demand".into()),
                    Some("on_demand".into()),
                )
                .price_full(
                    0.0142,
                    "Hrs",
                    Some("Preemptible".into()),
                    Some("preemptible".into()),
                ),
        )
        .build();

    let products = client
        .query_products(ProductFilter::builder().vendor("gcp").build())
        .await
        .unwrap();

    let product = &products[0];

    // Get all prices
    assert_eq!(product.prices.len(), 2);

    // Filter by purchase option
    let on_demand = product
        .prices()
        .purchase_option("on_demand")
        .first()
        .unwrap();
    assert!((on_demand.usd_f64().unwrap() - 0.0475).abs() < 0.001);

    let preemptible = product
        .prices()
        .purchase_option("preemptible")
        .first()
        .unwrap();
    assert!((preemptible.usd_f64().unwrap() - 0.0142).abs() < 0.001);
}
