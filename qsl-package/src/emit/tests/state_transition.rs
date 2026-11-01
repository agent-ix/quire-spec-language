// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1271 (FR-105-AC-13 to AC-17): S4 emits a derived `state`/`transition`
//! node for a Boolean model field, the derived `state_clause` nodes that bind
//! several authored clauses by conjunction, and nothing for every ineligible
//! pair. Every package goes through the real compile, emission and IR's I2
//! strict read (`Verified`), and the transition's body is compared with the
//! shape of QSpec's published `positive-state-transition.json` read in place
//! from `$QSPEC_DIR` (nothing of QSpec is copied here).

use qsl_semantics::model::intake::meaning;

use super::*;

#[path = "../../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code, reason = "this fixture uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
mod base;

const NATIVE_BOOLEAN: &str = "ix://quire/native/Boolean";
const NATIVE_INTEGER: &str = "ix://quire/native/Integer";

fn param(operation: &str, name: &str, type_ref: &str) -> Value {
    let node = format!("{operation}/{name}");
    json!({
        "identity": base::id(&node), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": base::origin(&node),
    })
}

/// `Switch.name(params)` modifying the named fields of `Switch`.
fn operation(name: &str, params: &[(&str, &str)], modifies: &[&str]) -> Value {
    operation_of("Switch", name, params, modifies)
}

/// `owner.name(params)` modifying the named fields of `Switch`.
fn operation_of(owner: &str, name: &str, params: &[(&str, &str)], modifies: &[&str]) -> Value {
    let node = format!("{owner}/{name}");
    let modifies: Vec<String> = modifies
        .iter()
        .map(|field| base::id(&format!("Switch/{field}")))
        .collect();
    json!({
        "identity": base::id(&node), "name": name,
        "params": params.iter().map(|(p, t)| param(&node, p, t)).collect::<Vec<_>>(),
        "pre": [], "post": [],
        "frame": {"modifies": modifies, "creates": [], "deletes": []},
        "origin": base::origin(&node),
    })
}

const ALL_FIELDS: [&str; 6] = ["on", "lit", "armed", "count", "seq", "peer"];

/// The model document: `Switch` with required Boolean `on` and `lit`,
/// optional `armed`, Integer `count`, Boolean `seq` sequence and reference
/// `peer`; `SubSwitch` specializing it; and the operations.
fn document() -> Value {
    let mut armed = base::field("Switch", "armed", NATIVE_BOOLEAN);
    armed["presence"] = json!("optional");
    let mut seq = base::field("Switch", "seq", NATIVE_BOOLEAN);
    seq["multiplicity"] = json!({"lower": 0, "upper": 8, "ordered": true, "unique": false});
    let operations = vec![
        operation("set", &[("flag", NATIVE_BOOLEAN)], &ALL_FIELDS),
        operation("set2", &[("n", NATIVE_INTEGER)], &ALL_FIELDS),
        operation("keep", &[("flag", NATIVE_BOOLEAN)], &["lit"]),
        operation("plain", &[], &["on", "lit"]),
    ];
    let types = vec![
        base::declaration(
            "Switch",
            "thing",
            json!({
                "identityFields": [base::id("Switch/sid")],
                "fields": [
                    base::field("Switch", "sid", NATIVE_INTEGER),
                    base::field("Switch", "on", NATIVE_BOOLEAN),
                    base::field("Switch", "lit", NATIVE_BOOLEAN),
                    armed,
                    base::field("Switch", "count", NATIVE_INTEGER),
                    seq,
                    base::field("Switch", "peer", &base::id("Switch")),
                ],
                "operations": operations,
            }),
        ),
        base::declaration(
            "SubSwitch",
            "thing",
            json!({
                "supertypes": [base::id("Switch")],
                "identityFields": [base::id("SubSwitch/subid")],
                "fields": [base::field("SubSwitch", "subid", NATIVE_INTEGER)],
                // Declared on the subtype, it modifies the supertype's `on`.
                "operations": [operation_of(
                    "SubSwitch",
                    "subset",
                    &[("flag", NATIVE_BOOLEAN)],
                    &["on"],
                )],
            }),
        ),
    ];
    base::domain_document(types, vec![base::construct("thing", meaning::OBJECT_TYPE)])
}

struct Compiled {
    emission: Emission,
    wire: Value,
    evidence: CheckedPackageEvidence,
}

