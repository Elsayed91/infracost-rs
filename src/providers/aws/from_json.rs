//! Parse AWS CLI JSON output into pricing builders.

use serde_json::Value;

use crate::Result;

use super::super::json_utils::{aws_az_to_region, parse_u64};
use super::ebs::EbsType;

// ============================================================
// Parsed structs
// ============================================================

/// Parsed fields from an AWS EBS volume JSON (`aws ec2 describe-volumes`).
#[derive(Debug)]
pub(crate) struct ParsedAwsEbs {
    pub ebs_type: EbsType,
    pub region: Option<String>,
    pub size_gb: Option<u64>,
    pub iops: Option<u64>,
    pub throughput_mibps: Option<u64>,
}

/// Parsed fields from an AWS EBS Snapshot JSON (`aws ec2 describe-snapshots`).
#[derive(Debug)]
pub(crate) struct ParsedAwsSnapshot {
    pub region: Option<String>,
    pub size_gb: Option<u64>,
}

/// Parsed fields from an AWS Elastic IP JSON (`aws ec2 describe-addresses`).
#[derive(Debug)]
pub(crate) struct ParsedAwsElasticIp {
    pub region: Option<String>,
}

/// Parsed fields from an AWS NAT Gateway JSON (`aws ec2 describe-nat-gateways`).
#[derive(Debug)]
pub(crate) struct ParsedAwsNatGateway {
    pub region: Option<String>,
}

/// Parsed fields from an AWS ALB JSON (`aws elbv2 describe-load-balancers`).
#[derive(Debug)]
pub(crate) struct ParsedAwsAlb {
    pub region: Option<String>,
}

// ============================================================
// Parse functions
// ============================================================

/// Parse an AWS EBS volume JSON into [`ParsedAwsEbs`].
///
/// Expected input: a single object from `aws ec2 describe-volumes`.
///
/// Required fields: `VolumeType`.
/// Optional fields: `AvailabilityZone`, `Size`, `Iops`, `Throughput`.
pub(crate) fn parse_ebs_json(json: &Value) -> Result<ParsedAwsEbs> {
    let volume_type = json["VolumeType"]
        .as_str()
        .ok_or_else(|| crate::Error::validation("missing required field 'VolumeType'"))?;

    let ebs_type = EbsType::from(volume_type);

    let region = json["AvailabilityZone"].as_str().and_then(aws_az_to_region);

    let size_gb = parse_u64(&json["Size"]);
    let iops = parse_u64(&json["Iops"]);
    let throughput_mibps = parse_u64(&json["Throughput"]);

    Ok(ParsedAwsEbs {
        ebs_type,
        region,
        size_gb,
        iops,
        throughput_mibps,
    })
}

/// Parse an AWS EBS Snapshot JSON into [`ParsedAwsSnapshot`].
///
/// Expected input: a single object from `aws ec2 describe-snapshots`.
///
/// Optional fields: `VolumeSize`, `Region`.
pub(crate) fn parse_snapshot_json(json: &Value) -> Result<ParsedAwsSnapshot> {
    let size_gb = parse_u64(&json["VolumeSize"]);
    let region = json["Region"].as_str().map(|s| s.to_string());

    Ok(ParsedAwsSnapshot { region, size_gb })
}

/// Parse an AWS Elastic IP JSON into [`ParsedAwsElasticIp`].
///
/// Expected input: a single object from `aws ec2 describe-addresses`.
///
/// Optional fields: `NetworkBorderGroup` (used as region).
pub(crate) fn parse_elastic_ip_json(json: &Value) -> Result<ParsedAwsElasticIp> {
    let region = json["NetworkBorderGroup"].as_str().map(|s| s.to_string());

    Ok(ParsedAwsElasticIp { region })
}

/// Parse an AWS NAT Gateway JSON into [`ParsedAwsNatGateway`].
///
/// Expected input: a single object from `aws ec2 describe-nat-gateways`.
///
/// Extracts region from `SubnetId` prefix or `NatGatewayAddresses[0].NetworkInterfaceId`.
/// Falls back to trying `Region` field if present.
pub(crate) fn parse_nat_gateway_json(json: &Value) -> Result<ParsedAwsNatGateway> {
    // Try the SubnetId — AWS subnet IDs don't directly encode region,
    // so look for an explicit Region field first, then try AZ-based fields.
    let region = json["Region"].as_str().map(|s| s.to_string()).or_else(|| {
        // Some NAT Gateway outputs include AvailabilityZone
        json["AvailabilityZone"].as_str().and_then(aws_az_to_region)
    });

    Ok(ParsedAwsNatGateway { region })
}

