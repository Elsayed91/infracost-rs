//! AWS NAT Gateway pricing.

use crate::providers::macros::resource_builder;

// ============================================================
// Builder
// ============================================================

resource_builder! {
    /// Builder for querying AWS NAT Gateway prices.
    ///
    /// Returns the hourly rate for NAT Gateway. Additional data processing
    /// charges apply ($0.045/GB).
    pub struct NatGatewayBuilder {
        catalog: aws_catalog,
        resource: "nat-gateway",
        vendor: "aws",
        optional param: data_processed_gb(u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    #[tokio::test]
    async fn test_nat_gateway_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.045);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_with_data_processing() {
        // NAT Gateway with 1000 GB data processed per month
        // Cost = ($0.045 * 730) + ($0.045 * 1000) = $32.85 + $45.00 = $77.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 77.85);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_hourly_only() {
        // NAT Gateway with no data processing specified
        // Cost = $0.045 * 730 = $32.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 32.85);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_zero_data() {
        // NAT Gateway with 0 GB data processed
        // Cost = ($0.045 * 730) + ($0.045 * 0) = $32.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .data_processed_gb(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 32.85);
        assert_eq!(result.unit, "month");
    }
}
