//! YAML-driven pricing catalog.
//!
//! This module provides a declarative way to define cloud resource pricing
//! using YAML files. The pricing engine handles all the common logic:
//! building filters, querying the API, applying post-filters, and computing costs.
//!
//! Resource definitions live under `resources/{provider}/{resource}.yaml`.
//! Each file is a YAML list of `ResourceDef` entries.

pub mod engine;
pub mod types;

use std::sync::LazyLock;
use types::ResourceCatalog;

/// GCP resource catalog, loaded once from embedded YAML files.
pub static GCP_CATALOG: LazyLock<ResourceCatalog> = LazyLock::new(|| {
    ResourceCatalog::from_parts(
        "gcp",
        &[
            include_str!("../../resources/gcp/disk.yaml"),
            include_str!("../../resources/gcp/snapshot.yaml"),
            include_str!("../../resources/gcp/static-ip.yaml"),
            include_str!("../../resources/gcp/nat-gateway.yaml"),
            include_str!("../../resources/gcp/forwarding-rule.yaml"),
            include_str!("../../resources/gcp/backend-service.yaml"),
            include_str!("../../resources/gcp/compute-instance.yaml"),
            include_str!("../../resources/gcp/cloud-sql.yaml"),
            include_str!("../../resources/gcp/bigquery-storage.yaml"),
        ],
    )
});

/// AWS resource catalog, loaded once from embedded YAML files.
pub static AWS_CATALOG: LazyLock<ResourceCatalog> = LazyLock::new(|| {
    ResourceCatalog::from_parts(
        "aws",
        &[
            include_str!("../../resources/aws/ebs.yaml"),
            include_str!("../../resources/aws/snapshot.yaml"),
            include_str!("../../resources/aws/elastic-ip.yaml"),
            include_str!("../../resources/aws/nat-gateway.yaml"),
            include_str!("../../resources/aws/alb.yaml"),
            include_str!("../../resources/aws/ec2-instance.yaml"),
            include_str!("../../resources/aws/rds.yaml"),
        ],
    )
});

/// Azure resource catalog, loaded once from embedded YAML files.
pub static AZURE_CATALOG: LazyLock<ResourceCatalog> = LazyLock::new(|| {
    ResourceCatalog::from_parts(
        "azure",
        &[
            include_str!("../../resources/azure/managed-disk/premium-ssd.yaml"),
            include_str!("../../resources/azure/managed-disk/standard-ssd.yaml"),
            include_str!("../../resources/azure/managed-disk/standard-hdd.yaml"),
            include_str!("../../resources/azure/snapshot.yaml"),
            include_str!("../../resources/azure/public-ip.yaml"),
        ],
    )
});

/// Get the GCP catalog.
pub fn gcp_catalog() -> &'static ResourceCatalog {
    &GCP_CATALOG
}

/// Get the AWS catalog.
pub fn aws_catalog() -> &'static ResourceCatalog {
    &AWS_CATALOG
}

/// Get the Azure catalog.
pub fn azure_catalog() -> &'static ResourceCatalog {
    &AZURE_CATALOG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcp_catalog_loads() {
        let cat = gcp_catalog();
        assert_eq!(cat.vendor, "gcp");
        assert!(cat.find("disk/pd-ssd").is_ok());
        assert!(cat.find("static-ip").is_ok());
        assert!(cat.find("snapshot/standard").is_ok());
        assert!(cat.find("snapshot/archive").is_ok());
        assert!(cat.find("nat-gateway").is_ok());
        assert!(cat.find("forwarding-rule").is_ok());
        assert!(cat.find("backend-service").is_ok());
        assert!(cat.find("backend-service/premium").is_ok());
        assert!(cat.find("backend-service/standard").is_ok());
        assert!(cat.find("compute-instance").is_ok());
        assert!(cat.find("cloud-sql").is_ok());
        assert!(cat.find("bigquery-storage").is_ok());
    }

    #[test]
    fn test_aws_catalog_loads() {
        let cat = aws_catalog();
        assert_eq!(cat.vendor, "aws");
        assert!(cat.find("ebs/gp3").is_ok());
        assert!(cat.find("ebs/io2").is_ok());
        assert!(cat.find("snapshot").is_ok());
        assert!(cat.find("elastic-ip").is_ok());
        assert!(cat.find("nat-gateway").is_ok());
        assert!(cat.find("alb").is_ok());
        assert!(cat.find("ec2-instance").is_ok());
        assert!(cat.find("rds").is_ok());
        assert!(cat.find("rds-storage/gp3").is_ok());
        assert!(cat.find("rds-storage/gp2").is_ok());
        assert!(cat.find("rds-storage/io1").is_ok());
        assert!(cat.find("rds-storage/io2").is_ok());
        assert!(cat.find("rds-storage/magnetic").is_ok());
    }

    #[test]
    fn test_azure_catalog_loads() {
        let cat = azure_catalog();
        assert_eq!(cat.vendor, "azure");
        assert!(cat.find("managed-disk/premium-ssd/p10").is_ok());
        assert!(cat.find("managed-disk/standard-ssd/e10").is_ok());
        assert!(cat.find("managed-disk/standard-hdd/s10").is_ok());
        assert!(cat.find("snapshot").is_ok());
        assert!(cat.find("public-ip").is_ok());
    }

    #[test]
    fn test_gcp_disk_defaults_match() {
        let cat = gcp_catalog();
        let pd_ssd = cat.find("disk/pd-ssd").unwrap();
        let storage = pd_ssd
            .cost_components
            .iter()
            .find(|c| c.is_primary)
            .unwrap();
        assert_eq!(storage.default_price, 0.17);
        assert_eq!(storage.unit, "GiB-month");
    }

    #[test]
    fn test_aws_ebs_gp3_defaults_match() {
        let cat = aws_catalog();
        let gp3 = cat.find("ebs/gp3").unwrap();
        let storage = gp3.cost_components.iter().find(|c| c.is_primary).unwrap();
        assert_eq!(storage.default_price, 0.08);
        assert_eq!(storage.unit, "GB-month");
    }

    #[test]
    fn test_gcp_backend_service_defaults_match() {
        let cat = gcp_catalog();

        let premium = cat.find("backend-service/premium").unwrap();
        let data_proc = premium
            .cost_components
            .iter()
            .find(|c| c.is_primary)
            .unwrap();
        assert_eq!(data_proc.default_price, 0.008);
        assert_eq!(data_proc.unit, "GiB");

        let standard = cat.find("backend-service/standard").unwrap();
        let data_proc = standard
            .cost_components
            .iter()
            .find(|c| c.is_primary)
            .unwrap();
        assert_eq!(data_proc.default_price, 0.008);
        assert_eq!(data_proc.unit, "GiB");
    }

    #[test]
    fn test_azure_managed_disk_defaults_match() {
        let cat = azure_catalog();
        let p10 = cat.find("managed-disk/premium-ssd/p10").unwrap();
        let price_comp = &p10.cost_components[0];
        assert_eq!(price_comp.default_price, 19.71);
        assert_eq!(price_comp.unit, "month");
    }
}
