// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393 builds the timed declarations and the `Clock` type at S2; FR-230
//! checks them at S3 under the timed-complete facet. Run over TC-458's
//! `ConfigVersion` fixture package (`crate::model_operations`).

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{AssemblyCause, CheckCause, TemporalCause, TimedConstruct};
use qsl_semantics::value::HeaderLayer;
use serde_json::json;

use crate::model_operations::{
    add_sub_type, admit_and_assemble_with_body, admit_and_assemble_with_profiles,
    attempt_update_modifies_version_number, config_unit_with_body, config_unit_with_profiles,
    config_version_document, config_version_identity, tc465_deep_hierarchy_document, wire_field,
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
    clock_document_over(document())
}

/// `document` with the clock field `ConfigVersion.x`.
fn clock_document_over(document: Vec<u8>) -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(&document).expect("the fixture is JSON");
    let owner = "ix://example/config-version/ConfigVersion";
    let field = config_version_identity("x");
    let mut clock = wire_field(&field, "x", "ix://quire/native/Clock", "required", 1);
    // FCD admits a Clock field only with a non-unique multiplicity.
    clock["multiplicity"]["unique"] = json!(false);
    document["types"]
        .as_array_mut()
        .expect("the fixture declares types")
        .iter_mut()
        .find(|ty| ty["identity"] == owner)
        .expect("ConfigVersion is declared")["fields"]
        .as_array_mut()
        .expect("ConfigVersion declares fields")
        .push(clock);
    serde_json::to_vec(&document).expect("the fixture stays JSON")
}

fn package_units_document() -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(&document()).expect("the fixture is JSON");
    let origin = document["types"][0]["origin"].clone();
    let time = "ix://example/config-version/dimension/Time";
    let second = "ix://example/config-version/unit/s";
    document["dimensions"] = json!([{
        "identity": time,
        "name": "Time",
        "base": true,
        "terms": [],
        "origin": origin,
    }]);
    document["units"] = json!([
        {
            "identity": second,
            "name": "s",
            "dimension": time,
            "scale": "1",
            "offset": "0",
            "origin": origin,
        },
        {
            "identity": "ix://example/config-version/unit/ms",
            "name": "ms",
            "dimension": time,
            "target": second,
            "scale": "1/1000",
            "offset": "0",
            "origin": origin,
        },
    ]);
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

/// QSL-1030: the native Clock field with `unique:false` is admitted as a
/// model place while S3 checks the package's time source.
#[test]
fn a_package_clock_field_is_admitted_beside_a_checked_time_source() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &clock_document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("a package Clock field is admitted at intake");
    let checked = declarations
        .with_unwritten_timed_members()
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("the time source checks beside the Clock field");
    assert_eq!(checked.time_sources()[0].model, "Config");
    let timed = checked.timed_model("Config").expect("a timed model");
    let clocks: Vec<&str> = timed.clocks().iter().map(|clock| clock.name()).collect();
    assert_eq!(clocks, ["x"]);
}

/// FR-230-AC-5/AC-33: a package-owned unit resolves through its model owner
/// and binds the timed source to the package's nominal base Time node.
#[trace("FR-230-AC-5", "FR-230-AC-33")]
#[test]
fn package_time_units_reach_s3_with_their_model_owner() {
    let declarations = admit_and_assemble_with_profiles(
        &package_units_document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        "time Config dense unit Config::ms;\n",
    )
    .expect("FCD admits package dimensions and units");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("S3 resolves the package-owned Time unit");
    assert_eq!(checked.time_sources()[0].unit, "Config::ms");
    assert_ne!(
        checked.time_sources()[0].time_dimension.as_bytes(),
        &[0; 32]
    );
}

#[trace("TC-685")]
#[test]
fn dense_source_resolves_a_time_unit_at_s3() {
    for name in ["ms", "s"] {
        let body = format!("dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\ntime Config dense unit {name};\n");
        let declarations = admit_and_assemble_with_profiles(
            &document(),
            "profile t = \"quire.temporal.complete/v1\";\n",
            &body,
        )
        .expect("S2 retains the unit and time forms");
        let checked = declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .expect("a dense source with a Time unit checks");
        assert_eq!(checked.time_sources()[0].model, "Config");
        assert_eq!(checked.time_sources()[0].unit, name);
    }
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
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::TimeQuantity { cause, .. } if matches!(&**cause, qsl_semantics::check::TimeUnitCause::NoTimeDimension { name } if *name == "Time")))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn derived_duration_is_not_the_base_time_dimension() {
    let body = "dimension Time;\nunit s : Time = rational(1, 1);\ndimension Duration = Time;\nunit d : Duration = rational(1, 1);\ntime Config dense unit d;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the derived dimension");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("Duration and Time have distinct nodes");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::TimeQuantity { cause, .. } if matches!(&**cause, qsl_semantics::check::TimeUnitCause::NotTimeDimension { name } if name == "d")))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn affine_time_unit_cannot_be_a_model_time_source() {
    let body = "dimension Time;\nunit s : Time = rational(1, 1);\nunit degc : Time = rational(1, 1) * s + rational(273, 1);\ntime Config dense unit degc;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the affine unit");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 rejects a nonzero composed offset");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::TimeQuantity { cause, .. } if matches!(&**cause, qsl_semantics::check::TimeUnitCause::AffineUnit { name, .. } if name == "degc")))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn duplicate_source_time_dimensions_refuse_at_assembly() {
    let body = "dimension Time;\ndimension Time;\nunit s : Time = rational(1, 1);\ntime Config dense unit s;\n";
    let refusal = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect_err("S2 assembly rejects a duplicate source dimension");
    assert!(refusal.errors.iter().any(|error| matches!(&error.cause, AssemblyCause::DuplicateQuantityName { name, spans } if name == "Time" && spans.len() == 2) && error.cause.catalog_code().to_string() == "ambiguous_declaration/ambiguous-name"), "{refusal:?}");
}

#[trace("TC-685", "FR-230-AC-38")]
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
fn urgent_when_without_a_time_source_refuses_as_a_missing_binding() {
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        "urgent stalled when { true };\n",
    )
    .expect("S2 retains the urgency condition");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("urgency needs a timed model");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::UrgencyWithoutTimeSource { .. }))), "{refusals:?}");
}

