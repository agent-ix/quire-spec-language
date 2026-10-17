// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-365-AC-5, FR-365-AC-6 (TC-1047): a checked `reaches` over a relationship
//! end lowers to one `quire.op.model.reaches` application naming the
//! relationship's model node and the end's role, over the two references,
//! with no `deref`; a proof binds it by the `state_clause` node identity and
//! the path of argument positions; IR's v2 reader admits the package. Every
//! expected value is the TC-1047 row's.

use super::*;

fn multiplicity() -> Value {
    json!({"lower": 0, "upper": 1, "ordered": false, "unique": true})
}

/// A finite multi-valued multiplicity `{0,3}`.
fn many(unique: bool) -> Value {
    json!({"lower": 0, "upper": 3, "ordered": false, "unique": unique})
}

/// A relationship `Order/<id>` of fixture R, `Order` at both ends.
pub(super) fn relationship(
    id: &str,
    direction: &str,
    source_role: &str,
    target_role: &str,
) -> Value {
    json!({
        "identity": format!("ix://acme/orders/relationship/Order-{id}"),
        "category": "structural",
        "composite": false,
        "direction": direction,
        "origin": {"source": {
            "sourceIdentity": "ix://acme/orders/spec",
            "path": "spec.qspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "sourceEnd": {
            "type": "ix://acme/orders/Order",
            "role": source_role,
            "multiplicity": multiplicity(),
        },
        "targetEnd": {
            "type": "ix://acme/orders/Order",
            "role": target_role,
            "multiplicity": multiplicity(),
        },
    })
}

/// A `source-to-target` relationship `Order/<id>` whose target end holds
/// `target_multiplicity` of `Order`.
fn multi_valued_relationship(id: &str, source_role: &str, target_multiplicity: Value) -> Value {
    let mut relationship = relationship(id, "source-to-target", source_role, "");
    relationship["targetEnd"] = json!({
        "type": "ix://acme/orders/Order",
        "multiplicity": target_multiplicity,
    });
    relationship
}

/// The `acme/orders` document with the object type `Order` (the field `trail`
/// and the relationships `Order/next`, `Order/link` and `Order/mate`) and the
/// population `orders`.
pub(super) fn reachability_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(SPINE_MODEL_DOCUMENT).unwrap();
    let mut order = document["types"][0].clone();
    order["identity"] = json!("ix://acme/orders/Order");
    order["displayName"] = json!("Order");
    order["supertypes"] = json!([]);
    order["operations"] = json!([]);
    let mut trail = order["fields"][0].clone();
    trail["identity"] = json!("ix://acme/orders/Order/trail");
    trail["name"] = json!("trail");
    trail["typeRef"] = json!("ix://acme/orders/Order");
    trail["presence"] = json!("optional");
    trail["multiplicity"] = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
    order["fields"] = json!([trail]);
    order["relationships"] = json!([
        relationship("next", "source-to-target", "next", "previous"),
        relationship("link", "bidirectional", "outgoing", "incoming"),
        relationship("mate", "undirected", "mate", "mateOf"),
        multi_valued_relationship("kids", "kids", many(true)),
        multi_valued_relationship("litter", "litter", many(false)),
    ]);
    document["types"] = json!([order]);
    document["constructs"].as_array_mut().unwrap().push(json!({
        "kind": {"module": "acme/orders", "name": "population"},
        "moduleVersion": "1.0.0",
        "manifestDigest":
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "construct": {
            "identity": "none",
            "shape": "record",
            "members": {},
            "meaning": "quire.meaning.model.population/v1",
        },
    }));
    document["populations"] = json!([{
        "identity": "ix://acme/orders/orders",
        "displayName": "ix://acme/orders/orders",
        "kind": {"module": "acme/orders", "name": "population"},
        "members": ["ix://acme/orders/Order"],
        "extent": "closed",
        "origin": {"generated": {
            "generatorIdentity": "ix://acme/orders/orders",
            "generatorVersion": "1.0.0",
            "inputIdentities": ["ix://acme/orders/orders"],
        }},
    }]);
    serde_json::to_vec(&document).unwrap()
}

