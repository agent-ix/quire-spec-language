// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1021: a protocol reference expression resolves its target by typed
//! node key and emits exactly the carrier QSpec FR-322-AC-66 specifies, next
//! to the `protocol`/`protocol_clause` node it targets (FR-092-AC-16 to
//! AC-18, FR-218-AC-8 and AC-9). Every emitted package is read back through
//! IR's v2 reader.

use ix_trace_rs::trace;
use qsl_semantics::check::{CheckCause, CheckRefusal, LawRole};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use super::*;

const PROTOCOL_PROFILE: &str = "quire.protocol.finite-global/v1";

/// One protocol named `name`: a role over the spine model's `Gadget`, an
/// empty `run` sequence and a `finish`.
fn protocol(name: &str) -> String {
    format!(
        "protocol {name} using v over (input: Boolean) on origin {{\n\
         role R on M::Gadget;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// The spine-model unit with `header` after its `v` profile, then
/// `declarations`.
fn unit(header: &str, declarations: &str) -> String {
    let fixture = spine_model_without(&[FIELD_ACCESS, CONFORMING_EQUALITY]);
    let fixture = fixture.replace(
        "profile v = \"quire.value.complete/v1\";\n",
        &format!("profile v = \"quire.value.complete/v1\";\n{header}"),
    );
    format!("{fixture}{declarations}")
}

/// `unit` with the protocol profile selected.
fn selecting(declarations: &str) -> String {
    unit(
        &format!("profile p = \"{PROTOCOL_PROFILE}\";\n"),
        declarations,
    )
}

/// The refusals the spine-model unit `text` checks to.
fn refusals(text: &str) -> Vec<CheckRefusal> {
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("the unit refuses")
}

fn node_of<'w>(wire: &'w Value, tag: &str, form: &str) -> &'w Value {
    let matching: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == tag && node["semantic_form"] == form)
        .collect();
    let [node] = matching[..] else {
        panic!("one {tag}/{form} node: {matching:#?}");
    };
    node
}

fn reference_key(text: &str) -> (String, Value) {
    let (_, wire, _, read) = emit_model_unit(text, None);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let reference = node_of(&wire, "protocol", "protocol_reference").clone();
    (
        reference["node_id"]["digest"].as_str().unwrap().to_owned(),
        wire,
    )
}

const REFERS_TO_FILL: &str = "function refers using v(): Boolean pure { protocol Fill }\n";

/// The unit TC-1021 step 3 emits: `Fill`, and a function that refers to it.
pub(super) fn reference_unit() -> String {
    selecting(&format!("{REFERS_TO_FILL}{}", protocol("Fill")))
}

/// FR-092-AC-16, FR-092-AC-18 (TC-1021 step 3): the unit emits the target
/// `protocol_clause` node and the one `protocol_reference` carrier, and
/// IR's reader admits it.
#[trace("FR-092-AC-16", "FR-092-AC-18", "TC-1021")]
#[test]
fn a_reference_emits_the_target_node_and_one_carrier_the_reader_admits() {
    let text = reference_unit();
    let (emission, wire, _, read) = emit_model_unit(&text, None);
    assert_eq!(emission.omitted, []);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");

    let protocol_nodes: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "protocol")
        .collect();
    assert_eq!(protocol_nodes.len(), 2, "no node for roles, run or finish");
    let clause = node_of(&wire, "protocol", "protocol_clause");
    let reference = node_of(&wire, "protocol", "protocol_reference");

    // AC-18: the target node.
    let law = json!({
        "role": "protocol_profile",
        "definition": {"authority": "agent-ix", "identity": PROTOCOL_PROFILE},
    });
    assert_eq!(clause["body"]["term"], "application");
    assert_eq!(clause["body"]["operator"], "protocol_control");
    assert_eq!(
        clause["body"]["operation"]["identity"],
        "quire.op.protocol.control"
    );
    assert_eq!(clause["body"]["operation"]["laws"], json!([law]));
    assert_eq!(
        clause["body"]["operation"]["member"],
        json!({"kind": "profile_operator", "operator": "sequence"})
    );
    assert_eq!(clause["declaration"]["qualified_name"], json!(["Fill"]));
    assert_eq!(
        clause["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
    let boolean = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean")
        .expect("the Boolean scalar type node is written");
    assert_eq!(clause["semantic_type"], boolean["node_id"]);
    assert_eq!(clause["body"]["result_type"], boolean["node_id"]);
    assert_eq!(wire["lock"]["profile_selections"], json!([law]));

    // AC-16: the carrier.
    assert_eq!(
        reference["body"],
        json!({"term": "reference", "target": clause["node_id"]})
    );
    assert_eq!(reference["semantic_type"], clause["semantic_type"]);
    assert_eq!(reference["dependencies"], json!([clause["node_id"]]));
    assert_eq!(
        reference["occurrences"],
        json!([{"role": "expression", "ordinal": 0}])
    );
    for absent in ["declaration", "owner", "nominal_identity_preimage"] {
        assert!(
            reference.get(absent).is_none_or(Value::is_null),
            "{absent}: {reference}"
        );
    }
    assert!(reference.get("recursion_group").is_none_or(Value::is_null));
    assert!(wire.get("protocol_references").is_none());
    assert!(wire["semantic_graph"].get("protocol_references").is_none());

    // The reference is placed at the two tokens of `protocol Fill`.
    let entries: Vec<&Value> = wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == reference["node_id"])
        .collect();
    let [entry] = entries[..] else {
        panic!("one source-map entry: {entries:?}");
    };
    assert_eq!(entry["role"], "expression");
    let region = &entry["regions"][0];
    let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
    assert_eq!(&text[start..end], "protocol Fill");
}