#[trace("TC-685", "FR-230-AC-5", "FR-230-AC-33")]
#[test]
fn qualified_package_unit_resolves_even_when_a_local_unit_has_the_same_name() {
    let body =
        "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit Config::ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &package_units_document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 keeps the qualified package unit name");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("S3 resolves the selected package's unit rather than the local ms");
    assert_eq!(checked.time_sources()[0].unit, "Config::ms");
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

#[trace("TC-685", "FR-230-AC-38")]
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
        matches!(&checked.time_sources()[0].source, qsl_semantics::check::TimeSourceKind::Tick { operation, period } if operation == &config_version_identity("attemptUpdate") && *period == quire_exact::Rational::new(1_i64.into(), 2_i64.into()).expect("half"))
    );
    assert!(checked.time_sources()[0].tick_operation_anchor.is_some());
    assert!(checked.time_sources()[0].tick_operation_span.is_some());
}

#[trace("TC-685", "FR-230-AC-38")]
#[test]
fn unqualified_tick_operation_resolves_to_its_declaration_identity() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick attemptUpdate period 1 ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains the unqualified operation");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("a unique model operation resolves");
    assert!(
        matches!(&checked.time_sources()[0].source, qsl_semantics::check::TimeSourceKind::Tick { operation, .. } if operation == &config_version_identity("attemptUpdate"))
    );
    assert!(checked.time_sources()[0].tick_operation_anchor.is_some());
}

#[trace("TC-685", "FR-230-AC-38")]
#[test]
fn a_model_with_multiple_object_nodes_refuses_a_tick_source_on_a_derived_object() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick Deep1::attemptUpdate period 1 ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &tc465_deep_hierarchy_document(1),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 keeps the derived owner name");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("the model alias has multiple semantic nodes");
    assert!(
        refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::AmbiguousTimedModel { name, .. } if name == "Config"))),
        "{refusals:?}"
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
fn a_ten_to_fortieth_tick_period_is_checked_exactly() {
    let huge = "10000000000000000000000000000000000000000";
    let body = format!("dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::attemptUpdate period {huge} ms unit ms;\n");
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        &body,
    )
    .expect("S2 retains the large exact period");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("a 10^40 period is representable");
    assert!(
        matches!(&checked.time_sources()[0].source, qsl_semantics::check::TimeSourceKind::Tick { period, .. } if *period == quire_exact::Rational::from_integer(quire_exact::Integer::power_of_ten(40)))
    );
}

#[trace("TC-685")]
#[test]
fn tick_period_respects_the_s3_rational_bit_limit() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::attemptUpdate period 10000000000000000000000000000000000000000 ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 does not impose the S3 ceiling");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default().with_rational_bits(32))
        .expect_err("the configured S3 ceiling stops the exact reduction");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::ResourceExhausted(limit) if limit.kind == qsl_semantics::check::CheckingLimitKind::RationalBits && limit.region.is_some())), "{refusals:?}");
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

/// FR-230-AC-3's S3 half (TC-685 step 3): the tick source over
/// `attemptUpdate` with period `rational(1,2) ms` checks to a tick source
/// with period `1/2`; `period 0 ms` refuses `invalid_model_binding`/
/// `malformed-declaration` at the period; an operation name that resolves
/// to nothing refuses `missing_declaration`/`missing-name` at the name. The
/// S2 half is `qsl-forms`' `time_forms` test.
#[trace("TC-685", "FR-230-AC-3")]
#[test]
fn tick_sources_check_and_refuse_at_their_spans() {
    let units = "dimension Time;\nunit ms : Time = rational(1, 1);\n";
    let temporal = "profile t = \"quire.temporal.complete/v1\";\n";
    let body = format!(
        "{units}time Config tick ConfigVersion::attemptUpdate period rational(1,2) ms unit ms;\n"
    );
    let checked = admit_and_assemble_with_profiles(&document(), temporal, &body)
        .expect("S2 retains the tick form")
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("the tick source checks");
    let qsl_semantics::check::TimeSourceKind::Tick { period, .. } =
        &checked.time_sources()[0].source
    else {
        panic!("a tick source: {:?}", checked.time_sources());
    };
    assert_eq!(
        *period,
        quire_exact::Rational::new(1_i64.into(), 2_i64.into()).expect("half")
    );
    for (line, code, tag, at) in [
        (
            "time Config tick ConfigVersion::attemptUpdate period 0 ms unit ms;\n",
            Code::InvalidModelBinding,
            "malformed-declaration",
            "0 ms",
        ),
        (
            "time Config tick ConfigVersion::absent period 1 ms unit ms;\n",
            Code::MissingDeclaration,
            "missing-name",
            "ConfigVersion::absent",
        ),
    ] {
        let body = format!("{units}{line}");
        let refusals = admit_and_assemble_with_profiles(&document(), temporal, &body)
            .expect("S2 retains the form")
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .expect_err("S3 refuses the tick source");
        let [refusal] = refusals.as_slice() else {
            panic!("{line}: one refusal: {refusals:?}");
        };
        let CheckCause::Temporal(cause) = &refusal.cause else {
            panic!("{line}: a timed refusal: {refusal:?}");
        };
        let (text, _) = config_unit_with_profiles(&document(), temporal, &body);
        let span = cause.span();
        assert_eq!(
            (cause.code(), cause.tag(), &text[span.start..span.end]),
            (code, tag, at),
            "{line}"
        );
    }
}

#[trace("TC-685")]
#[test]
fn a_tick_period_with_zero_denominator_refuses_at_the_magnitude() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::attemptUpdate period rational(1,0) ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 retains an exact value without resolving it");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 refuses an undefined rational");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::ZeroDenominator { .. }))), "{refusals:?}");
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

/// `document()` with `Sub` (a subtype of `ConfigVersion`) that declares its
/// own `attemptUpdate`, overriding the inherited one.
fn overriding_document() -> Vec<u8> {
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&add_sub_type(document(), &[])).expect("valid JSON");
    let mut operation = attempt_update_modifies_version_number();
    operation["identity"] = json!("ix://example/config-version/Sub/attemptUpdate");
    let sub = envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://example/config-version/Sub")
        .expect("Sub is declared");
    sub["operations"] = json!([operation]);
    serde_json::to_vec(&envelope).expect("valid JSON")
}

fn tick_over(
    document: &[u8],
    operation: &str,
) -> Result<Vec<qsl_semantics::check::ModelTimeSource>, Vec<qsl_semantics::check::CheckRefusal>> {
    let body = format!("dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick {operation} period 1 ms unit ms;\n");
    admit_and_assemble_with_profiles(
        document,
        "profile t = \"quire.temporal.complete/v1\";\n",
        &body,
    )
    .expect("S2 retains the tick form")
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .map(|checked| checked.time_sources().to_vec())
}

