// SPDX-License-Identifier: AGPL-3.0-or-later
//! The JSON shape of a structural node identity: `{"package": string,
//! "path": [string | node-identity object, ...], "ordinal": u64}`.
//!
//! [`node_key_to_json`] and [`node_key_from_json`] are the one shared codec.
//! Both walk nested path segments with an explicit stack, never the call
//! stack, so nesting depth is bounded only by memory, and they reserve their
//! own storage fallibly. Neither names a digest or a hex spelling.
//!
//! A [`Value`] this module builds can be as deep as the identity, and
//! `serde_json` drops a `Value` recursively: release one with
//! [`discard_node_json`] rather than by `drop` when its depth is untrusted.

use alloc::string::String;
use alloc::vec::Vec;
use quire_exact::{
    InvalidNodeKey, NodeKey, NodeKeyView, NodePathIter, PathSegment, PathSegmentView,
};
use serde_json::{Map, Number, Value};

/// Why a node identity did not convert to or from its JSON shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NodeWireError {
    /// The value is not an object with exactly `package`, `path` and `ordinal`.
    #[error("a node identity is an object with exactly `package`, `path` and `ordinal`")]
    Shape,
    /// `package` is not a string.
    #[error("a node identity's `package` is a string")]
    Package,
    /// `path` is not an array of strings and node identity objects.
    #[error("a node identity's `path` is an array of strings and node identities")]
    Path,
    /// `ordinal` is not an unsigned 64-bit integer.
    #[error("a node identity's `ordinal` is an unsigned integer")]
    Ordinal,
    /// Storage for the conversion could not be reserved.
    #[error("storage for a node identity conversion could not be reserved")]
    Allocation,
    /// The members are well shaped but name no admissible identity.
    #[error(transparent)]
    Key(#[from] InvalidNodeKey),
}

fn push<T>(vec: &mut Vec<T>, item: T) -> Result<(), NodeWireError> {
    vec.try_reserve(1).map_err(|_| NodeWireError::Allocation)?;
    vec.push(item);
    Ok(())
}

fn owned(text: &str) -> Result<String, NodeWireError> {
    let mut copy = String::new();
    copy.try_reserve_exact(text.len())
        .map_err(|_| NodeWireError::Allocation)?;
    copy.push_str(text);
    Ok(copy)
}

/// Release `value` without recursing, however deeply it nests.
pub fn discard_node_json(value: Value) {
    let mut pending = alloc::vec![value];
    while let Some(next) = pending.pop() {
        match next {
            Value::Array(items) => pending.extend(items),
            Value::Object(members) => pending.extend(members.into_iter().map(|(_, member)| member)),
            _ => {}
        }
    }
}

struct Encode<'a> {
    package: &'a str,
    ordinal: u64,
    rest: NodePathIter<'a>,
    path: Vec<Value>,
}

fn open(view: NodeKeyView<'_>) -> Encode<'_> {
    Encode {
        package: view.package(),
        ordinal: view.ordinal(),
        rest: view.path(),
        path: Vec::new(),
    }
}

fn close(frame: Encode<'_>) -> Result<Value, NodeWireError> {
    let mut object = Map::new();
    object.insert("package".into(), Value::String(owned(frame.package)?));
    object.insert("path".into(), Value::Array(frame.path));
    object.insert("ordinal".into(), Value::Number(Number::from(frame.ordinal)));
    Ok(Value::Object(object))
}

/// `key` as its JSON object.
///
/// # Errors
///
/// [`NodeWireError::Allocation`] when storage cannot be reserved.
pub fn node_key_to_json(key: &NodeKey) -> Result<Value, NodeWireError> {
    let mut stack = Vec::new();
    push(&mut stack, open(key.view()))?;
    loop {
        let Some(top) = stack.last_mut() else {
            return Err(NodeWireError::Shape);
        };
        match top.rest.next() {
            Some(PathSegmentView::Text(text)) => push(&mut top.path, Value::String(owned(text)?))?,
            Some(PathSegmentView::Node(view)) => push(&mut stack, open(view))?,
            None => {
                let Some(done) = stack.pop() else {
                    return Err(NodeWireError::Shape);
                };
                let value = close(done)?;
                match stack.last_mut() {
                    Some(parent) => push(&mut parent.path, value)?,
                    None => return Ok(value),
                }
            }
        }
    }
}

struct Decode<'a> {
    package: String,
    ordinal: u64,
    rest: core::slice::Iter<'a, Value>,
    segments: Vec<PathSegment>,
}

