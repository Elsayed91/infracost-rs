//! Generic pricing engine that executes YAML-defined pricing queries.

use std::collections::HashMap;

use crate::providers::{PriceResult, PriceSource};
use crate::types::ProductFilter;
use crate::{Client, Result};

use super::types::{substitute_params, *};

/// The pricing engine: builds filters from YAML definitions, queries the API,
/// applies post-filters, and computes costs.
pub struct PricingEngine;

impl PricingEngine {
    /// Fetch the primary cost component's unit price.
    pub async fn fetch(
        client: &Client,
        resource: &ResourceDef,
        vendor: &str,
        region: &str,
        api_key: Option<&str>,
        override_default: Option<f64>,
    ) -> Result<PriceResult> {
        let component = resource
            .cost_components
            .iter()
            .find(|c| c.is_primary)
            .or_else(|| resource.cost_components.first())
            .ok_or_else(|| {
                crate::Error::config(format!(
                    "Resource '{}' has no cost components",
                    resource.name
                ))
            })?;

        let default_price = override_default.unwrap_or(component.default_price);

        Self::fetch_component_price(
            client,
            component,
            vendor,
            region,
            api_key,
            default_price,
            None,
        )
        .await
    }

    /// Fetch total monthly cost by summing all cost components.
    pub async fn fetch_monthly(
        client: &Client,
        resource: &ResourceDef,
        vendor: &str,
        region: &str,
        api_key: Option<&str>,
        params: &HashMap<String, u64>,
    ) -> Result<PriceResult> {
        Self::fetch_monthly_with_string_params(
            client, resource, vendor, region, api_key, params, None,
        )
        .await
    }

    /// Fetch total monthly cost with optional string parameters for filter substitution.
    pub async fn fetch_monthly_with_string_params(
        client: &Client,
        resource: &ResourceDef,
        vendor: &str,
        region: &str,
        api_key: Option<&str>,
        params: &HashMap<String, u64>,
        string_params: Option<&HashMap<String, String>>,
    ) -> Result<PriceResult> {
        let mut total = 0.0;
        let mut all_from_api = true;

        for component in &resource.cost_components {
            let result = Self::fetch_component_price(
                client,
                component,
                vendor,
                region,
                api_key,
                component.default_price,
                string_params,
            )
            .await?;

            if !result.is_from_api() {
                all_from_api = false;
            }

            let component_cost = Self::calculate_component_cost(&result, component, params);
            total += component_cost;
        }

        let source = if all_from_api && (client.has_api_key() || api_key.is_some()) {
            PriceSource::Api
        } else {
            PriceSource::Default
        };

        tracing::debug!(
            target: "infracost",
            resource = %resource.name,
            region = %region,
            total_monthly = total,
            source = ?source,
            "Monthly cost calculated"
        );

        Ok(PriceResult {
            price: total,
            unit: "month".to_string(),
            source,
        })
    }