/// Parse an AWS ALB JSON into [`ParsedAwsAlb`].
///
/// Expected input: a single object from `aws elbv2 describe-load-balancers`.
///
/// Extracts region from `AvailabilityZones[0].ZoneName`.
pub(crate) fn parse_alb_json(json: &Value) -> Result<ParsedAwsAlb> {
    let region = json["AvailabilityZones"]
        .as_array()
        .and_then(|zones| zones.first())
        .and_then(|zone| zone["ZoneName"].as_str())
        .and_then(aws_az_to_region);

    Ok(ParsedAwsAlb { region })
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── parse_ebs_json ──────────────────────────────────────

    #[test]
    fn test_parse_ebs_gp3_all_fields() {
        let json = json!({
            "VolumeType": "gp3",
            "AvailabilityZone": "us-east-1a",
            "Size": 500,
            "Iops": 6000,
            "Throughput": 250
        });
        let parsed = parse_ebs_json(&json).unwrap();
        assert_eq!(parsed.ebs_type, EbsType::Gp3);
        assert_eq!(parsed.region.as_deref(), Some("us-east-1"));
        assert_eq!(parsed.size_gb, Some(500));
        assert_eq!(parsed.iops, Some(6000));
        assert_eq!(parsed.throughput_mibps, Some(250));
    }

    #[test]
    fn test_parse_ebs_io2() {
        let json = json!({
            "VolumeType": "io2",
            "AvailabilityZone": "eu-west-1b",
            "Size": 100,
            "Iops": 10000
        });
        let parsed = parse_ebs_json(&json).unwrap();
        assert_eq!(parsed.ebs_type, EbsType::Io2);
        assert_eq!(parsed.region.as_deref(), Some("eu-west-1"));
        assert_eq!(parsed.size_gb, Some(100));
        assert_eq!(parsed.iops, Some(10000));
        assert_eq!(parsed.throughput_mibps, None);
    }

    #[test]
    fn test_parse_ebs_missing_volume_type() {
        let json = json!({
            "AvailabilityZone": "us-east-1a",
            "Size": 500
        });
        let result = parse_ebs_json(&json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("VolumeType"));
    }

    #[test]
    fn test_parse_ebs_missing_optional_fields() {
        let json = json!({
            "VolumeType": "gp2"
        });
        let parsed = parse_ebs_json(&json).unwrap();
        assert_eq!(parsed.ebs_type, EbsType::Gp2);
        assert_eq!(parsed.region, None);
        assert_eq!(parsed.size_gb, None);
        assert_eq!(parsed.iops, None);
        assert_eq!(parsed.throughput_mibps, None);
    }

    #[test]
    fn test_parse_ebs_string_size() {
        let json = json!({
            "VolumeType": "sc1",
            "Size": "1000"
        });
        let parsed = parse_ebs_json(&json).unwrap();
        assert_eq!(parsed.ebs_type, EbsType::Sc1);
        assert_eq!(parsed.size_gb, Some(1000));
    }

    // ── parse_snapshot_json ─────────────────────────────────

    #[test]
    fn test_parse_snapshot_all_fields() {
        let json = json!({
            "VolumeSize": 100,
            "Region": "us-east-1"
        });
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("us-east-1"));
        assert_eq!(parsed.size_gb, Some(100));
    }

    #[test]
    fn test_parse_snapshot_no_region() {
        let json = json!({
            "VolumeSize": 50
        });
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.region, None);
        assert_eq!(parsed.size_gb, Some(50));
    }

    #[test]
    fn test_parse_snapshot_empty() {
        let json = json!({});
        let parsed = parse_snapshot_json(&json).unwrap();
        assert_eq!(parsed.region, None);
        assert_eq!(parsed.size_gb, None);
    }

    // ── parse_elastic_ip_json ───────────────────────────────

    #[test]
    fn test_parse_elastic_ip_with_border_group() {
        let json = json!({
            "PublicIp": "203.0.113.10",
            "AllocationId": "eipalloc-12345",
            "NetworkBorderGroup": "us-east-1"
        });
        let parsed = parse_elastic_ip_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("us-east-1"));
    }

    #[test]
    fn test_parse_elastic_ip_no_border_group() {
        let json = json!({
            "PublicIp": "203.0.113.10"
        });
        let parsed = parse_elastic_ip_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }

    // ── parse_nat_gateway_json ──────────────────────────────

    #[test]
    fn test_parse_nat_gateway_with_region() {
        let json = json!({
            "NatGatewayId": "nat-12345",
            "Region": "eu-west-1",
            "NatGatewayAddresses": [
                { "NetworkInterfaceId": "eni-12345" }
            ]
        });
        let parsed = parse_nat_gateway_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("eu-west-1"));
    }

    #[test]
    fn test_parse_nat_gateway_with_az() {
        let json = json!({
            "NatGatewayId": "nat-12345",
            "AvailabilityZone": "ap-southeast-1a"
        });
        let parsed = parse_nat_gateway_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("ap-southeast-1"));
    }

    #[test]
    fn test_parse_nat_gateway_no_region() {
        let json = json!({
            "NatGatewayId": "nat-12345"
        });
        let parsed = parse_nat_gateway_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }

    // ── parse_alb_json ──────────────────────────────────────

    #[test]
    fn test_parse_alb_with_az() {
        let json = json!({
            "LoadBalancerArn": "arn:aws:elasticloadbalancing:us-east-1:123456789012:loadbalancer/app/my-alb/abc123",
            "AvailabilityZones": [
                { "ZoneName": "us-east-1a", "SubnetId": "subnet-12345" },
                { "ZoneName": "us-east-1b", "SubnetId": "subnet-67890" }
            ]
        });
        let parsed = parse_alb_json(&json).unwrap();
        assert_eq!(parsed.region.as_deref(), Some("us-east-1"));
    }

    #[test]
    fn test_parse_alb_empty_az_list() {
        let json = json!({
            "LoadBalancerArn": "arn:aws:...",
            "AvailabilityZones": []
        });
        let parsed = parse_alb_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }

    #[test]
    fn test_parse_alb_no_az_field() {
        let json = json!({
            "LoadBalancerArn": "arn:aws:..."
        });
        let parsed = parse_alb_json(&json).unwrap();
        assert_eq!(parsed.region, None);
    }
}
