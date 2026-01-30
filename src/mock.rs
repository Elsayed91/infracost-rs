//! Mock client for testing.
//!
//! ```
//! use infracost::mock::{MockClient, MockProduct};
//! use infracost::{PricingClient, ProductFilter};
//!
//! # async fn example() -> Result<(), infracost::Error> {
//! // Builder pattern
//! let client = MockClient::builder()
//!     .with_product(
//!         MockProduct::new("gcp", "Compute Engine", "us-central1")
//!             .sku("pd-ssd")
//!             .price(0.170, "GB-month")
//!             .attribute("description", "SSD backed PD Capacity")
//!     )
//!     .build();
//!
//! // Or from a price table
//! let client = MockClient::from_prices(&[
//!     ("gcp", "Compute Engine", "us-central1", "pd-ssd", 0.170, "GB-month"),
//!     ("aws", "AmazonEC2", "us-east-1", "t3.micro", 0.0104, "Hrs"),
//! ]);
//!
//! let products = client
//!     .query_products(ProductFilter::builder().vendor("gcp").build())
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::client::PricingClient;
use crate::error::{Error, Result};
use crate::types::{Attribute, Price, Product, ProductFilter};
use async_trait::async_trait;
use std::sync::Arc;

/// Type alias for the mock callback function
type MockCallback = Box<dyn Fn(&ProductFilter) -> Vec<Product> + Send + Sync>;

/// Mock client for testing.
#[derive(Clone)]
pub struct MockClient {
    inner: Arc<MockClientInner>,
}

struct MockClientInner {
    products: Vec<Product>,
    error: Option<Error>,
    callback: Option<MockCallback>,
}

impl MockClient {
    /// Create a new mock client builder.
    pub fn builder() -> MockClientBuilder {
        MockClientBuilder::default()
    }

    /// Create an empty mock client (returns no products).
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(MockClientInner {
                products: vec![],
                error: None,
                callback: None,
            }),
        }
    }

    /// Create a mock client from a simple price table.
    ///
    /// # Arguments
    ///
    /// Each tuple contains: (vendor, service, region, sku, price, unit)
    ///
    /// # Examples
    ///
    /// ```
    /// use infracost::mock::MockClient;
    ///
    /// let client = MockClient::from_prices(&[
    ///     ("gcp", "Compute Engine", "us-central1", "pd-ssd", 0.170, "GB-month"),
    ///     ("gcp", "Compute Engine", "us-east1", "pd-ssd", 0.170, "GB-month"),
    ///     ("aws", "AmazonEC2", "us-east-1", "t3.micro", 0.0104, "Hrs"),
    /// ]);
    /// ```
    pub fn from_prices(prices: &[(&str, &str, &str, &str, f64, &str)]) -> Self {
        let products = prices
            .iter()
            .enumerate()
            .map(|(i, (vendor, service, region, sku, price, unit))| Product {
                product_hash: format!("mock-{}", i),
                vendor_name: vendor.to_string(),
                service: service.to_string(),
                product_family: None,
                region: Some(region.to_string()),
                sku: sku.to_string(),
                attributes: vec![],
                prices: vec![Price {
                    usd: price.to_string(),
                    unit: unit.to_string(),
                    description: None,
                    purchase_option: None,
                    start_usage_amount: None,
                    end_usage_amount: None,
                }],
            })
            .collect();

        Self {
            inner: Arc::new(MockClientInner {
                products,
                error: None,
                callback: None,
            }),
        }
    }

    /// Create a mock client from JSON data.
    ///
    /// # JSON Format
    ///
    /// ```json
    /// {
    ///   "products": [
    ///     {
    ///       "vendor": "gcp",
    ///       "service": "Compute Engine",
    ///       "region": "us-central1",
    ///       "sku": "pd-ssd",
    ///       "prices": [{ "usd": "0.170", "unit": "GB-month" }],
    ///       "attributes": { "description": "SSD backed PD Capacity" }
    ///     }
    ///   ]
    /// }
    /// ```
    pub fn from_json(json: &str) -> Result<Self> {
        let data: MockJsonData = serde_json::from_str(json)?;
        let products = data
            .products
            .into_iter()
            .enumerate()
            .map(|(i, p)| p.into_product(i))
            .collect();

        Ok(Self {
            inner: Arc::new(MockClientInner {
                products,
                error: None,
                callback: None,
            }),
        })
    }

    /// Create a mock client from a JSON file.
    pub fn from_json_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// Create a mock client with a callback function.
    ///
    /// The callback receives the filter and returns products dynamically.
    pub fn with_callback<F>(callback: F) -> Self
    where
        F: Fn(&ProductFilter) -> Vec<Product> + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(MockClientInner {
                products: vec![],
                error: None,
                callback: Some(Box::new(callback)),
            }),
        }
    }

    fn query(&self, filter: &ProductFilter) -> Result<Vec<Product>> {
        // Check for configured error first
        if let Some(ref err) = self.inner.error {
            return Err(match err {
                Error::Api { status, message } => Error::api(*status, message.clone()),
                Error::GraphQL(msg) => Error::graphql(msg.clone()),
                Error::MissingApiKey => Error::MissingApiKey,
                Error::NoProducts => Error::NoProducts,
                _ => Error::graphql("Mock error"),
            });
        }

        // Use callback if provided
        if let Some(ref callback) = self.inner.callback {
            return Ok(callback(filter));
        }

        // Filter products
        let products = self
            .inner
            .products
            .iter()
            .filter(|p| filter.matches(p))
            .cloned()
            .collect();

        Ok(products)
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::empty()
    }
}