#[trace("TC-685", "FR-230-AC-38")]
#[test]
fn a_timed_model_with_multiple_object_nodes_refuses_an_unqualified_tick_operation() {
    let refusals = tick_over(&overriding_document(), "attemptUpdate")
        .expect_err("the model alias has multiple semantic nodes");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::AmbiguousTimedModel { name, .. } if name == "Config"))), "{refusals:?}");
}

#[trace("TC-685", "FR-230-AC-38")]
#[test]
fn a_timed_model_with_multiple_object_nodes_refuses_a_qualified_tick_operation() {
    let refusals = tick_over(&overriding_document(), "Sub::attemptUpdate")
        .expect_err("the model alias has multiple semantic nodes");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::AmbiguousTimedModel { name, .. } if name == "Config"))), "{refusals:?}");
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

/// The declarations of FR-230-AC-7, each timed construct in source order.
const UNSELECTED_TIMED: &str = "time Config dense unit ms;\n\
     time invariant Deadline when { true } { self.x <= 3 ms }\n\
     urgent Eager when { true };\n\
     function f using v(c: Clock): Boolean pure { true }\n\
     function g using v(): Clock pure { 0 }\n\
     record Timer { x: Clock; n: Integer; }\n\
     type Alias = Clock;\n\
     type Maybe = Option<Clock>;\n\
     tuple Pair(Clock, Integer);\n\
     union Armed { On(Clock), Off }\n\
     type Watches = Set<Clock>;\n";

/// Each refusal of `refusals` as FR-230's facet gate refuses it: the
/// construct, its owning declaration, the selected header layers and the
/// byte offset of its span, after checking its code and required facet.
fn facet_refusals(
    refusals: &[qsl_semantics::check::CheckRefusal],
) -> Vec<(TimedConstruct, String, Vec<HeaderLayer>, usize)> {
    refusals
        .iter()
        .map(|refusal| {
            assert_eq!(refusal.cause.code(), Code::UnsupportedConstruct);
            assert_eq!(
                refusal.cause.catalog_code().expect("catalog").cause(),
                "declaration-form"
            );
            let CheckCause::Temporal(cause) = &refusal.cause else {
                panic!("a facet refusal expected: {refusal:?}");
            };
            let TemporalCause::TimedFacetRequired {
                construct,
                owner,
                selected,
                required,
                span,
            } = &**cause
            else {
                panic!("a facet refusal expected: {cause:?}");
            };
            assert_eq!(*required, HeaderLayer::TemporalComplete);
            (*construct, owner.clone(), selected.clone(), span.start)
        })
        .collect()
}

/// FR-230-AC-7 (TC-685 step 8): a unit whose header selects
/// `quire.model.complete/v1` alone refuses its `time` member, `time
/// invariant`, `urgent when` declaration and each `Clock` its own source
/// writes in a type position (a parameter, a result, a record field, an
/// alias target, the argument of `Option`, a tuple element, a union payload
/// and a collection element), `unsupported_construct`/`declaration-form` at
/// each span, in source order and every one, each with its family, its
/// owning declaration (the model `Config` for the `time` member, the `time
/// invariant` and the `urgent` declaration), the selected layers and the
/// required facet, and no other refusal. A unit whose header selects
/// `quire.value.complete/v1` alone refuses each `Clock` alike; S1 admits no
/// unit without a `profile`, so that is the AC's no-`profile` vector.
/// FR-230-AC-8 (TC-685 step 8): the unit of AC-7 with the header selection
/// `profile t = "quire.temporal.complete/v1";` reaches S3 with its `time`
/// member, `time invariant`, `urgent` declaration and each `Clock` in a type
/// position, and S3 refuses none of them `unsupported_construct`/
/// `declaration-form` or otherwise: the facet gate passes them all and each
/// checks. E3's resolution of
/// the selection to the temporal layer is FR-110-AC-13's
/// `qsl-replay` test, and every AC-1 to AC-5 test checks a unit selecting
/// the layer. The same unit without the selection is AC-7's.
#[trace("TC-685", "FR-230-AC-8")]
#[test]
fn a_unit_selecting_the_timed_facet_passes_the_gate() {
    let declarations = admit_and_assemble_with_profiles(
        &clock_document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        SELECTED_TIMED,
    )
    .expect("S2 retains every grammar-valid timed form");
    declarations
        .with_unwritten_timed_members()
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the facet admits every timed member: {refusals:?}"));
}

/// AC-8's unit: AC-7's timed members over the clock `x`, and the `Clock`
/// type positions a value of the facet checks.
const SELECTED_TIMED: &str = "dimension Time;\nunit ms : Time = rational(1, 1);\n\
     time Config dense unit ms;\n\
     time invariant Deadline when { true } { self.x <= 3 ms }\n\
     urgent Eager when { true };\n\
     function f using v(c: Clock): Boolean pure { true }\n\
     record Timer { x: Clock; n: Integer; }\n\
     type Alias = Clock;\n\
     type Maybe = Option<Clock>;\n";

