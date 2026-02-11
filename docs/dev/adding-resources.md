# Adding a New Resource

## Files You Touch

```
resources/{vendor}/{resource}.yaml     <- pricing definition
src/providers/{vendor}/{resource}.rs   <- async builder
src/providers/{vendor}/mod.rs          <- wire it in
src/blocking/{vendor}.rs               <- sync builder mirror
src/catalog/mod.rs                     <- register yaml
tests/{vendor}_{resource}_regional.rs  <- integration tests
```

## The Order

1. **Research** - use `irs` CLI to find query attributes that work across 7+ regions
2. **YAML** - define the resource pricing
3. **Catalog** - register the YAML in `src/catalog/mod.rs`
4. **Builder** - create the Rust builder (copy closest existing one)
5. **Wire** - add to `mod.rs` + blocking wrapper
6. **Test** - unit tests in builder, integration tests in `tests/`

## 1. Research

```bash
cargo build --features cli --bin irs

# explore
irs query -v gcp -r us-central1 --service "Compute Engine" --limit 20

# narrow down
irs query -v gcp -r us-central1 -a 'resourceGroup=SSD' --limit 5 -f json

# validate across regions (CRITICAL)
for r in us-central1 europe-west1 asia-southeast1; do
  irs query -v gcp -r "$r" -a 'resourceGroup=SSD' --limit 1 -f json
done
```

Avoid AWS `usagetype` - has regional prefixes. Use `group`, `productFamily`, `volumeApiName` instead.

## 2. YAML

Simple (hourly):
```yaml
- name: static-ip
  default_region: us-central1
  cost_components:
    - name: uptime
      is_primary: true
      unit: hour
      default_price: 0.01
      query:
        service: "Compute Engine"
        product_family: Network
        attributes:
          - { key: resourceGroup, value: IpAddress }
      pricing_model:
        type: hourly_to_monthly
```

With params (storage * size):
```yaml
- name: snapshot
  parameters:
    - { name: size_gb, required_for_monthly: true }
  cost_components:
    - name: storage
      is_primary: true
      unit: GB-month
      default_price: 0.05
      query: { ... }
      pricing_model:
        type: linear
        quantity_param: size_gb
```

Multi-component (uptime + data):
```yaml
- name: nat-gateway
  parameters:
    - { name: data_processed_gb, required_for_monthly: true }
  cost_components:
    - name: uptime
      is_primary: true
      unit: hour
      default_price: 0.0014
      query: { ... }
      pricing_model:
        type: hourly_to_monthly
    - name: data_processing
      is_primary: false
      unit: GiB
      default_price: 0.045
      query: { ... }
      pricing_model:
        type: linear
        quantity_param: data_processed_gb
```

Pricing models: `hourly_to_monthly`, `linear`, `linear_with_baseline`, `tiered`, `fixed`

## 3. Register in Catalog

`src/catalog/mod.rs` - add `include_str!` + test assertion:
```rust
include_str!("../../resources/gcp/your-resource.yaml"),
```

## 4. Builder

Copy `src/providers/gcp/static_ip.rs` (simplest) or `disk.rs` (with params/variants).

Key points:
- Constructor is `pub(crate)` not `pub`
- `fetch()` calls `PricingEngine::fetch()`
- `fetch_monthly()` builds HashMap of params, calls `PricingEngine::fetch_monthly()`
- Unit tests at the bottom

## 5. Wire

`src/providers/{vendor}/mod.rs`:
```rust
mod your_resource;
pub use your_resource::YourResourceBuilder;
// add method to provider impl block
```

`src/blocking/{vendor}.rs`: mirror the async builder using `self.runtime.block_on(...)`.

## 6. Test

```bash
cargo test                        # unit tests
cargo clippy --all-features       # 0 warnings
INFRACOST_API_KEY=xxx cargo test --test your_test -- --ignored  # integration
```

## Checklist

- [ ] IRS query works across 7+ regions
- [ ] YAML parses (`cargo check`)
- [ ] Catalog test passes
- [ ] Unit tests pass without API key
- [ ] Integration tests pass with API key
- [ ] Blocking wrapper exists
- [ ] `cargo clippy --all-features` clean

AI-automated workflow: `/add-resource` skill (`.claude/skills/add-resource/SKILL.md`)