/// FR-092-AC-17 (TC-1021 step 6): the reference's key is the digest of its
/// `quire.structural-node/v1` preimage, follows the bound target, repeats for
/// the same unit and changes with the target's key.
#[trace("FR-092-AC-17", "TC-1021")]
#[test]
fn a_references_key_follows_its_bound_target() {
    let both = format!("{}{}", protocol("Fill"), protocol("Drain"));
    let against_fill = selecting(&format!("{REFERS_TO_FILL}{both}"));
    let against_drain = selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol Drain }}\n{both}"
    ));
    let (fill, fill_wire) = reference_key(&against_fill);
    let (drain, _) = reference_key(&against_drain);
    let (again, _) = reference_key(&against_fill);
    assert_ne!(fill, drain, "different targets, different keys");
    assert_eq!(fill, again, "the same unit, the same key");

    let carrier = node_of(&fill_wire, "protocol", "protocol_reference");
    let preimage = json!({
        "version": "quire.structural-node/v1",
        "node_tag": "protocol",
        "semantic_form": "protocol_reference",
        "semantic_type": carrier["semantic_type"],
        "declaration": null,
        "recursion": null,
        "body": {"term": "reference", "target": carrier["body"]["target"]},
    });
    assert_eq!(fill, sha256_hex(&jcs(&preimage)));

    // `Fill` renamed: its node key changes, so the reference's does too.
    let renamed =
        selecting(&format!("{REFERS_TO_FILL}{}", protocol("Fill")).replace("Fill", "Pour"));
    let (poured, _) = reference_key(&renamed);
    assert_ne!(fill, poured, "a changed target key changes the reference");
}

/// FR-218-AC-8 (TC-1021 step 4): an unknown name, a name only qualified, a
/// name that resolves to a function and a second reference to one target.
#[trace("FR-218-AC-8", "TC-1021")]
#[test]
fn unresolved_non_protocol_and_repeated_references_refuse_by_cause() {
    let flow = protocol("Fill");
    let missing = refusals(&selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol Missing }}\n{flow}"
    )));
    let [missing] = missing.as_slice() else {
        panic!("one refusal: {missing:?}");
    };
    assert_eq!(missing.cause, CheckCause::MissingName("Missing".to_owned()));

    let qualified = refusals(&selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol M::Fill }}\n{flow}"
    )));
    let [qualified] = qualified.as_slice() else {
        panic!("one refusal: {qualified:?}");
    };
    assert_eq!(
        qualified.cause,
        CheckCause::MissingName("M::Fill".to_owned())
    );

    let function = refusals(&selecting(&format!(
        "function other using v(): Boolean pure {{ true }}\n\
         function refers using v(): Boolean pure {{ protocol other }}\n{flow}"
    )));
    let [function] = function.as_slice() else {
        panic!("one refusal: {function:?}");
    };
    assert_eq!(function.cause.code(), qsl_foundation::Code::IllTyped);
    assert_eq!(function.cause.cause(), Some("operator-ineligible"));

    let twice = refusals(&selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol Fill and protocol Fill }}\n{flow}"
    )));
    let [twice] = twice.as_slice() else {
        panic!("one refusal: {twice:?}");
    };
    assert_eq!(
        twice.cause.code(),
        qsl_foundation::Code::UnsupportedConstruct
    );
    assert_eq!(twice.cause.cause(), Some("expression-form"));
    let CheckCause::ProtocolReference(cause) = &twice.cause else {
        panic!("a protocol reference cause: {twice:?}");
    };
    assert_ne!(cause.first(), &twice.location, "names the first reference");
}

/// FR-092 "The protocol node a reference targets": with no protocol profile
/// selected there is no `protocol_profile` law to carry, so a reference
/// refuses `missing_declaration`/`missing-selection` and a protocol nobody
/// refers to emits as before.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn a_reference_without_a_selected_protocol_profile_refuses_missing_selection() {
    let refused = refusals(&unit("", &format!("{REFERS_TO_FILL}{}", protocol("Fill"))));
    let [refused] = refused.as_slice() else {
        panic!("one refusal: {refused:?}");
    };
    assert_eq!(
        refused.cause,
        CheckCause::MissingSelection {
            role: LawRole::ProtocolProfile
        }
    );
}

/// FR-218-AC-9 (TC-1021 step 5): a reference whose bound target is no node
/// of the graph refuses `missing_declaration`/`missing-name`, naming the
/// bound key, and writes no package.
#[trace("FR-218-AC-9", "TC-1021")]
#[test]
fn a_reference_bound_to_a_key_no_node_carries_refuses_missing_name() {
    let text = selecting(&format!("{REFERS_TO_FILL}{}", protocol("Fill")));
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let graph = assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let key_of = |form: &str| {
        graph
            .semantic_graph()
            .nodes()
            .find(|node| node.semantic_form() == form)
            .unwrap_or_else(|| panic!("a {form} node"))
            .key()
    };
    let (target, reference) = (key_of("protocol_clause"), key_of("protocol_reference"));
    let stale = graph.without_node(target);
    let refusal = emit_checked(&CheckedPackage::link(stale)).expect_err("no package");
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), Some("missing-name"));
    assert_eq!(
        refusal,
        EmitRefusal::StaleProtocolReference {
            reference: node_id(reference),
            target: node_id(target),
        }
    );
}
