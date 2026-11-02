// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1052 step 7: explicit Text literals check and evaluate through the
//! source compile spine under their selected text profile.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_replay::spine::{self, ParseRequest, SpineLimits};
use qsl_replay::{DependencyInput, SourceIdentity};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{admit_text, Meter, Outcome, TextPayload, TextProfile, TextType, Value};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn compile(source: &str) -> spine::CheckedUnit {
    let identity = SourceIdentity::new("a", "text-literals", "git", "1");
    let limits = SpineLimits::default();
    let cancel = quire_exact::Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "text-literals.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("source parses")
    .into_value();
    let models = spine::select(&parsed, &Default::default(), limits.model, &cancel)
        .expect("models select")
        .into_value();
    spine::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("source checks")
    .into_value()
}

fn text_value(value: &str, profile: TextProfile) -> Value {
    let text_type = TextType::new(0, 8, profile).expect("the fixture Text type is valid");
    let payload = TextPayload::from_source_literal(value, format!("{value:?}"));
    let text = admit_text(
        &payload,
        &text_type,
        &mut Meter::new(qsl_replay::spine::default_accounting(1_000_000)),
    )
    .completed()
    .expect("the supplied fixture Text value is admitted");
    Value::Text(text)
}

fn evaluate_bool(
    compiled: &spine::CheckedUnit,
    function: &str,
    argument: &str,
    profile: TextProfile,
) -> bool {
    let evaluation = compiled
        .package()
        .call(
            &QualifiedName::unqualified(function).expect("the fixture function name is valid"),
            vec![text_value(argument, profile)],
            &ObjectEnvironment::default(),
            &mut Meter::new(qsl_replay::spine::default_accounting(1_000_000)),
        )
        .expect("the admitted function runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) => value,
        outcome => panic!("{function}({argument:?}) returned {outcome:?}"),
    }
}

fn evaluate_text(compiled: &spine::CheckedUnit, function: &str) -> quire_exact::Text {
    let evaluation = compiled
        .package()
        .call(
            &QualifiedName::unqualified(function).expect("the fixture function name is valid"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(qsl_replay::spine::default_accounting(1_000_000)),
        )
        .expect("the admitted function runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Text(value))) => value,
        outcome => panic!("{function} returned {outcome:?}"),
    }
}

#[trace("FR-056-AC-51", "TC-1052")]
#[test]
fn explicit_text_bodies_evaluate_with_profile_normalization_and_equality() {
    let source = format!(
        "{HEADER}\
         function nfc_composed using v(): Text[1, 1; nfc] pure {{ Text(\"\\u00e9\") }}\n\
         function nfc_decomposed using v(): Text[1, 1; nfc] pure {{ Text(\"e\\u0301\") }}\n\
         function scalars_composed using v(): Text[1, 1; unicode-scalars] pure {{ Text(\"\\u00e9\") }}\n\
         function scalars_decomposed using v(): Text[2, 2; unicode-scalars] pure {{ Text(\"e\\u0301\") }}\n\
         function nfc_eq_composed using v(x: Text[0, 8; nfc]): Boolean pure {{ x = Text(\"\\u00e9\") }}\n\
         function nfc_eq_decomposed using v(x: Text[0, 8; nfc]): Boolean pure {{ x = Text(\"e\\u0301\") }}\n\
         function scalars_eq_composed using v(x: Text[0, 8; unicode-scalars]): Boolean pure {{ x = Text(\"\\u00e9\") }}\n\
         function scalars_eq_decomposed using v(x: Text[0, 8; unicode-scalars]): Boolean pure {{ x = Text(\"e\\u0301\") }}"
    );
    let compiled = compile(&source);

    for (function, expected) in [
        ("nfc_composed", "é"),
        ("nfc_decomposed", "é"),
        ("scalars_composed", "é"),
        ("scalars_decomposed", "e\u{301}"),
    ] {
        assert_eq!(
            evaluate_text(&compiled, function).retained(),
            expected,
            "{function}"
        );
    }

    let composed = "é";
    let decomposed = "e\u{301}";
    let empty = "";
    for (function, profile, matches_composed, matches_decomposed) in [
        ("nfc_eq_composed", TextProfile::Nfc, true, true),
        ("nfc_eq_decomposed", TextProfile::Nfc, true, true),
        (
            "scalars_eq_composed",
            TextProfile::UnicodeScalars,
            true,
            false,
        ),
        (
            "scalars_eq_decomposed",
            TextProfile::UnicodeScalars,
            false,
            true,
        ),
    ] {
        assert_eq!(
            evaluate_bool(&compiled, function, composed, profile),
            matches_composed,
            "{function} with composed input"
        );
        assert_eq!(
            evaluate_bool(&compiled, function, decomposed, profile),
            matches_decomposed,
            "{function} with decomposed input"
        );
        assert!(
            !evaluate_bool(&compiled, function, empty, profile),
            "{function} with empty input"
        );
    }
}
