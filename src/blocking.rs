//! Blocking API wrapper. Requires the `blocking` feature.
//!
//! ```no_run
//! use infracost_rs::blocking::Client;
//!
//! fn main() -> Result<(), infracost_rs::Error> {
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

/// Generate a blocking builder struct that wraps an async builder.
///
/// Common methods (region, api_key, override_default, fetch, fetch_price, fetch_monthly)
/// are always generated. Additional setter methods can be specified.
macro_rules! blocking_builder {
    (
        $(#[$meta:meta])*
        pub struct $name:ident wraps $inner:ty {
            $(fn $setter:ident($ptype:ty);)*
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            inner: $inner,
            runtime: std::sync::Arc<tokio::runtime::Runtime>,
        }

        impl $name {
            /// Set the region.
            pub fn region(mut self, region: impl Into<String>) -> Self {
                self.inner = self.inner.region(region);
                self
            }

            /// Set the API key for this request.
            pub fn api_key(mut self, key: impl Into<String>) -> Self {
                self.inner = self.inner.api_key(key);
                self
            }

            /// Override the default fallback price.
            pub fn override_default(mut self, price: f64) -> Self {
                self.inner = self.inner.override_default(price);
                self
            }

            $(
                pub fn $setter(mut self, value: $ptype) -> Self {
                    self.inner = self.inner.$setter(value);
                    self
                }
            )*

            /// Fetch the full price result.
            pub fn fetch(self) -> crate::error::Result<crate::providers::PriceResult> {
                self.runtime.block_on(self.inner.fetch())
            }

            /// Fetch just the price value.
            pub fn fetch_price(self) -> crate::error::Result<f64> {
                self.fetch().map(|r| r.price)
            }

            /// Fetch the total monthly cost.
            pub fn fetch_monthly(self) -> crate::error::Result<crate::providers::PriceResult> {
                self.runtime.block_on(self.inner.fetch_monthly())
            }
        }
    };
}

mod aws;
mod azure;
mod gcp;

pub use aws::BlockingAwsProvider;
pub use azure::BlockingAzureProvider;
pub use gcp::BlockingGcpProvider;

use crate::cache::PriceCache;
use crate::error::Result;
use crate::types::{Product, ProductFilter};
use std::sync::Arc;
use std::time::Duration;

/// Shared tokio runtime for all blocking clients.
///
/// Using a single runtime avoids the overhead of creating one per client instance.
/// The `expect` here is acceptable: runtime creation failure is an unrecoverable
/// OS-level error (e.g., out of file descriptors).
static BLOCKING_RUNTIME: std::sync::LazyLock<Arc<tokio::runtime::Runtime>> =
    std::sync::LazyLock::new(|| {
        Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create shared blocking runtime"),
        )
    });

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
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Ok(Self {
            inner: crate::Client::new(api_key)?,
            runtime: BLOCKING_RUNTIME.clone(),
        })
    }

    /// Create a new blocking client from the `INFRACOST_API_KEY` environment variable.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            inner: crate::Client::from_env()?,
            runtime: BLOCKING_RUNTIME.clone(),
        })
    }

    /// Create an anonymous blocking client without an API key.
    pub fn anonymous() -> Result<Self> {
        Ok(Self {
            inner: crate::Client::anonymous()?,
            runtime: BLOCKING_RUNTIME.clone(),
        })
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

    /// Access GCP resource pricing with built-in defaults.
    pub fn gcp(&self) -> BlockingGcpProvider {
        BlockingGcpProvider {
            client: self.inner.clone(),
            runtime: self.runtime.clone(),
        }
    }

    /// Access AWS resource pricing with built-in defaults.
    pub fn aws(&self) -> BlockingAwsProvider {
        BlockingAwsProvider {
            client: self.inner.clone(),
            runtime: self.runtime.clone(),
        }
    }

    /// Access Azure resource pricing with built-in defaults.
    pub fn azure(&self) -> BlockingAzureProvider {
        BlockingAzureProvider {
            client: self.inner.clone(),
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

    /// Error instead of returning default prices when API is unavailable.
    pub fn error_on_fallback(mut self, enabled: bool) -> Self {
        self.inner = self.inner.error_on_fallback(enabled);
        self
    }

    /// Enable caching with a custom cache implementation.
    pub fn with_cache<C: PriceCache + 'static>(mut self, cache: C) -> Self {
        self.inner = self.inner.with_cache(cache);
        self
    }

    /// Set cache TTL (default: 24 hours).
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.inner = self.inner.cache_ttl(ttl);
        self
    }

    /// Build the blocking client.
    pub fn build(self) -> Result<Client> {
        Ok(Client {
            inner: self.inner.build()?,
            runtime: BLOCKING_RUNTIME.clone(),
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
        let client = Client::new("test-key").unwrap();
        assert!(client.inner.has_api_key());
    }

    #[test]
    fn test_blocking_client_anonymous() {
        let client = Client::anonymous().unwrap();
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
