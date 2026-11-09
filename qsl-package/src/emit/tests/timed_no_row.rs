// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337 "Timed clause emission" (TC-1326, QSpec FR-322 "Temporal clock
//! bindings", FR-322-AC-386, QSpec FR-415): a clause under
//! `quire.temporal.timed/v1` selects none of the three clock configurations,
//! so the emitter never writes a `temporal_bindings` row for it. A clause
//! over a model subject takes its model time from the package's
//! `timed_model` root and `time_source`, which the emitter writes and the
//! reader joins to the clause's model subject.

use quire_contract_model::{
    CheckedPackageRefusalCause as Cause, CheckedPackageRefusalCode as RefusalCode,
};

use super::temporal_clause::{node_with, target};
use super::*;
use crate::checked_v2::{read_checked_package_v2, V2ReadRefusal};
use crate::emit::{EmitRefusal, TemporalClockSelection};

const TIMED: &str = "quire.temporal.timed/v1";
const SOURCE: &str = "time Config dense unit ms;\n";
const BODY: &str = "always [0 ms, 5 ms] holds(c.versionNumber = 2)";

/// The ConfigVersion domain package without operations: the timed model
/// then needs no operation anchor.
fn document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(
        super::admission_corpus::config_version_unit::DOMAIN_PACKAGE.as_bytes(),
    )
    .expect("the fixture is JSON");
    document["types"][1]["operations"] = json!([]);
    crate::emit::tests::wire_bytes(&document).expect("the fixture remains JSON")
}

/// The unit selecting `profile` as `t`, the time source `source` and the
/// clause `Bound` over the ConfigVersion model subject.
fn unit(profile: &str, source: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile c = \"quire.temporal.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model Config = {:?};\n\
         dimension Time;\nunit ms : Time = rational(1, 1);\n\
         {source}\
         temporal Bound using t over (c: Config::ConfigVersion) clock \"model-time\" \
         on origin {{ {BODY} }}\n",
        super::admission_corpus::config_version_unit::PACKAGE_IDENTITY,
    )
}

fn checked(
    unit: &str,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let document = document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckingLimits::default(),
            quire_semantic_value::checking::CheckMode::Linked,
        )
}

fn evidence_for(wire: &Value) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let document = document();
    evidence
        .insert_domain_package_document(package_identity(&document), document)
        .expect("a fresh evidence key");
    evidence
}

fn nodes_of<'w>(wire: &'w Value, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect()
}

fn member<'n>(node: &'n Value, name: &str) -> &'n Value {
    node["body"]["members"]
        .as_array()
        .expect("members")
        .iter()
        .find(|member| member["name"] == name)
        .unwrap_or_else(|| panic!("a `{name}` member of {node}"))
}

/// The emission of the Timed unit with no clock selection supplied.
fn emitted(unit: &str) -> (Emission, Value) {
    let graph = checked(unit).unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    (emission, wire)
}

/// FR-337-AC-25 (QSpec FR-322-AC-386): a Timed clause with no clock
/// selection emits, and the emission writes no `temporal_bindings` row for
/// it, neither in the bytes nor in the rows the emission returns, while its
/// name row is written.
#[trace("TC-1326", "FR-337-AC-25", "FR-322-AC-386")]
#[test]
fn a_timed_clause_emits_without_a_temporal_binding_row() {
    let (emission, wire) = emitted(&unit(TIMED, SOURCE));
    assert_eq!(emission.temporal_rows().bindings, []);
    assert!(
        wire.get("temporal_bindings").is_none(),
        "no member: {}",
        wire["temporal_bindings"]
    );
    let [name] = &wire["temporal_clause_names"].as_array().unwrap()[..] else {
        panic!("one name row");
    };
    assert_eq!(name["name"], "Bound");
    assert_eq!(name["occurrence"], 0);
    let [reference] = &emission.temporal_rows().clauses[..] else {
        panic!("one clause reference");
    };
    assert_eq!(
        serde_json::to_value(&reference.declaration).unwrap(),
        name["declaration"]
    );
}

/// FR-337-AC-25: the clause's `model-time` clock is the clause's own text
/// and its model subject joins the written `timed_model` root and
/// `time_source`, whose unit is the declared `ms`; QSL's I2 read admits the
/// package with the Timed clause and no row.
#[trace("TC-1326", "FR-337-AC-25", "FR-322-AC-386")]
#[test]
fn a_timed_clause_over_a_model_joins_the_timed_model_root_and_time_source() {
    let (emission, wire) = emitted(&unit(TIMED, SOURCE));
    let [clause] = &nodes_of(&wire, "temporal_clause")[..] else {
        panic!("one temporal clause");
    };
    assert_eq!(clause["body"]["arguments"][1]["value"], "model-time");
    let [root] = &nodes_of(&wire, "timed_model")[..] else {
        panic!("one timed_model root");
    };
    let [source] = &nodes_of(&wire, "time_source")[..] else {
        panic!("one time_source");
    };
    assert_eq!(member(root, "source")["value"]["target"], source["node_id"]);
    let unit_node = &member(source, "unit")["value"]["target"];
    let [ms] = emission.selected_source_unit_rows() else {
        panic!("one source-owned unit row");
    };
    assert_eq!(
        ms.qualified_declaration
            .iter()
            .map(|segment| segment.as_ref())
            .collect::<Vec<_>>(),
        ["ms"]
    );
    assert_eq!(
        unit_node,
        &serde_json::to_value(&ms.structural_node).unwrap()
    );
    // The clause's `over` subject is the root's model.
    let over = target(&clause["body"]["arguments"][0]);
    let model = &member(root, "model")["value"]["target"];
    assert_eq!(
        model["path"][2],
        "ix://example/config-version/ConfigVersion"
    );
    let subject = node_with(&wire, &node_with(&wire, over)["semantic_type"]);
    assert_eq!(subject["body"]["members"][0]["target"], *model, "{subject}");
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence_for(&wire),
    );
    assert!(read.is_ok(), "the I2 read admits the no-row Timed clause");
}

