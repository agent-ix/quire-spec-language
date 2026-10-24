// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1054 rows 5 to 11: a
//! reachability over a relationship end whose walked population has an
//! authored bound links to the one `bounded_domain`/`model_population` node
//! of `(E, N)`, and one without links to nothing. Every expected value is the
//! TC row's, over TC-1046's fixture R.
//!
//! The rows' "the reader admits the package and selects the population"
//! clause is the last test, which reads each package through IR's v2 reader;
//! every other check reads the emitted wire.

use super::reachability::{
    applications, lower_over, model_node, node_of, reachability_document, relationship, Lowered,
    NO_LOOP,
};
use super::*;

const BOUND: &str = "bound OrdersBound on M::orders [3]";

/// Fixture R with the object types `Rush` (a subtype of `Order`) and
/// `Person`, and the population `people` of `Person`.
pub(super) fn document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&reachability_document()).unwrap();
    let order = document["types"][0].clone();
    let mut rush = order.clone();
    rush["identity"] = json!("ix://acme/orders/Rush");
    rush["displayName"] = json!("Rush");
    rush["supertypes"] = json!(["ix://acme/orders/Order"]);
    rush["fields"] = json!([]);
    rush["relationships"] = json!([]);
    let mut person = rush.clone();
    person["identity"] = json!("ix://acme/orders/Person");
    person["displayName"] = json!("Person");
    person["supertypes"] = json!([]);
    let mut friend = relationship("friend", "source-to-target", "friend", "friendOf");
    friend["identity"] = json!("ix://acme/orders/relationship/Person-friend");
    friend["sourceEnd"]["type"] = json!("ix://acme/orders/Person");
    friend["targetEnd"]["type"] = json!("ix://acme/orders/Person");
    person["relationships"] = json!([friend]);
    document["types"]
        .as_array_mut()
        .unwrap()
        .extend([rush, person]);
    let mut people = document["populations"][0].clone();
    people["identity"] = json!("ix://acme/orders/people");
    people["displayName"] = json!("ix://acme/orders/people");
    people["members"] = json!(["ix://acme/orders/Person"]);
    people["origin"]["generated"]["generatorIdentity"] = json!("ix://acme/orders/people");
    people["origin"]["generated"]["inputIdentities"] = json!(["ix://acme/orders/people"]);
    document["populations"].as_array_mut().unwrap().push(people);
    serde_json::to_vec(&document).unwrap()
}

fn lower(clauses: &str) -> Lowered {
    lower_over(document(), clauses)
}

/// Every `model_population` node of the package.
fn population_nodes(wire: &Value) -> Vec<&Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == "model_population")
        .collect()
}

/// The `operation.member` of the one reachability application.
fn member(lowered: &Lowered) -> &Value {
    let reaches = applications(&lowered.wire, "quire.op.model.reaches");
    let [reach] = reaches.as_slice() else {
        panic!("one reachability node");
    };
    &reach["body"]["operation"]["member"]
}

/// The `type` occurrences of `node`: ordinal and the source text of its
/// source-map region, in ordinal order.
fn type_occurrences<'t>(lowered: &'t Lowered, node: &Value) -> Vec<(u64, &'t str)> {
    let mut found: Vec<(u64, &str)> = lowered.wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == node["node_id"] && entry["role"] == "type")
        .map(|entry| {
            let region = &entry["regions"][0];
            let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
            let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
            (
                entry["ordinal"].as_u64().unwrap(),
                &lowered.text[start..end],
            )
        })
        .collect();
    found.sort_unstable();
    found
}

fn type_roles(node: &Value) -> Vec<&Value> {
    node["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|occurrence| occurrence["role"] == "type")
        .collect()
}

fn binding<'n>(node: &'n Value, name: &str) -> &'n Value {
    node["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == name)
        .unwrap_or_else(|| panic!("no `{name}` binding in {}", node["body"]))
}

fn assert_anonymous(node: &Value) {
    assert!(node.get("declaration").is_none_or(Value::is_null), "{node}");
    assert!(node.get("owner").is_none_or(Value::is_null), "{node}");
}

