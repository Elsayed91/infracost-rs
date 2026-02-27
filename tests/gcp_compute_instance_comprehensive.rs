//! Comprehensive integration tests for GCP Compute Instance pricing.
//!
//! Tests ALL major machine families across regions and purchase options
//! to validate 95%+ success rate.

use infracost_rs::{Client, providers::gcp::PurchaseOption};
use std::env;

fn get_test_client() -> Client {
    let api_key = env::var("INFRACOST_API_KEY")
        .expect("INFRACOST_API_KEY must be set to run integration tests");
    Client::new(api_key).unwrap()
}

/// Test configuration for a machine family
#[allow(dead_code)]
struct FamilyTest {
    family: &'static str,
    machine_type: &'static str,
    expected_cores: u64,
    expected_ram_gib: u64,
}

/// Get all major GCP machine families to test
fn get_test_families() -> Vec<FamilyTest> {
    vec![
        // N2 (2nd Gen Intel) - General purpose
        FamilyTest {
            family: "N2",
            machine_type: "n2-standard-4",
            expected_cores: 4,
            expected_ram_gib: 16,
        },
        // N2D (AMD) - General purpose
        FamilyTest {
            family: "N2D",
            machine_type: "n2d-standard-4",
            expected_cores: 4,
            expected_ram_gib: 16,
        },
        // E2 (Cost-optimized) - General purpose
        FamilyTest {
            family: "E2",
            machine_type: "e2-standard-4",
            expected_cores: 4,
            expected_ram_gib: 16,
        },
        // C2 (Compute-optimized)
        FamilyTest {
            family: "C2",
            machine_type: "c2-standard-4",
            expected_cores: 4,
            expected_ram_gib: 16,
        },
        // C2D (AMD Compute-optimized)
        FamilyTest {
            family: "C2D",
            machine_type: "c2d-standard-4",
            expected_cores: 4,
            expected_ram_gib: 16,
        },
        // M1 (Memory-optimized)
        FamilyTest {
            family: "M1",
            machine_type: "m1-ultramem-40",
            expected_cores: 40,
            expected_ram_gib: 961,
        },
        // M2 (Memory-optimized)
        FamilyTest {
            family: "M2",
            machine_type: "m2-ultramem-208",
            expected_cores: 208,
            expected_ram_gib: 5888,
        },
        // M3 (Memory-optimized)
        FamilyTest {
            family: "M3",
            machine_type: "m3-ultramem-32",
            expected_cores: 32,
            expected_ram_gib: 976,
        },
        // C3 (Compute-optimized, latest gen)
        FamilyTest {
            family: "C3",
            machine_type: "c3-standard-4",
            expected_cores: 4,
            expected_ram_gib: 16,
        },
        // N1 (1st Gen Intel) - Legacy but widely used
        FamilyTest {
            family: "N1",
            machine_type: "n1-standard-4",
            expected_cores: 4,
            expected_ram_gib: 15, // N1 has 15 GiB not 16
        },
    ]
}

/// Test regions to validate
fn get_test_regions() -> Vec<&'static str> {
    vec![
        "us-central1",
        "us-east1",
        "europe-west1",
        "europe-north1",
        "asia-southeast1",
        "australia-southeast1",
        "southamerica-east1",
    ]
}

#[derive(Debug)]
#[allow(dead_code)]
struct TestResult {
    family: String,
    region: String,
    purchase_option: String,
    success: bool,
    price: Option<f64>,
    error: Option<String>,
}

