// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393 builds the timed declarations and the `Clock` type at S2 and leaves whether a unit admits them to S3. Until
//! FR-230's check reads them, S3 refuses each `unsupported_construct`/
//! `not-yet-implemented` at its own span, so none compiles unchecked. Run
//! over TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{AssemblyCause, CheckCause, TemporalCause, TimedConstruct};
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, admit_and_assemble_with_profiles,
    attempt_update_modifies_version_number, config_unit_with_body, config_version_document,
    config_version_identity, wire_field,
};

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

fn clock_document() -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(&document()).expect("the fixture is JSON");
    let owner = "ix://example/config-version/ConfigVersion";
    let field = config_version_identity("x");
    document["types"]
        .as_array_mut()
        .expect("the fixture declares types")
        .iter_mut()
        .find(|ty| ty["identity"] == owner)
        .expect("ConfigVersion is declared")["fields"]
        .as_array_mut()
        .expect("ConfigVersion declares fields")
        .push(wire_field(
            &field,
            "x",
            "ix://quire/native/Clock",
            "required",
            1,
        ));
    serde_json::to_vec(&document).expect("the fixture stays JSON")
}

/// The byte offset of `needle` in the unit `body` is written into.
fn offset(body: &str, needle: &str) -> usize {
    let (text, _) = config_unit_with_body(&document(), body);
    text.find(needle).expect("the needle is in the unit")
}

#[trace("TC-685")]
#[test]
fn s2_keeps_a_dense_time_member_for_the_temporal_layer() {
    let body = "time Config dense unit ms;\n";
    let assembled = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    );
    assert!(
        assembled.is_ok(),
        "S2 retains a grammar-valid time member: {assembled:?}"
    );
}

#[trace("TC-685")]
#[test]
#[ignore = "IR-990: FCD I1 rejects the package Clock field typeRef"]
fn a_package_clock_field_reaches_the_timed_model_check() {
    let assembled = admit_and_assemble_with_profiles(
        &clock_document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        "time Config dense unit ms;\n",
    );
    assert!(
        assembled.is_ok(),
        "a package Clock field reaches S3: {assembled:?}"
    );
}

#[trace("TC-685")]
#[test]
fn dense_source_resolves_a_time_unit_at_s3() {
    let body = "dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\ntime Config dense unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the unit and time forms");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("a dense source with a Time unit checks");
    assert_eq!(checked.time_sources()[0].model, "Config");
    assert_eq!(checked.time_sources()[0].unit, "ms");
}

#[trace("TC-685")]
#[test]
fn unknown_time_unit_refuses_at_the_written_name() {
    let body = "dimension Time;\nunit s : Time = rational(1, 1);\ntime Config dense unit hour;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 does not resolve the unit");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 resolves the time unit");
    assert!(refusals.iter().any(|refusal| matches!(
        &refusal.cause,
        CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::TimeQuantity { cause, span } if matches!(&**cause, qsl_semantics::check::TimeUnitCause::UnknownUnit { name } if name == "hour") && span.end - span.start == 4)
    )), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn a_length_unit_cannot_be_the_time_source() {
    let body = "dimension Time;\ndimension Length;\nunit m : Length = rational(1, 1);\ntime Config dense unit m;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the wrong dimension");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 requires the Time dimension");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::TimeQuantity { cause, .. } if matches!(&**cause, qsl_semantics::check::TimeUnitCause::NotTimeDimension { name } if name == "m")))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn a_source_without_base_time_refuses_before_looking_up_the_unit() {
    let body = "dimension Length;\nunit m : Length = rational(1, 1);\ntime Config dense unit m;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the time declaration");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 requires a base Time node");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::TimeQuantity { cause, .. } if matches!(&**cause, qsl_semantics::check::TimeUnitCause::NoTimeDimension)))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn time_source_names_an_admitted_model() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Missing dense unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the unresolved alias");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 resolves the model alias");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::MissingTimedModel { name, .. } if name == "Missing"))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn qualified_package_unit_is_not_misdiagnosed_as_a_missing_source_unit() {
    let body =
        "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit Config::ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 keeps the qualified unit name");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("package unit lookup is pending");
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::TimedNotChecked {
                construct: TimedConstruct::TimeSource,
                ..
            }
        )),
        "{refusals:?}"
    );
}