    /// Fetch a single cost component's unit price (public for validation tools).
    pub async fn fetch_component_price(
        client: &Client,
        component: &CostComponentDef,
        vendor: &str,
        region: &str,
        api_key: Option<&str>,
        default_price: f64,
        string_params: Option<&HashMap<String, String>>,
    ) -> Result<PriceResult> {
        let unit = &component.unit;

        // Check if we have an API key
        let has_key = api_key.is_some() || client.has_api_key();
        if !has_key && !client.error_on_fallback() {
            tracing::debug!(
                target: "infracost",
                component = %component.name,
                region = %region,
                price = default_price,
                unit = %unit,
                "No API key — using default price"
            );
            return Ok(PriceResult::from_default(default_price, unit));
        }

        // Build the filter from YAML query definition (with parameter substitution if provided)
        let filter =
            Self::build_filter_with_params(&component.query, vendor, region, string_params);

        // Apply parameter substitution to post_filter and price_filter if string_params provided
        let post_filter = if let (Some(pf), Some(params)) = (&component.post_filter, string_params)
        {
            Some(pf.substitute(params))
        } else {
            component.post_filter.clone()
        };

        let price_filter =
            if let (Some(pf), Some(params)) = (&component.price_filter, string_params) {
                Some(pf.substitute(params))
            } else {
                component.price_filter.clone()
            };

        match client.query_products_with_key(filter, api_key).await {
            Ok(products) if !products.is_empty() => {
                // Apply post-filter if defined
                let selected = if let Some(ref pf) = post_filter {
                    Self::apply_post_filter(&products, pf)
                } else {
                    Some(&products[0])
                };

                let mut price = selected
                    .map(|p| {
                        // Apply price-level filter if defined (e.g., Consumption for Azure)
                        if let Some(ref pf) = price_filter
                            && let Some(ref po) = pf.purchase_option
                        {
                            return p
                                .prices()
                                .purchase_option(po)
                                .first_nonzero_f64_or(default_price);
                        }
                        p.first_nonzero_price_or(default_price)
                    })
                    .unwrap_or(default_price);

                // Apply price transform if defined
                if let Some(ref transform) = component.price_transform {
                    if let Some(divisor) = transform.divide_by {
                        price /= divisor;
                    }
                    if let Some(multiplier) = transform.multiply_by {
                        price *= multiplier;
                    }
                }

                tracing::debug!(
                    target: "infracost",
                    component = %component.name,
                    region = %region,
                    price = price,
                    unit = %unit,
                    products_found = products.len(),
                    "API price resolved"
                );
                Ok(PriceResult::from_api(price, unit))
            }
            Ok(_) if !client.error_on_fallback() => {
                tracing::debug!(
                    target: "infracost",
                    component = %component.name,
                    region = %region,
                    price = default_price,
                    unit = %unit,
                    "API returned no products — using default price"
                );
                Ok(PriceResult::from_default(default_price, unit))
            }
            Err(ref e) if !client.error_on_fallback() => {
                tracing::debug!(
                    target: "infracost",
                    component = %component.name,
                    region = %region,
                    price = default_price,
                    unit = %unit,
                    error = %e,
                    "API error — using default price"
                );
                Ok(PriceResult::from_default(default_price, unit))
            }
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    /// Build a ProductFilter from the YAML query definition.
    fn build_filter(query: &QueryDef, vendor: &str, region: &str) -> ProductFilter {
        Self::build_filter_with_params(query, vendor, region, None)
    }

    /// Build a ProductFilter from the YAML query definition, with optional parameter substitution.
    ///
    /// When `string_params` is provided, `{{param_name}}` placeholders in attribute values
    /// are replaced with the corresponding parameter values.
    fn build_filter_with_params(
        query: &QueryDef,
        vendor: &str,
        region: &str,
        string_params: Option<&HashMap<String, String>>,
    ) -> ProductFilter {
        let mut builder = ProductFilter::builder().vendor(vendor).region(region);

        if let Some(ref service) = query.service {
            builder = builder.service(service);
        }
        if let Some(ref pf) = query.product_family {
            builder = builder.product_family(pf);
        }
        for attr in &query.attributes {
            let value = if let Some(params) = string_params {
                substitute_params(&attr.value, params)
            } else {
                attr.value.clone()
            };
            builder = builder.attribute(&attr.key, &value);
        }
        for regex in &query.attribute_regexes {
            builder = builder.attribute_regex(&regex.key, &regex.pattern);
        }

        builder.build()
    }

    /// Apply post-filter rules to select the right product from API results.
    fn apply_post_filter<'a>(
        products: &'a [crate::types::Product],
        pf: &PostFilterDef,
    ) -> Option<&'a crate::types::Product> {
        products.iter().find(|product| {
            // Check description-based filters
            if pf.description_starts_with.is_some() || !pf.description_excludes.is_empty() {
                let desc = product.attribute("description").unwrap_or("");

                if let Some(ref prefix) = pf.description_starts_with
                    && !desc.starts_with(prefix.as_str())
                {
                    return false;
                }

                for substr in &pf.description_contains {
                    if !desc.contains(substr.as_str()) {
                        return false;
                    }
                }

                for exclude in &pf.description_excludes {
                    if desc.contains(exclude.as_str()) {
                        return false;
                    }
                }
            }

            // Check usagetype-based filters
            if pf.usagetype_ends_with.is_some() || !pf.usagetype_excludes.is_empty() {
                let usage = product.attribute("usagetype").unwrap_or("");

                if let Some(ref suffix) = pf.usagetype_ends_with
                    && !usage.ends_with(suffix.as_str())
                {
                    return false;
                }

                for exclude in &pf.usagetype_excludes {
                    if usage.contains(exclude.as_str()) {
                        return false;
                    }
                }
            }

            true
        })
    }