/// The unit selecting the Switch model and the protocol profile (unless
/// `protocol` is false), followed by `clauses`.
fn unit(clauses: &str, protocol: bool) -> (String, qsl_semantics::model::intake::PackageInput) {
    let document = serde_json::to_vec(&document()).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let profile = if protocol {
        "profile p = \"quire.protocol.finite-global/v1\";\n"
    } else {
        ""
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {profile}\
         model M = {package:?};\n\
         {clauses}\n",
        package = base::PACKAGE,
    );
    (unit, packages)
}

fn check(
    clauses: &str,
    protocol: bool,
) -> (CheckedPackage, qsl_semantics::model::intake::PackageInput) {
    let (unit, packages) = unit(clauses, protocol);
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}\n{unit}"));
    (CheckedPackage::link(graph), packages)
}

fn compile(clauses: &str) -> Compiled {
    compile_with(clauses, true)
}

fn compile_with(clauses: &str, protocol: bool) -> Compiled {
    let (package, _) = check(clauses, protocol);
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(
            base::PACKAGE,
            serde_json::to_vec(&document()).unwrap().as_slice(),
        )
        .expect("a fresh evidence key");
    Compiled {
        emission,
        wire,
        evidence,
    }
}

impl Compiled {
    /// The structural key of `id`, a wire id this package holds (FR-322:
    /// across packages only keys compare).
    fn key(&self, id: &Value) -> NodeKey {
        qsl_semantics::check::wire_id::WireGraph::of_package_value(&self.wire)
            .unwrap()
            .key_of_value(id)
            .unwrap()
    }

    /// QSL's I2 read, which runs IR's strict reader, of the emitted bytes.
    fn read(&self) -> Read {
        read_with(&self.emission, &self.evidence)
    }

    fn verified(&self) {
        assert!(
            matches!(self.read(), Read::Verified { .. }),
            "the package reads back Verified: {:?}",
            self.read()
        );
    }

    fn forms(&self, tag: &str, form: &str) -> Vec<&Value> {
        super::owners::node_where(&self.wire, tag, form)
    }

    fn transitions(&self) -> Vec<&Value> {
        self.forms("state", "transition")
    }

    fn node(&self, id: &Value) -> &Value {
        super::nodes(&self.wire)
            .iter()
            .find(|node| node["node_id"] == *id)
            .unwrap_or_else(|| panic!("node {id} is in the package"))
    }

    /// The argument `name` of a transition body.
    fn bound<'v>(transition: &'v Value, name: &str) -> &'v Value {
        transition["body"]["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|argument| argument["name"] == name)
            .map(|argument| &argument["value"])
            .unwrap_or_else(|| panic!("binding {name}"))
    }

    /// The `state_clause` nodes of `kind`.
    fn clauses(&self, kind: &str) -> Vec<&Value> {
        self.forms("state", "state_clause")
            .into_iter()
            .filter(|node| node["body"]["operation"]["member"]["clause"] == kind)
            .collect()
    }

    /// The condition node a `state_clause` references.
    fn condition(&self, clause: &Value) -> &Value {
        self.node(&clause["body"]["arguments"][2]["target"])
    }
}

fn target(value: &Value) -> Value {
    value["target"].clone()
}

fn member_targets(value: &Value) -> Vec<Value> {
    value["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(target)
        .collect()
}

/// The clauses of AC-13's unit, which `extra` follows.
fn unit_13(extra: &str) -> String {
    format!(
        "pre P1 using v on M::Switch::set {{ flag or self.on }}\n\
         post Q1 using v on M::Switch::set {{ self.on }}\n\
         post Qa using v on M::Switch::set {{ self.armed = pre(self.armed) }}\n\
         post Qc using v on M::Switch::set {{ self.count = pre(self.count) }}\n\
         post Qs using v on M::Switch::set {{ self.seq = pre(self.seq) }}\n\
         {extra}"
    )
}

const BASE_POST: &str = "post Q1 using v on M::Switch::set { self.on }\n";
const Q2: &str = "post Q2 using v on M::Switch::set { self.on = pre(self.on) or flag }\n";
const P2: &str = "pre P2 using v on M::Switch::set { not flag or self.lit }\n";

fn clause_named<'c>(compiled: &'c Compiled, kind: &str, condition: &Value) -> &'c Value {
    compiled
        .clauses(kind)
        .into_iter()
        .find(|clause| clause["body"]["arguments"][2]["target"] == *condition)
        .unwrap_or_else(|| panic!("a {kind} clause over {condition}"))
}

