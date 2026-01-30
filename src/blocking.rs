//! Blocking API wrapper. Requires the `blocking` feature.
//!
//! ```no_run
//! use infracost::blocking::Client;
//!
//! fn main() -> Result<(), infracost::Error> {
//!     let client = Client::from_env()?;
//!     let products = client
//!         .products()
//!         .vendor("gcp")
//!         .service("Compute Engine")
//!         .region("us-central1")
//!         .fetch()?;
//!
//!     for p in products {
//!         println!("{}: ${}", p.sku, p.prices[0].usd);
//!     }
//!     Ok(())
//! }
//! ```

use crate::error::{Error, Result};
use crate::types::{Product, ProductFilter};
use std::time::Duration;

/// Blocking client for the Infracost Cloud Pricing API.
///
/// This is a synchronous wrapper around the async [`crate::Client`].
#[derive(Clone)]
pub struct Client {
    inner: crate::Client,
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
}

impl Client {
    /// Create a new blocking client with an API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            inner: crate::Client::new(api_key),
            runtime: std::sync::Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime"),
            ),
        }
    }

    /// Create a new blocking client from the `INFRACOST_API_KEY` environment variable.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            inner: crate::Client::from_env()?,
            runtime: std::sync::Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| Error::config(format!("Failed to create runtime: {}", e)))?,
            ),
        })
    }

    /// Create an anonymous blocking client without an API key.
    pub fn anonymous() -> Self {
        Self {
            inner: crate::Client::anonymous(),
            runtime: std::sync::Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime"),
            ),
        }
    }

    /// Create a new blocking client builder.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Start building a product query.
    pub fn products(&self) -> BlockingProductQueryBuilder {
        BlockingProductQueryBuilder {
            inner: self.inner.products(),
            runtime: self.runtime.clone(),
        }
    }

    /// Execute a raw query with a filter.
    pub fn query_products(&self, filter: ProductFilter) -> Result<Vec<Product>> {
        self.runtime.block_on(self.inner.query_products(filter))
    }

    /// Execute a raw query with a filter and optional API key override.
    pub fn query_products_with_key(
        &self,
        filter: ProductFilter,
        api_key: Option<&str>,
    ) -> Result<Vec<Product>> {
        self.runtime
            .block_on(self.inner.query_products_with_key(filter, api_key))
    }
}

/// Builder for constructing a blocking Client.
#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    inner: crate::ClientBuilder,
}

impl ClientBuilder {
    /// Set the API key.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.inner = self.inner.api_key(api_key);
        self
    }

    /// Set a custom API endpoint.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.inner = self.inner.endpoint(endpoint);
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// Build the blocking client.
    pub fn build(self) -> Result<Client> {
        Ok(Client {
            inner: self.inner.build()?,
            runtime: std::sync::Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| Error::config(format!("Failed to create runtime: {}", e)))?,
            ),
        })
    }
}

/// Blocking query builder for product queries.
pub struct BlockingProductQueryBuilder {
    inner: crate::ProductQueryBuilder,
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
}

impl BlockingProductQueryBuilder {
    /// Set the vendor name.
    pub fn vendor(mut self, vendor: impl Into<String>) -> Self {
        self.inner = self.inner.vendor(vendor);
        self
    }

    /// Set the service name.
    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.inner = self.inner.service(service);
        self
    }

    /// Set the product family.
    pub fn product_family(mut self, product_family: impl Into<String>) -> Self {
        self.inner = self.inner.product_family(product_family);
        self
    }

    /// Set the region.
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.inner = self.inner.region(region);
        self
    }

    /// Set an exact SKU to filter by.
    pub fn sku(mut self, sku: impl Into<String>) -> Self {
        self.inner = self.inner.sku(sku);
        self
    }

    /// Add an exact attribute match filter.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.attribute(key, value);
        self
    }

    /// Add a regex attribute match filter.
    pub fn attribute_regex(mut self, key: impl Into<String>, regex: impl Into<String>) -> Self {
        self.inner = self.inner.attribute_regex(key, regex);
        self
    }

    /// Override the API key for this request.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.inner = self.inner.api_key(api_key);
        self
    }

    /// Set a raw ProductFilter.
    pub fn filter(mut self, filter: ProductFilter) -> Self {
        self.inner = self.inner.filter(filter);
        self
    }

    /// Execute the query and return matching products.
    pub fn fetch(self) -> Result<Vec<Product>> {
        self.runtime.block_on(self.inner.fetch())
    }

    /// Execute the query and return the first matching product.
    pub fn fetch_one(self) -> Result<Option<Product>> {
        self.runtime.block_on(self.inner.fetch_one())
    }

    /// Execute the query and return the first matching product, or an error if none found.
    pub fn fetch_one_required(self) -> Result<Product> {
        self.runtime.block_on(self.inner.fetch_one_required())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocking_client_new() {
        let client = Client::new("test-key");
        assert!(client.inner.has_api_key());
    }

    #[test]
    fn test_blocking_client_anonymous() {
        let client = Client::anonymous();
        assert!(!client.inner.has_api_key());
    }

    #[test]
    fn test_blocking_client_builder() {
        let client = Client::builder()
            .api_key("test-key")
            .endpoint("https://custom.endpoint/graphql")
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        assert!(client.inner.has_api_key());
    }
}