    /// Calculate a component's monthly cost from its unit price and params.
    fn calculate_component_cost(
        result: &PriceResult,
        component: &CostComponentDef,
        params: &HashMap<String, u64>,
    ) -> f64 {
        let price = result.price;

        match &component.pricing_model {
            PricingModelDef::Linear { quantity_param } => {
                let qty = params.get(quantity_param).copied().unwrap_or(0);
                price * qty as f64
            }

            PricingModelDef::LinearWithBaseline {
                quantity_param,
                baseline,
            } => {
                let qty = params.get(quantity_param).copied().unwrap_or(*baseline);
                let billable = qty.saturating_sub(*baseline);
                price * billable as f64
            }

            PricingModelDef::Tiered {
                quantity_param,
                tiers,
            } => {
                let qty = params.get(quantity_param).copied().unwrap_or(0);
                Self::calculate_tiered_cost(qty, tiers)
            }

            PricingModelDef::HourlyToMonthly => price * 730.0,

            PricingModelDef::Fixed => price,
        }
    }

    /// Calculate cost using tiered pricing.
    fn calculate_tiered_cost(quantity: u64, tiers: &[TierDef]) -> f64 {
        let mut cost = 0.0;
        let mut remaining = quantity;
        let mut prev_limit: u64 = 0;

        for tier in tiers {
            if remaining == 0 {
                break;
            }

            let tier_capacity = match tier.limit {
                Some(limit) => limit.saturating_sub(prev_limit),
                None => remaining, // unlimited tier
            };

            let used = remaining.min(tier_capacity);
            cost += used as f64 * tier.default_price;
            remaining = remaining.saturating_sub(used);

            if let Some(limit) = tier.limit {
                prev_limit = limit;
            }
        }

        cost
    }