fn header(value: &Value) -> Result<Decode<'_>, NodeWireError> {
    let object = value.as_object().ok_or(NodeWireError::Shape)?;
    if object.len() != 3 {
        return Err(NodeWireError::Shape);
    }
    let package = object.get("package").ok_or(NodeWireError::Shape)?;
    let path = object.get("path").ok_or(NodeWireError::Shape)?;
    let ordinal = object.get("ordinal").ok_or(NodeWireError::Shape)?;
    Ok(Decode {
        package: owned(package.as_str().ok_or(NodeWireError::Package)?)?,
        ordinal: ordinal.as_u64().ok_or(NodeWireError::Ordinal)?,
        rest: path.as_array().ok_or(NodeWireError::Path)?.iter(),
        segments: Vec::new(),
    })
}

/// The node identity `value` spells.
///
/// # Errors
///
/// A [`NodeWireError`] naming the first malformed member, a failed storage
/// reservation, or the identity's own refusal ([`InvalidNodeKey`]).
pub fn node_key_from_json(value: &Value) -> Result<NodeKey, NodeWireError> {
    let mut stack = Vec::new();
    push(&mut stack, header(value)?)?;
    loop {
        let Some(top) = stack.last_mut() else {
            return Err(NodeWireError::Shape);
        };
        match top.rest.next() {
            Some(Value::String(text)) => push(&mut top.segments, PathSegment::Text(owned(text)?))?,
            Some(nested @ Value::Object(_)) => push(&mut stack, header(nested)?)?,
            Some(_) => return Err(NodeWireError::Path),
            None => {
                let Some(done) = stack.pop() else {
                    return Err(NodeWireError::Shape);
                };
                let key = NodeKey::try_new(done.package, done.segments, done.ordinal)?;
                match stack.last_mut() {
                    Some(parent) => push(&mut parent.segments, PathSegment::Node(key))?,
                    None => return Ok(key),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(value: &str) -> PathSegment {
        PathSegment::Text(value.into())
    }

    fn nested(levels: usize) -> NodeKey {
        let mut key = NodeKey::new("lib", alloc::vec![text("leaf")], 0).unwrap();
        for level in 0..levels {
            key = NodeKey::new(
                "lib",
                alloc::vec![text("wrap"), PathSegment::Node(key)],
                level as u64,
            )
            .unwrap();
        }
        key
    }

    #[test]
    fn a_nested_identity_round_trips() {
        let inner = NodeKey::new("dep", alloc::vec![text("a"), text("")], 7).unwrap();
        let key = NodeKey::new(
            "lib",
            alloc::vec![text("op"), PathSegment::Node(inner), text("z")],
            3,
        )
        .unwrap();
        let wire = node_key_to_json(&key).unwrap();
        assert_eq!(node_key_from_json(&wire).unwrap(), key);
    }

    #[test]
    fn the_wire_shape_carries_package_path_and_ordinal_only() {
        let key = NodeKey::new("lib", alloc::vec![text("a")], 2).unwrap();
        let wire = node_key_to_json(&key).unwrap();
        assert_eq!(wire, json!({"package": "lib", "path": ["a"], "ordinal": 2}));
    }

    #[test]
    fn nesting_far_beyond_any_small_depth_round_trips_without_stack_growth() {
        let key = nested(10_000);
        let wire = node_key_to_json(&key).unwrap();
        assert_eq!(node_key_from_json(&wire).unwrap(), key);
        discard_node_json(wire);
    }

    #[test]
    fn malformed_shapes_are_refused_by_member() {
        let refuse = |value: Value| node_key_from_json(&value).unwrap_err();
        assert_eq!(refuse(json!("x")), NodeWireError::Shape);
        assert_eq!(
            refuse(json!({"package": "a", "path": ["p"]})),
            NodeWireError::Shape
        );
        assert_eq!(
            refuse(json!({"package": "a", "path": ["p"], "ordinal": 0, "x": 1})),
            NodeWireError::Shape
        );
        assert_eq!(
            refuse(json!({"package": 1, "path": ["p"], "ordinal": 0})),
            NodeWireError::Package
        );
        assert_eq!(
            refuse(json!({"package": "a", "path": "p", "ordinal": 0})),
            NodeWireError::Path
        );
        assert_eq!(
            refuse(json!({"package": "a", "path": [1], "ordinal": 0})),
            NodeWireError::Path
        );
        assert_eq!(
            refuse(json!({"package": "a", "path": ["p"], "ordinal": -1})),
            NodeWireError::Ordinal
        );
    }

    #[test]
    fn an_inadmissible_identity_is_refused_by_the_key() {
        let empty_package = json!({"package": "", "path": ["p"], "ordinal": 0});
        assert_eq!(
            node_key_from_json(&empty_package),
            Err(NodeWireError::Key(InvalidNodeKey::EmptyPackage))
        );
        let empty_path = json!({"package": "a", "path": [], "ordinal": 0});
        assert_eq!(
            node_key_from_json(&empty_path),
            Err(NodeWireError::Key(InvalidNodeKey::EmptyPath))
        );
    }
}
