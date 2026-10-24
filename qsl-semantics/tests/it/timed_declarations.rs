// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393 builds the timed declarations and the `Clock` type at S2; FR-230
//! checks them at S3 under the timed-complete facet. Run over TC-458's
//! `ConfigVersion` fixture package (`crate::model_operations`).

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{AssemblyCause, CheckCause, TemporalCause, TimedConstruct};
use serde_json::json;

use crate::model_operations::{
    add_sub_type, admit_and_assemble_with_body, admit_and_assemble_with_profiles,
    attempt_update_modifies_version_number, config_unit_with_body, config_version_document,
    config_version_identity, tc465_deep_hierarchy_document, wire_field,
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
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("the time source checks beside the Clock field");
    assert_eq!(checked.time_sources()[0].model, "Config");
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
    let body = "dimension Time;\ndimension Duration = Time;\nunit d : Duration = rational(1, 1);\ntime Config dense unit d;\n";
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
        matches!(&checked.time_sources()[0].source, qsl_semantics::check::TimeSourceKind::Tick { operation, period } if operation == &config_version_identity("attemptUpdate") && *period == quire_exact::Rational::new(1_i64.into(), 2_i64.into()).expect("half"))
    );
}

#[trace("TC-685")]
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
}

#[trace("TC-685")]
#[test]
fn tick_operation_on_a_derived_object_resolves_its_inherited_declaration() {
    let body = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick Deep1::attemptUpdate period 1 ms unit ms;\n";
    let declarations = admit_and_assemble_with_profiles(
        &tc465_deep_hierarchy_document(1),
        "profile t = \"quire.temporal.complete/v1\";\n",
        body,
    )
    .expect("S2 keeps the derived owner name");
    let checked = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("S3 resolves the inherited operation");
    assert!(
        matches!(&checked.time_sources()[0].source, qsl_semantics::check::TimeSourceKind::Tick { operation, .. } if operation == &config_version_identity("attemptUpdate"))
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

#[trace("TC-685")]
#[test]
fn an_unqualified_tick_operation_declared_twice_is_ambiguous() {
    let refusals = tick_over(&overriding_document(), "attemptUpdate")
        .expect_err("two declarations share the name");
    assert!(refusals.iter().any(|refusal| matches!(&refusal.cause, CheckCause::Temporal(cause) if matches!(&**cause, TemporalCause::AmbiguousTickOperation { name, .. } if name == "attemptUpdate"))), "{refusals:?}");
}

#[trace("TC-685")]
#[test]
fn a_qualified_tick_operation_resolves_to_the_overriding_declaration() {
    let sources = tick_over(&overriding_document(), "Sub::attemptUpdate")
        .expect("the override hides the inherited declaration");
    assert!(
        matches!(&sources[0].source, qsl_semantics::check::TimeSourceKind::Tick { operation, .. } if operation == "ix://example/config-version/Sub/attemptUpdate"),
        "{sources:?}"
    );
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
