//! AWS Elastic IP pricing.

use crate::providers::macros::resource_builder;

// ============================================================
// Builder
// ============================================================

resource_builder! {
    /// Builder for querying AWS Elastic IP prices.
    ///
    /// Returns the price for an idle (unused) Elastic IP address.
    pub struct ElasticIpBuilder {
        catalog: aws_catalog,
        resource: "elastic-ip",
        vendor: "aws",
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_elastic_ip_builder_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.005);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_elastic_ip_fetch_monthly() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        // $0.005/hour × 730 hours = $3.65/month
        assert_eq!(result.price, 3.65);
        assert_eq!(result.unit, "month");
    }
}