/// The unit holding `clauses` over the document, emitted, with the checked
/// graph's claim-domain counts and the document's reader verdict.
pub(super) struct Lowered {
    /// The compiled unit's source text.
    pub(super) text: String,
    pub(super) wire: Value,
    pub(super) verified: bool,
    /// IR's v2 reading of the package alone, when it admits it.
    pub(super) ir: Option<Box<quire_contract_model::CheckedPackageV2>>,
    /// The unbounded population domains each `claim` occurrence's
    /// requirement record names, from the checked graph.
    pub(super) claim_domains: Vec<usize>,
}

pub(super) fn lower(clauses: &str) -> Lowered {
    lower_over(reachability_document(), clauses)
}

/// [`lower`] over the domain package `document`.
pub(super) fn lower_over(document: Vec<u8>, clauses: &str) -> Lowered {
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         {clauses}\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{clauses}: {refusals:?}"));
    let claim_domains = graph
        .requirements()
        .iter()
        .filter(|(key, _)| key.origin().role().as_str() == "claim")
        .map(|(_, record)| match record.requirements().extent() {
            qsl_semantics::family::ClaimExtent::Unbounded(domains) => domains.len(),
            qsl_semantics::family::ClaimExtent::Bounded => 0,
        })
        .collect();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(digest.as_str(), &document[..])
        .expect("a fresh evidence key");
    let wire = wire(&emission);
    let mut features = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        features.support_feature(feature.as_str().unwrap());
    }
    let ir = match quire_contract_model::read_checked_package(
        emission.package.bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &features,
    ) {
        quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(admitted) => Some(admitted),
        _ => None,
    };
    Lowered {
        text: unit,
        wire,
        ir,
        verified: matches!(read_with(&emission, &evidence), Read::Verified { .. }),
        claim_domains,
    }
}

pub(super) fn applications<'w>(wire: &'w Value, identity: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == identity)
        .collect()
}

pub(super) fn node_of<'w>(wire: &'w Value, id: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .expect("the referenced node is written")
}

/// The written node whose owner is the declaration `node`.
pub(super) fn model_node<'w>(wire: &'w Value, node: &str) -> &'w Value {
    let found: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|candidate| candidate["owner"]["node"] == format!("ix://acme/orders/{node}"))
        .collect();
    let [declared] = found.as_slice() else {
        panic!("one model node for {node}");
    };
    declared
}

fn state_clauses(wire: &Value) -> Vec<&Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == "state_clause")
        .collect()
}

/// The state clause's condition: the node its third argument references.
fn condition<'w>(wire: &'w Value, clause: &Value) -> &'w Value {
    let arguments = clause["body"]["arguments"].as_array().unwrap();
    node_of(wire, &arguments[2]["target"])
}