#[async_trait]
impl PricingClient for MockClient {
    async fn query_products(&self, filter: ProductFilter) -> Result<Vec<Product>> {
        self.query(&filter)
    }

    async fn query_products_with_key(
        &self,
        filter: ProductFilter,
        _api_key: Option<&str>,
    ) -> Result<Vec<Product>> {
        self.query(&filter)
    }
}

/// Builder for MockClient.
#[derive(Default)]
pub struct MockClientBuilder {
    products: Vec<Product>,
    error: Option<Error>,
}

impl MockClientBuilder {
    /// Add a product to the mock client.
    pub fn with_product(mut self, product: MockProduct) -> Self {
        self.products.push(product.build());
        self
    }

    /// Add multiple products.
    pub fn with_products(mut self, products: impl IntoIterator<Item = MockProduct>) -> Self {
        for product in products {
            self.products.push(product.build());
        }
        self
    }

    /// Configure the mock to return an error.
    pub fn with_error(mut self, error: Error) -> Self {
        self.error = Some(error);
        self
    }

    /// Build the mock client.
    pub fn build(self) -> MockClient {
        MockClient {
            inner: Arc::new(MockClientInner {
                products: self.products,
                error: self.error,
                callback: None,
            }),
        }
    }
}

/// Builder for creating mock products.
pub struct MockProduct {
    vendor: String,
    service: String,
    region: Option<String>,
    sku: String,
    product_family: Option<String>,
    prices: Vec<Price>,
    attributes: Vec<Attribute>,
}

impl MockProduct {
    /// Create a new mock product.
    pub fn new(
        vendor: impl Into<String>,
        service: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            vendor: vendor.into(),
            service: service.into(),
            region: Some(region.into()),
            sku: "mock-sku".to_string(),
            product_family: None,
            prices: vec![],
            attributes: vec![],
        }
    }

    /// Set the SKU.
    pub fn sku(mut self, sku: impl Into<String>) -> Self {
        self.sku = sku.into();
        self
    }

    /// Set the product family.
    pub fn product_family(mut self, family: impl Into<String>) -> Self {
        self.product_family = Some(family.into());
        self
    }

    /// Add a price.
    pub fn price(mut self, usd: f64, unit: impl Into<String>) -> Self {
        self.prices.push(Price {
            usd: usd.to_string(),
            unit: unit.into(),
            description: None,
            purchase_option: None,
            start_usage_amount: None,
            end_usage_amount: None,
        });
        self
    }

    /// Add a price with full details.
    pub fn price_full(
        mut self,
        usd: f64,
        unit: impl Into<String>,
        description: Option<String>,
        purchase_option: Option<String>,
    ) -> Self {
        self.prices.push(Price {
            usd: usd.to_string(),
            unit: unit.into(),
            description,
            purchase_option,
            start_usage_amount: None,
            end_usage_amount: None,
        });
        self
    }

    /// Add an attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push(Attribute {
            key: key.into(),
            value: Some(value.into()),
        });
        self
    }

    fn build(self) -> Product {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Product {
            product_hash: format!("mock-{}", id),
            vendor_name: self.vendor,
            service: self.service,
            product_family: self.product_family,
            region: self.region,
            sku: self.sku,
            attributes: self.attributes,
            prices: self.prices,
        }
    }
}


// JSON deserialization types
#[derive(serde::Deserialize)]
struct MockJsonData {
    products: Vec<MockJsonProduct>,
}

#[derive(serde::Deserialize)]
struct MockJsonProduct {
    vendor: String,
    service: String,
    region: Option<String>,
    sku: String,
    #[serde(default)]
    product_family: Option<String>,
    #[serde(default)]
    prices: Vec<MockJsonPrice>,
    #[serde(default)]
    attributes: std::collections::HashMap<String, String>,
}

