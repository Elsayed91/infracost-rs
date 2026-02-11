//! AWS EBS Snapshot pricing.

use crate::providers::macros::resource_builder;

// ============================================================
// Builder
// ============================================================

resource_builder! {
    /// Builder for querying AWS EBS Snapshot prices.
    pub struct SnapshotBuilder {
        catalog: aws_catalog,
        resource: "snapshot",
        vendor: "aws",
        required param: size_gb(u64) => "size_gb is required for fetch_monthly",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    #[tokio::test]
    async fn test_snapshot_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
        assert_eq!(result.unit, "GB-month");
    }

    #[tokio::test]
    async fn test_snapshot_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .size_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        // $0.05/GB-month × 100 GB = $5.00/month
        assert_eq!(result.price, 5.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_snapshot_fetch_monthly_requires_size() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .fetch_monthly()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size_gb is required"));
    }
}
