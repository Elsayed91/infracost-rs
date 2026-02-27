//! Azure Snapshot pricing.

use crate::providers::macros::resource_builder;

// ============================================================
// Builder
// ============================================================

resource_builder! {
    /// Builder for querying Azure Snapshot prices.
    ///
    /// Returns the per-GB-month price for Standard (HDD) snapshots.
    pub struct SnapshotBuilder {
        catalog: azure_catalog,
        resource: "snapshot",
        vendor: "azure",
        required param: size_gb(u64) => "size_gb is required for fetch_monthly",
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_snapshot_builder_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .snapshot()
            .region("eastus")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
        assert_eq!(result.unit, "GB-month");
    }

    #[tokio::test]
    async fn test_snapshot_fetch_monthly() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .snapshot()
            .region("eastus")
            .size_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 0.05 × 100 = 5.00
        assert_eq!(result.price, 5.00);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_snapshot_fetch_monthly_requires_size() {
        let client = Client::anonymous().unwrap();
        let result = client
            .azure()
            .snapshot()
            .region("eastus")
            .fetch_monthly()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size_gb is required"));
    }
}
