//! Azure Load Balancer Rules pricing.
//!
//! Load Balancer Rules use two-tier hourly pricing:
//! - First 5 rules: $0.025/rule/hr (tier1)
//! - Additional rules beyond 5: $0.01/rule/hr (tier2)
//! - Monthly = (min(rule_count, 5) * tier1_price + max(0, rule_count - 5) * tier2_price) * 730

use crate::catalog::{azure_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::{PriceResult, PriceSource};

// ============================================================
// Constants
// ============================================================

use super::super::HOURS_PER_MONTH;

/// Maximum rules in the first tier.
const TIER1_MAX: u64 = 5;

// ============================================================
// Builder
// ============================================================

/// Builder for querying Azure Load Balancer Rules prices.
///
/// Pricing is two-tier hourly:
/// - First 5 rules: $0.025/rule/hr
/// - Additional rules beyond 5: $0.01/rule/hr
/// - Monthly = (min(rule_count, 5) * tier1_price + max(0, rule_count - 5) * tier2_price) * 730
pub struct LoadBalancerRulesBuilder {
    client: Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    rule_count: Option<u64>,
}

impl LoadBalancerRulesBuilder {
    /// Create a new load balancer rules builder.
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            rule_count: None,
        }
    }

    /// Set the Azure region.
    ///
    /// Note: Load Balancer pricing uses the `Global` region in the Azure API.
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the API key for this request.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the default fallback price for the primary (tier1) component.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set the number of load balancer rules.
    pub fn rule_count(mut self, count: u64) -> Self {
        self.rule_count = Some(count);
        self
    }

    /// Fetch the primary (tier1) hourly per-rule price.
    ///
    /// Returns the hourly price for the first 5 rules. For the full tiered
    /// monthly calculation, use `fetch_monthly()` instead.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = azure_catalog().find("load-balancer-rules")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            &self.client,
            resource,
            "azure",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch just the price value (tier1 hourly per-rule price).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the total monthly cost based on the tiered pricing model.
    ///
    /// - First 5 rules: tier1_price/rule/hr * 730 hrs
    /// - Additional rules beyond 5: tier2_price/rule/hr * 730 hrs
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let rule_count = self
            .rule_count
            .ok_or_else(|| crate::Error::validation("rule_count is required for fetch_monthly"))?;

        let resource = azure_catalog().find("load-balancer-rules")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);

        // Fetch tier1 (first 5 rules) price
        let tier1_component = resource
            .cost_components
            .iter()
            .find(|c| c.name == "tier1")
            .ok_or_else(|| crate::Error::config("Missing tier1 cost component"))?;

        let tier1_default = self
            .override_default
            .unwrap_or(tier1_component.default_price);
        let tier1_result = PricingEngine::fetch_component_price(
            &self.client,
            tier1_component,
            "azure",
            region,
            self.api_key.as_deref(),
            tier1_default,
            None,
        )
        .await?;

        // Fetch tier2 (additional rules) price
        let tier2_component = resource
            .cost_components
            .iter()
            .find(|c| c.name == "tier2")
            .ok_or_else(|| crate::Error::config("Missing tier2 cost component"))?;

        let tier2_result = PricingEngine::fetch_component_price(
            &self.client,
            tier2_component,
            "azure",
            region,
            self.api_key.as_deref(),
            tier2_component.default_price,
            None,
        )
        .await?;

        // Calculate tiered monthly cost
        let first_five = rule_count.min(TIER1_MAX) as f64;
        let additional = rule_count.saturating_sub(TIER1_MAX) as f64;

        let tier1_monthly = first_five * tier1_result.price * HOURS_PER_MONTH;
        let tier2_monthly = additional * tier2_result.price * HOURS_PER_MONTH;
        // Round to 2 decimal places to avoid floating-point drift (e.g. 3 * 0.025 * 730).
        let total = ((tier1_monthly + tier2_monthly) * 100.0).round() / 100.0;

        let source = if rule_count <= TIER1_MAX {
            // Only tier1 contributes cost — use its source
            if tier1_result.is_from_api() {
                PriceSource::Api
            } else {
                PriceSource::Default
            }
        } else {
            // Both tiers contribute — require both from API
            if tier1_result.is_from_api() && tier2_result.is_from_api() {
                PriceSource::Api
            } else {
                PriceSource::Default
            }
        };

        Ok(PriceResult {
            price: total,
            unit: "month".to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_load_balancer_rules_fetch_returns_default_hourly() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .load_balancer_rules()
            .region("Global")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.025);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_load_balancer_rules_zero_rules() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .load_balancer_rules()
            .rule_count(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_load_balancer_rules_three_rules() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .load_balancer_rules()
            .rule_count(3)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 3 x 0.025 x 730 = 54.75
        assert_eq!(result.price, 54.75);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_load_balancer_rules_five_rules() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .load_balancer_rules()
            .rule_count(5)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 5 x 0.025 x 730 = 91.25
        assert!((result.price - 91.25).abs() < 1e-9);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_load_balancer_rules_ten_rules() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .load_balancer_rules()
            .rule_count(10)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // first 5: 5 x 0.025 x 730 = 91.25
        // additional 5: 5 x 0.01 x 730 = 36.50
        // total: 127.75
        assert!((result.price - 127.75).abs() < 1e-9);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_load_balancer_rules_fetch_monthly_requires_rule_count() {
        let client = Client::anonymous().unwrap();
        let result = client.azure().load_balancer_rules().fetch_monthly().await;

        assert!(result.is_err());
    }
}