#[trace("TC-685", "FR-230-AC-7")]
#[test]
fn a_unit_without_the_timed_facet_refuses_every_timed_construct() {
    let declarations = admit_and_assemble_with_body(&document(), UNSELECTED_TIMED)
        .expect("S2 retains every grammar-valid timed form");
    let refusals = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("S3 requires the temporal facet");
    let (text, _) = config_unit_with_body(&document(), UNSELECTED_TIMED);
    let model = vec![HeaderLayer::ModelComplete];
    let row = |construct, owner: &str, at: usize| (construct, owner.to_owned(), model.clone(), at);
    let clocks = clock_offsets(&text);
    assert_eq!(
        facet_refusals(&refusals),
        [
            row(
                TimedConstruct::TimeSource,
                "Config",
                offset(UNSELECTED_TIMED, "time Config")
            ),
            row(
                TimedConstruct::TimeInvariant,
                "Config",
                offset(UNSELECTED_TIMED, "time invariant")
            ),
            row(
                TimedConstruct::Urgent,
                "Config",
                offset(UNSELECTED_TIMED, "urgent Eager")
            ),
            row(TimedConstruct::Clock, "f", clocks[0]),
            row(TimedConstruct::Clock, "g", clocks[1]),
            row(TimedConstruct::Clock, "Timer", clocks[2]),
            row(TimedConstruct::Clock, "Alias", clocks[3]),
            row(TimedConstruct::Clock, "Maybe", clocks[4]),
            row(TimedConstruct::Clock, "Pair", clocks[5]),
            row(TimedConstruct::Clock, "Armed", clocks[6]),
            row(TimedConstruct::Clock, "Watches", clocks[7]),
        ]
    );

    // A header that selects `quire.value.complete/v1` alone, the least a
    // unit's header selects (S1 requires one `profile`): the `Clock`
    // positions refuse alike, naming that layer.
    let clocks = "function f using v(c: Clock): Boolean pure { true }\n\
                  record Timer { x: Clock; n: Integer; }\n\
                  type Maybe = Option<Clock>;\n";
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{clocks}"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "fr-230-ac-7", "fixture", "fixture:1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let refusals = qsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler retains each Clock for S3's gate")
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .expect_err("S3 requires the temporal facet");
    let clocks = clock_offsets(&text);
    let value = |owner: &str, at: usize| {
        (
            TimedConstruct::Clock,
            owner.to_owned(),
            vec![HeaderLayer::ValueComplete],
            at,
        )
    };
    assert_eq!(
        facet_refusals(&refusals),
        [
            value("f", clocks[0]),
            value("Timer", clocks[1]),
            value("Maybe", clocks[2])
        ]
    );
}

/// The byte offset of every `Clock` type written in `text`, in order.
fn clock_offsets(text: &str) -> Vec<usize> {
    text.match_indices("Clock")
        .map(|(at, _)| at)
        .filter(|at| !text[..*at].ends_with("native/"))
        .collect()
}

/// FR-230 "Records, tuples and unions": under the timed facet a
/// clock-bearing value read whole refuses `clock-read-outside-constraint`
/// at its type, here as the target of `convert`, as the type of
/// `allInstances` and as an accumulation's type, and a literal of a
/// clock-bearing record outside a reset refuses `clock-write` at the
/// literal; S3 checks no other construct of their declarations, and a
/// `Clock` in a declared type position is admitted.
#[trace("TC-685")]
#[test]
fn a_clock_bearing_value_inside_an_expression_refuses_by_its_use() {
    let body = "invariant B using v on Config::ConfigVersion at current { \
                convert<Clock>(self) = convert<Clock>(self) }\n\
                record R { x: Clock; n: Integer; }\n\
                invariant C using v on Config::ConfigVersion at current { \
                count<R>(r in allInstances<R>(Config::config_history): true) = 0 }\n\
                function mk using v(): Integer pure { let r = R { x: 0 ms, n: 1 } in r.n }\n";
    let refusals = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        &format!("dimension Time;\nunit ms : Time = rational(1, 1);\n{body}"),
    )
    .expect("the assembler retains each clock-bearing use for S3")
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .expect_err("no clock rule admits these uses");
    let found: Vec<(&str, usize)> = refusals
        .iter()
        .map(|refusal| match &refusal.cause {
            CheckCause::Temporal(cause) => match **cause {
                TemporalCause::ClockReadOutsideConstraint { span } => ("read", span.start),
                TemporalCause::ClockWrite { span } => ("write", span.start),
                ref other => panic!("a clock refusal expected: {other:?}"),
            },
            other => panic!("a clock refusal expected: {other:?}"),
        })
        .collect();
    let (text, _) = config_unit_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        &format!("dimension Time;\nunit ms : Time = rational(1, 1);\n{body}"),
    );
    let clocks = clock_offsets(&text);
    let at = |needle: &str| text.find(needle).expect("in the unit");
    assert_eq!(
        found,
        [
            ("read", clocks[0]),
            ("read", clocks[1]),
            ("read", at("R>(r in")),
            ("read", at("R>(Config::")),
            ("write", at("R { x: 0 ms")),
        ]
    );
}

const SECONDS_AND_MILLISECONDS: &str =
    "dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\n";

/// The unit `declarations` after a dense `Config` source in milliseconds,
/// checked; the refusals when it does not check.
fn check_delays(
    timed: bool,
    declarations: &str,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    check_delays_under(
        if timed {
            "time Config dense unit ms;\n"
        } else {
            ""
        },
        declarations,
    )
}

/// As [`check_delays`], with `time` the model's time-source line.
fn check_delays_under(
    time: &str,
    declarations: &str,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let body = format!("{SECONDS_AND_MILLISECONDS}{time}{declarations}\n");
    admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        &body,
    )
    .expect("S2 retains the delay declaration")
    .check(quire_semantic_value::checking::CheckingLimits::default())
}

/// The one probabilistic refusal of `declarations`: its code, tag and span text.
fn delay_refusal(timed: bool, declarations: &str) -> (Code, &'static str, String) {
    let refusals = check_delays(timed, declarations).expect_err("the delay is refused");
    let [refusal] = &refusals[..] else {
        panic!("one refusal, got {refusals:?}");
    };
    let CheckCause::Probabilistic(cause) = &refusal.cause else {
        panic!("a probabilistic cause, got {refusal:?}");
    };
    let body = format!(
        "{SECONDS_AND_MILLISECONDS}{}{declarations}\n",
        if timed {
            "time Config dense unit ms;\n"
        } else {
            ""
        }
    );
    let (text, _) = config_unit_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        &body,
    );
    let span = cause.span().expect("a delay refusal carries its span");
    (
        cause.code(),
        cause.tag(),
        text[span.start..span.end].to_owned(),
    )
}

#[trace("TC-708")]
#[test]
fn a_delay_is_recorded_exactly_in_the_models_unit_and_a_free_delay_is_none() {
    let checked = check_delays(
        true,
        "delay ConfigVersion::attemptUpdate ~ uniform[1 s, 3 s];",
    )
    .expect("a delay on a dense model checks");
    let model = checked.probabilistic_model();
    let [delay] = model.delays() else {
        panic!("one delay, got {:?}", model.delays());
    };
    let thousand = |n: i64| quire_exact::Rational::new(n.into(), 1_i64.into()).expect("integer");
    assert_eq!(
        delay.distribution(),
        &qsl_semantics::check::DelayDistribution::UniformRange {
            a: thousand(1000),
            b: thousand(3000),
        }
    );
    assert_eq!(
        model.delay(delay.operation().key()),
        Some(delay.distribution())
    );
    let free = check_delays(true, "").expect("no delay checks");
    assert!(free.probabilistic_model().delays().is_empty());
}

