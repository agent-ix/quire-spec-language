// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine source controls for independent quantity-name lookup domains.

use ix_trace_rs::trace;
use qsl_replay::{compile_admitted, DependencyInput, ReplayLimits, SourceIdentity, StageLimits};
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use serde_json::Value;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
const UNITS: &str = "dimension Length;\nunit m : Length = rational(1, 1);\n";

fn assert_admitted(source: &str) {
    let compiled = compile_admitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "quantity-names.native",
        source.as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .unwrap_or_else(|error| panic!("QSL admission failed for {source}: {error:?}"));
    let wire: Value = serde_json::from_slice(compiled.package().bytes()).unwrap();
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let read = CheckedPackageV2::read(
        compiled.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    let CheckedPackageV2ReadResult::Admitted(package) = read else {
        panic!("strict IR admission failed for {source}: {read:?}");
    };
    assert_eq!(
        package.graph().nodes.len(),
        wire["semantic_graph"]["nodes"].as_array().unwrap().len()
    );
}

#[trace("TC-483", "FR-091-AC-44")]
#[test]
fn unit_names_can_match_fields_parameters_cases_variants_and_aliases() {
    for body in [
        "record Reading { m: Boolean; }\nfunction f using v(a: Quantity<m>, r: Reading): Boolean pure { r.m }",
        "function f using v(m: Quantity<m>): Boolean pure { m = m }",
        "enum Status { m, Other }\nfunction f using v(a: Quantity<m>, s: Status): Boolean pure { s = Status::m }",
        "union Signal { m, Other }\nfunction f using v(a: Quantity<m>, s: Signal): Boolean pure { s = s }",
        "type m = Boolean;\nfunction f using v(a: Quantity<m>, b: m): Boolean pure { b }",
    ] {
        assert_admitted(&format!("{HEADER}{UNITS}{body}\n"));
    }
}

#[trace("TC-483", "FR-091-AC-44")]
#[test]
fn distinct_and_case_distinct_type_names_pass_strict_v2() {
    for name in ["Reading", "M"] {
        for reverse in [false, true] {
            let record = format!("record {name} {{ sample: Boolean; }}");
            let declarations = if reverse {
                format!("{record}\n{UNITS}")
            } else {
                format!("{UNITS}{record}")
            };
            assert_admitted(&format!("{HEADER}{declarations}\nfunction f using v(a: Quantity<m>, r: {name}): Boolean pure {{ r.sample }}\n"));
        }
    }
}
