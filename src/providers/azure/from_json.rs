//! Parse Azure CLI JSON output into pricing builders.

use serde_json::Value;

use super::super::json_utils::parse_u64;
use super::managed_disk::{ManagedDiskSize, ManagedDiskType};
use crate::Result;

// ============================================================
// Parsed Structs (shared between async and blocking)
// ============================================================

#[derive(Debug)]
pub(crate) struct ParsedAzureManagedDisk {
    pub disk_type: ManagedDiskType,
    pub size: ManagedDiskSize,
    pub region: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ParsedAzureSnapshot {
    pub region: Option<String>,
    pub size_gb: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct ParsedAzurePublicIp {
    pub region: Option<String>,
}

// ============================================================
// Parse Functions
// ============================================================

/// Parse an Azure managed disk JSON (from `az disk show --output json`).
pub(crate) fn parse_managed_disk_json(json: &Value) -> Result<ParsedAzureManagedDisk> {
    let sku_name = json["sku"]["name"].as_str().ok_or_else(|| {
        crate::Error::validation("missing required field 'sku.name' in disk JSON")
    })?;
    let disk_type = ManagedDiskType::from_sku_name(sku_name)?;

    let gb = json
        .get("diskSizeGb")
        .or_else(|| json.get("diskSizeGB"))
        .and_then(parse_u64)
        .ok_or_else(|| {
            crate::Error::validation("missing required field 'diskSizeGb' in disk JSON")
        })?;
    let size = ManagedDiskSize::from_size_gb(disk_type, gb)?;

    let region = json["location"].as_str().map(|s| s.to_string());

    Ok(ParsedAzureManagedDisk {
        disk_type,
        size,
        region,
    })
}

/// Parse an Azure snapshot JSON (from `az snapshot show --output json`).
pub(crate) fn parse_snapshot_json(json: &Value) -> Result<ParsedAzureSnapshot> {
    let size_gb = json
        .get("diskSizeGb")
        .or_else(|| json.get("diskSizeGB"))
        .and_then(parse_u64);

    let region = json["location"].as_str().map(|s| s.to_string());

    Ok(ParsedAzureSnapshot { region, size_gb })
}

/// Parse an Azure public IP JSON (from `az network public-ip show --output json`).
pub(crate) fn parse_public_ip_json(json: &Value) -> Result<ParsedAzurePublicIp> {
    let region = json["location"].as_str().map(|s| s.to_string());

    Ok(ParsedAzurePublicIp { region })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================
    // parse_managed_disk_json
    // ============================================================

    #[test]
    fn test_parse_managed_disk_premium_lrs() {
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "diskSizeGb": 128,
            "location": "eastus"
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, ManagedDiskType::PremiumSsd);
        assert_eq!(parsed.size, ManagedDiskSize::P10);
        assert_eq!(parsed.region.as_deref(), Some("eastus"));
    }

    #[test]
    fn test_parse_managed_disk_standard_ssd() {
        let json = json!({
            "sku": { "name": "StandardSSD_LRS" },
            "diskSizeGb": 256,
            "location": "westus2"
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, ManagedDiskType::StandardSsd);
        assert_eq!(parsed.size, ManagedDiskSize::E15);
        assert_eq!(parsed.region.as_deref(), Some("westus2"));
    }

    #[test]
    fn test_parse_managed_disk_standard_hdd() {
        let json = json!({
            "sku": { "name": "Standard_LRS" },
            "diskSizeGb": 64,
            "location": "northeurope"
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, ManagedDiskType::StandardHdd);
        assert_eq!(parsed.size, ManagedDiskSize::S6);
        assert_eq!(parsed.region.as_deref(), Some("northeurope"));
    }

    #[test]
    fn test_parse_managed_disk_missing_sku() {
        let json = json!({
            "diskSizeGb": 128,
            "location": "eastus"
        });
        let result = parse_managed_disk_json(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sku.name"));
    }

    #[test]
    fn test_parse_managed_disk_missing_disk_size() {
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "location": "eastus"
        });
        let result = parse_managed_disk_json(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("diskSizeGb"));
    }

    #[test]
    fn test_parse_managed_disk_boundary_sizes() {
        // 128 GB -> P10
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "diskSizeGb": 128
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.size, ManagedDiskSize::P10);

        // 129 GB -> P15
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "diskSizeGb": 129
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.size, ManagedDiskSize::P15);

        // 4 GB -> P1
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "diskSizeGb": 4
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.size, ManagedDiskSize::P1);

        // 5 GB -> P2
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "diskSizeGb": 5
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.size, ManagedDiskSize::P2);
    }

    #[test]
    fn test_parse_managed_disk_uppercase_gb_field() {
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "diskSizeGB": 512,
            "location": "eastus"
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.size, ManagedDiskSize::P20);
    }

    #[test]
    fn test_parse_managed_disk_no_location() {
        let json = json!({
            "sku": { "name": "Premium_LRS" },
            "diskSizeGb": 64
        });
        let parsed = parse_managed_disk_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }

    // ============================================================
    // parse_snapshot_json
    // ============================================================

    #[test]
    fn test_parse_snapshot_json() {
        let json = json!({
            "diskSizeGb": 100,
            "location": "eastus"
        });
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.size_gb, Some(100));
        assert_eq!(parsed.region.as_deref(), Some("eastus"));
    }

    #[test]
    fn test_parse_snapshot_json_uppercase_gb() {
        let json = json!({
            "diskSizeGB": 50,
            "location": "westeurope"
        });
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.size_gb, Some(50));
        assert_eq!(parsed.region.as_deref(), Some("westeurope"));
    }

    #[test]
    fn test_parse_snapshot_json_empty() {
        let json = json!({});
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.region, None);
        assert_eq!(parsed.size_gb, None);
    }

    // ============================================================
    // parse_public_ip_json
    // ============================================================

    #[test]
    fn test_parse_public_ip_json() {
        let json = json!({
            "location": "eastus"
        });
        let parsed = parse_public_ip_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("eastus"));
    }

    #[test]
    fn test_parse_public_ip_json_empty() {
        let json = json!({});
        let parsed = parse_public_ip_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }
}