fn claims(clause: &Value) -> Vec<u64> {
    clause["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|occurrence| occurrence["role"] == "claim")
        .map(|occurrence| occurrence["ordinal"].as_u64().unwrap())
        .collect()
}

pub(super) const NO_LOOP: &str =
    "invariant NoLoop using v on M::Order at current { not reaches(self, self, next) }";

/// TC-1047 row 1: the lowered node's every part.
#[trace("TC-1047", "FR-365-AC-5")]
#[test]
fn a_reachability_over_an_end_lowers_to_one_reaches_application() {
    let lowered = lower(NO_LOOP);
    let wire = &lowered.wire;
    let reaches = applications(wire, "quire.op.model.reaches");
    let [reach] = reaches.as_slice() else {
        panic!("one reachability node");
    };
    let body = &reach["body"];
    assert_eq!(reach["semantic_form"], "reachability");
    assert_eq!(body["operator"], "reaches");
    let operation = &body["operation"];
    assert_eq!(operation["identity"], "quire.op.model.reaches");
    assert_eq!(operation["laws"], json!([]));
    assert_eq!(operation["leaves"], json!([]));
    assert_eq!(operation["mode"], Value::Null);
    let relation = model_node(wire, "relationship/Order-next");
    assert_eq!(
        operation["member"],
        json!({
            "kind": "relationship_end",
            "declaration": relation["node_id"],
            "name": "next",
        })
    );
    let arguments = body["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    assert_eq!(arguments[0]["term"], "reference");
    assert_eq!(arguments[1]["term"], "reference");
    assert_eq!(
        arguments[0], arguments[1],
        "both reference self's parameter node"
    );
    let parameter = &arguments[0]["target"];
    assert_eq!(
        applications(wire, "quire.op.model.deref"),
        Vec::<&Value>::new()
    );
    let boolean = node_of(wire, &body["result_type"]);
    assert_eq!(boolean["semantic_form"], "boolean");
    assert_eq!(reach["semantic_type"], body["result_type"]);
    let mut dependencies = reach["dependencies"].as_array().unwrap().clone();
    let mut expected = vec![parameter.clone(), relation["node_id"].clone()];
    dependencies.sort_by_key(|id| id.to_string());
    expected.sort_by_key(|id| id.to_string());
    assert_eq!(
        dependencies, expected,
        "the parameter node and the relationship node"
    );
    let condition = state_clauses(wire)
        .into_iter()
        .map(|clause| condition(wire, clause))
        .next()
        .expect("one state clause");
    assert_eq!(
        condition["body"]["operation"]["identity"],
        "quire.op.boolean.not"
    );
    assert_eq!(
        condition["body"]["arguments"][0]["target"],
        reach["node_id"]
    );
    assert!(lowered.verified, "IR's v2 reader admits the package");
}

/// TC-1047 row 2: the role, not the relationship alone, is named.
#[trace("TC-1047", "FR-365-AC-5")]
#[test]
fn the_member_names_the_role_over_the_relationship() {
    for (role, relationship) in [("incoming", "link"), ("mateOf", "mate")] {
        let lowered = lower(&format!(
            "invariant NoLoop using v on M::Order at current {{ not reaches(self, self, {role}) }}"
        ));
        let reaches = applications(&lowered.wire, "quire.op.model.reaches");
        let [reach] = reaches.as_slice() else {
            panic!("one reachability node for {role}");
        };
        let relation = model_node(&lowered.wire, &format!("relationship/Order-{relationship}"));
        assert_eq!(
            reach["body"]["operation"]["member"],
            json!({
                "kind": "relationship_end",
                "declaration": relation["node_id"],
                "name": role,
            })
        );
        assert!(lowered.verified, "{role}: the reader admits the package");
    }
}

/// TC-1047 row 8: a finite multi-valued destination end lowers to the same
/// node shape as a single-valued one, and the reader admits the package.
#[trace("TC-1047", "FR-365-AC-5")]
#[test]
fn a_multi_valued_destination_lowers_to_the_same_node_shape() {
    for role in ["kids", "litter"] {
        let lowered = lower(&format!(
            "invariant NoLoop using v on M::Order at current {{ not reaches(self, self, {role}) }}"
        ));
        let reaches = applications(&lowered.wire, "quire.op.model.reaches");
        let [reach] = reaches.as_slice() else {
            panic!("one reachability node for {role}");
        };
        let relation = model_node(&lowered.wire, &format!("relationship/Order-{role}"));
        assert_eq!(
            reach["body"]["operation"]["member"],
            json!({
                "kind": "relationship_end",
                "declaration": relation["node_id"],
                "name": role,
            })
        );
        assert_eq!(reach["body"]["arguments"].as_array().unwrap().len(), 2);
        assert_eq!(
            applications(&lowered.wire, "quire.op.model.deref"),
            Vec::<&Value>::new()
        );
        assert!(lowered.verified, "{role}: the reader admits the package");
    }
}

/// TC-1047 row 3: the field-edge form lowers as before.
#[trace("TC-1047", "FR-365-AC-5")]
#[test]
fn the_field_edge_form_still_lowers_to_reaches_field() {
    let lowered =
        lower("invariant NoLoop using v on M::Order at current { not reaches(self, self, trail) }");
    assert_eq!(
        applications(&lowered.wire, "quire.op.model.reaches"),
        Vec::<&Value>::new()
    );
    let fields = applications(&lowered.wire, "quire.op.model.reaches_field");
    let [field] = fields.as_slice() else {
        panic!("one reaches_field application");
    };
    let order = model_node(&lowered.wire, "Order");
    assert_eq!(
        field["body"]["operation"]["member"],
        json!({"kind": "field", "declaration": order["node_id"], "name": "trail"})
    );
}

/// TC-1047 row 4: one reachability node under two `state_clause` nodes whose
/// conditions differ, each with its own `claim` occurrence.
#[trace("TC-1047", "FR-365-AC-6")]
#[test]
fn one_reachability_is_one_node_under_two_clause_identities() {
    let lowered = lower(
        "invariant A using v on M::Order at current { not reaches(self, self, next) }\n\
         invariant B using v on M::Order at current { reaches(self, self, next) }",
    );
    let wire = &lowered.wire;
    let reaches = applications(wire, "quire.op.model.reaches");
    let [reach] = reaches.as_slice() else {
        panic!("one reachability node");
    };
    let clauses = state_clauses(wire);
    assert_eq!(clauses.len(), 2, "two state_clause nodes");
    assert_ne!(clauses[0]["node_id"], clauses[1]["node_id"]);
    assert_ne!(
        condition(wire, clauses[0])["node_id"],
        condition(wire, clauses[1])["node_id"],
        "the conditions differ"
    );
    for clause in &clauses {
        assert_eq!(claims(clause), vec![0], "its own claim occurrence");
    }
    let referencing: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| {
            node["body"]["arguments"]
                .as_array()
                .is_some_and(|arguments| arguments.iter().any(|a| a["target"] == reach["node_id"]))
        })
        .collect();
    assert!(!referencing.is_empty());
}

/// TC-1047 row 4b: two clauses of equal kind, parameters, anchor and
/// condition are one `state_clause` node with two `claim` occurrences,
/// ordinals 0 and 1 in source order.
#[trace("TC-1047", "FR-365-AC-6")]
#[test]
fn two_equal_clauses_are_one_node_with_two_claim_occurrences() {
    let lowered = lower(
        "invariant A using v on M::Order at current { not reaches(self, self, next) }\n\
         invariant B using v on M::Order at current { not reaches(self, self, next) }",
    );
    let wire = &lowered.wire;
    assert_eq!(applications(wire, "quire.op.model.reaches").len(), 1);
    let clauses = state_clauses(wire);
    let [clause] = clauses.as_slice() else {
        panic!("one state_clause node");
    };
    assert_eq!(claims(clause), vec![0, 1]);
}

/// TC-1047 row 5: one reachability written twice in a condition is one node
/// on two argument paths, positions 0 and 1 of the `and` application.
#[trace("TC-1047", "FR-365-AC-6")]
#[test]
fn one_reachability_written_twice_is_one_node_on_two_argument_paths() {
    let lowered = lower(
        "invariant Twice using v on M::Order at current \
         { reaches(self, self, next) and reaches(self, self, next) }",
    );
    let wire = &lowered.wire;
    let reaches = applications(wire, "quire.op.model.reaches");
    let [reach] = reaches.as_slice() else {
        panic!("one reachability node");
    };
    let clauses = state_clauses(wire);
    let [clause] = clauses.as_slice() else {
        panic!("one state_clause node");
    };
    let and = condition(wire, clause);
    assert_eq!(and["body"]["operation"]["identity"], "quire.op.boolean.and");
    let arguments = and["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    assert_eq!(arguments[0]["target"], reach["node_id"], "position 0");
    assert_eq!(arguments[1]["target"], reach["node_id"], "position 1");
}

/// TC-1047 row 6: two different roles give two nodes.
#[trace("TC-1047", "FR-365-AC-6")]
#[test]
fn two_roles_give_two_reachability_nodes() {
    let lowered = lower(
        "invariant A using v on M::Order at current { reaches(self, self, next) }\n\
         invariant B using v on M::Order at current { reaches(self, self, outgoing) }",
    );
    let reaches = applications(&lowered.wire, "quire.op.model.reaches");
    assert_eq!(reaches.len(), 2);
    assert_ne!(reaches[0]["node_id"], reaches[1]["node_id"]);
}

/// TC-1047 row 7: the node names no population, bound or observation; the
/// `state_clause` node's member names the clause kind.
#[trace("TC-1047", "FR-365-AC-6")]
#[test]
fn the_node_carries_no_population_bound_or_observation() {
    let lowered = lower(NO_LOOP);
    let wire = &lowered.wire;
    let reaches = applications(wire, "quire.op.model.reaches");
    let [reach] = reaches.as_slice() else {
        panic!("one reachability node");
    };
    let body = reach["body"].as_object().unwrap();
    for absent in ["population", "bound", "observation"] {
        assert!(!body.contains_key(absent), "no {absent} in the body");
    }
    assert_eq!(
        reach["dependencies"].as_array().unwrap().len(),
        2,
        "the parameter node and the relationship node, no population"
    );
    assert_eq!(
        lowered.claim_domains,
        vec![1],
        "the clause's requirement record holds the orders domain"
    );
    let clauses = state_clauses(wire);
    let [clause] = clauses.as_slice() else {
        panic!("one state_clause node");
    };
    assert_eq!(clause["body"]["operation"]["member"]["clause"], "invariant");
}
