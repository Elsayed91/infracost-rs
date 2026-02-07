//! Parse GCP CLI JSON output into pricing builders.

use serde_json::Value;

use super::super::json_utils::{
    gcp_region_from_link, gcp_zone_to_region, last_path_segment, parse_u64,
};
use super::disk::DiskType;
use crate::Result;

// ============================================================
// Parsed Structs (shared between async and blocking)
// ============================================================

#[derive(Debug)]
pub(crate) struct ParsedGcpDisk {
    pub disk_type: DiskType,
    pub region: Option<String>,
    pub size_gb: Option<u64>,
    pub iops: Option<u64>,
    pub throughput: Option<u64>,
    pub regional: bool,
}

pub(crate) struct ParsedGcpSnapshot {
    pub region: Option<String>,
    pub size_gb: Option<u64>,
}

pub(crate) struct ParsedGcpStaticIp {
    pub region: Option<String>,
}

pub(crate) struct ParsedGcpNatGateway {
    pub region: Option<String>,
}

// ============================================================
// Parse Functions
// ============================================================

/// Parse a GCP disk JSON (from `gcloud compute disks describe --format=json`).
pub(crate) fn parse_disk_json(json: &Value) -> Result<ParsedGcpDisk> {
    let type_str = json["type"]
        .as_str()
        .ok_or_else(|| crate::Error::validation("missing required field 'type' in disk JSON"))?;
    let disk_type = DiskType::from(last_path_segment(type_str));

    let region = json["zone"]
        .as_str()
        .and_then(gcp_zone_to_region)
        .or_else(|| {
            json["region"]
                .as_str()
                .map(|r| last_path_segment(r).to_string())
        });

    let size_gb = parse_u64(&json["sizeGb"]);
    let iops = parse_u64(&json["provisionedIops"]);
    let throughput = parse_u64(&json["provisionedThroughput"]);

    let regional = json["replicaZones"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    Ok(ParsedGcpDisk {
        disk_type,
        region,
        size_gb,
        iops,
        throughput,
        regional,
    })
}

/// Parse a GCP snapshot JSON (from `gcloud compute snapshots describe --format=json`).
pub(crate) fn parse_snapshot_json(json: &Value) -> Result<ParsedGcpSnapshot> {
    let size_gb = parse_u64(&json["storageBytes"]).map(|bytes| bytes / 1_073_741_824);

    let region = json["storageLocations"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            json["selfLink"]
                .as_str()
                .and_then(gcp_region_from_link)
                .map(|s| s.to_string())
        });

    Ok(ParsedGcpSnapshot { region, size_gb })
}

/// Parse a GCP static IP JSON (from `gcloud compute addresses describe --format=json`).
pub(crate) fn parse_static_ip_json(json: &Value) -> Result<ParsedGcpStaticIp> {
    let region = json["region"]
        .as_str()
        .map(|r| last_path_segment(r).to_string())
        .or_else(|| {
            json["selfLink"]
                .as_str()
                .and_then(gcp_region_from_link)
                .map(|s| s.to_string())
        });

    Ok(ParsedGcpStaticIp { region })
}