/// The `Population<Order>[maximum]` chain the population node `population`
/// of `wire` must be: the `Integer` max literal, the `Set<Reference<Order>>`
/// semantic type over the `Reference<Order>` node, and the dependencies.
fn assert_population_node(wire: &Value, population: &Value, maximum: &str) {
    assert_eq!(population["node_tag"], "bounded_domain");
    assert_anonymous(population);
    let max = binding(population, "max");
    assert_eq!(max["value"]["value"], maximum);
    let integer = node_of(wire, &max["value"]["type"]);
    assert_eq!(integer["semantic_form"], "integer");
    assert_eq!(
        population["body"]["members"].as_array().unwrap().len(),
        1,
        "the one binding `max`"
    );
    let set = node_of(wire, &population["semantic_type"]);
    assert_eq!(set["semantic_form"], "set");
    assert_anonymous(set);
    assert_anonymous(integer);
    let reference = node_of(wire, &set["body"]["members"][0]["target"]);
    assert_eq!(reference["semantic_form"], "reference");
    assert_anonymous(reference);
    let order = model_node(wire, "Order");
    assert_eq!(reference["body"]["members"][0]["target"], order["node_id"]);
    // FR-322 "References": a `bounded_domain` node lists exactly its
    // `semantic_type`; the chain `Set<Reference<Order>>` -> `Reference<Order>`
    // -> `Order` is each node's own exact list.
    assert_eq!(population["dependencies"], json!([set["node_id"]]));
    assert_eq!(set["dependencies"], json!([reference["node_id"]]));
    assert_eq!(reference["dependencies"], json!([order["node_id"]]));
}

/// TC-1054 row 5: the member holds `population`,
/// the id of the one anonymous `model_population` node, which holds the one
/// `type` occurrence of the `[3]`.
#[trace("TC-1054", "FR-380-AC-4", "FR-365-AC-13")]
#[test]
fn a_bounded_walked_population_is_linked_as_one_anonymous_node() {
    let lowered = lower(&format!("{BOUND}\n{NO_LOOP}"));
    let wire = &lowered.wire;
    let relation = model_node(wire, "relationship/Order-next");
    let populations = population_nodes(wire);
    let [population] = populations.as_slice() else {
        panic!("one population node");
    };
    assert_eq!(
        member(&lowered),
        &json!({
            "kind": "relationship_end",
            "declaration": relation["node_id"],
            "name": "next",
            "population": population["node_id"],
        })
    );
    assert_population_node(wire, population, "3");
    let reaches = applications(wire, "quire.op.model.reaches");
    assert!(
        reaches[0]["dependencies"]
            .as_array()
            .unwrap()
            .contains(&population["node_id"]),
        "the application's dependencies hold the population node"
    );
    assert_eq!(type_roles(population).len(), 1);
    assert_eq!(type_occurrences(&lowered, population), [(0, "[3]")]);
    assert_eq!(lowered.claim_domains, [0], "the clause's extent is bounded");
}

/// TC-1054 row 6: `[0]` is a valid maximum.
#[trace("TC-1054", "FR-380-AC-4")]
#[test]
fn a_zero_bound_emits_the_node_with_max_zero() {
    let lowered = lower(&format!("bound OrdersBound on M::orders [0]\n{NO_LOOP}"));
    let populations = population_nodes(&lowered.wire);
    let [population] = populations.as_slice() else {
        panic!("one population node");
    };
    assert_population_node(&lowered.wire, population, "0");
    assert_eq!(member(&lowered)["population"], population["node_id"]);
}

/// TC-1054 row 7: without the `bound` the member
/// holds exactly `kind`, `declaration` and `name`, no population node is
/// emitted, and the application keeps its node id.
#[trace("TC-1054", "FR-380-AC-6", "FR-365-AC-13")]
#[test]
fn an_unbounded_walked_population_keeps_its_member_and_node_identity() {
    let unbounded = lower(NO_LOOP);
    let relation = model_node(&unbounded.wire, "relationship/Order-next");
    assert_eq!(
        member(&unbounded),
        &json!({
            "kind": "relationship_end",
            "declaration": relation["node_id"],
            "name": "next",
        })
    );
    assert_eq!(population_nodes(&unbounded.wire), Vec::<&Value>::new());
    // The node id TC-1047 row 1 records is the one of the same clause over
    // the fixture without the extra types and population.
    let plain = super::reachability::lower(NO_LOOP);
    assert_eq!(
        applications(&unbounded.wire, "quire.op.model.reaches")[0]["node_id"],
        applications(&plain.wire, "quire.op.model.reaches")[0]["node_id"],
    );
}

