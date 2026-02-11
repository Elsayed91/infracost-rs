//! AWS Application Load Balancer pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//!
//! # Per-unit pricing (original behavior)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.aws().alb().fetch().await?;
//! println!("${}/hour", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with LCU usage
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.aws().alb()
//!     .lcu_hours(10000)  // 10,000 LCU-hours per month
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
    /// Builder for querying AWS Application Load Balancer prices.
    ///
    /// Returns the hourly rate for ALB. Additional LCU (Load Balancer Capacity Units)
    /// charges apply based on usage.
    pub struct AlbBuilder {
        catalog: aws_catalog,
        resource: "alb",
        vendor: "aws",
        optional param: lcu_hours(u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    #[tokio::test]
    async fn test_alb_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0225);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_hourly_only() {
        // ALB with no LCU usage specified - hourly cost only
        // Cost = $0.0225/hour * 730 hours = $16.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 16.425);
        assert_eq!(result.unit, "month");
        assert!(result.is_from_default());
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_with_lcu() {
        // ALB with 10,000 LCU-hours per month
        // Cost = ($0.0225 * 730) + ($0.008 * 10000)
        //      = $16.425 + $80 = $96.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(10000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 96.425);
        assert_eq!(result.unit, "month");
        assert!(result.is_from_default());
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_minimal_lcu() {
        // ALB with minimal LCU usage (730 LCU-hours = 1 LCU for whole month)
        // Cost = ($0.0225 * 730) + ($0.008 * 730)
        //      = $16.425 + $5.84 = $22.265/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(730)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 22.265);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_zero_lcu() {
        // ALB with zero LCU-hours specified (same as not specifying)
        // Cost = $0.0225 * 730 = $16.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 16.425);
        assert_eq!(result.unit, "month");
    }
}
