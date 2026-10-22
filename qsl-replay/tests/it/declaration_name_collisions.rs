// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-level quantity declaration names are unambiguous before emission.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, AdmittedCompilation, AdmittedRefusal, Code, CompileRefusal, DependencyInput,
    ReplayLimits, SourceIdentity, StageLimits,
};
use qsl_semantics::check::AssemblyCause;
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use serde_json::Value;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn compile(source: &str) -> Result<AdmittedCompilation, AdmittedRefusal> {
    compile_admitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "names.native",
        source.as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
}

#[trace("TC-483", "FR-091-AC-44")]
#[test]
fn unit_record_and_unit_dimension_collisions_are_typed_before_emission() {
    for other in ["record m { sample: Boolean }", "dimension m;"] {
        for reverse in [false, true] {
            for used in [false, true] {
                let unit = "unit m : Length = rational(1, 1);";
                let declarations = if reverse {
                    format!("{unit}\n{other}")
                } else {
                    format!("{other}\n{unit}")
                };
                let function = if used {
                    "function f using v(a: Quantity<m>): Boolean pure { true }"
                } else {
                    ""
                };
                let source = format!("{HEADER}dimension Length;\n{declarations}\n{function}\n");
                let error = compile(&source).expect_err("ambiguous declarations must not emit");
                let AdmittedRefusal::Package(error) = error else {
                    panic!("expected package refusal: {error:?}");
                };
                let CompileRefusal::Assembly { refusal, .. } = *error else {
                    panic!("expected assembly refusal: {error:?}");
                };
                let collision = refusal.errors.iter().find(|error| {
                    matches!(&error.cause, AssemblyCause::AmbiguousTypeName { name, .. } if name == "m")
                }).expect("typed name collision");
                assert_eq!(collision.cause.code(), Code::AmbiguousDeclaration);
                let AssemblyCause::AmbiguousTypeName {
                    candidates, native, ..
                } = &collision.cause
                else {
                    unreachable!();
                };
                assert!(!native);
                assert_eq!(candidates.len(), 2);
                assert!(candidates[0].start < candidates[1].start);
                for span in candidates {
                    assert_eq!(&source[span.start..span.end], "m");
                }
                assert_eq!(&source[collision.span.start..collision.span.end], "m");
                assert!(source[..collision.span.start].ends_with("unit "));
            }
        }
    }
}

#[trace("TC-483", "FR-091-AC-44")]
#[test]
fn distinct_and_case_distinct_names_compile_and_pass_strict_ir_admission() {
    for name in ["Reading", "M"] {
        for reverse in [false, true] {
            let unit = "dimension Length;\nunit m : Length = rational(1, 1);";
            let record = format!("record {name} {{ sample: Boolean }}");
            let declarations = if reverse {
                format!("{record}\n{unit}")
            } else {
                format!("{unit}\n{record}")
            };
            let source = format!("{HEADER}{declarations}\nfunction f using v(a: Quantity<m>, r: {name}): Boolean pure {{ r.sample }}\n");
            let compiled = compile(&source).expect("distinct declaration names compile");
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
                panic!("strict IR admission failed: {read:?}");
            };
            assert_eq!(
                package.graph().nodes.len(),
                wire["semantic_graph"]["nodes"].as_array().unwrap().len()
            );
        }
    }
}
