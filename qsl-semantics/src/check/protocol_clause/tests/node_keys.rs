//! The `protocol_clause` and `protocol_reference` node keys' dependence on
//! the unit's `protocol_profile` binding.

use quire_semantic_value::checking::CheckingLimits;

use super::assemble;

/// The `protocol_clause` and `protocol_reference` node keys (and the
/// clause's preimage) of one protocol and its reference, lowered with
/// `protocol` as the unit's `protocol_profile` selection.
fn protocol_nodes_under(
    protocol: &crate::value::DefinitionReference,
) -> [(quire_exact::NodeKey, Vec<u8>); 2] {
    let (_, mut assembled) = assemble(
        "function refers using v(): Boolean pure { protocol Flow }\n\
         protocol Flow using v over (input: Boolean) on origin {\n\
         role R on M::Actor;\n\
         run sequence Main { }\n\
         finish End as (outcome: Boolean) { true };\n\
         }",
    );
    assembled.lock_evidence =
        crate::check::LockEvidence::default().with_protocol_profile(protocol.clone());
    let graph = assembled
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let node = |form: &str| {
        let mut found = graph
            .semantic_graph()
            .nodes()
            .filter(|node| node.semantic_form() == form);
        let node = found.next().unwrap_or_else(|| panic!("a {form} node"));
        assert!(found.next().is_none(), "one {form} node");
        (node.key(), node.preimage().to_vec())
    };
    [node("protocol_clause"), node("protocol_reference")]
}

/// QSL-780 AC-3: the `protocol_profile` binding the lock evidence names is
/// part of the protocol node's identity. The same protocol and reference
/// lowered under two bindings key differently, the reference follows its
/// target's key, and the same binding keys identically (so the difference
/// is the binding's, and an old binding does not silently match).
#[test]
fn a_changed_protocol_profile_binding_changes_the_node_keys() {
    let binding = |identity: &str| crate::value::DefinitionReference {
        authority: "agent-ix".to_owned(),
        identity: identity.to_owned(),
    };
    let old = binding("quire.protocol.finite-global/v1");
    let new = binding("quire.protocol.finite-global/v2");
    let [old_clause, old_reference] = protocol_nodes_under(&old);
    let [new_clause, new_reference] = protocol_nodes_under(&new);
    assert_eq!(
        [old_clause.clone(), old_reference.clone()],
        protocol_nodes_under(&old)
    );
    assert_ne!(
        old_clause.0, new_clause.0,
        "the clause key follows the binding"
    );
    assert_ne!(old_clause.1, new_clause.1, "the preimage names the binding");
    assert_ne!(
        old_reference.0, new_reference.0,
        "the reference follows its target"
    );
    let preimage = String::from_utf8_lossy(&new_clause.1).into_owned();
    assert!(preimage.contains("finite-global/v2"), "{preimage}");
    assert!(!preimage.contains("finite-global/v1"), "{preimage}");
}