/// FR-105-AC-13: one transition for (`set`, `on`) with `pre` the node of
/// `P1`, `post` the node of `Q1` and `inputs` `[flag]`; no derived node, no
/// transition for `lit` (nothing constrains it) or for `armed`, `count` and
/// `seq` although a postcondition constrains each; the package reads back
/// `Verified`.
#[trace("TC-1271", "FR-105-AC-13")]
#[test]
fn a_boolean_field_constrained_by_one_postcondition_gets_one_transition() {
    let compiled = compile(&unit_13(""));
    let transitions = compiled.transitions();
    let [transition] = transitions[..] else {
        panic!("one transition: {transitions:#?}");
    };
    // P1, Q1, Qa, Qc, Qs and the type-exclusion postconditions the checker
    // adds are all authored; no derived node exists.
    assert_eq!(compiled.clauses("precondition").len(), 1);
    assert_eq!(compiled.clauses("postcondition").len(), 4);
    let anchors = compiled.forms("state", "operation_anchor");
    let [anchor] = anchors[..] else {
        panic!("one anchor: {anchors:#?}");
    };
    assert_eq!(
        Compiled::bound(transition, "anchor")["target"],
        anchor["node_id"]
    );
    let subject = compiled.node(&target(Compiled::bound(transition, "subject")));
    assert_eq!(subject["semantic_form"], "query");
    assert_eq!(
        subject["body"]["operation"]["identity"],
        "quire.op.record.project"
    );
    assert_eq!(subject["body"]["operation"]["member"]["name"], "on");
    let post = compiled.node(&target(Compiled::bound(transition, "post")));
    assert_eq!(
        post["body"]["arguments"][2]["target"], subject["node_id"],
        "Q1's condition is the subject read itself"
    );
    let pre = member_targets(Compiled::bound(transition, "pre"));
    assert_eq!(pre.len(), 1);
    assert_eq!(
        compiled.node(&pre[0])["body"]["operation"]["member"]["clause"],
        "precondition"
    );
    let inputs = member_targets(Compiled::bound(transition, "inputs"));
    assert_eq!(inputs.len(), 1);
    let flag = compiled.node(&inputs[0]);
    assert_eq!(flag["body"]["members"][0]["value"]["value"], "flag");
    assert_eq!(transition["occurrences"].as_array().unwrap().len(), 1);
    compiled.verified();
}

/// FR-105-AC-14 and AC-15: a second precondition and postcondition give one
/// derived `precondition` and one derived `postcondition` node holding the
/// left fold of `quire.op.boolean.and` over the authored conditions in
/// ascending clause-name order; the authored nodes stay; swapping the source
/// order of `Q1` and `Q2` changes no node id.
#[trace("TC-1271", "FR-105-AC-14", "FR-105-AC-15")]
#[test]
fn several_clauses_are_bound_by_an_exact_conjunction_in_clause_name_order() {
    let compiled = compile(&unit_13(&format!("{Q2}{P2}")));
    let transitions = compiled.transitions();
    let [transition] = transitions[..] else {
        panic!("one transition");
    };
    assert_eq!(compiled.clauses("precondition").len(), 3, "P1, P2, derived");
    assert_eq!(
        compiled.clauses("postcondition").len(),
        6,
        "Q1, Q2, 3 more, derived"
    );
    let derived_post = compiled.node(&target(Compiled::bound(transition, "post")));
    let derived_pre = compiled.node(&member_targets(Compiled::bound(transition, "pre"))[0]);
    // The derived nodes have no declaration and one claim occurrence per
    // contributing authored clause.
    assert_eq!(derived_post["occurrences"].as_array().unwrap().len(), 2);
    assert_eq!(derived_pre["occurrences"].as_array().unwrap().len(), 2);
    assert!(derived_post.get("declaration").is_none());

    let and_of = |clause: &Value| {
        let condition = compiled.condition(clause);
        assert_eq!(
            condition["body"]["operation"]["identity"],
            "quire.op.boolean.and"
        );
        member_targets(&json!({"members": condition["body"]["arguments"]}))
    };
    // Q1's condition is the subject read; Q2's is the `or` expression.
    let q1 = Compiled::bound(transition, "subject")["target"].clone();
    let q2_condition = compiled
        .forms("expression", "binary")
        .into_iter()
        .find(|node| {
            node["body"]["operation"]["identity"] == "quire.op.boolean.or"
                && compiled
                    .clauses("postcondition")
                    .iter()
                    .any(|c| c["body"]["arguments"][2]["target"] == node["node_id"])
        })
        .expect("Q2's condition");
    assert_eq!(and_of(derived_post), [q1, q2_condition["node_id"].clone()]);
    let pre_operands = and_of(derived_pre);
    assert_eq!(pre_operands.len(), 2);
    for operand in &pre_operands {
        assert_eq!(
            clause_named(&compiled, "precondition", operand)["semantic_form"],
            "state_clause"
        );
    }
    compiled.verified();

    // The authored nodes are the same with and without the transition: the
    // 5 clause nodes of AC-13 are all present in the AC-14 package.
    let base = compile(&unit_13(""));
    for kind in ["precondition", "postcondition"] {
        for clause in base.clauses(kind) {
            // Two packages: their nodes compare by structural key (FR-322).
            let key = base.key(&clause["node_id"]);
            assert!(
                compiled
                    .clauses(kind)
                    .iter()
                    .any(|c| compiled.key(&c["node_id"]) == key),
                "{kind} {} stays",
                clause["node_id"]
            );
        }
    }
}

