//! YAML catalog types for declarative pricing definitions.

use serde::Deserialize;

/// Root of a vendor's resource catalog, assembled from per-resource YAML files.
#[derive(Debug)]
pub struct ResourceCatalog {
    pub vendor: String,
    pub resources: Vec<ResourceDef>,
}

impl ResourceCatalog {
    /// Build a catalog by merging multiple YAML fragments.
    ///
    /// Each fragment is a `Vec<ResourceDef>` (a YAML list of resources).
    pub fn from_parts(vendor: impl Into<String>, yamls: &[&str]) -> Self {
        let vendor = vendor.into();
        let mut resources = Vec::new();
        for yaml in yamls {
            let defs: Vec<ResourceDef> = serde_yml::from_str(yaml)
                .unwrap_or_else(|e| panic!("{vendor} YAML parse error: {e}"));
            resources.extend(defs);
        }
        Self { vendor, resources }
    }

    /// Find a resource definition by name.
    pub fn find(&self, name: &str) -> crate::Result<&ResourceDef> {
        self.resources
            .iter()
            .find(|r| r.name == name)
            .ok_or_else(|| {
                crate::Error::config(format!(
                    "Resource '{}' not found in {} catalog",
                    name, self.vendor
                ))
            })
    }
}

/// A single resource definition (e.g., "disk/pd-ssd").
#[derive(Debug, Deserialize)]
pub struct ResourceDef {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default = "default_region_gcp")]
    pub default_region: String,
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
    pub cost_components: Vec<CostComponentDef>,
}

fn default_region_gcp() -> String {
    "us-central1".to_string()
}

/// Parameter definition for a resource.
#[derive(Debug, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    #[serde(default)]
    pub required_for_monthly: bool,
}

/// A single cost component within a resource.
#[derive(Debug, Deserialize)]
pub struct CostComponentDef {
    pub name: String,
    #[serde(default)]
    pub is_primary: bool,
    pub unit: String,
    pub default_price: f64,
    #[serde(default)]
    pub min_price: Option<f64>,
    #[serde(default)]
    pub max_price: Option<f64>,
    pub query: QueryDef,
    #[serde(default)]
    pub post_filter: Option<PostFilterDef>,
    #[serde(default)]
    pub price_filter: Option<PriceFilterDef>,
    #[serde(default)]
    pub price_transform: Option<PriceTransformDef>,
    pub pricing_model: PricingModelDef,
}

/// Query definition for building ProductFilter.
#[derive(Debug, Deserialize)]
pub struct QueryDef {
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub product_family: Option<String>,
    #[serde(default)]
    pub attributes: Vec<AttributeDef>,
    #[serde(default)]
    pub attribute_regexes: Vec<AttributeRegexDef>,
}

/// Exact-match attribute for queries.
#[derive(Debug, Deserialize)]
pub struct AttributeDef {
    pub key: String,
    pub value: String,
}

/// Regex-match attribute for queries.
#[derive(Debug, Deserialize)]
pub struct AttributeRegexDef {
    pub key: String,
    pub pattern: String,
}

/// Post-query filtering rules applied to API results.
#[derive(Debug, Deserialize)]
pub struct PostFilterDef {
    #[serde(default)]
    pub description_starts_with: Option<String>,
    #[serde(default)]
    pub description_contains: Vec<String>,
    #[serde(default)]
    pub description_excludes: Vec<String>,
    #[serde(default)]
    pub usagetype_ends_with: Option<String>,
    #[serde(default)]
    pub usagetype_excludes: Vec<String>,
}

/// Price-level filtering (e.g., Consumption vs Reservation for Azure).
#[derive(Debug, Deserialize)]
pub struct PriceFilterDef {
    #[serde(default)]
    pub purchase_option: Option<String>,
}

/// Transform applied to the price after extraction.
#[derive(Debug, Deserialize)]
pub struct PriceTransformDef {
    #[serde(default)]
    pub divide_by: Option<f64>,
    #[serde(default)]
    pub multiply_by: Option<f64>,
}

/// How a cost component's price maps to monthly cost.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PricingModelDef {
    /// Monthly cost = price * params[quantity_param]
    #[serde(rename = "linear")]
    Linear { quantity_param: String },

    /// Monthly cost = price * max(0, params[quantity_param] - baseline)
    #[serde(rename = "linear_with_baseline")]
    LinearWithBaseline {
        quantity_param: String,
        baseline: u64,
    },

    /// Monthly cost = sum of tier calculations
    #[serde(rename = "tiered")]
    Tiered {
        quantity_param: String,
        tiers: Vec<TierDef>,
    },

    /// Monthly cost = price * 730
    #[serde(rename = "hourly_to_monthly")]
    HourlyToMonthly,

    /// Monthly cost = price (no multiplication)
    #[serde(rename = "fixed")]
    Fixed,
}

