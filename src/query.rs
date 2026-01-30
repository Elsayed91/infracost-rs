//! Fluent query builder for product queries.

use crate::client::Client;
use crate::error::Result;
use crate::types::{AttributeFilter, Product, ProductFilter};

/// Builder for product queries. Get one via [`Client::products`].
///
/// ```no_run
/// # use infracost_rs::Client;
/// # async fn example() -> Result<(), infracost_rs::Error> {
/// let client = Client::from_env()?;
/// let products = client
///     .products()
///     .vendor("gcp")
///     .service("Compute Engine")
///     .region("us-central1")
///     .attribute("description", "SSD backed PD Capacity")
///     .fetch()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ProductQueryBuilder {
    client: Client,
    filter: ProductFilter,
    api_key_override: Option<String>,
}

impl ProductQueryBuilder {
    /// Create a new query builder.
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            filter: ProductFilter::default(),
            api_key_override: None,
        }
    }

    /// Set the vendor name (e.g., "gcp", "aws", "azure").
    ///
    /// This is required for useful results.
    pub fn vendor(mut self, vendor: impl Into<String>) -> Self {
        self.filter.vendor_name = Some(vendor.into());
        self
    }

    /// Set the service name (e.g., "Compute Engine", "AmazonEC2").
    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.filter.service = Some(service.into());
        self
    }

    /// Set the product family (e.g., "Storage", "Compute").
    pub fn product_family(mut self, product_family: impl Into<String>) -> Self {
        self.filter.product_family = Some(product_family.into());
        self
    }

    /// Set the region (e.g., "us-central1", "us-east-1").
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.filter.region = Some(region.into());
        self
    }

    /// Set an exact SKU to filter by.
    pub fn sku(mut self, sku: impl Into<String>) -> Self {
        self.filter.sku = Some(sku.into());
        self
    }

    /// Add an exact attribute match filter.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filter
            .attribute_filters
            .push(AttributeFilter::exact(key, value));
        self
    }

    /// Add a regex attribute match filter (evaluated server-side).
    pub fn attribute_regex(mut self, key: impl Into<String>, regex: impl Into<String>) -> Self {
        self.filter
            .attribute_filters
            .push(AttributeFilter::regex(key, regex));
        self
    }

    /// Override the API key for this request only.
    ///
    /// Useful when using a shared client with per-request authentication.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key_override = Some(api_key.into());
        self
    }

    /// Set a raw ProductFilter, replacing any previously set filters.
    pub fn filter(mut self, filter: ProductFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Get the current filter being built.
    pub fn get_filter(&self) -> &ProductFilter {
        &self.filter
    }

    /// Execute the query and return matching products.
    pub async fn fetch(self) -> Result<Vec<Product>> {
        self.client
            .execute_query(self.filter, self.api_key_override.as_deref())
            .await
    }

    /// Execute the query and return the first matching product.
    pub async fn fetch_one(self) -> Result<Option<Product>> {
        let products = self.fetch().await?;
        Ok(products.into_iter().next())
    }

    /// Execute the query and return the first matching product, or an error if none found.
    pub async fn fetch_one_required(self) -> Result<Product> {
        self.fetch_one()
            .await?
            .ok_or(crate::error::Error::NoProducts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_filter_construction() {
        let client = Client::anonymous();
        let builder = client
            .products()
            .vendor("gcp")
            .service("Compute Engine")
            .region("us-central1")
            .attribute("key1", "value1")
            .attribute_regex("key2", ".*pattern.*");

        let filter = builder.get_filter();
        assert_eq!(filter.vendor_name.as_deref(), Some("gcp"));
        assert_eq!(filter.service.as_deref(), Some("Compute Engine"));
        assert_eq!(filter.region.as_deref(), Some("us-central1"));
        assert_eq!(filter.attribute_filters.len(), 2);
        assert_eq!(filter.attribute_filters[0].key, "key1");
        assert_eq!(filter.attribute_filters[0].value.as_deref(), Some("value1"));
        assert_eq!(filter.attribute_filters[1].key, "key2");
        assert_eq!(
            filter.attribute_filters[1].value_regex.as_deref(),
            Some(".*pattern.*")
        );
    }

    #[test]
    fn test_query_builder_api_key_override() {
        let client = Client::anonymous();
        let builder = client.products().api_key("override-key");
        assert_eq!(builder.api_key_override.as_deref(), Some("override-key"));
    }
}
