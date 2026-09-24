// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-019/021: independently authored native package construction expectations.

#[path = "../package_construction_cases/correspondence.rs"]
mod correspondence;
#[path = "../package_construction_cases/features.rs"]
mod features;
#[path = "../package_construction_cases/fixed.rs"]
mod fixed;
#[path = "../package_construction_cases/identity.rs"]
mod identity;
#[path = "../package_construction_cases/inventory.rs"]
mod inventory;
#[path = "../package_construction_cases/limits.rs"]
mod limits;
use crate::support::native_rule_model;
use crate::support::package_vector_setup;
#[path = "../package_construction_cases/schema.rs"]
mod schema;
#[path = "../package_construction_cases/static_changes.rs"]
mod static_changes;
#[path = "../package_construction_cases/vectors.rs"]
mod vectors;

use ix_trace_rs::trace;
use qsl_foundation::{ByteDigest, Code, SourceIdentity};
use quire_contract_ir as ir;
use quire_spec_language::checking::{
    check, CheckBindings, CheckLimits, CheckedPackage, ClauseBinding,
};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::{link_native, parse, Limits, LinkLimits};

const HEADER: &str = include_str!("../fixtures/native-package/header.native");

fn checked<'a>(
    text: &str,
    identity: &str,
    revision: &str,
    formal_revision: u64,
    path: &str,
    models: &'a [NativeModel],
    clauses: Vec<ClauseBinding>,
) -> CheckedPackage<'a> {
    let unit = parse(
        SourceIdentity {
            authority: "agent-ix".into(),
            identity: identity.into(),
            revision_namespace: "draft".into(),
            revision: revision.into(),
        },
        path,
        text.as_bytes(),
        Limits::default(),
    )
    .expect("actual parser setup before package construction");
    let source = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("PackageSource").unwrap(),
            ir::SourceRevision::new(formal_revision).unwrap(),
        ),
    );
    let linked = link_native(unit, models, LinkLimits::default())
        .expect("actual native linker setup before package construction");
    check(
        linked,
        CheckBindings { source, clauses },
        CheckLimits::default(),
    )
    .expect("actual checker setup before package construction")
}

fn binding() -> ClauseBinding {
    ClauseBinding {
        name: "Rule".into(),
        requirement: ir::RequirementRef::parse("example/package-rules", "Rule", 2).unwrap(),
        clause: ir::ClauseId::new("rule").unwrap(),
        execution_point: ir::ExecutionPoint::Handler {
            name: ir::AnchorName::new("validate").unwrap(),
        },
    }
}

fn minimal<'a>(
    models: &'a [NativeModel],
    identity: &str,
    revision: &str,
    formal_revision: u64,
    path: &str,
) -> CheckedPackage<'a> {
    checked(
        &vectors::source(&models[0]),
        identity,
        revision,
        formal_revision,
        path,
        models,
        vec![binding()],
    )
}