/// TC-1054 row 8: two reachabilities over one population
/// share the link and the one node.
#[trace("TC-1054", "FR-365-AC-13")]
#[test]
fn two_reachabilities_over_one_population_share_one_link() {
    let lowered = lower(&format!(
        "{BOUND}\n{NO_LOOP}\n\
         invariant NoOut using v on M::Order at current {{ not reaches(self, self, outgoing) }}"
    ));
    let populations = population_nodes(&lowered.wire);
    let [population] = populations.as_slice() else {
        panic!("one population node");
    };
    let reaches = applications(&lowered.wire, "quire.op.model.reaches");
    assert_eq!(reaches.len(), 2);
    for reach in reaches {
        assert_eq!(
            reach["body"]["operation"]["member"]["population"],
            population["node_id"]
        );
    }
    assert_eq!(type_occurrences(&lowered, population), [(0, "[3]")]);
}

/// TC-1054 row 14: a claim with reachabilities over two different
/// populations carries two links, each to its own node.
#[trace("TC-1054", "FR-365-AC-13")]
#[test]
fn reachabilities_over_two_populations_carry_two_links_to_two_nodes() {
    let lowered = lower(&format!(
        "{BOUND}\nbound PeopleBound on M::people [2]\n\
         invariant NoLoop using v on M::Order at current {{ not reaches(self, self, next) }}\n\
         invariant NoFriendLoop using v on M::Person at current {{ not reaches(self, self, friend) }}"
    ));
    let populations = population_nodes(&lowered.wire);
    assert_eq!(populations.len(), 2, "one node per population");
    let reaches = applications(&lowered.wire, "quire.op.model.reaches");
    assert_eq!(reaches.len(), 2);
    let links: Vec<&Value> = reaches
        .iter()
        .map(|reach| &reach["body"]["operation"]["member"]["population"])
        .collect();
    assert_ne!(links[0], links[1], "each reachability has its own link");
    for population in &populations {
        assert_eq!(
            links
                .iter()
                .filter(|link| **link == &population["node_id"])
                .count(),
            1,
            "each node is linked once"
        );
    }
    let mut occurrences: Vec<Vec<(u64, &str)>> = populations
        .iter()
        .map(|population| type_occurrences(&lowered, population))
        .collect();
    occurrences.sort();
    assert_eq!(occurrences, [[(0, "[2]")], [(0, "[3]")]]);
}

/// TC-1054 row 9: a bound no reachability walks emits no node
/// and no occurrence.
#[trace("TC-1054", "FR-380-AC-6")]
#[test]
fn a_bound_no_reachability_walks_emits_nothing() {
    let lowered = lower(&format!(
        "{BOUND}\nbound PeopleBound on M::people [2]\n{NO_LOOP}"
    ));
    let populations = population_nodes(&lowered.wire);
    let [population] = populations.as_slice() else {
        panic!("only the `orders` node");
    };
    assert_eq!(member(&lowered)["population"], population["node_id"]);
    assert_eq!(type_occurrences(&lowered, population), [(0, "[3]")]);
    let maxima: Vec<&Value> = populations
        .iter()
        .map(|node| &binding(node, "max")["value"]["value"])
        .collect();
    assert_eq!(maxima, ["3"], "no node for `people`'s [2]");
    let unwalked = lower(&format!("{BOUND}\nbound PeopleBound on M::people [2]"));
    assert_eq!(population_nodes(&unwalked.wire), Vec::<&Value>::new());
}

/// TC-1054 row 10: the node is `Population<Order>[3]` for a
/// clause over `Rush`, whose near end's type is `Order`.
#[trace("TC-1054", "FR-380-AC-5")]
#[test]
fn the_near_ends_type_names_the_population_node() {
    let lowered = lower(&format!(
        "{BOUND}\n\
         invariant NoLoopRush using v on M::Rush at current {{ not reaches(self, self, next) }}"
    ));
    let populations = population_nodes(&lowered.wire);
    let [population] = populations.as_slice() else {
        panic!("one population node");
    };
    // `assert_population_node` finds `Reference<Order>` under the set.
    assert_population_node(&lowered.wire, population, "3");
    assert_eq!(member(&lowered)["population"], population["node_id"]);
}

