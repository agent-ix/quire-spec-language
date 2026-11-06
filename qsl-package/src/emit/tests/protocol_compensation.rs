// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-8: S4 emits and reads back compensation templates.

use super::child_composite::fixture;
use super::*;
use qsl_semantics::check::{CheckCause, CheckRefusal, ProtocolConstruct, ProtocolReferenceCause};

fn emitted_compensation_unit() -> (Emission, Value, Read) {
    emitted_unit("", "true", "false").expect("unit checks")
}

/// The unit with `declarations` before `Flow`, whose `Partial` template's
/// trigger guard is `guard` and retry relation is `retry`.
fn unit_text(declarations: &str, guard: &str, retry: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         profile t = \"quire.temporal.event-position.false-extension/v1\";\n\
         model M = {package:?};\n\
         {declarations}\n\
         protocol Flow using p over (input: M::Ledger) on origin {{\n\
         role R on M::Ledger;\n\
         compensate Full for Main::Applied as (fwd: M::Money) by R \
           on M::Ledger::pay using t clock \"c\" {{\n\
           capture saved: M::Money = fwd;\n\
           activate first (trigA: M::Money) when {{ trigA = saved }} {{\n\
             capture activatedA: M::Money = trigA;\n\
           }}\n\
           within [0, 10]; attempts 1 of M::Money;\n\
           retry (earlyA: M::Money, lateA: M::Money) {{ false }};\n\
           commit Main::Finalized; recover (recA: M::Money) {{ recA = saved }};\n\
         }}\n\
         compensate Partial for Main::Applied as (partial: M::Money) by R \
           on M::Ledger::pay using t clock \"c\" {{\n\
           activate first (trigB: M::Money) when {{ {guard} }} {{ }}\n\
           within [0, 10]; attempts 1 of M::Money;\n\
           retry (earlyB: M::Money, lateB: M::Money) {{ {retry} }};\n\
           commit never; recover (recB: M::Money) {{ true }};\n\
         }}\n\
         run sequence Main {{\n\
           attempt Update by R on M::Ledger::pay contracts [] \
             as (tried: M::Money) {{ true }};\n\
           effect Applied of Update as (applied: M::Money) {{ true }};\n\
           commit Finalized by R as (committed: Boolean) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n}}\n",
        package = fixture::PACKAGE,
    )
}

/// [`unit_text`] checked and emitted, with the check's refusals when it has
/// any.
fn emitted_unit(
    declarations: &str,
    guard: &str,
    retry: &str,
) -> Result<(Emission, Value, Read), Vec<CheckRefusal>> {
    emitted_text(&unit_text(declarations, guard, retry))
}

fn emitted_text(unit: &str) -> Result<(Emission, Value, Read), Vec<CheckRefusal>> {
    let document = serde_json::to_vec(&fixture::document()).expect("fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("unit assembles")
        .check(CheckingLimits::default())?;
    let selections = graph
        .protocols()
        .iter()
        .flat_map(|protocol| &protocol.obligations.compensations)
        .filter(|template| template.temporal_profile.is_some())
        .map(|template| crate::emit::TemporalClockSelection {
            source: graph.source().clone(),
            declaration_span: template.span,
            clock: quire_contract_model::ClockConfiguration::EventPosition {
                sequence_authority: "compensation-events".into(),
            },
        })
        .collect::<Vec<_>>();
    let emission = crate::emit::emit_checked_with_temporal_selections(
        &CheckedPackage::link(graph),
        &selections,
        crate::EmitLimits::fixture(),
    )
    .expect("unit emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("fresh evidence key");
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    );
    Ok((emission, wire, read))
}

/// FR-383-AC-20, TC-1063 row 8i: a compensation retains its temporal-profile
/// requirement even when no temporal clause is present; the package without
/// the selection is `missing_declaration`/`missing-selection`.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_compensation_without_a_selected_temporal_profile_is_refused_by_i2() {
    let unit = unit_text("", "true", "false")
        .replace(
            "temporal Due using t over (c: M::Ledger) clock \"steps\" on origin { always true }\n",
            "",
        )
        .replace("requires temporal Due;\n", "");
    let (emission, mut wire, read) = emitted_text(&unit).expect("unit checks");
    assert!(
        matches!(read, Read::Verified { .. }),
        "positive control: {read:?}"
    );
    wire["lock"]["profile_selections"]
        .as_array_mut()
        .expect("lock selections")
        .retain(|selection| selection["role"] != "temporal_profile");
    let mut evidence = read_evidence(&emission);
    let document = serde_json::to_vec(&fixture::document()).unwrap();
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .unwrap();
    let outcome = read_v2(&jcs(&wire), library(), V2ReadLimits::fixture(), &evidence);
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) = outcome else {
        panic!("missing temporal selection must refuse: {outcome:?}");
    };
    assert_eq!(
        refusal.code,
        quire_contract_model::CheckedPackageRefusalCode::MissingDeclaration
    );
    assert_eq!(
        refusal.cause,
        Some(quire_contract_model::CheckedPackageRefusalCause::MissingSelection)
    );
    assert_eq!(
        refusal.loci[0].path.as_ref().map(|path| path.as_str()),
        Some("/semantic_graph/nodes/1")
    );
}

