// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-24/25: cyclic protocol references use the checked graph's
//! recursion-group encoding and read back through I2.

use super::*;

const PROTOCOL_PROFILE: &str = "quire.protocol.finite-global/v1";

fn package_text(declarations: &str) -> String {
    let fixture = spine_model_without(&[FIELD_ACCESS, CONFORMING_EQUALITY]);
    let fixture = fixture.replace(
        "profile v = \"quire.model.complete/v1\";\n",
        &format!("profile v = \"quire.model.complete/v1\";\nprofile p = \"{PROTOCOL_PROFILE}\";\n"),
    );
    format!(
        "{fixture}\
         function refers using v(left: Boolean, right: Boolean): Boolean pure {{ protocol Flow }}\n\
         {declarations}"
    )
}

fn protocol_declarations(cyclic: bool) -> String {
    let join = if cyclic {
        "join predicate(refers) [west, east];"
    } else {
        "join all [west, east];"
    };
    let flow = "protocol Flow using v over (input: M::Gadget) on origin {\n\
        role R on M::Gadget;\n\
        run parallel Main {\n\
          branch west sequence West { }\n\
          branch east sequence East { }\n\
        } $JOIN\n\
        finish End as (outcome: Boolean) { true };\n\
        }\n";
    format!(
        "{}protocol Idle using v over (input: M::Gadget) on origin {{\n\
         role R on M::Gadget;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n",
        flow.replace("$JOIN", join)
    )
}

fn checked(text: &str) -> (qsl_semantics::check::CheckedGraph, String) {
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied model document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let graph = assemble_with_models(text.as_bytes(), &packages)
        .expect("unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("unit checks: {refusals:?}"));
    (graph, digest)
}

fn declaration_is_idle(node: &qsl_semantics::check::SemanticNode) -> bool {
    node.declaration()
        .is_some_and(|segments| segments.len() == 1 && segments[0].as_str() == "Idle")
}

#[trace("FR-383-AC-24", "FR-383-AC-25")]
#[test]
fn a_cyclic_predicate_join_protocol_is_grouped_and_reads_back() {
    let cyclic_text = package_text(&protocol_declarations(true));
    let (graph, model_digest) = checked(&cyclic_text);
    let cycle_nodes: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.recursion().is_some())
        .collect();
    assert!(
        !cycle_nodes.is_empty(),
        "the checked graph has a cycle group"
    );
    let groups: BTreeSet<_> = cycle_nodes
        .iter()
        .map(|node| *node.recursion().unwrap().group())
        .collect();
    assert_eq!(groups.len(), 1, "one shared cyclic component");
    assert!(
        cycle_nodes
            .iter()
            .any(|node| node.semantic_form() == "protocol_clause"),
        "the protocol clause belongs to the component"
    );
    assert!(
        cycle_nodes
            .iter()
            .any(|node| node.semantic_form() == "protocol_reference"),
        "the protocol reference carrier belongs to the component"
    );
    let group_size = cycle_nodes.len();
    let ordinals: BTreeSet<_> = cycle_nodes
        .iter()
        .map(|node| {
            let recursion = node.recursion().unwrap();
            assert_eq!(recursion.size(), group_size);
            recursion.ordinal()
        })
        .collect();
    assert_eq!(ordinals, (0..group_size).collect());
    assert!(
        cycle_nodes
            .iter()
            .any(|node| String::from_utf8_lossy(node.preimage()).contains("group_reference")),
        "key preimages replace in-group references with group_reference"
    );

    let idle_cyclic = graph
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "protocol_clause" && declaration_is_idle(node))
        .expect("acyclic protocol clause in the mixed unit")
        .key();
    let (acyclic_graph, _) = checked(&package_text(&protocol_declarations(false)));
    let idle_acyclic = acyclic_graph
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "protocol_clause" && declaration_is_idle(node))
        .expect("acyclic protocol clause in acyclic unit")
        .key();
    assert_eq!(
        idle_cyclic, idle_acyclic,
        "acyclic key inputs stay unchanged"
    );

    let emission = emit_checked(&CheckedPackage::link(graph)).expect("package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(model_digest.as_str(), SPINE_MODEL_DOCUMENT)
        .expect("model evidence admitted");
    let pinned =
        qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id());
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &pinned,
    );
    assert!(matches!(read, Read::Verified { .. }), "I2 read: {read:?}");
    let wire_nodes = nodes(&wire);
    let members: Vec<_> = wire_nodes
        .iter()
        .filter(|node| node.get("recursion_group").is_some())
        .collect();
    assert_eq!(members.len(), group_size);
    assert!(members
        .iter()
        .all(|node| node["recursion_group"] == members[0]["recursion_group"]));
    let idle = wire_nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "protocol_clause"
                && node["declaration"]["qualified_name"] == json!(["Idle"])
        })
        .expect("acyclic protocol clause");
    assert!(idle.get("recursion_group").is_none());
}