/// TC-1054 row 11: a `Population<M::Order>[3]` parameter type
/// and a bound of 3 walked by a reachability are one node holding two `type`
/// occurrences, ordinals 0 and 1 in ascending source order, whichever is
/// written first.
#[trace("TC-1054", "FR-380-AC-5")]
#[test]
fn a_parameter_type_and_a_bound_share_one_node_with_two_ordinals() {
    let function = "function f using v(p: Population<M::Order>[3]): Integer pure { 0 }";
    for declarations in [
        format!("{BOUND}\n{NO_LOOP}\n{function}"),
        format!("{function}\n{BOUND}\n{NO_LOOP}"),
    ] {
        let lowered = lower(&declarations);
        let populations = population_nodes(&lowered.wire);
        let [population] = populations.as_slice() else {
            panic!("{declarations}: one population node");
        };
        assert_eq!(member(&lowered)["population"], population["node_id"]);
        let occurrences = type_occurrences(&lowered, population);
        assert_eq!(occurrences.len(), 2, "{declarations}: {occurrences:?}");
        assert_eq!(occurrences[0].0, 0);
        assert_eq!(occurrences[1].0, 1);
        // The lower ordinal is the earlier occurrence in the source: the
        // bound's `[3]` when the bound is written before the function.
        let bound_first = declarations.find(BOUND) < declarations.find(function);
        let bound_ordinal = occurrences
            .iter()
            .find(|(_, text)| *text == "[3]")
            .map(|(ordinal, _)| *ordinal);
        assert_eq!(
            bound_ordinal,
            Some(u64::from(!bound_first)),
            "{declarations}: {occurrences:?}"
        );
    }
}

/// The selector IR's reader returns for each reachability application of
/// `lowered`, with the application's wire `population` member.
fn selections(
    lowered: &Lowered,
) -> Vec<(
    Value,
    quire_contract_model::CheckedNodeId,
    quire_contract_model::CheckedOccurrence,
)> {
    let ir = lowered
        .ir
        .as_ref()
        .expect("IR's v2 reader admits the package");
    applications(&lowered.wire, "quire.op.model.reaches")
        .into_iter()
        .map(|reach| {
            let id = quire_contract_model::CheckedNodeId {
                domain: reach["node_id"]["domain"].as_str().unwrap().into(),
                digest: reach["node_id"]["digest"].as_str().unwrap().into(),
            };
            let selector = ir
                .reaches_population(&id)
                .unwrap_or_else(|error| {
                    panic!("the reader cannot resolve the population: {error:?}")
                })
                .expect("the application names a population");
            (
                reach["body"]["operation"]["member"]["population"].clone(),
                selector.binding,
                selector.occurrence,
            )
        })
        .collect()
}

/// TC-1054 rows 5, 6, 8, 9 and 11, read through IR: the reader admits each
/// package and its `reaches_population` selector names the node the member
/// holds and that node's `Type` occurrence of least ordinal.
#[trace("TC-1054", "FR-380-AC-4")]
#[test]
fn ir_reads_each_bounded_package_and_selects_its_population() {
    let function = "function f using v(p: Population<M::Order>[3]): Integer pure { 0 }";
    let units = [
        format!("{BOUND}\n{NO_LOOP}"),
        format!("bound OrdersBound on M::orders [0]\n{NO_LOOP}"),
        format!(
            "{BOUND}\n{NO_LOOP}\n\
             invariant NoOut using v on M::Order at current {{ not reaches(self, self, outgoing) }}"
        ),
        format!("{BOUND}\nbound PeopleBound on M::people [2]\n{NO_LOOP}"),
        format!("{BOUND}\n{NO_LOOP}\n{function}"),
        format!("{function}\n{BOUND}\n{NO_LOOP}"),
    ];
    for unit in units {
        let lowered = lower(&unit);
        assert!(
            lowered.verified,
            "{unit}: IR's v2 reader admits the package"
        );
        for (member, binding, occurrence) in selections(&lowered) {
            let population = node_of(&lowered.wire, &member);
            assert_eq!(
                json!(binding.digest.as_ref()),
                population["node_id"]["digest"],
                "{unit}: the selector names the member's node"
            );
            let least = type_occurrences(&lowered, population)
                .first()
                .map(|(ordinal, _)| *ordinal);
            assert_eq!(Some(occurrence.ordinal), least, "{unit}");
            assert_eq!(
                occurrence.role,
                quire_contract_model::CheckedOccurrenceRole::Type
            );
        }
    }
}