#[trace("FR-383-AC-8", "TC-1061")]
#[test]
fn full_and_partial_compensation_templates_emit_and_read_back() {
    let (_, wire, read) = emitted_compensation_unit();
    assert!(
        matches!(read, Read::Verified { .. }),
        "I2 admits the compensation package: {read:?}"
    );

    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let templates: Vec<&Value> = nodes
        .iter()
        .filter(|node| node["semantic_form"] == "compensation")
        .collect();
    assert_eq!(templates.len(), 2);
    let clause = nodes
        .iter()
        .find(|node| node["semantic_form"] == "protocol_clause")
        .unwrap();
    let args = clause["body"]["arguments"].as_array().unwrap();
    let listed = args
        .iter()
        .find(|arg| arg["name"] == "compensations")
        .unwrap();
    assert_eq!(listed["value"]["members"].as_array().unwrap().len(), 2);

    for template in &templates {
        let names: Vec<&str> = template["body"]["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| arg["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "forward",
                "forward_binder",
                "role",
                "operation",
                "clock",
                "registration_captures",
                "trigger",
                "activation_captures",
                "within",
                "attempts",
                "retry",
                "commit",
                "recovery",
            ]
        );
    }
    let full = templates
        .iter()
        .find(|node| node["declaration"]["qualified_name"][2] == "Full")
        .unwrap();
    let args = full["body"]["arguments"].as_array().unwrap();
    assert_eq!(args[5]["value"]["members"].as_array().unwrap().len(), 1);
    assert_eq!(args[7]["value"]["members"].as_array().unwrap().len(), 1);
    assert_eq!(args[11]["value"]["members"].as_array().unwrap().len(), 1);
    let partial = templates
        .iter()
        .find(|node| node["declaration"]["qualified_name"][2] == "Partial")
        .unwrap();
    assert_eq!(
        partial["body"]["arguments"][11]["value"]["members"],
        json!([]),
        "commit never is the absent commit reference"
    );
}

/// QSL-1131, QSL-851: a compensation template's guard and retry relation are
/// bodies like any other. One that calls a function is emitted and admitted;
/// one that refers to a protocol directly is refused typed
/// (`ProtocolReferenceInBody`), whatever the other protocol's `finish`
/// binder is called.
#[test]
fn a_compensation_body_calls_a_function_but_cannot_refer_to_a_protocol() {
    let other = "protocol Other using p over (input: M::Ledger) on origin {\n\
                 role R on M::Ledger;\nrun sequence Main { }\n\
                 finish End as (done: Boolean) { true };\n}\n";
    let ok = "function ok using v(): Boolean pure { true }\n";
    let (_, _, read) = emitted_unit(ok, "ok()", "ok()").expect("calls check");
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let refused = CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::NotEmitted {
        construct: ProtocolConstruct::ProtocolReferenceInBody,
    }));
    for (guard, retry) in [("protocol Other", "false"), ("true", "protocol Other")] {
        let Err(refusals) = emitted_unit(other, guard, retry) else {
            panic!("a protocol reference in a compensation body is refused: {guard} {retry}");
        };
        assert_eq!(refusals.len(), 1, "{guard} {retry}");
        assert_eq!(refusals[0].cause, refused, "{guard} {retry}");
    }
}