#[derive(serde::Deserialize)]
struct MockJsonPrice {
    usd: String,
    unit: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    purchase_option: Option<String>,
}

impl MockJsonProduct {
    fn into_product(self, id: usize) -> Product {
        Product {
            product_hash: format!("mock-json-{}", id),
            vendor_name: self.vendor,
            service: self.service,
            product_family: self.product_family,
            region: self.region,
            sku: self.sku,
            attributes: self
                .attributes
                .into_iter()
                .map(|(k, v)| Attribute {
                    key: k,
                    value: Some(v),
                })
                .collect(),
            prices: self
                .prices
                .into_iter()
                .map(|p| Price {
                    usd: p.usd,
                    unit: p.unit,
                    description: p.description,
                    purchase_option: p.purchase_option,
                    start_usage_amount: None,
                    end_usage_amount: None,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_empty() {
        let client = MockClient::empty();
        let products = client.query_products(ProductFilter::default()).await.unwrap();
        assert!(products.is_empty());
    }

    #[tokio::test]
    async fn test_mock_client_with_products() {
        let client = MockClient::builder()
            .with_product(
                MockProduct::new("gcp", "Compute Engine", "us-central1")
                    .sku("pd-ssd")
                    .price(0.170, "GB-month"),
            )
            .build();

        let products = client
            .query_products(
                ProductFilter::builder()
                    .vendor("gcp")
                    .build(),
            )
            .await
            .unwrap();

        assert_eq!(products.len(), 1);
        assert_eq!(products[0].vendor_name, "gcp");
        assert_eq!(products[0].sku, "pd-ssd");
    }

    #[tokio::test]
    async fn test_mock_client_from_prices() {
        let client = MockClient::from_prices(&[
            ("gcp", "Compute Engine", "us-central1", "pd-ssd", 0.170, "GB-month"),
            ("gcp", "Compute Engine", "us-east1", "pd-ssd", 0.170, "GB-month"),
            ("aws", "AmazonEC2", "us-east-1", "t3.micro", 0.0104, "Hrs"),
        ]);

        let gcp_products = client
            .query_products(ProductFilter::builder().vendor("gcp").build())
            .await
            .unwrap();
        assert_eq!(gcp_products.len(), 2);

        let aws_products = client
            .query_products(ProductFilter::builder().vendor("aws").build())
            .await
            .unwrap();
        assert_eq!(aws_products.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_client_from_json() {
        let json = r#"{
            "products": [
                {
                    "vendor": "gcp",
                    "service": "Compute Engine",
                    "region": "us-central1",
                    "sku": "pd-ssd",
                    "prices": [{"usd": "0.170", "unit": "GB-month"}],
                    "attributes": {"description": "SSD backed PD Capacity"}
                }
            ]
        }"#;

        let client = MockClient::from_json(json).unwrap();
        let products = client
            .query_products(ProductFilter::builder().vendor("gcp").build())
            .await
            .unwrap();

        assert_eq!(products.len(), 1);
        assert_eq!(products[0].attribute("description"), Some("SSD backed PD Capacity"));
    }

    #[tokio::test]
    async fn test_mock_client_with_callback() {
        let client = MockClient::with_callback(|filter| {
            match filter.region.as_deref() {
                Some("us-central1") => vec![Product {
                    product_hash: "cb-1".to_string(),
                    vendor_name: "gcp".to_string(),
                    service: "Compute Engine".to_string(),
                    product_family: None,
                    region: Some("us-central1".to_string()),
                    sku: "pd-ssd".to_string(),
                    attributes: vec![],
                    prices: vec![Price {
                        usd: "0.170".to_string(),
                        unit: "GB-month".to_string(),
                        description: None,
                        purchase_option: None,
                        start_usage_amount: None,
                        end_usage_amount: None,
                    }],
                }],
                _ => vec![],
            }
        });

        let products = client
            .query_products(ProductFilter::builder().region("us-central1").build())
            .await
            .unwrap();
        assert_eq!(products.len(), 1);

        let products = client
            .query_products(ProductFilter::builder().region("europe-west1").build())
            .await
            .unwrap();
        assert!(products.is_empty());
    }

    #[tokio::test]
    async fn test_mock_client_with_error() {
        let client = MockClient::builder()
            .with_error(Error::api(429, "Rate limited"))
            .build();

        let result = client.query_products(ProductFilter::default()).await;
        assert!(matches!(result, Err(Error::Api { status: 429, .. })));
    }
}
