// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1076 row 8 (FR-391-AC-6): a state clause that names a domain population
//! `M::P` emits QSpec FR-322's population reference, one `relation`/
//! `population` node per declared population read, as operand 0 of the
//! `all_instances` application, and IR's v2 reader admits the package.

use super::population_bound::document;
use super::reachability::{applications, lower_over, node_of, Lowered};
use super::*;

/// The `acme/orders` document of TC-1054 with a second population `archive`
/// of `Order`, so a clause on `Person` (covered only by `people`) can name
/// either population of `Order`.
fn two_populations() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&document()).unwrap();
    let mut archive = document["populations"][0].clone();
    archive["identity"] = json!("ix://acme/orders/archive");
    archive["displayName"] = json!("ix://acme/orders/archive");
    archive["origin"]["generated"]["generatorIdentity"] = json!("ix://acme/orders/archive");
    archive["origin"]["generated"]["inputIdentities"] = json!(["ix://acme/orders/archive"]);
    document["populations"].as_array_mut().unwrap().push(archive);
    serde_json::to_vec(&document).unwrap()
}

fn clause(name: &str, population: &str) -> String {
    format!(
        "invariant {name} using v on M::Person at current {{ \
         forall(o in allInstances<M::Order>(M::{population}) : true) }}\n"
    )
}

const READS: [(&str, &str); 3] = [("A", "orders"), ("B", "archive"), ("C", "orders")];

fn lower_reads(extra: &str) -> Lowered {
    let clauses: String = READS
        .iter()
        .map(|(name, population)| clause(name, population))
        .collect();
    lower_over(two_populations(), &format!("{extra}{clauses}"))
}

/// The `relation`/`population` nodes of `wire`, by the declaration they own.
fn population_references(wire: &Value) -> Vec<(String, &Value)> {
    let mut found: Vec<(String, &Value)> = nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == "relation" && node["semantic_form"] == "population")
        .map(|node| (node["owner"]["node"].as_str().unwrap().to_owned(), node))
        .collect();
    found.sort_by(|left, right| left.0.cmp(&right.0));
    found
}

/// The population node each `all_instances` application references as
/// operand 0, in the node order of the wire.
fn operand_zero(wire: &Value) -> Vec<Value> {
    applications(wire, "quire.op.model.all_instances")
        .iter()
        .map(|application| application["body"]["arguments"][0].clone())
        .collect()
}

/// FR-391-AC-6: `A` and `B` reference two nodes, one per declared
/// population, `C` the same node as `A`; each node is the FR-322 shape and
/// the full reader admits the package.
#[trace("TC-1076", "FR-391-AC-6")]
#[test]
fn a_named_population_is_one_reference_node_per_declared_population() {
    let lowered = lower_reads("");
    let references = population_references(&lowered.wire);
    let owners: Vec<&str> = references.iter().map(|(owner, _)| owner.as_str()).collect();
    assert_eq!(
        owners,
        ["ix://acme/orders/archive", "ix://acme/orders/orders"]
    );
    for (_, node) in &references {
        assert_eq!(node["owner"]["kind"], "model");
        assert_eq!(node["owner"]["identity"], "acme/orders");
        assert_eq!(node["semantic_type"], node["node_id"], "self-typed");
        assert_eq!(node["body"], json!({"term": "aggregate", "members": []}));
        assert_eq!(node["dependencies"], json!([]));
        assert!(node["declaration"].is_null());
    }
    assert_ne!(references[0].1["node_id"], references[1].1["node_id"]);

    // Operand 0 of each application is a `reference` to its population's
    // node, and the application lists the node as a dependency.
    let operands = operand_zero(&lowered.wire);
    assert_eq!(operands.len(), 2, "A and C are one application");
    for operand in &operands {
        assert_eq!(operand["term"], "reference");
        let target = node_of(&lowered.wire, &operand["target"]);
        assert_eq!(target["semantic_form"], "population");
    }
    let targets: Vec<&Value> = operands.iter().map(|operand| &operand["target"]).collect();
    let orders = &references[1].1["node_id"];
    assert!(targets.contains(&orders), "A and C reference `orders`");
    assert!(targets.contains(&&references[0].1["node_id"]));
    for application in applications(&lowered.wire, "quire.op.model.all_instances") {
        assert!(application["dependencies"]
            .as_array()
            .unwrap()
            .contains(&application["body"]["arguments"][0]["target"]));
    }

    assert!(lowered.verified, "IR's v2 reader admits the package");
    assert!(lowered.ir.is_ok(), "{:?}", lowered.ir.err());
}

/// FR-391-AC-6: the authored maximum enters through the application's
/// `result_type`, never the population reference, so a bound on `orders`
/// leaves its node key unchanged.
#[trace("TC-1076", "FR-391-AC-6")]
#[test]
fn a_population_bound_leaves_the_reference_node_unchanged() {
    let unbounded = lower_reads("");
    let bounded = lower_reads("bound OrdersBound on M::orders [3]\n");
    assert_eq!(
        population_references(&unbounded.wire)
            .iter()
            .map(|(_, node)| node["node_id"].clone())
            .collect::<Vec<_>>(),
        population_references(&bounded.wire)
            .iter()
            .map(|(_, node)| node["node_id"].clone())
            .collect::<Vec<_>>(),
    );
    let orders = population_references(&unbounded.wire)[1].1["node_id"].clone();
    let result_type = |lowered: &Lowered| {
        let found: Vec<Value> = applications(&lowered.wire, "quire.op.model.all_instances")
            .into_iter()
            .filter(|application| application["body"]["arguments"][0]["target"] == orders)
            .map(|application| application["body"]["result_type"].clone())
            .collect();
        let [one] = found.as_slice() else {
            panic!("one application reads `orders`");
        };
        one.clone()
    };
    assert_ne!(
        result_type(&unbounded),
        result_type(&bounded),
        "the maximum enters through the result type"
    );
    assert!(bounded.verified, "IR's v2 reader admits the bounded package");
}
