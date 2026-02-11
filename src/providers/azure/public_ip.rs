//! Azure Public IP pricing.

use crate::providers::macros::resource_builder;

// ============================================================
// Builder
// ============================================================

resource_builder! {
    /// Builder for querying Azure Public IP prices.
    ///
    /// Returns the per-hour price for a Standard static public IPv4 address.
    pub struct PublicIpBuilder {
        catalog: azure_catalog,
        resource: "public-ip",
        vendor: "azure",
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_public_ip_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .public_ip()
            .region("eastus")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.005);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_public_ip_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .public_ip()
            .region("eastus")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 0.005 × 730 = 3.65
        assert_eq!(result.price, 3.65);
        assert_eq!(result.unit, "month");
    }
}
