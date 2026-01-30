//! GraphQL types generated via cynic for the Infracost API.
//!
//! This module is private and contains the raw GraphQL types.
//! Users should interact with the types in `crate::types` instead.

// Register the Infracost GraphQL schema
#[cynic::schema("infracost")]
mod schema {}

/// Price from GraphQL response
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema = "infracost", graphql_type = "Price")]
pub struct GqlPrice {
    #[cynic(rename = "USD")]
    pub usd: String,
    pub unit: String,
    pub description: Option<String>,
    #[cynic(rename = "purchaseOption")]
    pub purchase_option: Option<String>,
    #[cynic(rename = "startUsageAmount")]
    pub start_usage_amount: Option<String>,
    #[cynic(rename = "endUsageAmount")]
    pub end_usage_amount: Option<String>,
}

/// Attribute from GraphQL response
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema = "infracost", graphql_type = "Attribute")]
pub struct GqlAttribute {
    pub key: String,
    pub value: Option<String>,
}

/// Product from GraphQL response
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema = "infracost", graphql_type = "Product")]
pub struct GqlProduct {
    #[cynic(rename = "productHash")]
    pub product_hash: String,
    #[cynic(rename = "vendorName")]
    pub vendor_name: String,
    pub service: String,
    #[cynic(rename = "productFamily")]
    pub product_family: Option<String>,
    pub region: Option<String>,
    pub sku: String,
    pub attributes: Option<Vec<Option<GqlAttribute>>>,
    pub prices: Option<Vec<Option<GqlPrice>>>,
}

/// Input filter for attribute queries (GraphQL input type)
#[derive(cynic::InputObject, Debug, Clone)]
#[cynic(schema = "infracost", graphql_type = "AttributeFilter")]
pub struct GqlAttributeFilter {
    pub key: String,
    pub value: Option<String>,
    #[cynic(rename = "value_regex")]
    pub value_regex: Option<String>,
}

/// Input filter for product queries (GraphQL input type)
#[derive(cynic::InputObject, Debug, Clone)]
#[cynic(schema = "infracost", graphql_type = "ProductFilter")]
pub struct GqlProductFilter {
    #[cynic(rename = "vendorName")]
    pub vendor_name: Option<String>,
    pub service: Option<String>,
    #[cynic(rename = "productFamily")]
    pub product_family: Option<String>,
    pub region: Option<String>,
    pub sku: Option<String>,
    #[cynic(rename = "attributeFilters")]
    pub attribute_filters: Option<Vec<GqlAttributeFilter>>,
}

/// Root query for products
#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "infracost",
    graphql_type = "Query",
    variables = "ProductQueryVariables"
)]
pub struct ProductQuery {
    #[arguments(filter: $filter)]
    pub products: Option<Vec<Option<GqlProduct>>>,
}

/// Variables for product queries
#[derive(cynic::QueryVariables, Debug)]
pub struct ProductQueryVariables {
    pub filter: Option<GqlProductFilter>,
}

// Conversion from public types to GraphQL types
impl From<&crate::types::ProductFilter> for GqlProductFilter {
    fn from(filter: &crate::types::ProductFilter) -> Self {
        GqlProductFilter {
            vendor_name: filter.vendor_name.clone(),
            service: filter.service.clone(),
            product_family: filter.product_family.clone(),
            region: filter.region.clone(),
            sku: filter.sku.clone(),
            attribute_filters: if filter.attribute_filters.is_empty() {
                None
            } else {
                Some(
                    filter
                        .attribute_filters
                        .iter()
                        .map(|af| GqlAttributeFilter {
                            key: af.key.clone(),
                            value: af.value.clone(),
                            value_regex: af.value_regex.clone(),
                        })
                        .collect(),
                )
            },
        }
    }
}

impl From<crate::types::ProductFilter> for GqlProductFilter {
    fn from(filter: crate::types::ProductFilter) -> Self {
        (&filter).into()
    }
}

// Conversion from GraphQL types to public types
impl From<GqlProduct> for crate::types::Product {
    fn from(gql: GqlProduct) -> Self {
        crate::types::Product {
            product_hash: gql.product_hash,
            vendor_name: gql.vendor_name,
            service: gql.service,
            product_family: gql.product_family,
            region: gql.region,
            sku: gql.sku,
            attributes: gql
                .attributes
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .map(|a| crate::types::Attribute {
                    key: a.key,
                    value: a.value,
                })
                .collect(),
            prices: gql
                .prices
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .map(|p| crate::types::Price {
                    usd: p.usd,
                    unit: p.unit,
                    description: p.description,
                    purchase_option: p.purchase_option,
                    start_usage_amount: p.start_usage_amount,
                    end_usage_amount: p.end_usage_amount,
                })
                .collect(),
        }
    }
}
