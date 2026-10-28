// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-8: S4 emits and reads back compensation templates.

use super::*;
use super::child_composite::fixture;

fn emitted_compensation_unit() -> (Emission, Value, Read) {
    let document = serde_json::to_vec(&fixture::document()).expect("fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model M = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         protocol Flow using v over (input: M::Ledger) on origin {{\n\
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
           activate first (trigB: M::Money) when {{ true }} {{ }}\n\
           within [0, 10]; attempts 1 of M::Money;\n\
           retry (earlyB: M::Money, lateB: M::Money) {{ false }};\n\
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
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{unit}\n{refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("unit emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(digest.as_str(), document.as_slice())
        .expect("fresh evidence key");
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    );
    (emission, wire, read)
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