/// Parse a GCP NAT gateway JSON (from `gcloud compute routers nats describe --format=json`).
pub(crate) fn parse_nat_gateway_json(json: &Value) -> Result<ParsedGcpNatGateway> {
    let region = json["region"]
        .as_str()
        .map(|r| last_path_segment(r).to_string())
        .or_else(|| {
            json["selfLink"]
                .as_str()
                .and_then(gcp_region_from_link)
                .map(|s| s.to_string())
        });

    Ok(ParsedGcpNatGateway { region })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================
    // parse_disk_json
    // ============================================================

    #[test]
    fn test_parse_disk_json_zonal() {
        let json = json!({
            "type": "projects/my-project/zones/us-central1-a/diskTypes/pd-ssd",
            "zone": "projects/my-project/zones/us-central1-a",
            "sizeGb": "500",
            "provisionedIops": 15000
        });
        let parsed = parse_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, DiskType::PdSsd);
        assert_eq!(parsed.region.as_deref(), Some("us-central1"));
        assert_eq!(parsed.size_gb, Some(500));
        assert_eq!(parsed.iops, Some(15000));
        assert_eq!(parsed.throughput, None);
        assert!(!parsed.regional);
    }

    #[test]
    fn test_parse_disk_json_regional() {
        let json = json!({
            "type": "projects/my-project/regions/us-central1/diskTypes/pd-balanced",
            "region": "projects/my-project/regions/us-central1",
            "sizeGb": "200",
            "replicaZones": [
                "projects/my-project/zones/us-central1-a",
                "projects/my-project/zones/us-central1-b"
            ]
        });
        let parsed = parse_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, DiskType::PdBalanced);
        assert_eq!(parsed.region.as_deref(), Some("us-central1"));
        assert_eq!(parsed.size_gb, Some(200));
        assert!(parsed.regional);
    }

    #[test]
    fn test_parse_disk_json_simple_type() {
        let json = json!({
            "type": "pd-extreme",
            "sizeGb": "100"
        });
        let parsed = parse_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, DiskType::PdExtreme);
        assert_eq!(parsed.region, None);
        assert_eq!(parsed.size_gb, Some(100));
    }

    #[test]
    fn test_parse_disk_json_missing_type() {
        let json = json!({
            "sizeGb": "100"
        });
        let result = parse_disk_json(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("type"));
    }

    #[test]
    fn test_parse_disk_json_missing_optional_fields() {
        let json = json!({
            "type": "pd-standard"
        });
        let parsed = parse_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, DiskType::PdStandard);
        assert_eq!(parsed.region, None);
        assert_eq!(parsed.size_gb, None);
        assert_eq!(parsed.iops, None);
        assert_eq!(parsed.throughput, None);
        assert!(!parsed.regional);
    }

    #[test]
    fn test_parse_disk_json_with_throughput() {
        let json = json!({
            "type": "hyperdisk-balanced",
            "zone": "us-central1-a",
            "sizeGb": "1000",
            "provisionedIops": 3000,
            "provisionedThroughput": 140
        });
        let parsed = parse_disk_json(&json).unwrap();
        assert_eq!(parsed.disk_type, DiskType::HyperdiskBalanced);
        assert_eq!(parsed.region.as_deref(), Some("us-central1"));
        assert_eq!(parsed.size_gb, Some(1000));
        assert_eq!(parsed.iops, Some(3000));
        assert_eq!(parsed.throughput, Some(140));
        assert!(!parsed.regional);
    }

    // ============================================================
    // parse_snapshot_json
    // ============================================================

    #[test]
    fn test_parse_snapshot_json_with_storage_bytes() {
        let json = json!({
            "storageBytes": "10737418240",
            "storageLocations": ["us-central1"]
        });
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.size_gb, Some(10)); // 10737418240 / 1073741824 = 10
        assert_eq!(parsed.region.as_deref(), Some("us-central1"));
    }

    #[test]
    fn test_parse_snapshot_json_region_from_self_link() {
        let json = json!({
            "selfLink": "https://www.googleapis.com/compute/v1/projects/my-project/regions/europe-west1/snapshots/my-snap"
        });
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("europe-west1"));
        assert_eq!(parsed.size_gb, None);
    }

    #[test]
    fn test_parse_snapshot_json_empty() {
        let json = json!({});
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.region, None);
        assert_eq!(parsed.size_gb, None);
    }

    // ============================================================
    // parse_static_ip_json
    // ============================================================

    #[test]
    fn test_parse_static_ip_json_with_region() {
        let json = json!({
            "region": "projects/my-project/regions/us-east1"
        });
        let parsed = parse_static_ip_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("us-east1"));
    }

    #[test]
    fn test_parse_static_ip_json_from_self_link() {
        let json = json!({
            "selfLink": "https://www.googleapis.com/compute/v1/projects/my-project/regions/asia-east1/addresses/my-ip"
        });
        let parsed = parse_static_ip_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("asia-east1"));
    }

    #[test]
    fn test_parse_static_ip_json_empty() {
        let json = json!({});
        let parsed = parse_static_ip_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }

    // ============================================================
    // parse_nat_gateway_json
    // ============================================================

    #[test]
    fn test_parse_nat_gateway_json_with_region() {
        let json = json!({
            "region": "projects/my-project/regions/us-west1"
        });
        let parsed = parse_nat_gateway_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("us-west1"));
    }

    #[test]
    fn test_parse_nat_gateway_json_from_self_link() {
        let json = json!({
            "selfLink": "https://www.googleapis.com/compute/v1/projects/my-project/regions/europe-north1/routers/my-router"
        });
        let parsed = parse_nat_gateway_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("europe-north1"));
    }

    #[test]
    fn test_parse_nat_gateway_json_empty() {
        let json = json!({});
        let parsed = parse_nat_gateway_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }
}