#[trace("TC-708", "FR-253-AC-3")]
#[test]
fn an_exponential_delay_with_a_zero_rate_refuses_at_the_rate() {
    let (code, tag, at) =
        delay_refusal(true, "delay ConfigVersion::attemptUpdate ~ exponential(0);");
    assert_eq!(
        (code, tag, at.as_str()),
        (Code::InvalidModelBinding, "malformed-declaration", "0")
    );
}

#[trace("TC-708", "FR-253-AC-3")]
#[test]
fn a_discrete_delay_listed_twice_refuses_at_the_repeat() {
    let (code, tag, at) = delay_refusal(
        true,
        "delay ConfigVersion::attemptUpdate ~ discrete { 1 ms: 1, 1 ms: 2 };",
    );
    assert_eq!(
        (code, tag, at.as_str()),
        (Code::InvalidModelBinding, "malformed-declaration", "1 ms")
    );
}

#[trace("TC-708", "FR-253-AC-3")]
#[test]
fn a_uniform_range_that_runs_backwards_refuses_at_the_distribution() {
    let (code, tag, at) = delay_refusal(
        true,
        "delay ConfigVersion::attemptUpdate ~ uniform[3 ms, 1 ms];",
    );
    assert_eq!(
        (code, tag, at.as_str()),
        (
            Code::InvalidModelBinding,
            "malformed-declaration",
            "uniform[3 ms, 1 ms]"
        )
    );
}

#[trace("TC-708", "FR-253-AC-3")]
#[test]
fn a_delay_in_an_untimed_model_refuses_at_the_declaration() {
    let declaration = "delay ConfigVersion::attemptUpdate ~ uniform;";
    let (code, tag, at) = delay_refusal(false, declaration);
    assert_eq!(
        (code, tag, at.as_str()),
        (
            Code::InvalidProbabilisticModel,
            "delay-on-untimed-model",
            declaration
        )
    );
}

/// The one probabilistic cause that checking `declarations` under the
/// time-source line `time` refuses, and the source text of its span.
fn delay_cause(
    time: &str,
    declarations: &str,
) -> (qsl_semantics::check::ProbabilisticCause, String) {
    let refusals = check_delays_under(time, declarations).expect_err("the delay is refused");
    let [refusal] = &refusals[..] else {
        panic!("one refusal, got {refusals:?}");
    };
    let CheckCause::Probabilistic(cause) = &refusal.cause else {
        panic!("a probabilistic cause, got {refusal:?}");
    };
    let body = format!("{SECONDS_AND_MILLISECONDS}{time}{declarations}\n");
    let (text, _) = config_unit_with_profiles(
        &document(),
        "profile t = \"quire.temporal.complete/v1\";\n",
        &body,
    );
    let span = cause.span().expect("a delay refusal carries its span");
    ((**cause).clone(), text[span.start..span.end].to_owned())
}

const DENSE: &str = "time Config dense unit ms;\n";
const ATTEMPT: &str = "delay ConfigVersion::attemptUpdate";

/// A `delay` the checker refuses as malformed: its fault and the span text.
fn malformed_delay(declarations: &str) -> (qsl_semantics::check::DelayFault, String) {
    let (cause, at) = delay_cause(DENSE, declarations);
    let qsl_semantics::check::ProbabilisticCause::InvalidDelay { fault, .. } = cause else {
        panic!("an invalid delay, got {cause:?}");
    };
    (fault, at)
}

fn milliseconds(numerator: i64, denominator: i64) -> quire_exact::Rational {
    quire_exact::Rational::new(numerator.into(), denominator.into()).expect("a rational")
}

/// QSL-1110 (SR-17238 FND-001): a `delay` on a `time tick` model refuses
/// like an untimed one, because only a `time dense` source admits delays.
#[trace("TC-708")]
#[test]
fn a_delay_on_a_tick_model_refuses_at_the_declaration() {
    let declaration = format!("{ATTEMPT} ~ uniform;");
    let tick = "time Config tick ConfigVersion::attemptUpdate period 1 ms unit ms;\n";
    assert!(
        check_delays_under(tick, "").is_ok(),
        "the tick source alone checks"
    );
    let (cause, at) = delay_cause(tick, &declaration);
    assert_eq!(
        (cause.code(), cause.tag(), at.as_str()),
        (
            Code::InvalidProbabilisticModel,
            "delay-on-untimed-model",
            declaration.as_str()
        )
    );
}

/// QSL-1110 (SR-17238 FND-002): each malformed distribution is refused with
/// its own fault at the offending value, where the first fault of a
/// `discrete` list wins.
#[trace("TC-708")]
#[test]
fn each_malformed_delay_value_refuses_with_its_own_fault() {
    use qsl_semantics::check::DelayFault;
    for (declaration, fault, at) in [
        ("~ discrete { 1 ms: 0 }", DelayFault::NonPositiveWeight, "0"),
        (
            "~ discrete { 1 ms: 2, 3 ms: rational(-1, 2) }",
            DelayFault::NonPositiveWeight,
            "rational(-1, 2)",
        ),
        (
            "~ discrete { rational(-1, 1) ms: 1 }",
            DelayFault::NegativeDelay,
            "rational(-1, 1) ms",
        ),
        (
            "~ uniform[rational(-1, 1) ms, 2 ms]",
            DelayFault::UniformRange,
            "uniform[rational(-1, 1) ms, 2 ms]",
        ),
    ] {
        let found = malformed_delay(&format!("{ATTEMPT} {declaration};"));
        assert_eq!(found, (fault, at.to_owned()), "{declaration}");
    }
}

/// QSL-1110 (SR-17238 FND-002): a delay on an operation no model declares is
/// unresolved (the second-delay case is
/// `a_second_delay_of_one_operation_refuses_at_the_second_declaration`).
#[trace("TC-708")]
#[test]
fn a_delay_on_an_unknown_operation_refuses_as_unresolved() {
    let (cause, at) = delay_cause(DENSE, "delay ConfigVersion::noSuchOperation ~ uniform;");
    assert!(
        matches!(
            &cause,
            qsl_semantics::check::ProbabilisticCause::UnresolvedOperation { .. }
        ),
        "{cause:?}"
    );
    assert_eq!(
        (cause.code(), cause.tag(), at.as_str()),
        (
            Code::MissingDeclaration,
            "missing-name",
            "ConfigVersion::noSuchOperation"
        )
    );
}