/// FR-337-AC-26 (QSpec FR-322-AC-386): a Timed clause over a model with no
/// `time` declaration has no time source to join, so the check refuses it
/// and no package is written.
#[trace("TC-1326", "FR-337-AC-26", "FR-322-AC-386")]
#[test]
fn a_timed_clause_over_a_model_without_a_time_source_refuses() {
    let source = unit(TIMED, "");
    let refusals = checked(&source).expect_err("no time source to bind");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let qsl_semantics::check::CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("a temporal refusal: {refusal:?}");
    };
    let qsl_semantics::check::TemporalCause::ModelTimeOnUntimedModel { model, span } = &**cause
    else {
        panic!("a model-time refusal: {refusal:?}");
    };
    assert_eq!(model.as_str(), "Config");
    assert_eq!(&source[span.start..span.end], "\"model-time\"");
}

/// FR-337-AC-27 (QSpec FR-322-AC-386): a clock selection supplied for a
/// Timed clause is a surplus selection: the emitter refuses it at the
/// clause's span and writes no package, whatever configuration is supplied.
#[trace("TC-1326", "FR-337-AC-27", "FR-322-AC-386")]
#[test]
fn a_clock_selection_supplied_for_a_timed_clause_is_refused() {
    use quire_contract_model::ClockConfiguration as Clock;
    let unit = unit(TIMED, SOURCE);
    let graph = checked(&unit).expect("the unit checks");
    let [temporal] = graph.temporal_clauses() else {
        panic!("one temporal clause");
    };
    let span = temporal.clause().span();
    let source = graph.source().clone();
    let package = CheckedPackage::link(graph);
    for clock in [
        Clock::EventPosition {
            sequence_authority: "test-order-events".into(),
        },
        Clock::TimestampedEvent {
            timestamp_unit: "test-rfc3339".into(),
        },
    ] {
        let selection = TemporalClockSelection::new(source.clone(), span, clock);
        let refusal = crate::emit::emit_checked_with_temporal_selections(
            &package,
            &[selection],
            crate::EmitLimits::fixture(),
        )
        .expect_err("a Timed clause takes no clock selection");
        match &refusal {
            EmitRefusal::SurplusTemporalClockSelection {
                source_ref,
                span: refused,
            } => {
                assert_eq!(source_ref, &source);
                assert_eq!(*refused, span);
            }
            other => panic!("a surplus selection refusal, not {other:?}"),
        }
        assert_eq!(refusal.code(), qsl_foundation::Code::InvalidPackage);
        assert_eq!(refusal.cause(), Some("timed-node-reference"));
    }
}

/// FR-337-AC-28 (QSpec FR-322-AC-386): the reader refuses a `temporal_bindings`
/// row supplied for a Timed clause `invalid_package`/`timed-node-reference`
/// at that row, whichever configuration it carries; it admits the same
/// package without the row.
#[trace("TC-1326", "FR-337-AC-28", "FR-322-AC-386")]
#[test]
fn the_reader_refuses_a_supplied_row_for_a_timed_clause_at_the_row() {
    let (emission, wire) = emitted(&unit(TIMED, SOURCE));
    let declaration = wire["temporal_clause_names"][0]["declaration"].clone();
    let mut supplied = wire.clone();
    supplied["temporal_bindings"] = json!([{
        "declaration": declaration,
        "occurrence": 0,
        "clock_name": "model-time",
        "clock": {"kind": "event_position", "sequence_authority": "test-order-events"},
    }]);
    let refusal = match read_v2(
        &crate::emit::tests::wire_bytes(&supplied).unwrap(),
        library(),
        V2ReadLimits::fixture(),
        &evidence_for(&wire),
    ) {
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => *refusal,
        other => panic!("an IR refusal, not {other:?}"),
    };
    assert_eq!(
        (refusal.code, refusal.cause),
        (RefusalCode::InvalidPackage, Some(Cause::TimedNodeReference))
    );
    assert_eq!(
        refusal.loci[0].path.as_ref().map(|path| path.as_str()),
        Some("/temporal_bindings/0")
    );
    assert!(read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence_for(&wire),
    )
    .is_ok());
}
