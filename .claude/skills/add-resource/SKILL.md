---
name: add-resource
description: Add a new cloud resource pricing to infracost-rs. Dispatches specialized agents for research, implementation, review, and testing.
user-invocable: true
---

Add a new cloud resource pricing to infracost-rs.

**Trigger:** User asks to add/implement/price a new cloud resource (e.g., "add GCP backend services", "price AWS Lambda", "implement Azure VMs")

## Arguments

Parse from user input ($ARGUMENTS):
- **VENDOR**: aws, gcp, or azure
- **RESOURCE_NAME**: the resource to add (e.g., backend-service, compute-instance, lambda)

If unclear, ask: "Which vendor and resource? e.g., `gcp backend-service`"

## Phase 1: Research

Dispatch the **irs-researcher** sub-agent via Task tool (`subagent_type: "irs-researcher"`):

```
Research pricing for {VENDOR} {RESOURCE_NAME}.

VENDOR: {vendor}
RESOURCE: {resource_name}

Find:
1. The exact IRS query attributes that return this resource's pricing
2. Test across these regions:
   - AWS: us-east-1, us-west-2, eu-west-1, eu-central-1, ap-southeast-1, ap-northeast-1, sa-east-1
   - GCP: us-central1, us-east1, europe-west1, europe-north1, asia-southeast1, australia-southeast1, southamerica-east1
   - Azure: eastus, westus2, westeurope, northeurope, southeastasia, japaneast, brazilsouth
3. All cost components (uptime, data processing, storage, etc.)
4. Default prices from the API
5. The pricing unit for each component
6. Whether post-filters are needed
7. Which pricing model applies

Output in the structured format from your instructions.
```

**Gate:** Before proceeding, validate:
- Tested ALL 7 regions? If not, send back.
- Attributes universal (no regional prefixes)? AWS `usagetype` used directly = reject.
- Query returns exactly 1 product? If multiple, need post_filter.
- Default prices documented?

## Phase 2: Implement

Dispatch **resource-implementer** (`subagent_type: "resource-implementer"`, `model: "opus"`):

```
Implement {VENDOR} {RESOURCE_NAME} pricing.

RESEARCH RESULTS:
{paste complete research output from Phase 1}

VENDOR: {vendor}
RESOURCE_NAME: {resource-name in kebab-case}
RUST_MODULE: {resource_name in snake_case}

Read existing examples before writing:
- Simple: src/providers/gcp/static_ip.rs + resources/gcp/static-ip.yaml
- With params: src/providers/gcp/snapshot.rs + resources/gcp/snapshot.yaml
- Multi-component: src/providers/gcp/forwarding_rule.rs + resources/gcp/forwarding-rule.yaml
- With variants: src/providers/gcp/disk.rs + resources/gcp/disk.yaml

Create all required files:
1. resources/{vendor}/{resource}.yaml
2. Register in src/catalog/mod.rs
3. src/providers/{vendor}/{module}.rs (async builder + unit tests)
4. Wire into src/providers/{vendor}/mod.rs
5. Add blocking wrapper in src/blocking/{vendor}.rs

Run `cargo check` after each phase. Run `cargo test` at the end.
```

## Phase 3: Review

Dispatch **resource-reviewer** (`subagent_type: "resource-reviewer"`):

```
Review the implementation of {VENDOR} {RESOURCE_NAME}.

Check all files that were created/modified:
- resources/{vendor}/{resource}.yaml
- src/providers/{vendor}/{module}.rs
- src/providers/{vendor}/mod.rs
- src/catalog/mod.rs
- src/blocking/{vendor}.rs

Research results for cross-reference:
{paste research output}

Run cargo check, cargo test, cargo clippy --all-features.
Report PASS or FAIL with specific issues.
```

**Gate:**
- PASS: proceed to Phase 4
- FAIL: dispatch `resource-implementer` again with the issues, then re-review
- Max 3 fix-review cycles. After 3, stop and ask the user.

## Phase 4: Test

Dispatch **resource-tester** (`subagent_type: "resource-tester"`):

```
Create integration tests for {VENDOR} {RESOURCE_NAME}.

Read an existing test file first as template:
- tests/gcp_static_ip_regional_pricing.rs (simple)
- tests/gcp_forwarding_rule_regional_pricing.rs (multi-component)

Create tests/{vendor}_{resource}_regional_pricing.rs with:
1. Per-region comparison tests (7 regions) - convenience vs raw ProductFilter
2. Source tracking test - verify PriceSource::Api
3. Monthly conversion test

Run `cargo test` to verify unit tests still pass.
List the test functions created.
```

## Phase 5: Final Verification

Run in the main session:

```bash
cargo test
cargo clippy --all-features
```

## Report

Summarize: what was added, files created/modified, test results, how to run integration tests.

## Rules

1. **Always dispatch agents via Task tool** - do not implement directly
2. **Always validate between phases** - don't blindly pass output forward
3. **Stop and ask the user** if research fails, review keeps failing, or requirements are unclear
4. **Models:** irs-researcher=sonnet, resource-implementer=opus, resource-reviewer=sonnet, resource-tester=sonnet
