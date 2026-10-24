// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230-AC-32/AC-38: a checked time source reaches S4 as typed timed nodes,
//! preserves its resolved model/unit identities, and passes I2 read-back.

use super::admission_corpus::config_version_unit;
use super::*;
use crate::checked_v2::{read_checked_package_v2, CheckedTimedModel};
use quire_contract_model::CheckedPackageEvidence;

fn package_units_document() -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(config_version_unit::DOMAIN_PACKAGE.as_bytes())
            .expect("the fixture is JSON");
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

#[trace("FR-230-AC-32", "FR-230-AC-38")]
#[test]
fn a_checked_time_source_emits_and_reads_back_its_resolved_nodes() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = config_version_unit::unit_text(&qsl_semantics::model::key::hex(digest)).replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!(
        "{unit}dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the time source checks: {refusals:?}"));
    let [checked_source] = graph.time_sources() else {
        panic!("one checked time source");
    };
    let model_node = checked_source.model_node.to_string();
    let unit_node = checked_source.unit_node.to_string();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);

    let wire = wire(&emission);
    assert!(
        nodes(&wire)
            .iter()
            .any(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "text"),
        "the timed source metadata's Text type is in the graph"
    );
    let [root] = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "timed_model")
        .collect::<Vec<_>>()[..]
    else {
        panic!("one emitted timed root");
    };
    let time_dimension = root["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "time_dimension")
        .unwrap()["value"]["target"]["digest"]
        .as_str()
        .unwrap();
    assert!(
        nodes(&wire)
            .iter()
            .any(|node| node["node_id"]["digest"] == time_dimension),
        "the time dimension node {time_dimension} is in the emitted graph"
    );
    let model = root["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "model")
        .unwrap();
    assert_eq!(model["value"]["target"]["digest"], model_node);
    let source_id = root["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "source")
        .unwrap()["value"]["target"]["digest"]
        .as_str()
        .unwrap();
    let source_node = nodes(&wire)
        .iter()
        .find(|node| node["node_id"]["digest"] == source_id)
        .expect("the root references its typed source");
    assert_eq!(source_node["semantic_form"], "time_source");
    assert_eq!(
        source_node["body"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["name"] == "unit")
            .unwrap()["value"]["target"]["digest"],
        unit_node
    );

    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(qsl_semantics::model::key::hex(digest), document.to_vec())
        .expect("a fresh evidence key");
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the emitted timed model: {failure:?}"));
    let typed = CheckedTimedModel::from_admitted(&read.into_value().admitted)
        .expect("I2 reconstructs the typed timed model");
    let [timed] = typed.as_slice() else {
        panic!("one timed model reads back");
    };
    assert_eq!(timed.model.digest.as_ref(), model_node);
    assert_eq!(
        timed.source.kind,
        crate::checked_v2::CheckedTimeSourceKind::Dense
    );
    assert!(timed.clock_places.is_empty());
    assert!(timed.resets.is_empty());
    assert!(timed.guards.is_empty());
    assert!(timed.invariants.is_empty());
    assert!(timed.urgencies.is_empty());
    assert!(timed.normalized_constraints.is_empty());
}

#[trace("FR-230-AC-5", "FR-230-AC-32", "FR-230-AC-38")]
#[test]
fn a_package_owned_time_unit_emits_its_model_owned_nominal_nodes() {
    let document = package_units_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = config_version_unit::unit_text(&qsl_semantics::model::key::hex(digest)).replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!("{unit}time Config dense unit Config::ms;\n");
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package time source checks: {refusals:?}"));
    let [checked_source] = graph.time_sources() else {
        panic!("one checked time source");
    };
    let dimension_node = checked_source.time_dimension.to_string();
    let unit_node = checked_source.unit_node.to_string();
    let emission =
        emit_checked(&CheckedPackage::link(graph)).expect("the package-owned timed model emits");
    let wire = wire(&emission);
    for digest in [&dimension_node, &unit_node] {
        assert!(
            nodes(&wire)
                .iter()
                .any(|node| node["node_id"]["digest"].as_str() == Some(digest)),
            "the model-owned nominal node {digest} is emitted"
        );
    }
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(qsl_semantics::model::key::hex(digest), document)
        .expect("a fresh evidence key");
    read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits package-owned time nodes: {failure:?}"));
}

#[trace("FR-230-AC-32", "FR-230-AC-35")]
#[test]
fn a_tick_time_source_emits_its_exact_period_and_operation_anchor() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = config_version_unit::unit_text(&qsl_semantics::model::key::hex(digest)).replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!(
        "{unit}dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config tick ConfigVersion::attemptUpdate period 1 ms unit ms;\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the tick source checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the tick model emits");
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(qsl_semantics::model::key::hex(digest), document.to_vec())
        .expect("a fresh evidence key");
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the tick model: {failure:?}"));
    let typed = CheckedTimedModel::from_admitted(&read.into_value().admitted)
        .expect("I2 reconstructs the tick model");
    let [timed] = typed.as_slice() else {
        panic!("one timed model reads back");
    };
    assert_eq!(
        timed.source.kind,
        crate::checked_v2::CheckedTimeSourceKind::Tick
    );
    assert_eq!(timed.source.period.as_deref(), Some("1/1"));
    assert!(timed.source.operation.is_some());
}