/// QSL-1110 (SR-17238 FND-002): the boundary distributions check and are
/// recorded exactly in the model's unit: `uniform[a, a]`, a zero delay, a
/// `discrete` list in mixed units, `exponential` unconverted and `discrete`
/// weights as written, read back through `CheckedProbabilisticModel::delay`.
#[trace("TC-708")]
#[test]
fn boundary_and_recorded_delay_distributions_read_back_exactly() {
    use qsl_semantics::check::DelayDistribution;
    let recorded = |declaration: &str| -> DelayDistribution {
        let checked = check_delays(true, &format!("{ATTEMPT} ~ {declaration};"))
            .unwrap_or_else(|refusals| panic!("{declaration} checks: {refusals:?}"));
        let model = checked.probabilistic_model();
        let [delay] = model.delays() else {
            panic!("one delay, got {:?}", model.delays());
        };
        model
            .delay(delay.operation().key())
            .expect("the delay is recorded under its operation")
            .clone()
    };
    assert_eq!(
        recorded("uniform[2 ms, 2 ms]"),
        DelayDistribution::UniformRange {
            a: milliseconds(2, 1),
            b: milliseconds(2, 1),
        }
    );
    assert_eq!(
        recorded("uniform[0 ms, 0 ms]"),
        DelayDistribution::UniformRange {
            a: milliseconds(0, 1),
            b: milliseconds(0, 1),
        }
    );
    assert_eq!(
        recorded("discrete { 0 ms: 1 }"),
        DelayDistribution::Discrete(vec![(milliseconds(0, 1), milliseconds(1, 1))])
    );
    assert_eq!(
        recorded("discrete { 1 ms: 1, 1 s: 2, 0 ms: 3 }"),
        DelayDistribution::Discrete(vec![
            (milliseconds(1, 1), milliseconds(1, 1)),
            (milliseconds(1000, 1), milliseconds(2, 1)),
            (milliseconds(0, 1), milliseconds(3, 1)),
        ])
    );
    assert_eq!(
        recorded("discrete { 1 ms: 90, 3 ms: 10 }"),
        DelayDistribution::Discrete(vec![
            (milliseconds(1, 1), milliseconds(90, 1)),
            (milliseconds(3, 1), milliseconds(10, 1)),
        ])
    );
    assert_eq!(
        recorded("exponential(rational(1, 1000))"),
        DelayDistribution::Exponential {
            rate: milliseconds(1, 1000),
        }
    );
}

#[trace("TC-708", "FR-253-AC-3")]
#[test]
fn a_second_delay_of_one_operation_refuses_at_the_second_declaration() {
    let first = "delay ConfigVersion::attemptUpdate ~ uniform;";
    let second = "delay ConfigVersion::attemptUpdate ~ exponential(1);";
    let (code, tag, at) = delay_refusal(true, &format!("{first}\n{second}"));
    assert_eq!(
        (code, tag, at.as_str()),
        (Code::InvalidModelBinding, "malformed-declaration", second)
    );
}

/// The fixture package as `identity`, with a base dimension `Time` and the
/// units `(name, target, scale)`, and the clock field `ConfigVersion.x` when
/// `clocked` (a tick model's package must hold no clock).
pub(crate) fn time_package(
    identity: &str,
    clocked: bool,
    units: &[(&str, Option<&str>, &str)],
) -> Vec<u8> {
    // A clocked package's `attemptUpdate` also writes `x`.
    let base = if clocked {
        let mut operation = attempt_update_modifies_version_number();
        operation["frame"]["modifies"]
            .as_array_mut()
            .expect("a modifies list")
            .push(json!(config_version_identity("x")));
        clock_document_over(config_version_document(
            operation,
            Vec::new(),
            Vec::new(),
            json!([]),
        ))
    } else {
        document()
    };
    let mut package: serde_json::Value = serde_json::from_str(
        &String::from_utf8(base)
            .expect("UTF-8")
            .replace("example/config-version", identity),
    )
    .expect("the renamed fixture is JSON");
    let origin = package["types"][0]["origin"].clone();
    let time = format!("ix://{identity}/dimension/Time");
    package["dimensions"] = json!([{
        "identity": time, "name": "Time", "base": true, "terms": [], "origin": origin,
    }]);
    package["units"] = units
        .iter()
        .map(|(name, target, scale)| {
            let mut unit = json!({
                "identity": format!("ix://{identity}/unit/{name}"), "name": name,
                "dimension": time, "scale": scale, "offset": "0", "origin": origin,
            });
            if let Some(target) = target {
                unit["target"] = json!(format!("ix://{identity}/unit/{target}"));
            }
            unit
        })
        .collect();
    serde_json::to_vec(&package).expect("JSON")
}

/// `R` (`Time`, root `s`, `ms` of `1/1000`), clocked unless the body
/// declares a tick source.
fn rpc(body: &str) -> (&'static str, &'static str, Vec<u8>) {
    (
        "R",
        "example/config-version",
        time_package(
            "example/config-version",
            !body.contains(" tick "),
            &[("s", None, "1"), ("ms", Some("s"), "1/1000")],
        ),
    )
}

/// `F` (`Time`, root `sec` only).
fn flow() -> (&'static str, &'static str, Vec<u8>) {
    (
        "F",
        "example/flow",
        time_package("example/flow", false, &[("sec", None, "1")]),
    )
}

/// `G` (`Time`, root `msec`, `sec` of `1000`).
fn clock_package() -> (&'static str, &'static str, Vec<u8>) {
    (
        "G",
        "example/clock",
        time_package(
            "example/clock",
            false,
            &[("msec", None, "1"), ("sec", Some("msec"), "1000")],
        ),
    )
}

/// The unit selecting `packages` (alias, identity, document) under the timed
/// facet, with a source `Time` and `s` and then `body`, and its S3 outcome;
/// the unit's text for locating refusals.
fn check_selected(
    packages: &[(&str, &str, Vec<u8>)],
    body: &str,
) -> (
    String,
    Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>>,
) {
    let (text, assembled) = assemble_selected(packages, body);
    let checked = assembled
        .unwrap_or_else(|refusal| panic!("the unit assembles: {refusal:?}"))
        .with_unwritten_timed_members()
        .check(quire_semantic_value::checking::CheckingLimits::default());
    (text, checked)
}

/// [`check_selected`]'s unit, assembled.
fn assemble_selected(
    packages: &[(&str, &str, Vec<u8>)],
    body: &str,
) -> (
    String,
    Result<qsl_semantics::check::PackageDeclarations, qsl_semantics::check::AssemblyRefusal>,
) {
    assemble_selected_as(("test", "fr-230-time-owners"), packages, body)
}

