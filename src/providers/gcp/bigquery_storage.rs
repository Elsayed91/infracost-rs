//! GCP BigQuery Storage pricing.
//!
//! BigQuery offers two storage billing models:
//! - **Logical billing** (default): Active and long-term logical storage
//! - **Physical billing**: Active and long-term physical storage
//!
//! Each dataset uses one billing model. This builder supports all four
//! storage components independently.
//!
//! # Per-unit pricing (active logical storage)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.gcp().bigquery_storage().fetch().await?;
//! println!("${}/GiB-month", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with logical billing
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.gcp().bigquery_storage()
//!     .active_logical_storage_gb(500)
//!     .long_term_logical_storage_gb(200)
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::catalog::{engine::PricingEngine, gcp_catalog};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP BigQuery Storage prices.
pub struct BigQueryStorageBuilder {
    client: Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    active_logical_storage_gb: Option<u64>,
    long_term_logical_storage_gb: Option<u64>,
    active_physical_storage_gb: Option<u64>,
    long_term_physical_storage_gb: Option<u64>,
}

impl BigQueryStorageBuilder {
    /// Create a new BigQuery Storage builder.
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            active_logical_storage_gb: None,
            long_term_logical_storage_gb: None,
            active_physical_storage_gb: None,
            long_term_physical_storage_gb: None,
        }
    }

    /// Set the GCP region (e.g., "us-central1").
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the API key for this request.
    ///
    /// If not set and the client has no default key, returns built-in defaults.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the default fallback price.
    ///
    /// By default, the library uses built-in prices when the API is unavailable.
    /// Use this to specify a custom fallback.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set active logical storage in GiB.
    ///
    /// Default price: $0.023/GiB-month (us-central1).
    pub fn active_logical_storage_gb(mut self, gb: u64) -> Self {
        self.active_logical_storage_gb = Some(gb);
        self
    }

    /// Set long-term logical storage in GiB.
    ///
    /// Default price: $0.016/GiB-month (us-central1).
    pub fn long_term_logical_storage_gb(mut self, gb: u64) -> Self {
        self.long_term_logical_storage_gb = Some(gb);
        self
    }

    /// Set active physical storage in GiB.
    ///
    /// Default price: $0.04/GiB-month (us-central1).
    pub fn active_physical_storage_gb(mut self, gb: u64) -> Self {
        self.active_physical_storage_gb = Some(gb);
        self
    }

    /// Set long-term physical storage in GiB.
    ///
    /// Default price: $0.02/GiB-month (us-central1).
    pub fn long_term_physical_storage_gb(mut self, gb: u64) -> Self {
        self.long_term_physical_storage_gb = Some(gb);
        self
    }

    /// Fetch just the price value (primary component: active logical storage).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result for the primary component (active logical storage).
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("bigquery-storage")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            &self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch total monthly cost based on storage usage.
    ///
    /// All parameters are optional and default to 0 if not set.
    /// In practice, a dataset uses either logical or physical billing,
    /// but this builder allows any combination.
    ///
    /// # Examples
    ///
    /// Logical billing:
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.gcp().bigquery_storage()
    ///     .active_logical_storage_gb(500)
    ///     .long_term_logical_storage_gb(200)
    ///     .fetch_monthly().await?;
    /// // Cost = (500 * $0.023) + (200 * $0.016) = $11.50 + $3.20 = $14.70/month
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Physical billing:
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.gcp().bigquery_storage()
    ///     .active_physical_storage_gb(100)
    ///     .long_term_physical_storage_gb(50)
    ///     .fetch_monthly().await?;
    /// // Cost = (100 * $0.04) + (50 * $0.02) = $4.00 + $1.00 = $5.00/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("bigquery-storage")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);

        let mut params = HashMap::new();
        params.insert(
            "active_logical_storage_gb".to_string(),
            self.active_logical_storage_gb.unwrap_or(0),
        );
        params.insert(
            "long_term_logical_storage_gb".to_string(),
            self.long_term_logical_storage_gb.unwrap_or(0),
        );
        params.insert(
            "active_physical_storage_gb".to_string(),
            self.active_physical_storage_gb.unwrap_or(0),
        );
        params.insert(
            "long_term_physical_storage_gb".to_string(),
            self.long_term_physical_storage_gb.unwrap_or(0),
        );

        PricingEngine::fetch_monthly(
            &self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            &params,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_bigquery_storage_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .bigquery_storage()
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.023);
        assert_eq!(result.unit, "gibibyte month");
    }

    #[tokio::test]
    async fn test_bigquery_storage_fetch_monthly_logical_only() {
        // Active logical: 500 GiB * $0.023 = $11.50
        // Long-term logical: 200 GiB * $0.016 = $3.20
        // Total: $14.70/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .bigquery_storage()
            .region("us-central1")
            .active_logical_storage_gb(500)
            .long_term_logical_storage_gb(200)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 14.7);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_bigquery_storage_fetch_monthly_physical_only() {
        // Active physical: 100 GiB * $0.04 = $4.00
        // Long-term physical: 50 GiB * $0.02 = $1.00
        // Total: $5.00/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .bigquery_storage()
            .region("us-central1")
            .active_physical_storage_gb(100)
            .long_term_physical_storage_gb(50)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 5.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_bigquery_storage_fetch_monthly_active_logical_only() {
        // Active logical only: 1000 GiB * $0.023 = $23.00
        let client = Client::anonymous();
        let result = client
            .gcp()
            .bigquery_storage()
            .active_logical_storage_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 23.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_bigquery_storage_fetch_monthly_no_params_is_zero() {
        // No storage specified = $0/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .bigquery_storage()
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 0.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_bigquery_storage_override_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .bigquery_storage()
            .override_default(0.05)
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
    }

    #[tokio::test]
    async fn test_bigquery_storage_fetch_monthly_all_components() {
        // All four components:
        // Active logical: 100 GiB * $0.023 = $2.30
        // Long-term logical: 200 GiB * $0.016 = $3.20
        // Active physical: 50 GiB * $0.04 = $2.00
        // Long-term physical: 25 GiB * $0.02 = $0.50
        // Total: $8.00/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .bigquery_storage()
            .active_logical_storage_gb(100)
            .long_term_logical_storage_gb(200)
            .active_physical_storage_gb(50)
            .long_term_physical_storage_gb(25)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 8.0);
        assert_eq!(result.unit, "month");
    }
}
