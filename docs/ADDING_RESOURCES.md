# Adding New Provider Resources

Guide for adding new cloud resource pricing to the convenience API.

## 1. Research the Query

**Before writing any code**, find the exact query parameters using the CLI:

```bash
# Find the product
INFRACOST_API_KEY="ico-xxx" cargo run --features cli --bin irs -- \
  query -v <vendor> -r <region> -a 'productName=<name>' --limit 5 -f json

# Refine with additional attributes until you get exactly ONE result
cargo run --features cli --bin irs -- query -v aws -r us-east-1 \
  -a 'volumeApiName=gp3' \
  -a 'servicecode=AmazonEC2' \
  --limit 1 -f json
```

**Key attributes to identify:**
- `productName` - Primary product identifier
- `meterName` / `description` - Distinguishes variants
- `skuName` - Size/tier identifier (Azure)
- `servicecode` - Filters vendor-specific services (AWS)

**Record the default price** from the API response for your defaults.

## 2. Create the Resource Module

Create `src/providers/<vendor>/<resource>.rs`:

```rust
//! <Vendor> <Resource> pricing.

use crate::types::ProductFilter;
use crate::{Client, Result};
use super::super::PriceResult;

// ============================================================
// Defaults (from API research)
// ============================================================

const DEFAULT_PRICE: f64 = 0.05;  // From API query
const UNIT: &str = "GB-month";     // Match API unit

// ============================================================
// Types (if resource has variants)
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    TypeA,
    TypeB,
}

impl ResourceType {
    fn api_value(&self) -> &'static str {
        match self {
            Self::TypeA => "type-a",  // Exact API attribute value
            Self::TypeB => "type-b",
        }
    }

    fn default_price(&self) -> f64 {
        match self {
            Self::TypeA => 0.05,
            Self::TypeB => 0.10,
        }
    }
}

// ============================================================
// Builder
// ============================================================

pub struct ResourceBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    require_api: bool,
}

impl<'a> ResourceBuilder<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            require_api: false,
        }
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    pub fn require_api(mut self) -> Self {
        self.require_api = true;
        self
    }

    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    pub async fn fetch(self) -> Result<PriceResult> {
        let default_price = self.override_default.unwrap_or(DEFAULT_PRICE);

        // Check if we should use defaults
        let effective_key = self.api_key.as_deref().or_else(|| {
            if self.client.has_api_key() { Some("") } else { None }
        });

        if effective_key.is_none() && !self.require_api {
            return Ok(PriceResult::from_default(default_price, UNIT));
        }

        // Query API
        let filter = self.build_filter();
        match self.client.query_products_with_key(filter, self.api_key.as_deref()).await {
            Ok(products) if !products.is_empty() => {
                let price = products[0].first_nonzero_price_or(default_price);
                Ok(PriceResult::from_api(price, UNIT))
            }
            Ok(_) if !self.require_api => Ok(PriceResult::from_default(default_price, UNIT)),
            Err(_) if !self.require_api => Ok(PriceResult::from_default(default_price, UNIT)),
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    fn build_filter(&self) -> ProductFilter {
        ProductFilter::builder()
            .vendor("<vendor>")
            .region(self.region.as_deref().unwrap_or("<default-region>"))
            .attribute("<key>", "<value>")  // From research
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client.<vendor>().<resource>().region("<region>").fetch().await.unwrap();
        assert!(result.is_from_default());
        assert_eq!(result.price, DEFAULT_PRICE);
        assert_eq!(result.unit, UNIT);
    }
}
```

## 3. Wire It Up

**In `src/providers/<vendor>/mod.rs`:**
```rust
mod resource;
pub use resource::ResourceBuilder;

impl<'a> VendorProvider<'a> {
    pub fn resource(self) -> ResourceBuilder<'a> {
        ResourceBuilder::new(self.client)
    }
}
```

## 4. Add Integration Test

**In `tests/integration.rs`:**
```rust
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_<vendor>_<resource>_provider() {
    let client = get_client().expect("INFRACOST_API_KEY must be set");
    let result = client.<vendor>().<resource>().region("<region>").fetch().await.expect("Query should succeed");

    assert!(result.is_from_api());
    assert!(result.price > 0.0);
    assert_eq!(result.unit, "<unit>");
}
```

## 5. Verify

```bash
# Unit tests (no API key needed)
cargo test

# Integration test
INFRACOST_API_KEY="ico-xxx" cargo test --test integration test_<vendor>_<resource> -- --ignored

# Manual verification
cargo run --example <vendor>_pricing
```

## Common Pitfalls

| Issue | Solution |
|-------|----------|
| Query returns multiple products | Add more attribute filters until exactly 1 result |
| Wrong price (e.g., reserved vs on-demand) | Filter by purchase_option: `products[0].prices().purchase_option("Consumption").first_nonzero_f64_or(default)` |
| Empty results | Check attribute names are exact (case-sensitive) |
| Price is $0 | Use `first_nonzero_price_or()` to skip free tiers |

## Checklist

- [ ] CLI query returns exactly 1 product
- [ ] Default price matches API response
- [ ] Unit matches API response (e.g., "GB-month", "hour", "month")
- [ ] Unit test passes without API key
- [ ] Integration test passes with API key
- [ ] Example updated and runs correctly