/// The `(name, level)` of the `parameter` node `reference` names.
fn parameter<'w>(nodes: &'w [Value], reference: &Value) -> (&'w str, &'w str) {
    let node = nodes
        .iter()
        .find(|node| node["node_id"] == reference["target"])
        .unwrap_or_else(|| panic!("{reference} names a node"));
    assert_eq!(node["semantic_form"], "parameter", "{node}");
    let members = &node["body"]["members"];
    (
        members[0]["value"]["value"].as_str().unwrap(),
        members[1]["value"]["value"].as_str().unwrap(),
    )
}

/// QSL-1555: a query binder written inside a compensation body is lowered
/// as the `parameter` its source names. A `sum` in each of the `Partial`
/// trigger guard, retry relation and recovery predicate binds `g`, `r` and
/// `line`, and an activation capture of `Full` that reads the registration
/// capture `saved` reads `saved`'s own parameter, not the protocol input's.
#[trace("FR-383-AC-8", "TC-1061")]
#[test]
fn a_sum_binder_in_a_recovery_predicate_is_a_parameter_named_by_its_source() {
    let unit = unit_text(
        "type Total = Integer;",
        "sum<Total>(g in trigB.lines: g.amount_minor) >= 0",
        "sum<Total>(r in lateB.lines: r.amount_minor) >= 0",
    )
    .replace(
        "activate first (trigB: M::Money)",
        "activate first (trigB: M::Lines)",
    )
    .replace(
        "attempts 1 of M::Money;\nretry (earlyB: M::Money, lateB: M::Money)",
        "attempts 1 of M::Lines;\nretry (earlyB: M::Lines, lateB: M::Lines)",
    )
    .replace(
        "recover (recB: M::Money) { true }",
        "recover (recB: M::Lines) { sum<Total>(line in recB.lines: line.amount_minor) >= 0 }",
    )
    .replace(
        "capture activatedA: M::Money = trigA;",
        "capture activatedA: M::Money = saved;",
    );
    let (_, wire, _) =
        emitted_text(&unit).unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let nodes = nodes(&wire);
    let template = |name: &str| {
        nodes
            .iter()
            .find(|node| {
                node["semantic_form"] == "compensation"
                    && node["declaration"]["qualified_name"][2] == name
            })
            .unwrap_or_else(|| panic!("the template {name}"))
    };
    let argument = |template: &'_ Value, name: &str| -> Value {
        template["body"]["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|argument| argument["name"] == name)
            .unwrap_or_else(|| panic!("the argument {name}"))["value"]
            .clone()
    };

    let sums: Vec<&Value> = nodes
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.collection.sum.integer")
        .collect();
    let mut bound: Vec<&Value> = sums
        .iter()
        .map(|sum| {
            let binding = &sum["body"]["arguments"][1];
            assert_eq!(binding["term"], "binding", "{sum}");
            &binding["name"]
        })
        .collect();
    bound.sort_by_key(|name| name.as_str());
    assert_eq!(bound, [&json!("g"), &json!("line"), &json!("r")]);
    for name in ["g", "line", "r"] {
        let parameters: Vec<&Value> = nodes
            .iter()
            .filter(|node| {
                node["semantic_form"] == "parameter"
                    && node["body"]["members"][0]["value"]["value"] == name
            })
            .collect();
        assert_eq!(
            parameters.len(),
            1,
            "one parameter node `{name}`: {parameters:?}"
        );
    }

    let recovery = argument(template("Partial"), "recovery");
    assert_eq!(
        parameter(nodes, &recovery["members"][0]["value"]).0,
        "recB",
        "{recovery}"
    );

    let activation = argument(template("Full"), "activation_captures");
    let capture = &activation["members"][0];
    assert_eq!(
        parameter(nodes, &capture["members"][0]["value"]).0,
        "activatedA"
    );
    let read = &capture["members"][1]["value"];
    assert_eq!(parameter(nodes, read), ("saved", "2"), "{capture}");
}
