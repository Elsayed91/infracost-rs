//! Core types: [`Product`], [`Price`], [`ProductFilter`].

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Product from Infracost API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// Unique hash identifying this product
    pub product_hash: String,
    /// Vendor name (e.g., "gcp", "aws", "azure")
    pub vendor_name: String,
    /// Service name (e.g., "Compute Engine", "AmazonEC2")
    pub service: String,
    /// Product family (e.g., "Storage", "Compute")
    pub product_family: Option<String>,
    /// Region (e.g., "us-central1", "us-east-1")
    pub region: Option<String>,
    /// Stock keeping unit identifier
    pub sku: String,
    /// Product attributes
    pub attributes: Vec<Attribute>,
    /// Pricing information
    pub prices: Vec<Price>,
}

impl Product {
    /// Get the first price
    pub fn price(&self) -> Result<&Price> {
        self.prices.first().ok_or_else(|| Error::no_prices(&self.sku))
    }

    /// Get the first price as f64
    pub fn price_f64(&self) -> Result<f64> {
        self.price()?.usd_f64()
    }

    /// Get a builder for filtering prices
    pub fn prices(&self) -> PriceFilter<'_> {
        PriceFilter::new(&self.prices)
    }

    /// Get an attribute value by key
    pub fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.key == key)
            .and_then(|a| a.value.as_deref())
    }
}

/// Price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    /// Price in USD as a string
    pub usd: String,
    /// Unit of measurement (e.g., "Hrs", "GB-month")
    pub unit: String,
    /// Description of the price
    pub description: Option<String>,
    /// Purchase option (e.g., "on_demand", "reserved")
    pub purchase_option: Option<String>,
    /// Start of usage tier
    pub start_usage_amount: Option<String>,
    /// End of usage tier
    pub end_usage_amount: Option<String>,
}

impl Price {
    /// Parse USD price as f64
    pub fn usd_f64(&self) -> Result<f64> {
        self.usd.parse::<f64>().map_err(|e| {
            Error::invalid_price(&self.usd, e.to_string())
        })
    }
}

/// Product attribute key-value pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute key
    pub key: String,
    /// Attribute value
    pub value: Option<String>,
}

/// Builder for filtering prices within a product
#[derive(Debug)]
pub struct PriceFilter<'a> {
    prices: &'a [Price],
    unit: Option<&'a str>,
    purchase_option: Option<&'a str>,
    description: Option<&'a str>,
}

impl<'a> PriceFilter<'a> {
    /// Create a new price filter
    fn new(prices: &'a [Price]) -> Self {
        Self {
            prices,
            unit: None,
            purchase_option: None,
            description: None,
        }
    }

    /// Filter by unit (e.g., "Hrs", "GB-month")
    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Filter by purchase option (e.g., "on_demand")
    pub fn purchase_option(mut self, purchase_option: &'a str) -> Self {
        self.purchase_option = Some(purchase_option);
        self
    }

    /// Filter by description
    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    /// Get the first matching price
    pub fn first(&self) -> Result<&'a Price> {
        self.iter()
            .next()
            .ok_or(Error::NoProducts)
    }

    /// Get the first matching price as f64
    pub fn first_f64(&self) -> Result<f64> {
        self.first()?.usd_f64()
    }

    /// Iterate over matching prices
    pub fn iter(&self) -> impl Iterator<Item = &'a Price> + '_ {
        self.prices.iter().filter(move |p| {
            let unit_match = self.unit.is_none_or(|u| p.unit == u);
            let purchase_match = self
                .purchase_option
                .is_none_or(|po| p.purchase_option.as_deref() == Some(po));
            let desc_match = self.description.is_none_or(|d| {
                p.description.as_deref().is_some_and(|pd| pd.contains(d))
            });
            unit_match && purchase_match && desc_match
        })
    }

    /// Collect all matching prices
    pub fn collect(&self) -> Vec<&'a Price> {
        self.iter().collect()
    }
}