/// A single tier in tiered pricing.
#[derive(Debug, Deserialize)]
pub struct TierDef {
    /// Upper limit of this tier (None = unlimited).
    pub limit: Option<u64>,
    pub default_price: f64,
    #[serde(default)]
    pub min_price: Option<f64>,
    #[serde(default)]
    pub max_price: Option<f64>,
    #[serde(default)]
    pub query_filter: Option<TierQueryFilterDef>,
}

/// Additional query filters specific to a tier.
#[derive(Debug, Deserialize)]
pub struct TierQueryFilterDef {
    #[serde(default)]
    pub attribute_regex: Option<AttributeRegexDef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_resource() {
        let yaml = r#"
- name: static-ip
  default_region: us-central1
  cost_components:
    - name: uptime
      is_primary: true
      unit: hour
      default_price: 0.01
      query:
        service: "Compute Engine"
        product_family: "Network"
        attributes:
          - { key: resourceGroup, value: IpAddress }
      pricing_model:
        type: hourly_to_monthly
"#;
        let resources: Vec<ResourceDef> = serde_yml::from_str(yaml).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "static-ip");
        assert_eq!(resources[0].cost_components[0].default_price, 0.01);
    }

    #[test]
    fn test_parse_tiered_pricing() {
        let yaml = r#"
- name: ebs/io2
  default_region: us-east-1
  cost_components:
    - name: iops
      unit: IOPS-month
      default_price: 0.065
      query:
        attributes:
          - { key: group, value: "EBS IOPS" }
          - { key: volumeApiName, value: io2 }
      pricing_model:
        type: tiered
        quantity_param: iops
        tiers:
          - { limit: 32000, default_price: 0.065 }
          - { limit: 64000, default_price: 0.0455 }
          - { limit: null, default_price: 0.03185 }
"#;
        let resources: Vec<ResourceDef> = serde_yml::from_str(yaml).unwrap();
        let component = &resources[0].cost_components[0];
        match &component.pricing_model {
            PricingModelDef::Tiered { tiers, .. } => {
                assert_eq!(tiers.len(), 3);
                assert_eq!(tiers[0].limit, Some(32000));
                assert_eq!(tiers[2].limit, None);
            }
            _ => panic!("Expected tiered pricing model"),
        }
    }

    #[test]
    fn test_parse_linear_with_baseline() {
        let yaml = r#"
- name: ebs/gp3
  default_region: us-east-1
  cost_components:
    - name: iops
      unit: IOPS-month
      default_price: 0.005
      query:
        attributes:
          - { key: group, value: "EBS IOPS" }
      pricing_model:
        type: linear_with_baseline
        quantity_param: iops
        baseline: 3000
"#;
        let resources: Vec<ResourceDef> = serde_yml::from_str(yaml).unwrap();
        let component = &resources[0].cost_components[0];
        match &component.pricing_model {
            PricingModelDef::LinearWithBaseline { baseline, .. } => {
                assert_eq!(*baseline, 3000);
            }
            _ => panic!("Expected linear_with_baseline pricing model"),
        }
    }

    #[test]
    fn test_parse_post_filter() {
        let yaml = r#"
- name: disk/pd-ssd
  default_region: us-central1
  cost_components:
    - name: storage
      is_primary: true
      unit: GiB-month
      default_price: 0.17
      query:
        service: "Compute Engine"
        product_family: Storage
        attributes:
          - { key: resourceGroup, value: SSD }
      post_filter:
        description_starts_with: "SSD backed PD Capacity"
        description_excludes: ["Confidential Mode", "High Availability", "Storage Pools"]
      pricing_model:
        type: linear
        quantity_param: size_gb
"#;
        let resources: Vec<ResourceDef> = serde_yml::from_str(yaml).unwrap();
        let pf = resources[0].cost_components[0]
            .post_filter
            .as_ref()
            .unwrap();
        assert_eq!(
            pf.description_starts_with.as_deref(),
            Some("SSD backed PD Capacity")
        );
        assert_eq!(pf.description_excludes.len(), 3);
    }

    #[test]
    fn test_from_parts() {
        let cat = ResourceCatalog::from_parts(
            "test",
            &[
                "- name: foo\n  cost_components:\n    - name: x\n      is_primary: true\n      unit: hr\n      default_price: 1.0\n      query: {}\n      pricing_model: { type: fixed }\n",
                "- name: bar\n  cost_components:\n    - name: y\n      is_primary: true\n      unit: mo\n      default_price: 2.0\n      query: {}\n      pricing_model: { type: fixed }\n",
            ],
        );
        assert_eq!(cat.vendor, "test");
        assert_eq!(cat.resources.len(), 2);
        assert!(cat.find("foo").is_ok());
        assert!(cat.find("bar").is_ok());
    }
}