/// FR-105-AC-14: a third clause nests the fold to the left, and a clause
/// declared twice with an equal body counts once.
#[trace("TC-1271", "FR-105-AC-14", "FR-105-AC-15")]
#[test]
fn a_third_clause_nests_left_and_an_equal_body_counts_once() {
    let third = "pre P3 using v on M::Switch::set { flag or self.lit }\n";
    let compiled = compile(&unit_13(&format!("{Q2}{P2}{third}")));
    let transitions = compiled.transitions();
    let [transition] = transitions[..] else {
        panic!("one transition");
    };
    let derived_pre = compiled.node(&member_targets(Compiled::bound(transition, "pre"))[0]);
    let fold = compiled.condition(derived_pre);
    assert_eq!(
        fold["body"]["operation"]["identity"],
        "quire.op.boolean.and"
    );
    let left = compiled.node(&fold["body"]["arguments"][0]["target"]);
    assert_eq!(
        left["body"]["operation"]["identity"], "quire.op.boolean.and",
        "(P1 and P2) and P3: the left operand is itself the fold"
    );

    // `P1` declared twice with equal bodies counts as one clause in R.
    let twice = compile(&unit_13(
        "pre P1again using v on M::Switch::set { flag or self.on }\n",
    ));
    let [transition] = twice.transitions()[..] else {
        panic!("one transition");
    };
    let pre = member_targets(Compiled::bound(transition, "pre"));
    assert_eq!(
        twice.node(&pre[0])["body"]["operation"]["member"]["clause"],
        "precondition"
    );
    assert_eq!(twice.clauses("precondition").len(), 1, "no derived node");
    assert_eq!(
        twice.clauses("precondition")[0]["occurrences"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "two declarations, one node"
    );
    twice.verified();
}

/// FR-105-AC-16: each ineligible variant, beside a postcondition that would
/// constrain the field were it eligible, gives no transition, no derived
/// node and no refusal.
#[trace("TC-1271", "FR-105-AC-16")]
#[test]
fn an_ineligible_pair_gets_no_transition_and_no_refusal() {
    let cases: Vec<(&str, String, bool)> = vec![
        (
            "a non-Boolean parameter",
            "post Q using v on M::Switch::set2 { self.on }".to_owned(),
            true,
        ),
        (
            "a frame that does not modify the field",
            "post Q using v on M::Switch::keep { self.on }".to_owned(),
            true,
        ),
        (
            "an optional Boolean",
            "post Q using v on M::Switch::set { self.armed = pre(self.armed) }".to_owned(),
            true,
        ),
        (
            "a sequence",
            "post Q using v on M::Switch::set { self.seq = pre(self.seq) }".to_owned(),
            true,
        ),
        (
            "an Integer",
            "post Q using v on M::Switch::set { self.count = pre(self.count) }".to_owned(),
            true,
        ),
        (
            "another receiver's field",
            "post Q using v on M::Switch::set { self.peer.on }".to_owned(),
            true,
        ),
        (
            "a rebound receiver",
            "post Q using v on M::Switch::set { let s = self in s.on }".to_owned(),
            true,
        ),
        (
            "a pre-state read only",
            "post Q using v on M::Switch::set { pre(self.on) }".to_owned(),
            true,
        ),
        (
            "an execution point",
            // The binding is supplied below.
            "post Q using v on M::Switch::set { self.on }".to_owned(),
            true,
        ),
        (
            "a proper subtype",
            "post Q using v on M::SubSwitch::set { self.on }".to_owned(),
            true,
        ),
        (
            "an inherited Boolean field modified by a subtype operation",
            "post Q using v on M::SubSwitch::subset { self.on }".to_owned(),
            true,
        ),
        (
            "no protocol profile",
            "post Q using v on M::Switch::set { self.on }".to_owned(),
            false,
        ),
    ];
    for (label, clause, protocol) in cases {
        if label == "an execution point" {
            continue;
        }
        let compiled = compile_with(&clause, protocol);
        assert_eq!(compiled.transitions().len(), 0, "{label}");
        // IR itself refuses the subtype-anchored state clause (IllTyped /
        // OperatorIneligible at /semantic_graph/nodes/N/body/arguments/0, the
        // state_clause parameter aggregate). That is pre-existing on main and
        // is about neither a deref nor a transition, so the subtype variants
        // are checked at emission only. FR-105 excludes subtypes (rationale
        // in FR-120).
        if !label.contains("subtype") {
            assert!(
                matches!(compiled.read(), Read::Verified { .. }),
                "{label} reads back Verified: {:?}",
                compiled.read()
            );
        }
    }
    // The result of `result.on` needs an operation with a result; a
    // postcondition reading no receiver field also constrains nothing.
    let neither = compile("post Q using v on M::Switch::set { flag }");
    assert_eq!(neither.transitions().len(), 0);
    // Control: the same operation with an eligible postcondition does emit.
    let control = compile("post Q using v on M::Switch::set { self.on }");
    assert_eq!(control.transitions().len(), 1);
    control.verified();
    // A subtype clause added to an eligible unit removes the transition.
    let with_subtype = compile(&unit_13(
        "post QS using v on M::SubSwitch::set { self.on }\n",
    ));
    assert_eq!(with_subtype.transitions().len(), 0);
}

/// FR-105-AC-17: a transition has one claim occurrence per
/// contributing postcondition, not only the first.
#[trace("TC-1271", "FR-105-AC-13", "FR-105-AC-17")]
#[test]
fn a_transition_carries_one_claim_occurrence_per_constraining_postcondition() {
    let compiled = compile(&format!("{BASE_POST}{Q2}"));
    let transitions = compiled.transitions();
    let [transition] = transitions[..] else {
        panic!("one transition");
    };
    let claims = transition["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|occurrence| occurrence["role"] == "claim")
        .count();
    assert_eq!(claims, 2, "Q1 and Q2 each claim the transition");
}

/// FR-105-AC-17: two eligible subjects with the same preconditions share one
/// derived precondition node and give two transitions with different `post`
/// and `subject`; the bytes are identical across two compiles; each
/// transition occurrence carries the region of a contributing postcondition.
#[trace("TC-1271", "FR-105-AC-17")]
#[test]
fn two_subjects_share_the_derived_precondition_and_compile_deterministically() {
    let source = unit_13(&format!(
        "{Q2}{P2}post Q3 using v on M::Switch::set {{ self.lit }}\n"
    ));
    let first = compile(&source);
    let second = compile(&source);
    assert_eq!(
        first.emission.package().bytes(),
        second.emission.package().bytes()
    );
    let transitions = first.transitions();
    assert_eq!(transitions.len(), 2);
    let pres: BTreeSet<_> = transitions
        .iter()
        .map(|t| serde_json::to_string(&member_targets(Compiled::bound(t, "pre"))).unwrap())
        .collect();
    assert_eq!(pres.len(), 1, "one shared precondition node");
    let posts: BTreeSet<_> = transitions
        .iter()
        .map(|t| Compiled::bound(t, "post")["target"].to_string())
        .collect();
    let subjects: BTreeSet<_> = transitions
        .iter()
        .map(|t| Compiled::bound(t, "subject")["target"].to_string())
        .collect();
    assert_eq!(posts.len(), 2);
    assert_eq!(subjects.len(), 2);
    assert_eq!(
        first.clauses("precondition").len(),
        3,
        "P1, P2 and one derived precondition, not two"
    );
    // The shared derived node was built once: one claim occurrence per
    // contributing clause (P1, P2), not one more pair per subject.
    let shared_pre = first.node(&member_targets(Compiled::bound(transitions[0], "pre"))[0]);
    assert_eq!(shared_pre["occurrences"].as_array().unwrap().len(), 2);
    // Each transition's occurrences resolve to regions of its postconditions.
    let source_map = first.wire["source_map"].as_array().unwrap();
    for transition in &transitions {
        let entries: Vec<&Value> = source_map
            .iter()
            .filter(|entry| entry["node_id"] == transition["node_id"])
            .collect();
        assert!(!entries.is_empty());
        for entry in entries {
            assert_eq!(entry["role"], "claim");
            assert!(!entry["regions"].as_array().unwrap().is_empty());
        }
    }
    first.verified();
}

/// FR-105-AC-17: a fault injected on the transition node, and on a derived
/// node, refuses the whole compile and emits no state node.
#[trace("TC-1271", "FR-105-AC-17")]
#[test]
fn a_fault_on_the_transition_or_a_derived_node_refuses_the_compile() {
    let source = unit_13(&format!("{Q2}{P2}"));
    let compiled = compile(&source);
    let transition = compiled.transitions()[0]["node_id"].clone();
    let derived = compiled.clauses("postcondition");
    let derived = derived
        .iter()
        .find(|clause| clause["occurrences"].as_array().unwrap().len() == 2)
        .expect("the derived postcondition")["node_id"]
        .clone();
    let (package, _) = check(&source, true);
    let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(&compiled.wire).unwrap();
    for (label, id) in [("transition", transition), ("derived", derived)] {
        let reason = format!("fault injection (test): {label}");
        // The fault sees the emitter's structural id of the wire's node.
        let id: CheckedNodeId = crate::emit::checked_node_id_of(&graph.key_of_value(&id).unwrap());
        let refusal = emit_package_with_fault(&package, package.graph().memoized_regions(), |at| {
            (*at == id).then(|| EmitRefusal::Encoding {
                reason: reason.clone(),
            })
        })
        .expect_err("a fault refuses the whole package");
        assert!(
            matches!(&refusal, EmitRefusal::Encoding { reason: got } if *got == reason),
            "{label}: {refusal:?}"
        );
    }
}

/// Masks every node reference of `value` so two packages with different node
/// ids can be compared by shape.
fn masked(value: &Value) -> Value {
    match value {
        Value::Object(map)
            if map.len() == 3
                && map.contains_key("package")
                && map.contains_key("path")
                && map.contains_key("ordinal") =>
        {
            json!("<node>")
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), masked(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(masked).collect()),
        other => other.clone(),
    }
}

/// The golden source: QSpec's `positive-state-transition.json` under
/// `$QSPEC_DIR`, read in place. The emitted transition, anchor and
/// `state_clause` nodes have the same keys and values as the vector's, with
/// node references masked.
#[trace("TC-1271", "FR-105-AC-13")]
#[test]
fn the_emitted_nodes_have_the_published_vector_shape() {
    let qspec = std::env::var_os("QSPEC_DIR").expect("QSPEC_DIR names the QSpec checkout");
    let path = std::path::Path::new(&qspec)
        .join("proposals/checked-package-v2/fixtures/positive-state-transition.json");
    let vector: Value = serde_json::from_slice(&std::fs::read(&path).expect("the vector"))
        .expect("the vector parses");
    let vector_nodes = vector["semantic_graph"]["nodes"].as_array().unwrap();
    let published = |form: &str| -> Vec<&Value> {
        vector_nodes
            .iter()
            .filter(|node| node["semantic_form"] == form)
            .collect()
    };
    let compiled = compile(&unit_13(""));
    let emitted = |form: &str| -> Vec<&Value> { compiled.forms("state", form) };

    let [published_transition] = published("transition")[..] else {
        panic!("one published transition");
    };
    let [transition] = compiled.transitions()[..] else {
        panic!("one emitted transition");
    };
    assert_eq!(masked(transition), masked(published_transition));
    assert_eq!(
        Compiled::bound(transition, "pre")["term"],
        Compiled::bound(published_transition, "pre")["term"]
    );

    let [published_anchor] = published("operation_anchor")[..] else {
        panic!("one published anchor");
    };
    let [anchor] = emitted("operation_anchor")[..] else {
        panic!("one emitted anchor");
    };
    // One `anchor` occurrence per clause naming the operation here; the
    // vector has the one clause pair.
    let without_occurrences = |node: &Value| {
        let mut node = masked(node);
        node.as_object_mut().unwrap().remove("occurrences");
        node
    };
    assert_eq!(
        without_occurrences(anchor),
        without_occurrences(published_anchor)
    );

    for kind in ["precondition", "postcondition"] {
        let published_clause = published("state_clause")
            .into_iter()
            .find(|node| node["body"]["operation"]["member"]["clause"] == kind)
            .expect("a published clause");
        let clause = compiled
            .clauses(kind)
            .into_iter()
            .find(|clause| {
                // The authored P1 / Q1 of the vector.
                compiled.condition(clause)["semantic_form"]
                    == if kind == "precondition" {
                        "binary"
                    } else {
                        "query"
                    }
            })
            .expect("an emitted clause");
        assert_eq!(
            masked(&clause["body"]),
            masked(&published_clause["body"]),
            "{kind}"
        );
        assert!(clause["semantic_type"].is_object());
    }
    // The vector's wire spelling of the operator and operation.
    assert_eq!(transition["body"]["operator"], "state_transition");
    assert_eq!(
        transition["body"]["operation"]["identity"],
        "quire.op.state.transition"
    );
}

/// FR-105-AC-16: a `pre` or `post` of the operation carrying an accepted
/// execution point gives the operation no transition and no refusal, though
/// the same unit without the point does emit one.
#[trace("TC-1271", "FR-105-AC-16")]
#[test]
fn an_execution_point_on_a_clause_of_the_operation_gives_no_transition() {
    use quire_contract_model::{AnchorName, ExecutionPoint};

    let clauses = unit_13("");
    let control = compile(&clauses);
    assert_eq!(control.transitions().len(), 1, "control");
    control.verified();
    let control_rows = control.emission.selected_operation_member_rows();
    assert!(
        !control_rows.is_empty(),
        "the control package has operation rows"
    );
    let control_read = control.read();
    let Read::Verified { admitted, .. } = control_read else {
        panic!("the control package is admitted: {control_read:?}");
    };
    let selected = control
        .emission
        .selected_operation_members(&admitted, 32)
        .expect("the rows join the admitted control package");
    for row in control_rows {
        assert_eq!(selected.read(&admitted, &row.anchor), Ok(row));
    }
    // Point-specific emissions exercise the no-transition behavior; the
    // valid control emission above exercises the typed row join.
    for (clause, point) in [
        (
            "P1",
            ExecutionPoint::Pre {
                operation: AnchorName::new("set").unwrap().into(),
            },
        ),
        (
            "Q1",
            ExecutionPoint::Post {
                operation: AnchorName::new("set").unwrap().into(),
            },
        ),
    ] {
        let (unit, packages) = unit(&clauses, true);
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("agent-ix", "test:tc-1271", "fixture", "fixture:1"),
            "program.native",
            unit.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 admits the unit");
        let raw = parsed.source().reference().clone();
        let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let models = qsl_semantics::model::intake::admit_unit(
            &built.selections().models,
            &packages,
            qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .expect("intake admits the package");
        let graph = PackageDeclarations::assemble_with_execution_points_and_cancel(
            raw,
            "pkg".to_owned(),
            built,
            models,
            Vec::new(),
            Default::default(),
            qsl_semantics::check::AssemblyRun {
                cancel: &quire_exact::Cancel::new(),
                execution_points: BTreeMap::from([(clause.to_owned(), point)]),
            },
        )
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{clause}: the package checks: {refusals:?}"));
        let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
        let rows = emission.selected_operation_member_rows();
        let [first, second] = rows else {
            panic!("both distinct model-owned operation anchors have rows")
        };
        assert_ne!(
            first.anchor, second.anchor,
            "each clause has a full distinct anchor"
        );
        assert_eq!(
            first.selected_operation_identity, second.selected_operation_identity,
            "both anchors share the same structured selected operation identity"
        );
        for row in rows {
            assert_eq!(row.selected_document.as_ref(), base::PACKAGE);
            assert_eq!(
                row.declaring_form,
                quire_contract_model::ModelForm::ObjectType
            );
            assert_eq!(
                row.original_model_identity.as_ref(),
                base::id("Switch").as_str()
            );
            assert_eq!(
                row.original_operation_identity.as_ref(),
                base::id("Switch/set").as_str()
            );
        }
        assert_eq!(
            super::owners::node_where(&wire(&emission), "state", "transition").len(),
            0,
            "a point on {clause}"
        );
    }
}
/// The node ids of the conditions of the authored (not derived)
/// preconditions of `compiled`. The wire lists nodes by structural node key,
/// not by source position, and carries no clause names, so this order says
/// nothing about the source.
fn authored_conditions(compiled: &Compiled) -> Vec<Value> {
    compiled
        .clauses("precondition")
        .into_iter()
        .map(|clause| compiled.condition(clause))
        .filter(|condition| condition["body"]["operation"]["identity"] != "quire.op.boolean.and")
        .map(|condition| condition["node_id"].clone())
        .collect()
}

/// The node id the condition `body` checks to. Node identity is structural,
/// so a unit holding only that one clause yields the id it has in any unit.
fn condition_id(body: &str) -> NodeKey {
    let alone = compile(&format!(
        "{BASE_POST}pre P1 using v on M::Switch::set {{ {body} }}\n"
    ));
    let [id] =
        <[Value; 1]>::try_from(authored_conditions(&alone)).expect("one authored precondition");
    alone.key(&id)
}

/// The operands of a left fold of `quire.op.boolean.and` rooted at `node`,
/// leftmost first.
fn fold_operands(compiled: &Compiled, node: &Value) -> Vec<NodeKey> {
    if node["body"]["operation"]["identity"] != "quire.op.boolean.and" {
        return vec![compiled.key(&node["node_id"])];
    }
    let mut operands = fold_operands(
        compiled,
        compiled.node(&node["body"]["arguments"][0]["target"]),
    );
    operands.push(compiled.key(&node["body"]["arguments"][1]["target"]));
    operands
}

/// FR-105-AC-14 and AC-15: the fold takes the clauses in ascending clause
/// name order whatever the source order; a node several declarations check to
/// sorts by its least clause name; a third clause makes a left-nested fold.
#[trace("TC-1271", "FR-105-AC-14", "FR-105-AC-15")]
#[test]
fn the_fold_follows_ascending_clause_names_not_source_order() {
    let c = |name: &str, body: &str| format!("pre {name} using v on M::Switch::set {{ {body} }}\n");
    let (a, b, d) = (
        "flag or self.on",
        "not flag or self.lit",
        "flag or self.lit",
    );
    // Source order is P3, P1, P2; clause-name order is P1, P2, P3.
    let compiled = compile(&format!(
        "{BASE_POST}{}{}{}",
        c("P3", d),
        c("P1", a),
        c("P2", b)
    ));
    let [transition] = compiled.transitions()[..] else {
        panic!("one transition");
    };
    let derived = compiled.node(&member_targets(Compiled::bound(transition, "pre"))[0]);
    // The authored clauses' condition nodes, in clause-name order P1, P2, P3.
    let [p1, p2, p3] = [a, b, d].map(condition_id);
    assert_eq!(
        fold_operands(&compiled, compiled.condition(derived)),
        [p1, p2, p3]
    );

    // One node declared as P1 and P9 sorts by its least name P1, before P5.
    let shared = compile(&format!(
        "{BASE_POST}{}{}{}",
        c("P9", a),
        c("P5", b),
        c("P1", a)
    ));
    let [transition] = shared.transitions()[..] else {
        panic!("one transition");
    };
    let derived = shared.node(&member_targets(Compiled::bound(transition, "pre"))[0]);
    // P9 and P1 check to the one node, which sorts by its least name P1.
    let operands = fold_operands(&shared, shared.condition(derived));
    assert_eq!(operands.len(), 2, "two clauses, not three");
    assert_eq!(
        operands,
        [condition_id(a), condition_id(b)],
        "the node declared as P1 and P9 comes first"
    );
}