/// [`assemble_selected`] under the source owner `(authority, identity)`.
pub(crate) fn assemble_selected_as(
    (authority, identity): (&str, &str),
    packages: &[(&str, &str, Vec<u8>)],
    body: &str,
) -> (
    String,
    Result<qsl_semantics::check::PackageDeclarations, qsl_semantics::check::AssemblyRefusal>,
) {
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::intake::{admit_unit, package_input};
    let inputs = package_input(packages.iter().map(|(_, _, bytes)| bytes.as_slice()));
    let selections: String = packages
        .iter()
        .map(|(alias, identity, bytes)| {
            let digest = inputs
                .iter()
                .find(|(_, document)| *document == bytes)
                .map(|(digest, _)| qsl_semantics::model::key::hex(digest))
                .expect("a supplied document");
            format!(
                "model {alias} = \"{identity}\" version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n"
            )
        })
        .collect();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n\
         profile tt = \"quire.temporal.timed/v1\";\n\
         {selections}dimension Time;\nunit s : Time = rational(1, 1);\n{body}\n"
    );
    let built = crate::model_operations::parse_and_build(&text);
    let models = admit_unit(
        &built.selections().models,
        &inputs,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 admits the packages: {refusal:?}"));
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new(authority, identity, "fixture", "fixture:1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit")
    .source()
    .reference()
    .clone();
    let assembled =
        qsl_semantics::check::PackageDeclarations::assemble(raw, built, models, Vec::new());
    (text, assembled)
}

/// The unit selecting `R` and `F`, with `body`.
fn check_two(
    body: &str,
) -> (
    String,
    Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>>,
) {
    check_selected(&[rpc(body), flow()], body)
}

/// The one time-quantity refusal of `body` over `packages`: its code, tag
/// and the text it is located at.
fn time_refusal(packages: &[(&str, &str, Vec<u8>)], body: &str) -> (Code, &'static str, String) {
    let (text, checked) = check_selected(packages, body);
    let refusals = checked.expect_err("refused");
    let [refusal] = refusals.as_slice() else {
        panic!("{body}: one refusal: {refusals:?}");
    };
    let CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("{body}: a timed refusal: {refusal:?}");
    };
    let span = cause.span();
    (
        cause.code(),
        cause.tag(),
        text[span.start..span.end].to_owned(),
    )
}

/// FR-230-AC-18 (TC-685 step 7): with `R` (`Time`, `s`, `ms`) and `F`
/// (`Time`, `sec`) both selected and a source `Time` with `s`, and no
/// dimension equivalence, `time R dense unit R::ms` checks over `R`'s
/// `Time`, and `x <= 3 R::ms` and, under `unit R::ms`, `period
/// rational(1,2) R::s` check;
/// `time R dense unit F::sec` checks over `F`'s `Time`, and `time R dense
/// unit s` over the source `Time`. Under `R::ms`, `x <= 3 F::sec` and
/// `x <= 3 s` refuse `ill_typed`/`type-mismatch` at the constant, and under
/// `time R tick ... unit R::ms` (over a variant of `R` with no clock) the
/// period `1 F::sec` refuses at the period;
/// a unit `len` of a base dimension `Length` and a unit of a derived
/// `Duration = Time` refuse `time dense unit` `ill_typed`/`type-mismatch`.
/// FR-142's conversion refuses the same way on the other paths that read a
/// time quantity: a task's `wcet 1 F::sec` at the quantity and a timed
/// interval bound `3 F::sec` at its unit.
/// The three models' package identities are `qsl-package`'s
/// `three_time_dimensions_give_three_package_identities`.
#[trace("TC-685", "FR-230-AC-18")]
#[test]
fn time_dimensions_of_different_owners_do_not_mix() {
    let source_of = |body: &str| {
        let (_, checked) = check_two(body);
        let graph = checked.unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
        let [source] = graph.time_sources() else {
            panic!("one time source");
        };
        (source.time_dimension, source.unit.clone())
    };
    let (rpc_time, _) = source_of("time R dense unit R::ms;");
    let (flow_time, _) = source_of("time R dense unit F::sec;");
    let (own_time, _) = source_of("time R dense unit s;");
    assert_ne!(rpc_time, flow_time);
    assert_ne!(rpc_time, own_time);
    assert_ne!(flow_time, own_time);

    let (_, checked) = check_two(
        "time R dense unit R::ms;\n\
         pre P using v on R::ConfigVersion::attemptUpdate { self.x <= 3 R::ms }\n",
    );
    checked.expect("a constant of R's Time checks");
    let (_, checked) = check_two(
        "time R tick R::ConfigVersion::attemptUpdate period rational(1,2) R::s unit R::ms;\n",
    );
    checked.expect("a period of R's Time checks");

    for (body, at) in [
        (
            "time R dense unit R::ms;\n\
             pre P using v on R::ConfigVersion::attemptUpdate { self.x <= 3 F::sec }\n",
            "3 F::sec",
        ),
        (
            "time R dense unit R::ms;\n\
             pre P using v on R::ConfigVersion::attemptUpdate { self.x <= 3 s }\n",
            "3 s",
        ),
        (
            "time R tick R::ConfigVersion::attemptUpdate period 1 F::sec unit R::ms;\n",
            "1 F::sec",
        ),
        (
            "dimension Length;\nunit len : Length = rational(1, 1);\ntime R dense unit len;\n",
            "len",
        ),
        (
            "time R dense unit R::ms;\n\
             taskset Ctl {\n\
             task a periodic 4 R::ms deadline 4 R::ms wcet 1 F::sec priority 1 lo;\n\
             }\n",
            "1 F::sec",
        ),
        (
            "time R dense unit R::ms;\n\
             temporal C using tt over (c: R::ConfigVersion) clock \"model-time\" on origin \
             { eventually[0 R::ms, 3 F::sec] holds(c.versionNumber = 2) }\n",
            "F::sec",
        ),
        (
            "dimension Duration = Time;\nunit d : Duration = rational(1, 1);\n\
             time R dense unit d;\n",
            "d",
        ),
    ] {
        let (text, checked) = check_two(body);
        let refusals = checked.expect_err("refused");
        let [refusal] = refusals.as_slice() else {
            panic!("{body}: one refusal: {refusals:?}");
        };
        let CheckCause::Temporal(cause) = &refusal.cause else {
            panic!("{body}: a timed refusal: {refusal:?}");
        };
        let span = cause.span();
        assert_eq!(
            (cause.code(), cause.tag(), &text[span.start..span.end]),
            (Code::IllTyped, "type-mismatch", at),
            "{body}"
        );
    }
}

