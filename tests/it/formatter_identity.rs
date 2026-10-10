// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-003: actual checked identities through S1, S2, S3 and S4.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::DependencyInput;

use crate::support::front_end::emitted;

// Read actual authored literals without maintaining another source copy.
fn source_literal(test_source: &str, name: &str) -> String {
    syn::parse_file(test_source)
        .unwrap()
        .items
        .into_iter()
        .find_map(|item| {
            let syn::Item::Const(item) = item else {
                return None;
            };
            if item.ident != name {
                return None;
            }
            let syn::Expr::Lit(literal) = *item.expr else {
                panic!("{name} must be an authored source literal");
            };
            let syn::Lit::Str(source) = literal.lit else {
                panic!("{name} must be UTF-8 source");
            };
            Some(source.value())
        })
        .unwrap_or_else(|| panic!("fixture literal {name} exists"))
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn evaluator_source_call_fixture_keeps_checked_identity() {
    let source = source_literal(
        include_str!("../../qsl-eval/tests/it/source_call.rs"),
        "UNIT",
    );
    emitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the source-call unit checks before and after formatting");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn replay_composite_fixture_keeps_checked_identity() {
    let profile = source_literal(
        include_str!("../../qsl-replay/src/execute/tests.rs"),
        "PROFILE",
    );
    let body = source_literal(
        include_str!("../../qsl-replay/src/execute/tests/composite_parity.rs"),
        "UNIT",
    );
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{profile}{body}");
    emitted(
        SourceIdentity::new("test", "composite-parity", "fixture", "1"),
        "composite-parity.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the recursive-record, sequence, enum and rational fixture checks");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn wide_integer_fixture_units_keep_checked_identity() {
    // The repository-native TC-909 admitted vectors, including its comment
    // between unary minus and the i128::MIN magnitude.
    let header = source_literal(
        include_str!("../../qsl-replay/src/spine/wide_integer_tests.rs"),
        "HEADER",
    );
    for (parameter, body) in [
        ("Int[0, 18446744073709551615]", "x >= 0"),
        ("Int[0, 9223372036854775808]", "x >= 0"),
        ("Int[0, 18446744073709551616]", "x >= 0"),
        ("Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]", "x <= 170141183460469231731687303715884105727"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= -170141183460469231731687303715884105728"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= - 170141183460469231731687303715884105728"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= -// the minimum\n170141183460469231731687303715884105728"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= -170141183460469231731687303715884105727"),
    ] {
        let source = format!("{header}function u using v(x: {parameter}): Boolean pure {{ {body} }}\n");
        emitted(
            SourceIdentity::new("a", "u", "git", "1"), "unit.native", source.as_bytes(),
            &BTreeMap::new(), &DependencyInput::default(),
        ).unwrap_or_else(|refusal| panic!("{body}: the admitted wide-integer fixture checks: {refusal}"));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn checked_file_fixtures_keep_package_identity_and_second_pass_bytes() {
    let document = include_bytes!("../fixtures/spine-model.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    for (name, bytes) in [
        (
            "spine-compile.native",
            include_bytes!("../fixtures/spine-compile.native").as_slice(),
        ),
        (
            "spine-run.native",
            include_bytes!("../fixtures/spine-run.native").as_slice(),
        ),
        (
            "spine-model.native",
            include_bytes!("../fixtures/spine-model.native").as_slice(),
        ),
        (
            "value-format.native",
            include_bytes!("../fixtures/value-format.native").as_slice(),
        ),
    ] {
        emitted(
            SourceIdentity::new("test", name, "fixture", "1"),
            name,
            bytes,
            &packages,
            &DependencyInput::default(),
        )
        .unwrap_or_else(|refusal| panic!("{name}: original must check: {refusal}"));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn generated_config_version_unit_keeps_checked_identity() {
    let source = crate::support::config_version::spine::unit_text();
    emitted(
        crate::support::config_version::spine::unit_identity(),
        "config-version.native",
        source.as_bytes(),
        &crate::support::config_version::spine::domain_packages(),
        &DependencyInput::default(),
    )
    .expect("ConfigVersion fixture checks before and after formatting");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn changed_literal_changes_checked_identity() {
    let original = include_str!("../fixtures/spine-compile.native");
    let changed = original.replace("pure { 7 }", "pure { 8 }");
    assert_ne!(
        original, changed,
        "mutation must reach the authored literal"
    );
    let compile = |text: &str| {
        emitted(
            SourceIdentity::new("test", "identity-control", "fixture", "1"),
            "control.native",
            text.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
        )
        .unwrap()
        .package()
        .package_id()
    };
    assert_ne!(
        compile(original),
        compile(&changed),
        "checked identity must detect a semantic mutation"
    );
}