/// Filter for product queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductFilter {
    /// Vendor name (e.g., "gcp", "aws", "azure")
    pub vendor_name: Option<String>,
    /// Service name
    pub service: Option<String>,
    /// Product family
    pub product_family: Option<String>,
    /// Region
    pub region: Option<String>,
    /// SKU
    pub sku: Option<String>,
    /// Attribute filters
    pub attribute_filters: Vec<AttributeFilter>,
}

impl ProductFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for constructing a filter
    pub fn builder() -> ProductFilterBuilder {
        ProductFilterBuilder::default()
    }

    /// Check if a product matches this filter
    pub fn matches(&self, product: &Product) -> bool {
        if let Some(ref v) = self.vendor_name
            && !product.vendor_name.eq_ignore_ascii_case(v)
        {
            return false;
        }
        if let Some(ref s) = self.service
            && !product.service.eq_ignore_ascii_case(s)
        {
            return false;
        }
        if let Some(ref pf) = self.product_family
            && product
                .product_family
                .as_ref()
                .is_none_or(|f| !f.eq_ignore_ascii_case(pf))
        {
            return false;
        }
        if let Some(ref r) = self.region
            && product
                .region
                .as_ref()
                .is_none_or(|pr| !pr.eq_ignore_ascii_case(r))
        {
            return false;
        }
        if let Some(ref s) = self.sku
            && !product.sku.eq_ignore_ascii_case(s)
        {
            return false;
        }
        for af in &self.attribute_filters {
            if !af.matches(product) {
                return false;
            }
        }
        true
    }
}

/// Builder for ProductFilter
#[derive(Debug, Clone, Default)]
pub struct ProductFilterBuilder {
    filter: ProductFilter,
}

impl ProductFilterBuilder {
    /// Set the vendor name
    pub fn vendor(mut self, vendor: impl Into<String>) -> Self {
        self.filter.vendor_name = Some(vendor.into());
        self
    }

    /// Set the service name
    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.filter.service = Some(service.into());
        self
    }

    /// Set the product family
    pub fn product_family(mut self, product_family: impl Into<String>) -> Self {
        self.filter.product_family = Some(product_family.into());
        self
    }

    /// Set the region
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.filter.region = Some(region.into());
        self
    }

    /// Set the SKU
    pub fn sku(mut self, sku: impl Into<String>) -> Self {
        self.filter.sku = Some(sku.into());
        self
    }

    /// Add an exact attribute filter
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filter.attribute_filters.push(AttributeFilter {
            key: key.into(),
            value: Some(value.into()),
            value_regex: None,
        });
        self
    }

    /// Add a regex attribute filter
    pub fn attribute_regex(mut self, key: impl Into<String>, regex: impl Into<String>) -> Self {
        self.filter.attribute_filters.push(AttributeFilter {
            key: key.into(),
            value: None,
            value_regex: Some(regex.into()),
        });
        self
    }

    /// Build the filter
    pub fn build(self) -> ProductFilter {
        self.filter
    }
}

/// Filter for attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeFilter {
    /// Attribute key to match
    pub key: String,
    /// Exact value to match (optional)
    pub value: Option<String>,
    /// Regex pattern to match (optional)
    pub value_regex: Option<String>,
}

impl AttributeFilter {
    /// Create a new exact match attribute filter
    pub fn exact(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: Some(value.into()),
            value_regex: None,
        }
    }

    /// Create a new regex match attribute filter
    pub fn regex(key: impl Into<String>, regex: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: None,
            value_regex: Some(regex.into()),
        }
    }

    /// Check if a product's attributes match this filter
    pub fn matches(&self, product: &Product) -> bool {
        let attr_value = product.attribute(&self.key);

        if let Some(ref exact) = self.value
            && attr_value.is_none_or(|v| v != exact)
        {
            return false;
        }

        // Note: regex matching is done server-side for real queries
        // For mock client, we do a simple contains check
        if let Some(ref _regex) = self.value_regex {
            // Server handles regex matching, we can't easily do it here without regex crate
            // For mock filtering, we'll just check if the value exists
            if attr_value.is_none() {
                return false;
            }
        }

        true
    }
}
