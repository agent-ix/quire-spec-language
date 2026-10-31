// SPDX-License-Identifier: AGPL-3.0-or-later
//! Text spellings of the structural identities a wire names: a node, a
//! unit, a universe, an effective declaration and a variant. Each is the
//! compact JSON of its components, read back through the same shape, so a
//! wire string member carries one identity without a digest or a hex
//! spelling.

use qsl_foundation::node_wire::{discard_node_json, node_key_from_json, node_key_to_json};
use quire_exact::{
    EffectiveId, Integer, NodeKey, OriginalDeclarationKey, UnitId, UniverseId, VariantId,
};
use serde_json::{json, Map, Value};

fn node_json(node: &NodeKey) -> Option<Value> {
    node_key_to_json(node).ok()
}

fn node_of(value: &Value) -> Option<NodeKey> {
    node_key_from_json(value).ok()
}

fn written(value: Value) -> Option<String> {
    let text = serde_json::to_string(&value).ok();
    discard_node_json(value);
    text
}

fn parsed(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok()
}

fn member<'v>(object: &'v Map<String, Value>, name: &str) -> Option<&'v Value> {
    object.get(name)
}

/// A node as its compact JSON object.
pub(crate) fn node_text(node: &NodeKey) -> Option<String> {
    qsl_foundation::node_wire::node_key_to_text(node).ok()
}

/// The node `text` spells.
pub(crate) fn node_from_text(text: &str) -> Option<NodeKey> {
    qsl_foundation::node_wire::node_key_from_text(text).ok()
}

/// A unit identity: a declared unit's node, or a compound unit's terms.
pub(crate) fn unit_text(unit: &UnitId) -> Option<String> {
    if let Some(node) = unit.declared_key() {
        return written(json!({ "declared": node_json(node)? }));
    }
    let terms = unit.compound_terms()?;
    let mut spelled = Vec::with_capacity(terms.len());
    for (node, exponent) in terms {
        spelled.push(json!({ "node": node_json(node)?, "exponent": exponent.to_string() }));
    }
    written(json!({ "compound": spelled }))
}

/// The unit identity `text` spells.
pub(crate) fn unit_from_text(text: &str) -> Option<UnitId> {
    let value = parsed(text)?;
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    if let Some(declared) = member(object, "declared") {
        return Some(UnitId::declared(node_of(declared)?));
    }
    let mut terms = Vec::new();
    for term in member(object, "compound")?.as_array()? {
        let term = term.as_object()?;
        if term.len() != 2 {
            return None;
        }
        let node = node_of(member(term, "node")?)?;
        let exponent: Integer = member(term, "exponent")?.as_str()?.parse().ok()?;
        terms.push((node, exponent));
    }
    Some(UnitId::compound(terms))
}

/// A universe identity: its root type's node.
pub(crate) fn universe_text(universe: &UniverseId) -> Option<String> {
    node_text(universe.root_type())
}

/// The universe identity `text` spells.
pub(crate) fn universe_from_text(text: &str) -> Option<UniverseId> {
    node_from_text(text).map(UniverseId::new)
}

/// An effective declaration identity: its owner chain, root type first.
pub(crate) fn effective_text(id: &EffectiveId) -> Option<String> {
    let chain: Vec<Value> = id
        .owner_chain()
        .iter()
        .map(|original| json!({ "package": original.package(), "node": original.node() }))
        .collect();
    written(Value::Array(chain))
}

/// The effective declaration identity `text` spells.
pub(crate) fn effective_from_text(text: &str) -> Option<EffectiveId> {
    let value = parsed(text)?;
    let mut id: Option<EffectiveId> = None;
    for original in value.as_array()? {
        let original = original.as_object()?;
        if original.len() != 2 {
            return None;
        }
        let key = OriginalDeclarationKey::new(
            member(original, "package")?.as_str()?,
            member(original, "node")?.as_str()?,
        );
        id = Some(EffectiveId::new(id, key));
    }
    id
}

/// A variant identity: its owner declaration node and case name.
pub(crate) fn variant_text(variant: &VariantId) -> Option<String> {
    written(json!({ "owner": node_json(variant.owner())?, "case": variant.case() }))
}

/// The variant identity `text` spells.
pub(crate) fn variant_from_text(text: &str) -> Option<VariantId> {
    let value = parsed(text)?;
    let object = value.as_object()?;
    if object.len() != 2 {
        return None;
    }
    let owner = node_of(member(object, "owner")?)?;
    VariantId::new(owner, member(object, "case")?.as_str()?).ok()
}

#[cfg(test)]
mod deep_tests {
    use super::*;

    fn deep_key() -> quire_exact::NodeKey {
        let mut key = quire_exact::NodeKey::new(
            "fixture",
            vec![quire_exact::PathSegment::Text("leaf".into())],
            0,
        )
        .expect("a fixture key");
        for ordinal in 1..=200 {
            key = quire_exact::NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Node(key)],
                ordinal,
            )
            .expect("a nested key");
        }
        key
    }

    #[test]
    fn a_deeply_nested_node_round_trips_through_text() {
        let key = deep_key();
        let text = node_text(&key).expect("a deep key spells");
        assert_eq!(node_from_text(&text), Some(key));
    }
}