#[trace("TC-685")]
#[test]
fn duplicate_time_source_refuses_at_the_second_declaration() {
    let body = "dimension Time;\nunit s : Time = rational(1, 1);\ntime Config dense unit s;\ntime Config dense unit s;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains both declarations");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("one model has one time source");
    assert!(refusals.iter().any(|refusal| matches!(
        &refusal.cause,
        CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::DuplicateTimeSource { model, .. } if model == "Config")
    )), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn tick_source_resolves_its_operation_and_exact_period() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::attemptUpdate period rational(1,2) ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the tick form");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("the operation and positive period check");
    assert!(
        matches!(&checked.time_sources()[0].source, qsl_semantics::check::TimeSourceKind::Tick { operation, period } if operation == "ConfigVersion::attemptUpdate" && *period == quire_exact::Rational::new(1_i64.into(), 2_i64.into()).expect("half"))
    );
}

#[trace("TC-685")]
#[test]
fn tick_period_converts_exactly_to_the_model_unit() {
    let body = "dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\ntime Config tick ConfigVersion::attemptUpdate period 3 ms unit s;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains both unit names");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("S3 converts the period exactly");
    assert!(
        matches!(&checked.time_sources()[0].source, qsl_semantics::check::TimeSourceKind::Tick { period, .. } if *period == quire_exact::Rational::new(3_i64.into(), 1000_i64.into()).expect("three milliseconds"))
    );
}

#[trace("TC-685")]
#[test]
fn zero_tick_period_refuses_at_the_period() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::attemptUpdate period 0 ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the zero period");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("tick period must be positive");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::InvalidTickPeriod { .. }))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn missing_tick_operation_refuses_at_its_name() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::absent period 1 ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the unresolved operation name");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 resolves the operation");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::MissingTickOperation { name, .. } if name == "ConfigVersion::absent"))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn tick_operation_cannot_resolve_through_another_model_alias() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick Other::ConfigVersion::attemptUpdate period 1 ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the qualified operation name");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("an unrelated qualifier does not resolve");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::MissingTickOperation { name, .. } if name == "Other::ConfigVersion::attemptUpdate"))), "{refusals:?}");
}

#[trace("TC-1078")]
#[test]
fn each_timed_declaration_reaches_the_s3_facet_gate() {
    for (body, construct, at) in [
        (
            "time Config dense unit ms;\n",
            TimedConstruct::TimeSource,
            "time Config",
        ),
        (
            "time invariant i when { true } { self.x <= 3 ms }\n",
            TimedConstruct::TimeInvariant,
            "time invariant",
        ),
        (
            "urgent u when { true };\n",
            TimedConstruct::Urgent,
            "urgent u",
        ),
    ] {
        let declarations = admit_and_assemble_with_body(&document(), body)
            .expect("S2 retains a grammar-valid timed declaration");
        let refusals = declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .expect_err("S3 requires the temporal facet");
        let [error] = refusals.as_slice() else {
            panic!("{body}: one S3 error expected");
        };
        assert!(
            matches!(&error.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::TimedFacetRequired { construct: found, .. } if *found == construct)),
            "{body}: {error:?}"
        );
        assert_eq!(error.cause.code(), Code::UnsupportedConstruct, "{body}");
        assert_eq!(
            error.cause.catalog_code().expect("catalog").cause(),
            "declaration-form"
        );
        if let CheckCause::Temporal(cause) = &error.cause {
            assert_eq!(cause.span().start, offset(body, at), "{body}");
        }
    }
}

#[trace("TC-1078")]
#[test]
fn source_clock_type_still_refuses_at_assembly() {
    let body = "record R { x: Clock; }\n";
    let refusal = admit_and_assemble_with_body(&document(), body)
        .expect_err("a source Clock type has no checked representation yet");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error expected: {:?}", refusal.errors);
    };
    assert_eq!(
        error.cause,
        AssemblyCause::TimedNotChecked(TimedConstruct::Clock)
    );
    assert_eq!(error.span.start, offset(body, "Clock"));
}
