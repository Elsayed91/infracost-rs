//! GCP Static IP pricing.

use crate::providers::macros::resource_builder;

// ============================================================
// Builder
// ============================================================

resource_builder! {
    /// Builder for querying GCP static IP prices.
    pub struct StaticIpBuilder {
        catalog: gcp_catalog,
        resource: "static-ip",
        vendor: "gcp",
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_static_ip_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.01);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_static_ip_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 0.01 x 730 = 7.30
        assert_eq!(result.price, 7.30);
        assert_eq!(result.unit, "month");
    }
}