/// The clock bound of `body`'s one guard and the value of its one reset, in
/// the model's unit.
fn guard_and_reset(
    packages: &[(&str, &str, Vec<u8>)],
    body: &str,
) -> (quire_exact::Rational, quire_exact::Rational) {
    let (_, checked) = check_selected(packages, body);
    let graph = checked.unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
    let model = graph.timed_model("R").expect("R's timed model");
    let [guard] = model.guards() else {
        panic!("one guard");
    };
    let [part] = guard.parts() else {
        panic!("one convex part");
    };
    let [bound] = part.clocks() else {
        panic!("one clock constraint");
    };
    let [resets] = model.resets() else {
        panic!("one resetting clause");
    };
    let [reset] = resets.resets() else {
        panic!("one reset");
    };
    (bound.bound().clone(), reset.value().clone())
}

/// The period of `body`'s one tick source.
fn tick_period(packages: &[(&str, &str, Vec<u8>)], body: &str) -> quire_exact::Rational {
    let (_, checked) = check_selected(packages, body);
    let graph = checked.unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
    match &graph.time_sources()[0].source {
        qsl_semantics::check::TimeSourceKind::Tick { period, .. } => period.clone(),
        other => panic!("a tick source: {other:?}"),
    }
}

fn whole(value: i64) -> quire_exact::Rational {
    quire_exact::Rational::from_integer(value.into())
}

const GUARDED: &str = "time R dense unit R::ms;\n\
     pre P using v on R::ConfigVersion::attemptUpdate { self.x <= BOUND }\n\
     post Q using v on R::ConfigVersion::attemptUpdate { self.x = RESET }\n";

fn guarded(equivalence: &str, bound: &str, reset: &str) -> String {
    format!(
        "{equivalence}\n{}",
        GUARDED.replace("BOUND", bound).replace("RESET", reset)
    )
}

/// Part of FR-230-AC-27 (TC-685 step 16): over AC-18's `R` and `F` with
/// `dimension R::Time ~ F::Time by 1 R::s = 1 F::sec;` (root ratio 1),
/// under `time R dense unit R::ms` the bound `x <= 3 F::sec` converts to
/// `3000`, a reset of `x` to `2 F::sec` to `2000`, and `time R tick ...
/// period 1 F::sec unit R::ms` checks with period `1000`; `x <= 3 s` over
/// the unrelated source `Time` still refuses `ill_typed`/`type-mismatch` at
/// the constant, and the units `len` of `Length` and `d` of `Duration =
/// Time` still refuse `time dense unit`. The identity half (three package
/// identities, and one that differs without the equivalence) waits on
/// STD-493 item 9: no checked timed member reaches S4, and QSpec FR-322's
/// closed `scalar_type` forms in IR do not yet hold `dimension_equivalence`.
#[trace("TC-685")]
#[test]
fn an_equivalence_of_equal_roots_converts_the_other_owners_time() {
    const EQUIVALENT: &str = "dimension R::Time ~ F::Time by 1 R::s = 1 F::sec;";
    let packages = |body: &str| [rpc(body), flow()];
    let body = guarded(EQUIVALENT, "3 F::sec", "2 F::sec");
    assert_eq!(
        guard_and_reset(&packages(&body), &body),
        (whole(3000), whole(2000))
    );
    let tick = format!(
        "{EQUIVALENT}\ntime R tick R::ConfigVersion::attemptUpdate period 1 F::sec unit R::ms;"
    );
    assert_eq!(tick_period(&packages(&tick), &tick), whole(1000));
    for (body, at) in [
        (guarded(EQUIVALENT, "3 s", "0 R::ms"), "3 s"),
        (
            format!(
                "{EQUIVALENT}\ndimension Length;\nunit len : Length = rational(1, 1);\n\
                 time R dense unit len;"
            ),
            "len",
        ),
        (
            format!(
                "{EQUIVALENT}\ndimension Duration = Time;\nunit d : Duration = rational(1, 1);\n\
                 time R dense unit d;"
            ),
            "d",
        ),
    ] {
        assert_eq!(
            time_refusal(&packages(&body), &body),
            (Code::IllTyped, "type-mismatch", at.to_owned()),
            "{body}"
        );
    }
}

/// Part of FR-230-AC-28 (TC-685 step 16): with `R` and `G` (root `msec`, `sec` of
/// `1000`) and `dimension R::Time ~ G::Time by 1 R::s = 1000 G::msec;`
/// (root ratio 1000), under `time R dense unit R::ms` the bound `x <= 3
/// G::msec` converts to `3`, a reset to `2000 G::msec` to `2000`, and a
/// tick period of `1 G::sec` to `1000`; with `by 1 R::s = 1 G::msec` (root
/// ratio 1) the same bound converts to `3000`; `x <= 3 s` over the source
/// `Time` refuses `ill_typed`/`type-mismatch`. The identity half waits on
/// STD-493 item 9, as AC-27's.
#[trace("TC-685")]
#[test]
fn an_equivalence_of_unequal_roots_carries_its_root_ratio() {
    const THOUSAND: &str = "dimension R::Time ~ G::Time by 1 R::s = 1000 G::msec;";
    const ONE: &str = "dimension R::Time ~ G::Time by 1 R::s = 1 G::msec;";
    let packages = |body: &str| [rpc(body), clock_package()];
    let body = guarded(THOUSAND, "3 G::msec", "2000 G::msec");
    assert_eq!(
        guard_and_reset(&packages(&body), &body),
        (whole(3), whole(2000))
    );
    let tick = format!(
        "{THOUSAND}\ntime R tick R::ConfigVersion::attemptUpdate period 1 G::sec unit R::ms;"
    );
    assert_eq!(tick_period(&packages(&tick), &tick), whole(1000));
    let body = guarded(ONE, "3 G::msec", "0 R::ms");
    assert_eq!(
        guard_and_reset(&packages(&body), &body),
        (whole(3000), whole(0))
    );
    let body = guarded(THOUSAND, "3 s", "0 R::ms");
    assert_eq!(
        time_refusal(&packages(&body), &body),
        (Code::IllTyped, "type-mismatch", "3 s".to_owned())
    );
}
