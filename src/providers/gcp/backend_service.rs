//! GCP Backend Service (Load Balancer) pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//! Backend services can be either Premium (global) or Standard (regional) tier.
//!
//! # Per-unit pricing (per GiB data processing rate)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::BackendServiceTier;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let price = client.gcp().backend_service(BackendServiceTier::Premium).fetch().await?;
//! println!("${}/GiB", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with data processing
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::BackendServiceTier;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let cost = client.gcp().backend_service(BackendServiceTier::Premium)
//!     .data_processed_gb(1000)  // 1000 GB of data processed per month
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Full LB cost including forwarding rule
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::BackendServiceTier;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let cost = client.gcp().backend_service(BackendServiceTier::Premium)
//!     .forwarding_rules(1)
//!     .data_processed_gb(1000)
//!     .fetch_monthly().await?;
//! // Cost = ($0.025 * 730) + ($0.008 * 1000) = $26.25/month
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::catalog::{engine::PricingEngine, gcp_catalog};
use crate::{Client, Result};

use super::super::{PriceResult, PriceSource};

// ============================================================
// Types
// ============================================================

/// The network tier for GCP Backend Service pricing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendServiceTier {
    /// Premium tier (global external application load balancer).
    /// Default: $0.008/GiB inbound data processing.
    Premium,
    /// Standard tier (regional external application load balancer).
    /// Default: $0.008/GiB inbound data processing.
    Standard,
}

impl BackendServiceTier {
    fn catalog_name(&self) -> &'static str {
        match self {
            Self::Premium => "backend-service/premium",
            Self::Standard => "backend-service/standard",
        }
    }
}

impl From<&str> for BackendServiceTier {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "standard" => Self::Standard,
            _ => Self::Premium,
        }
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP Backend Service prices.
pub struct BackendServiceBuilder {
    client: Client,
    tier: BackendServiceTier,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    data_processed_gb: Option<u64>,
    forwarding_rules: Option<u64>,
}

impl BackendServiceBuilder {
    /// Create a new Backend Service builder.
    pub(crate) fn new(client: Client, tier: BackendServiceTier) -> Self {
        Self {
            client,
            tier,
            region: None,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
            forwarding_rules: None,
        }
    }

    /// Set the GCP region (e.g., "us-central1").
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the API key for this request.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the default fallback price.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set the amount of data processed in GB per month (required for `fetch_monthly`).
    pub fn data_processed_gb(mut self, gb: u64) -> Self {
        self.data_processed_gb = Some(gb);
        self
    }

    /// Include forwarding rule hourly charges in `fetch_monthly`.
    ///
    /// GCP load balancers require at least one forwarding rule ($0.025/hour).
    /// This adds the forwarding rule cost to the monthly total.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # use infracost_rs::providers::gcp::BackendServiceTier;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key")?;
    /// let cost = client.gcp().backend_service(BackendServiceTier::Premium)
    ///     .forwarding_rules(1)
    ///     .data_processed_gb(1000)
    ///     .fetch_monthly().await?;
    /// // Cost = ($0.025 * 730) + ($0.008 * 1000) = $26.25/month
    /// # Ok(())
    /// # }
    /// ```
    pub fn forwarding_rules(mut self, count: u64) -> Self {
        self.forwarding_rules = Some(count);
        self
    }

    /// Fetch just the price value (per-GiB data processing rate).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    /// Returns the per-GiB data processing rate.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find(self.tier.catalog_name())?;
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

    /// Fetch total monthly cost based on data processing and forwarding rules.
    ///
    /// Calculates: (forwarding_rule_hourly * 730 * count) + (data_rate * gb_processed)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # use infracost_rs::providers::gcp::BackendServiceTier;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key")?;
    /// let cost = client.gcp().backend_service(BackendServiceTier::Premium)
    ///     .region("us-central1")
    ///     .forwarding_rules(1)
    ///     .data_processed_gb(1000)
    ///     .fetch_monthly().await?;
    /// // Cost = ($0.025 * 730) + ($0.008 * 1000) = $26.25/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find(self.tier.catalog_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let data_gb = self.data_processed_gb.unwrap_or(0);
        let mut params = HashMap::new();
        params.insert("data_processed_gb".to_string(), data_gb);

        let mut result = PricingEngine::fetch_monthly(
            &self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            &params,
        )
        .await?;

        // Add forwarding rule cost if requested
        if let Some(count) = self.forwarding_rules
            && count > 0
        {
            let fr_resource = gcp_catalog().find("forwarding-rule")?;
            let fr_result = PricingEngine::fetch(
                &self.client,
                fr_resource,
                "gcp",
                region,
                self.api_key.as_deref(),
                None,
            )
            .await?;
            result.price += fr_result.price * super::super::HOURS_PER_MONTH * count as f64;
            if !fr_result.is_from_api() && result.source == PriceSource::Api {
                result.source = PriceSource::Default;
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backend_service_premium_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.008);
        assert_eq!(result.unit, "GiB");
    }

    #[tokio::test]
    async fn test_backend_service_standard_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Standard)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.008);
        assert_eq!(result.unit, "GiB");
    }

    #[tokio::test]
    async fn test_backend_service_premium_fetch_monthly() {
        // Premium backend service with 1000 GB of data processed
        // Cost = $0.008 * 1000 = $8.00/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 8.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_backend_service_standard_fetch_monthly() {
        // Standard backend service with 1000 GB of data processed
        // Cost = $0.008 * 1000 = $8.00/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Standard)
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 8.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_backend_service_fetch_monthly_no_data_defaults_to_zero() {
        // Backend service without data_processed_gb defaults to 0
        // Cost = $0.008 * 0 = $0.00/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_backend_service_fetch_monthly_zero_data() {
        // Explicitly setting 0 GB
        // Cost = $0.008 * 0 = $0.00/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .data_processed_gb(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_backend_service_with_forwarding_rule() {
        // 1 forwarding rule, no data
        // Cost = ($0.025 * 730) + ($0.008 * 0) = $18.25/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .forwarding_rules(1)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 18.25);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_backend_service_with_forwarding_rule_and_data() {
        // 1 forwarding rule + 1000 GB data
        // Cost = ($0.025 * 730) + ($0.008 * 1000) = $18.25 + $8.00 = $26.25/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .forwarding_rules(1)
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 26.25);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_backend_service_with_multiple_forwarding_rules() {
        // 3 forwarding rules + 500 GB data
        // Cost = ($0.025 * 730 * 3) + ($0.008 * 500) = $54.75 + $4.00 = $58.75/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .forwarding_rules(3)
            .data_processed_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 58.75);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_backend_service_tier_from_str() {
        assert_eq!(
            BackendServiceTier::from("premium"),
            BackendServiceTier::Premium
        );
        assert_eq!(
            BackendServiceTier::from("PREMIUM"),
            BackendServiceTier::Premium
        );
        assert_eq!(
            BackendServiceTier::from("standard"),
            BackendServiceTier::Standard
        );
        assert_eq!(
            BackendServiceTier::from("STANDARD"),
            BackendServiceTier::Standard
        );
        // Unknown defaults to Premium
        assert_eq!(
            BackendServiceTier::from("unknown"),
            BackendServiceTier::Premium
        );
    }

    #[tokio::test]
    async fn test_backend_service_override_default() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .override_default(0.015)
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.015);
    }
}