    /// Fetch tiered pricing with separate queries per tier.
    /// Used for resources like io2 where each tier has its own API query.
    pub async fn fetch_monthly_with_tiered_queries(
        client: &Client,
        resource: &ResourceDef,
        vendor: &str,
        region: &str,
        api_key: Option<&str>,
        params: &HashMap<String, u64>,
    ) -> Result<PriceResult> {
        let mut total = 0.0;
        let mut all_from_api = true;

        for component in &resource.cost_components {
            match &component.pricing_model {
                PricingModelDef::Tiered {
                    quantity_param,
                    tiers,
                } => {
                    let qty = params.get(quantity_param).copied().unwrap_or(0);
                    let has_key = api_key.is_some() || client.has_api_key();

                    if !has_key && !client.error_on_fallback() {
                        all_from_api = false;
                        total += Self::calculate_tiered_cost(qty, tiers);
                        continue;
                    }

                    // Fetch each tier's price from API
                    let mut remaining = qty;
                    let mut prev_limit: u64 = 0;

                    for tier in tiers {
                        if remaining == 0 {
                            break;
                        }

                        let tier_capacity = match tier.limit {
                            Some(limit) => limit.saturating_sub(prev_limit),
                            None => remaining,
                        };
                        let used = remaining.min(tier_capacity);

                        // Build tier-specific filter
                        let tier_price = if let Some(ref qf) = tier.query_filter {
                            let mut filter_builder =
                                Self::build_filter(&component.query, vendor, region);
                            // Add tier-specific regex filter
                            if let Some(ref attr_regex) = qf.attribute_regex {
                                filter_builder.attribute_filters.push(
                                    crate::types::AttributeFilter::regex(
                                        &attr_regex.key,
                                        &attr_regex.pattern,
                                    ),
                                );
                            }

                            match client
                                .query_products_with_key(filter_builder, api_key)
                                .await
                            {
                                Ok(products) if !products.is_empty() => {
                                    products[0].first_nonzero_price_or(tier.default_price)
                                }
                                _ => {
                                    all_from_api = false;
                                    tier.default_price
                                }
                            }
                        } else {
                            all_from_api = false;
                            tier.default_price
                        };

                        total += used as f64 * tier_price;
                        remaining = remaining.saturating_sub(used);

                        if let Some(limit) = tier.limit {
                            prev_limit = limit;
                        }
                    }
                }
                _ => {
                    // Non-tiered component: use standard fetch
                    let result = Self::fetch_component_price(
                        client,
                        component,
                        vendor,
                        region,
                        api_key,
                        component.default_price,
                        None,
                    )
                    .await?;

                    if !result.is_from_api() {
                        all_from_api = false;
                    }

                    total += Self::calculate_component_cost(&result, component, params);
                }
            }
        }

        let source = if all_from_api && (client.has_api_key() || api_key.is_some()) {
            PriceSource::Api
        } else {
            PriceSource::Default
        };

        tracing::debug!(
            target: "infracost",
            resource = %resource.name,
            region = %region,
            total_monthly = total,
            source = ?source,
            "Monthly cost calculated (tiered)"
        );

        Ok(PriceResult {
            price: total,
            unit: "month".to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiered_cost_calculation() {
        let tiers = vec![
            TierDef {
                limit: Some(32000),
                default_price: 0.065,
                min_price: None,
                max_price: None,
                query_filter: None,
            },
            TierDef {
                limit: Some(64000),
                default_price: 0.0455,
                min_price: None,
                max_price: None,
                query_filter: None,
            },
            TierDef {
                limit: None,
                default_price: 0.03185,
                min_price: None,
                max_price: None,
                query_filter: None,
            },
        ];

        // Tier 1 only: 10,000 IOPS
        let cost = PricingEngine::calculate_tiered_cost(10000, &tiers);
        assert_eq!(cost, 650.0);

        // Tier 1 + Tier 2: 50,000 IOPS
        let cost = PricingEngine::calculate_tiered_cost(50000, &tiers);
        assert_eq!(cost, 2899.0);

        // All 3 tiers: 100,000 IOPS
        let cost = PricingEngine::calculate_tiered_cost(100000, &tiers);
        assert_eq!(cost, 4682.6);

        // Exactly at tier boundary: 32,000 IOPS
        let cost = PricingEngine::calculate_tiered_cost(32000, &tiers);
        assert_eq!(cost, 2080.0);

        // Just over tier 1: 32,001 IOPS
        let cost = PricingEngine::calculate_tiered_cost(32001, &tiers);
        assert_eq!(cost, 2080.0455);
    }

    #[test]
    fn test_component_cost_linear() {
        let component = CostComponentDef {
            name: "storage".to_string(),
            is_primary: true,
            unit: "GB-month".to_string(),
            default_price: 0.08,
            min_price: None,
            max_price: None,
            query: QueryDef {
                service: None,
                product_family: None,
                attributes: vec![],
                attribute_regexes: vec![],
            },
            post_filter: None,
            price_filter: None,
            price_transform: None,
            pricing_model: PricingModelDef::Linear {
                quantity_param: "size_gb".to_string(),
            },
        };

        let result = PriceResult::from_default(0.08, "GB-month");
        let mut params = HashMap::new();
        params.insert("size_gb".to_string(), 500);

        let cost = PricingEngine::calculate_component_cost(&result, &component, &params);
        assert_eq!(cost, 40.0);
    }

    #[test]
    fn test_component_cost_linear_with_baseline() {
        let component = CostComponentDef {
            name: "iops".to_string(),
            is_primary: false,
            unit: "IOPS-month".to_string(),
            default_price: 0.005,
            min_price: None,
            max_price: None,
            query: QueryDef {
                service: None,
                product_family: None,
                attributes: vec![],
                attribute_regexes: vec![],
            },
            post_filter: None,
            price_filter: None,
            price_transform: None,
            pricing_model: PricingModelDef::LinearWithBaseline {
                quantity_param: "iops".to_string(),
                baseline: 3000,
            },
        };

        let result = PriceResult::from_default(0.005, "IOPS-month");
        let mut params = HashMap::new();
        params.insert("iops".to_string(), 6000);

        let cost = PricingEngine::calculate_component_cost(&result, &component, &params);
        // (6000 - 3000) * 0.005 = 15.0
        assert_eq!(cost, 15.0);
    }

    #[test]
    fn test_component_cost_hourly_to_monthly() {
        let component = CostComponentDef {
            name: "uptime".to_string(),
            is_primary: true,
            unit: "hour".to_string(),
            default_price: 0.005,
            min_price: None,
            max_price: None,
            query: QueryDef {
                service: None,
                product_family: None,
                attributes: vec![],
                attribute_regexes: vec![],
            },
            post_filter: None,
            price_filter: None,
            price_transform: None,
            pricing_model: PricingModelDef::HourlyToMonthly,
        };

        let result = PriceResult::from_default(0.005, "hour");
        let params = HashMap::new();

        let cost = PricingEngine::calculate_component_cost(&result, &component, &params);
        assert_eq!(cost, 3.65);
    }
}
