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

use alloc::string::{String, ToString};
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

/// `key` as its compact JSON text, members in the order `serde_json` writes a
/// [`Value`] object (`ordinal`, `package`, `path`). Written with an explicit
/// stack, so depth is bounded only by memory.
///
/// # Errors
///
/// [`NodeWireError::Allocation`] when storage cannot be reserved, or
/// [`NodeWireError::Shape`] when a string cannot be written.
pub fn node_key_to_text(key: &NodeKey) -> Result<String, NodeWireError> {
    fn reserve(out: &mut String, extra: usize) -> Result<(), NodeWireError> {
        out.try_reserve(extra)
            .map_err(|_| NodeWireError::Allocation)
    }
    fn string(out: &mut String, text: &str) -> Result<(), NodeWireError> {
        let quoted = serde_json::to_string(text).map_err(|_| NodeWireError::Shape)?;
        reserve(out, quoted.len())?;
        out.push_str(&quoted);
        Ok(())
    }
    fn head(out: &mut String, view: &NodeKeyView<'_>) -> Result<(), NodeWireError> {
        reserve(out, 32)?;
        out.push_str("{\"ordinal\":");
        out.push_str(&view.ordinal().to_string());
        out.push_str(",\"package\":");
        string(out, view.package())?;
        out.push_str(",\"path\":[");
        Ok(())
    }
    let mut out = String::new();
    let root = key.view();
    head(&mut out, &root)?;
    // Each frame: the remaining path and whether a segment was written yet.
    let mut stack: Vec<(NodePathIter<'_>, bool)> = Vec::new();
    push(&mut stack, (root.path(), false))?;
    while let Some(top) = stack.last_mut() {
        match top.0.next() {
            Some(segment) => {
                if top.1 {
                    reserve(&mut out, 1)?;
                    out.push(',');
                }
                top.1 = true;
                match segment {
                    PathSegmentView::Text(text) => string(&mut out, text)?,
                    PathSegmentView::Node(view) => {
                        head(&mut out, &view)?;
                        push(&mut stack, (view.path(), false))?;
                    }
                }
            }
            None => {
                reserve(&mut out, 1)?;
                out.push_str("]}");
                stack.pop();
            }
        }
    }
    Ok(out)
}

/// The node identity `text` spells, read in one pass with an explicit stack
/// and a hand-written tokenizer for the fixed shape, so depth is bounded only
/// by memory and cost is linear in the text.
///
/// # Errors
///
/// A [`NodeWireError`] for malformed text, a failed storage reservation, or the
/// identity's own refusal ([`InvalidNodeKey`]).
#[ix_trace_rs::string_edge]
pub fn node_key_from_text(text: &str) -> Result<NodeKey, NodeWireError> {
    #[derive(Clone, Copy)]
    enum State {
        ObjStart,
        ObjNext,
        ObjAfter,
        PathStart,
        PathNext,
        PathAfter,
    }
    struct Frame {
        state: State,
        package: Option<String>,
        ordinal: Option<u64>,
        path_seen: bool,
        segments: Vec<PathSegment>,
    }
    fn frame() -> Frame {
        Frame {
            state: State::ObjStart,
            package: None,
            ordinal: None,
            path_seen: false,
            segments: Vec::new(),
        }
    }
    fn skip(bytes: &[u8], at: &mut usize) {
        while matches!(bytes.get(*at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            *at += 1;
        }
    }
    fn expect(bytes: &[u8], at: &mut usize, want: u8) -> Result<(), NodeWireError> {
        skip(bytes, at);
        if bytes.get(*at) == Some(&want) {
            *at += 1;
            Ok(())
        } else {
            Err(NodeWireError::Shape)
        }
    }
    // A JSON string starting at `at`; the cursor moves past it.
    fn string(text: &str, at: &mut usize) -> Result<String, NodeWireError> {
        let bytes = text.as_bytes();
        let start = *at;
        if bytes.get(start) != Some(&b'"') {
            return Err(NodeWireError::Shape);
        }
        let mut end = start + 1;
        loop {
            match bytes.get(end) {
                Some(b'\\') => end += 2,
                Some(b'"') => break,
                Some(_) => end += 1,
                None => return Err(NodeWireError::Shape),
            }
        }
        *at = end + 1;
        let literal = text.get(start..*at).ok_or(NodeWireError::Shape)?;
        serde_json::from_str(literal).map_err(|_| NodeWireError::Shape)
    }
    let bytes = text.as_bytes();
    let mut at = 0;
    expect(bytes, &mut at, b'{')?;
    let mut stack = Vec::new();
    push(&mut stack, frame())?;
    loop {
        skip(bytes, &mut at);
        let Some(top) = stack.last_mut() else {
            return Err(NodeWireError::Shape);
        };
        let next = bytes.get(at).copied();
        match top.state {
            State::ObjAfter if next == Some(b',') => {
                at += 1;
                top.state = State::ObjNext;
            }
            State::ObjAfter | State::ObjStart if next == Some(b'}') => {
                at += 1;
                let Some(done) = stack.pop() else {
                    return Err(NodeWireError::Shape);
                };
                if !done.path_seen {
                    return Err(NodeWireError::Shape);
                }
                let package = done.package.ok_or(NodeWireError::Shape)?;
                let ordinal = done.ordinal.ok_or(NodeWireError::Shape)?;
                let key = NodeKey::try_new(package, done.segments, ordinal)?;
                match stack.last_mut() {
                    Some(parent) => push(&mut parent.segments, PathSegment::Node(key))?,
                    None => {
                        skip(bytes, &mut at);
                        return if at == bytes.len() {
                            Ok(key)
                        } else {
                            Err(NodeWireError::Shape)
                        };
                    }
                }
            }
            State::ObjStart | State::ObjNext => {
                let name = string(text, &mut at)?;
                expect(bytes, &mut at, b':')?;
                skip(bytes, &mut at);
                match name.as_str() {
                    "package" if top.package.is_none() => {
                        top.package =
                            Some(string(text, &mut at).map_err(|_| NodeWireError::Package)?);
                    }
                    "ordinal" if top.ordinal.is_none() => {
                        let start = at;
                        while matches!(bytes.get(at), Some(b'0'..=b'9')) {
                            at += 1;
                        }
                        let digits = text.get(start..at).ok_or(NodeWireError::Ordinal)?;
                        if digits.len() > 1 && digits.starts_with('0') {
                            return Err(NodeWireError::Ordinal);
                        }
                        top.ordinal = Some(digits.parse().map_err(|_| NodeWireError::Ordinal)?);
                    }
                    "path" if !top.path_seen => {
                        expect(bytes, &mut at, b'[')?;
                        top.path_seen = true;
                        top.state = State::PathStart;
                        continue;
                    }
                    _ => return Err(NodeWireError::Shape),
                }
                top.state = State::ObjAfter;
            }
            State::ObjAfter => return Err(NodeWireError::Shape),
            State::PathAfter if next == Some(b',') => {
                at += 1;
                top.state = State::PathNext;
            }
            State::PathAfter | State::PathStart if next == Some(b']') => {
                at += 1;
                top.state = State::ObjAfter;
            }
            State::PathStart | State::PathNext => match next {
                Some(b'"') => {
                    let name = string(text, &mut at).map_err(|_| NodeWireError::Path)?;
                    push(&mut top.segments, PathSegment::Text(name))?;
                    top.state = State::PathAfter;
                }
                Some(b'{') => {
                    at += 1;
                    top.state = State::PathAfter;
                    push(&mut stack, frame())?;
                }
                _ => return Err(NodeWireError::Path),
            },
            State::PathAfter => return Err(NodeWireError::Path),
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
    fn deep_text_round_trips_and_matches_the_value_spelling() {
        let key = nested(200);
        let text = node_key_to_text(&key).unwrap();
        assert_eq!(node_key_from_text(&text).unwrap(), key);
        let shallow = nested(3);
        let value = node_key_to_json(&shallow).unwrap();
        assert_eq!(
            node_key_to_text(&shallow).unwrap(),
            serde_json::to_string(&value).unwrap()
        );
    }

    #[test]
    fn two_thousand_deep_text_reads_in_one_pass() {
        let key = nested(2000);
        let text = node_key_to_text(&key).unwrap();
        assert_eq!(node_key_from_text(&text).unwrap(), key);
    }

    #[test]
    fn malformed_text_is_refused() {
        for bad in [
            "",
            "{}",
            r#"{"ordinal":1,"package":"p"}"#,
            r#"{"ordinal":1,"package":"p","path":[],"x":1}"#,
            r#"{"ordinal":01,"package":"p","path":["a"]}"#,
            r#"{"ordinal":1,"package":"p","path":[1]}"#,
            r#"{"ordinal":1,"package":"p","path":["a"]} x"#,
            r#"{"ordinal":1,"ordinal":1,"package":"p","path":["a"]}"#,
        ] {
            assert!(node_key_from_text(bad).is_err(), "{bad}");
        }
        let any_order = r#" {"path":["a"], "package":"p","ordinal":2} "#;
        assert!(node_key_from_text(any_order).is_ok());
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
