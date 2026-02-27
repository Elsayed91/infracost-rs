//! GCP Forwarding Rule (Load Balancer) pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//!
//! # Per-unit pricing (hourly uptime)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let price = client.gcp().forwarding_rule().fetch().await?;
//! println!("${}/hour", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with data processing
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let cost = client.gcp().forwarding_rule()
//!     .data_processed_gb(1000)  // 1000 GB of data processed per month
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use crate::providers::macros::resource_builder;

// ============================================================
// Builder
// ============================================================

resource_builder! {
    /// Builder for querying GCP Forwarding Rule prices.
    pub struct ForwardingRuleBuilder {
        catalog: gcp_catalog,
        resource: "forwarding-rule",
        vendor: "gcp",
        optional param: data_processed_gb(u64),
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_forwarding_rule_builder_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.025);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_forwarding_rule_fetch_monthly_with_data_processing() {
        // Forwarding Rule with 1000 GB of data processed
        // Cost = ($0.025 * 730 hours) + ($0.008 * 1000 GB) = $18.25 + $8.0 = $26.25/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 26.25);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_forwarding_rule_fetch_monthly_hourly_only() {
        // Forwarding Rule with no data processing (0 GB)
        // Cost = $0.025 * 730 hours = $18.25/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .data_processed_gb(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 18.25);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_forwarding_rule_fetch_monthly_without_data_defaults_to_zero() {
        // Forwarding Rule without specifying data_processed_gb defaults to 0
        // Cost = $0.025 * 730 hours = $18.25/month
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 18.25);
        assert_eq!(result.unit, "month");
    }
}
