// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-180 / QSpec FR-477 (TC-1143): an HP-1 relation is emitted as a typed
//! step-relation node and admitted by the complete QSL I2 reader.

use super::admission_corpus::config_version_unit;
use super::*;

pub(super) fn document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

fn document_digest() -> String {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    qsl_semantics::model::key::hex(digest)
}

fn relation_unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         relation Det using t over (x: Config::ConfigVersion::attemptUpdate, y: Config::ConfigVersion::attemptUpdate) \
         {{ x.result = y.result and pre(x.self.versionNumber) = pre(y.self.versionNumber) implies x.self.versionNumber = y.self.versionNumber }}\n",
        document_digest()
    )
}

pub(super) fn checked_graph() -> qsl_semantics::check::CheckedGraph {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    assemble_with_models(relation_unit().as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"))
}

/// FR-180/FR-477: both execution binders remain distinct, their body is
/// represented in the checked wire, and I2 admits the emitted package.
#[trace("TC-1143", "FR-180-AC-2", "FR-180-AC-3")]
#[test]
fn a_step_relation_emits_and_reads_back_its_execution_body() {
    let graph = checked_graph();
    assert_eq!(graph.step_relations().len(), 1);
    assert_eq!(graph.requirements().len(), 1);
    assert_eq!(
        graph
            .requirements()
            .values()
            .next()
            .expect("the model claim record")
            .requirements()
            .kind(),
        qsl_semantics::check::Capability::TemporalSatisfaction
    );
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let relation = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "step_relation")
        .expect("the step-relation node is present");
    assert_eq!(relation["node_tag"], "claim");
    let executions = relation["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "executions")
        .unwrap();
    let executions = executions["value"]["members"].as_array().unwrap();
    assert_eq!(executions.len(), 2);
    let parameter_levels: Vec<_> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "parameter")
        .filter_map(|node| node["body"]["members"].as_array())
        .filter_map(|members| {
            members
                .iter()
                .find(|member| member["name"] == "level")
                .and_then(|member| member["value"]["value"].as_str())
        })
        .collect();
    assert_eq!(
        parameter_levels
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        ["0", "1", "2", "3"].into_iter().collect()
    );
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(document_digest(), document().to_vec())
        .expect("the fixture evidence is unique");
    let admitted = crate::checked_v2::read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    )
    .unwrap_or_else(|failure| panic!("I2 admits the step relation: {failure:?}"))
    .into_value()
    .admitted;
    let typed = admitted
        .graph()
        .nodes
        .iter()
        .find(|node| node.semantic_form.as_ref() == "step_relation")
        .expect("the reader retains the relation node");
    let typed_body = typed.step_relation_body().expect("the typed relation body");
    assert_eq!(typed_body.executions.len(), 2);
    assert_eq!(typed_body.executions[0].variable.as_ref(), "x");
    assert_eq!(typed_body.executions[1].variable.as_ref(), "y");
}
