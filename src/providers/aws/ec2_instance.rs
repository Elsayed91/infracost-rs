//! AWS EC2 Instance pricing.
//!
//! Supports any EC2 instance type dynamically - no hardcoded types.
//!
//! # Per-unit pricing (hourly rate)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let price = client.aws().ec2_instance("t3.micro")
//!     .fetch().await?;
//! println!("${}/hour", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let cost = client.aws().ec2_instance("t3.micro")
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Windows instance
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let cost = client.aws().ec2_instance("m5.xlarge")
//!     .operating_system("Windows")
//!     .fetch_monthly().await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::catalog::{aws_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

/// Builder for querying AWS EC2 Instance prices.
pub struct Ec2InstanceBuilder {
    client: Client,
    instance_type: String,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    operating_system: String,
    tenancy: String,
    pre_installed_sw: String,
}

impl Ec2InstanceBuilder {
    pub(crate) fn new(client: Client, instance_type: impl Into<String>) -> Self {
        Self {
            client,
            instance_type: instance_type.into(),
            region: None,
            api_key: None,
            override_default: None,
            operating_system: "Linux".to_string(),
            tenancy: "Shared".to_string(),
            pre_installed_sw: "NA".to_string(),
        }
    }

    /// Set the AWS region (e.g., "us-east-1").
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

    /// Set the operating system (default: "Linux").
    /// Options: "Linux", "Windows", "RHEL", "SUSE"
    pub fn operating_system(mut self, os: impl Into<String>) -> Self {
        self.operating_system = os.into();
        self
    }

    /// Set the tenancy (default: "Shared").
    /// Options: "Shared", "Dedicated", "Host"
    pub fn tenancy(mut self, tenancy: impl Into<String>) -> Self {
        self.tenancy = tenancy.into();
        self
    }

    /// Set pre-installed software (default: "NA" for none).
    /// Options: "NA", "SQL Web", "SQL Ent", "SQL Std"
    pub fn pre_installed_sw(mut self, sw: impl Into<String>) -> Self {
        self.pre_installed_sw = sw.into();
        self
    }

    /// Build the string parameters map for query attribute substitution.
    fn build_string_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("instance_type".to_string(), self.instance_type.clone());
        params.insert(
            "operating_system".to_string(),
            self.operating_system.clone(),
        );
        params.insert("tenancy".to_string(), self.tenancy.clone());
        params.insert(
            "pre_installed_sw".to_string(),
            self.pre_installed_sw.clone(),
        );
        params
    }

    /// Fetch just the price value (hourly rate).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    /// Returns the hourly on-demand price.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("ec2-instance")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let string_params = self.build_string_params();

        let default_price = self
            .override_default
            .unwrap_or(resource.cost_components[0].default_price);
        let component = &resource.cost_components[0];

        PricingEngine::fetch_component_price(
            &self.client,
            component,
            "aws",
            region,
            self.api_key.as_deref(),
            default_price,
            Some(&string_params),
        )
        .await
    }

    /// Fetch total monthly cost (hourly price * 730).
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("ec2-instance")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let string_params = self.build_string_params();

        let params = HashMap::new(); // No quantity params needed for hourly_to_monthly

        PricingEngine::fetch_monthly_with_string_params(
            &self.client,
            resource,
            "aws",
            region,
            self.api_key.as_deref(),
            &params,
            Some(&string_params),
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ec2_instance_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0104);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_ec2_instance_fetch_monthly() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // $0.0104/hr * 730 = $7.592/month
        let expected = 0.0104 * 730.0;
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_ec2_instance_with_operating_system() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .operating_system("Windows")
            .fetch()
            .await
            .unwrap();

        // Without API key, still returns default
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0104); // Default from YAML
    }

    #[tokio::test]
    async fn test_ec2_instance_with_tenancy() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .ec2_instance("t3.micro")
            .tenancy("Dedicated")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
    }
}