#[test]
#[trace("TC-078", "TC-090", "FR-019-AC-1")]
#[trace("FR-019-AC-2", "FR-019-AC-8", "FR-021-AC-1")]
fn minimum_admitted_unit_has_exact_independent_static_bytes() {
    let models = [vectors::model()];
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let package = NativePackage::new(
        minimal(&models, "test:package", "draft:1", 1, "first.native"),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        std::str::from_utf8(package.bytes()).unwrap(),
        expected.artifact
    );
    assert_eq!(
        package.digest(),
        ByteDigest::of(expected.artifact.as_bytes())
    );
    assert_eq!(package.canonical_identity().to_string(), expected.digest);
    assert_eq!(package.reference().digest(), package.digest());
    assert_eq!(package.reference().format(), "native-linked-package/1");
    assert_eq!(package.checked().clauses().len(), 1);
    assert_eq!(package.checked().linked().models().len(), 1);
    let again = NativePackage::new(
        minimal(
            &models,
            "test:package",
            "draft:1",
            1,
            "another/location.native",
        ),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(package.bytes(), again.bytes());
    assert_eq!(package.canonical_identity(), again.canonical_identity());
    assert_eq!(
        package.checked().linked().unit().source().path(),
        "first.native"
    );
    assert_eq!(
        again.checked().linked().unit().source().path(),
        "another/location.native"
    );
}

#[test]
#[trace("TC-090", "FR-019-AC-9", "FR-021-AC-1", "FR-021-AC-6")]
fn canonical_unicode_controls_and_large_revision_are_exact() {
    let models = [vectors::model()];
    let expected = vectors::expected(
        &models[0],
        r#""test:\"\\/\u0000\b\f\n\r\té🦀""#,
        r#""draft:é\u0001""#,
        9_007_199_254_740_993,
    );
    let package = NativePackage::new(
        minimal(
            &models,
            "test:\"\\/\0\u{8}\u{c}\n\r\té🦀",
            "draft:é\u{1}",
            9_007_199_254_740_993,
            "unicode.native",
        ),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        std::str::from_utf8(package.bytes()).unwrap(),
        expected.artifact
    );
    assert_eq!(package.canonical_identity().to_string(), expected.digest);
    let wire: serde_json::Value = serde_json::from_slice(package.bytes()).unwrap();
    assert_eq!(
        wire["source"]["formal"]["revision"].as_u64(),
        Some(9_007_199_254_740_993)
    );
    assert_ne!(
        package
            .digest()
            .to_string()
            .strip_prefix("sha256:")
            .unwrap(),
        expected.digest
    );
    assert_eq!(
        package.usage().canonical.as_ref().unwrap().output_bytes,
        expected.canonical.len()
    );
    assert_eq!(
        package.usage().encode.as_ref().unwrap().output_bytes,
        expected.artifact.len()
    );
    assert!(package.usage().decode.is_none());
    assert_eq!(package.usage().admitted_input_bytes, 0);
}

#[test]
#[trace("TC-078", "TC-080", "TC-081")]
#[trace("FR-019-AC-1", "FR-019-AC-3", "FR-019-AC-5")]
#[trace("FR-019-AC-6", "FR-019-AC-7")]
fn actual_checked_clause_retains_complete_selected_model_and_dispositions() {
    let models = [native_rule_model::parts().model()];
    let text = format!(
        "{HEADER}model M = \"example/rule-tests\" version \"1\" digest \"{}\";\ninvariant Rule on M::Node at current {{ true }}\n",
        models[0].digest()
    );
    let binding = ClauseBinding {
        name: "Rule".into(),
        requirement: ir::RequirementRef::parse("example/package-rules", "Rule", 2).unwrap(),
        clause: ir::ClauseId::new("rule").unwrap(),
        execution_point: ir::ExecutionPoint::Handler {
            name: ir::AnchorName::new("validate").unwrap(),
        },
    };
    let package = NativePackage::new(
        checked(
            &text,
            "test:package-rule",
            "1",
            1,
            "rule.native",
            &models,
            vec![binding.clone()],
        ),
        PackageLimits::default(),
    )
    .unwrap();
    let wire: serde_json::Value = serde_json::from_slice(package.bytes()).unwrap();
    assert_eq!(wire["models"].as_array().unwrap().len(), 1);
    assert_eq!(wire["models"][0]["alias"], "M");
    assert_eq!(wire["models"][0]["digest"], models[0].digest().to_string());
    assert_eq!(
        wire["models"][0]["artifact"],
        std::str::from_utf8(models[0].artifact_bytes()).unwrap()
    );
    assert_eq!(
        wire["required_features"],
        serde_json::json!([
            "boolean",
            "integer",
            "object",
            "option",
            "reference",
            "sequence",
            "text"
        ])
    );
    assert_eq!(wire["clauses"].as_array().unwrap().len(), 1);
    let clause = &wire["clauses"][0];
    assert_eq!(clause["name"], "Rule");
    assert_eq!(
        clause["owner"],
        serde_json::to_value(&binding.requirement).unwrap()
    );
    assert_eq!(clause["clause"], "rule");
    assert_eq!(clause["kind"], "invariant");
    assert_eq!(
        clause["context"]["identity"]["key"]["name"],
        native_rule_model::symbol("Node").as_str()
    );
    assert_eq!(
        clause["execution_point"],
        serde_json::json!({"kind":"handler","name":"validate"})
    );
    assert_eq!(
        clause["runtime"]["context_observations"],
        serde_json::json!(["current"])
    );
    assert_eq!(clause["runtime"]["validate_frame"], false);
    assert_eq!(clause["projections"].as_array().unwrap().len(), 2);
    assert_eq!(
        clause["projections"][0],
        serde_json::json!({
            "target":"native-reference/1", "status":"available", "cost_model":"native-ref-cost/1-draft"
        })
    );
    assert_eq!(
        clause["projections"][1]["target"],
        "quire.contract.executable-projection/v1"
    );
    assert_eq!(clause["projections"][1]["status"], "unlowered");
    assert_eq!(clause["projections"][1]["code"], "unsupported_construct");
    assert_eq!(package.checked().clauses()[0].binding(), &binding);
    assert!(std::ptr::eq(
        package.checked().linked().models()[0]
            .native_model()
            .unwrap(),
        &models[0]
    ));
}

#[test]
#[trace("TC-088", "FR-019-AC-10", "FR-021-AC-6")]
fn construction_byte_limit_is_inclusive_and_failed_retry_has_fresh_usage() {
    let models = [vectors::model()];
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let bytes = expected.artifact.as_bytes();
    let make = |limit| {
        NativePackage::new(
            minimal(&models, "test:package", "draft:1", 1, "limits.native"),
            PackageLimits {
                artifact_bytes: limit,
                ..PackageLimits::default()
            },
        )
    };
    let exact = make(bytes.len()).unwrap();
    assert_eq!(exact.bytes(), bytes);
    let error = make(bytes.len() - 1).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert!(error.usage.encode.as_ref().unwrap().output_bytes < bytes.len());
    assert_eq!(make(bytes.len()).unwrap().usage(), exact.usage());
}

#[test]
#[trace("TC-083", "TC-084", "FR-020-AC-2", "FR-020-AC-3", "FR-020-AC-4")]
fn package_error_codes_extend_the_existing_native_vocabulary() {
    // Vocabulary compatibility only; the reader refusal cases stay pending.
    for (spelling, code) in [
        ("invalid_package", Code::InvalidPackage),
        ("unknown_wire", Code::UnknownWire),
        ("unknown_required_feature", Code::UnknownRequiredFeature),
    ] {
        assert_eq!(code.as_str(), spelling);
        assert_eq!(Code::from_code(spelling), Some(code));
        assert_eq!(Code::all().iter().filter(|&&item| item == code).count(), 1);
    }
}
