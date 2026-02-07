//! Shared helpers for parsing cloud CLI JSON output.

/// Extract the last path segment from a URL or path.
///
/// `"projects/xxx/zones/us-central1-a/diskTypes/pd-ssd"` -> `"pd-ssd"`
pub fn last_path_segment(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
}

/// Convert a GCP zone (or zone URL) to a region.
///
/// `"us-central1-a"` -> `"us-central1"`
/// `".../zones/us-central1-a"` -> `"us-central1"`
pub fn gcp_zone_to_region(zone: &str) -> Option<String> {
    let zone_name = last_path_segment(zone);
    // Strip trailing "-a", "-b", "-c", etc.
    let idx = zone_name.rfind('-')?;
    let suffix = &zone_name[idx + 1..];
    // Zone suffixes are single lowercase letters
    if suffix.len() == 1 && suffix.chars().next()?.is_ascii_lowercase() {
        Some(zone_name[..idx].to_string())
    } else {
        None
    }
}

/// Extract a region name from a GCP region self-link.
///
/// `".../regions/us-central1"` -> `"us-central1"`
pub fn gcp_region_from_link(link: &str) -> Option<&str> {
    // Look for "/regions/" and take the segment after it
    let marker = "/regions/";
    let start = link.find(marker)? + marker.len();
    let rest = &link[start..];
    // Take up to the next '/' or end
    Some(rest.split('/').next().unwrap_or(rest))
}

/// Convert an AWS availability zone to a region.
///
/// `"us-east-1a"` -> `"us-east-1"`
/// `"eu-west-2b"` -> `"eu-west-2"`
pub fn aws_az_to_region(az: &str) -> Option<String> {
    if az.is_empty() {
        return None;
    }
    // Strip trailing lowercase letter(s) that form the AZ suffix
    // AZ names end with a single letter: us-east-1a, eu-west-1b
    let trimmed = az.trim_end_matches(|c: char| c.is_ascii_lowercase());
    if trimmed.is_empty() || trimmed == az {
        return None;
    }
    Some(trimmed.to_string())
}

/// Parse a JSON value as u64, handling both string and number representations.
///
/// Handles `"500"` (string) and `500` (number).
pub fn parse_u64(val: &serde_json::Value) -> Option<u64> {
    match val {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // last_path_segment
    // ============================================================

    #[test]
    fn test_last_path_segment_url() {
        assert_eq!(
            last_path_segment("projects/xxx/zones/us-central1-a/diskTypes/pd-ssd"),
            "pd-ssd"
        );
    }

    #[test]
    fn test_last_path_segment_simple() {
        assert_eq!(last_path_segment("pd-ssd"), "pd-ssd");
    }

    #[test]
    fn test_last_path_segment_trailing_slash() {
        assert_eq!(last_path_segment("a/b/c/"), "");
    }

    #[test]
    fn test_last_path_segment_empty() {
        assert_eq!(last_path_segment(""), "");
    }

    // ============================================================
    // gcp_zone_to_region
    // ============================================================

    #[test]
    fn test_gcp_zone_to_region_simple() {
        assert_eq!(
            gcp_zone_to_region("us-central1-a"),
            Some("us-central1".to_string())
        );
    }

    #[test]
    fn test_gcp_zone_to_region_url() {
        assert_eq!(
            gcp_zone_to_region("projects/my-project/zones/us-central1-a"),
            Some("us-central1".to_string())
        );
    }

    #[test]
    fn test_gcp_zone_to_region_different_zones() {
        assert_eq!(
            gcp_zone_to_region("europe-west1-b"),
            Some("europe-west1".to_string())
        );
        assert_eq!(
            gcp_zone_to_region("asia-southeast1-c"),
            Some("asia-southeast1".to_string())
        );
    }

    #[test]
    fn test_gcp_zone_to_region_invalid() {
        assert_eq!(gcp_zone_to_region("us-central1"), None);
        assert_eq!(gcp_zone_to_region(""), None);
    }

    // ============================================================
    // gcp_region_from_link
    // ============================================================

    #[test]
    fn test_gcp_region_from_link() {
        assert_eq!(
            gcp_region_from_link(
                "https://www.googleapis.com/compute/v1/projects/my-project/regions/us-central1"
            ),
            Some("us-central1")
        );
    }

    #[test]
    fn test_gcp_region_from_link_with_more_path() {
        assert_eq!(
            gcp_region_from_link("projects/my-project/regions/europe-west1/subnetworks/default"),
            Some("europe-west1")
        );
    }

    #[test]
    fn test_gcp_region_from_link_no_regions() {
        assert_eq!(
            gcp_region_from_link("projects/my-project/zones/us-central1-a"),
            None
        );
    }

    // ============================================================
    // aws_az_to_region
    // ============================================================

    #[test]
    fn test_aws_az_to_region() {
        assert_eq!(
            aws_az_to_region("us-east-1a"),
            Some("us-east-1".to_string())
        );
        assert_eq!(
            aws_az_to_region("eu-west-2b"),
            Some("eu-west-2".to_string())
        );
        assert_eq!(
            aws_az_to_region("ap-southeast-1c"),
            Some("ap-southeast-1".to_string())
        );
    }

    #[test]
    fn test_aws_az_to_region_empty() {
        assert_eq!(aws_az_to_region(""), None);
    }

    #[test]
    fn test_aws_az_to_region_no_suffix() {
        assert_eq!(aws_az_to_region("us-east-1"), None);
    }

    // ============================================================
    // parse_u64
    // ============================================================

    #[test]
    fn test_parse_u64_number() {
        let val = serde_json::json!(500);
        assert_eq!(parse_u64(&val), Some(500));
    }

    #[test]
    fn test_parse_u64_string() {
        let val = serde_json::json!("500");
        assert_eq!(parse_u64(&val), Some(500));
    }

    #[test]
    fn test_parse_u64_null() {
        let val = serde_json::json!(null);
        assert_eq!(parse_u64(&val), None);
    }

    #[test]
    fn test_parse_u64_invalid_string() {
        let val = serde_json::json!("abc");
        assert_eq!(parse_u64(&val), None);
    }

    #[test]
    fn test_parse_u64_float() {
        let val = serde_json::json!(3.14);
        // serde_json stores 3.14 as a float, as_u64() returns None for floats
        assert_eq!(parse_u64(&val), None);
    }

    #[test]
    fn test_parse_u64_zero() {
        let val = serde_json::json!(0);
        assert_eq!(parse_u64(&val), Some(0));
    }
}