#[tokio::test]
#[ignore] // Run with: cargo test --test gcp_compute_instance_comprehensive -- --ignored
async fn test_all_families_all_regions_all_purchase_options() {
    let client = get_test_client();
    let families = get_test_families();
    let regions = get_test_regions();
    let purchase_options = vec![
        ("OnDemand", PurchaseOption::OnDemand),
        ("Preemptible", PurchaseOption::Preemptible),
    ];

    let mut results = Vec::new();
    let mut total_tests = 0;
    let mut successful_tests = 0;

    println!("\n========================================");
    println!("COMPREHENSIVE GCP COMPUTE INSTANCE TEST");
    println!("========================================\n");
    println!(
        "Testing {} families × {} regions × {} purchase options = {} total combinations\n",
        families.len(),
        regions.len(),
        purchase_options.len(),
        families.len() * regions.len() * purchase_options.len()
    );

    for family_test in &families {
        println!(
            "--- Testing {} ({}) ---",
            family_test.family, family_test.machine_type
        );

        for region in &regions {
            for (po_name, po_value) in &purchase_options {
                total_tests += 1;

                let result = client
                    .gcp()
                    .compute_instance()
                    .machine_type(family_test.machine_type)
                    .region(*region)
                    .purchase_option(*po_value)
                    .fetch_monthly()
                    .await;

                match result {
                    Ok(price_result) => {
                        successful_tests += 1;
                        print!(
                            "  ✓ [{:20}] {:12} ${:8.2}/mo",
                            region, po_name, price_result.price
                        );

                        // Validate price is reasonable
                        if price_result.price > 0.0 && price_result.price < 100000.0 {
                            println!(" (source: {:?})", price_result.source);
                        } else {
                            println!(" ⚠ SUSPICIOUS PRICE");
                        }

                        results.push(TestResult {
                            family: family_test.family.to_string(),
                            region: region.to_string(),
                            purchase_option: po_name.to_string(),
                            success: true,
                            price: Some(price_result.price),
                            error: None,
                        });
                    }
                    Err(e) => {
                        println!("  ✗ [{:20}] {:12} Error: {}", region, po_name, e);
                        results.push(TestResult {
                            family: family_test.family.to_string(),
                            region: region.to_string(),
                            purchase_option: po_name.to_string(),
                            success: false,
                            price: None,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }
        println!();
    }

    // Calculate success rate
    let success_rate = (successful_tests as f64 / total_tests as f64) * 100.0;

    println!("========================================");
    println!("RESULTS SUMMARY");
    println!("========================================");
    println!("Total tests:      {}", total_tests);
    println!("Successful:       {} ✓", successful_tests);
    println!("Failed:           {} ✗", total_tests - successful_tests);
    println!("Success rate:     {:.1}%", success_rate);
    println!("========================================\n");

    // Show failed combinations if any
    let failures: Vec<_> = results.iter().filter(|r| !r.success).collect();
    if !failures.is_empty() {
        println!("FAILED COMBINATIONS:");
        for failure in &failures {
            println!(
                "  • {} / {} / {} - {}",
                failure.family,
                failure.region,
                failure.purchase_option,
                failure
                    .error
                    .as_ref()
                    .unwrap_or(&"Unknown error".to_string())
            );
        }
        println!();
    }

    // Analyze failure patterns
    if !failures.is_empty() {
        println!("FAILURE ANALYSIS:");

        // Group by family
        let mut family_failures = std::collections::HashMap::new();
        for failure in &failures {
            *family_failures.entry(&failure.family).or_insert(0) += 1;
        }
        println!("  By family:");
        for (family, count) in &family_failures {
            println!("    {} - {} failures", family, count);
        }

        // Group by region
        let mut region_failures = std::collections::HashMap::new();
        for failure in &failures {
            *region_failures.entry(&failure.region).or_insert(0) += 1;
        }
        println!("  By region:");
        for (region, count) in &region_failures {
            println!("    {} - {} failures", region, count);
        }

        // Group by purchase option
        let mut po_failures = std::collections::HashMap::new();
        for failure in &failures {
            *po_failures.entry(&failure.purchase_option).or_insert(0) += 1;
        }
        println!("  By purchase option:");
        for (po, count) in &po_failures {
            println!("    {} - {} failures", po, count);
        }
    }

    // Assert 95% success rate
    assert!(
        success_rate >= 95.0,
        "Success rate {:.1}% is below 95% threshold. {} out of {} tests failed.",
        success_rate,
        total_tests - successful_tests,
        total_tests
    );

    println!(
        "\n✅ SUCCESS: Achieved {:.1}% success rate (target: 95%)",
        success_rate
    );
}

#[tokio::test]
#[ignore]
async fn test_spot_discount_validation() {
    let client = get_test_client();
    let families = vec![
        ("n2-standard-4", "N2"),
        ("e2-standard-4", "E2"),
        ("c2-standard-4", "C2"),
    ];

    println!("\n========================================");
    println!("SPOT PRICING DISCOUNT VALIDATION");
    println!("========================================\n");

    let mut all_discounts_valid = true;

    for (machine_type, family_name) in families {
        let ondemand = client
            .gcp()
            .compute_instance()
            .machine_type(machine_type)
            .region("us-central1")
            .fetch_monthly()
            .await
            .unwrap();

        let spot = client
            .gcp()
            .compute_instance()
            .machine_type(machine_type)
            .region("us-central1")
            .purchase_option(PurchaseOption::Preemptible)
            .fetch_monthly()
            .await
            .unwrap();

        let discount_pct = (1.0 - spot.price / ondemand.price) * 100.0;

        println!(
            "{:15} - OnDemand: ${:7.2} | Spot: ${:6.2} | Discount: {:.1}%",
            family_name, ondemand.price, spot.price, discount_pct
        );

        // Spot should be 50-80% cheaper
        if !(50.0..=90.0).contains(&discount_pct) {
            println!("  ⚠ WARNING: Unusual discount percentage");
            all_discounts_valid = false;
        }

        assert!(
            spot.price < ondemand.price,
            "Spot price should be less than on-demand"
        );
    }

    println!();
    assert!(
        all_discounts_valid,
        "Some spot discounts are outside expected range (50-90%)"
    );
    println!("✅ All spot discounts are within expected range");
}

#[tokio::test]
#[ignore]
async fn test_custom_machine_types() {
    let client = get_test_client();

    println!("\n========================================");
    println!("CUSTOM MACHINE TYPE VALIDATION");
    println!("========================================\n");

    // Test various custom configurations
    let custom_types = vec![
        ("n2-custom-2-4096", "N2 custom: 2 cores, 4 GiB"),
        ("n2-custom-4-8192", "N2 custom: 4 cores, 8 GiB"),
        ("e2-custom-4-16384", "E2 custom: 4 cores, 16 GiB"),
        ("c2d-custom-8-32768", "C2D custom: 8 cores, 32 GiB"),
    ];

    for (machine_type, description) in custom_types {
        let result = client
            .gcp()
            .compute_instance()
            .machine_type(machine_type)
            .region("us-central1")
            .fetch_monthly()
            .await;

        match result {
            Ok(price) => {
                println!("✓ {:40} ${:7.2}/mo", description, price.price);
                assert!(price.price > 0.0);
            }
            Err(e) => {
                println!("✗ {:40} Error: {}", description, e);
                panic!("Custom machine type {} failed: {}", machine_type, e);
            }
        }
    }

    println!("\n✅ All custom machine types work correctly");
}
