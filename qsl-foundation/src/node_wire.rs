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
    /// Caller work, storage or cancellation admission stopped the conversion.
    #[error("node identity conversion admission denied")]
    Denied,
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

/// Borrowed events and admitted traversal storage for the single structural
/// node-identity projection. The sink chooses how to retain or emit events;
/// this trait itself supplies no canonical writer, byte limit or allocation.
/// Every sink refusal propagates through its associated error unchanged.
pub trait NodeIdentitySink {
    /// The sink's typed refusal.
    type Error;
    /// Admit one traversal event.
    fn visit(&mut self) -> Result<(), Self::Error>;
    /// Begin a structural object.
    fn begin_object(&mut self) -> Result<(), Self::Error>;
    /// Name a structural member.
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error>;
    /// Emit the admitted ordinal as a JSON number.
    fn ordinal(&mut self, ordinal: u64) -> Result<(), Self::Error>;
    /// Emit exact borrowed text.
    fn text(&mut self, text: &str) -> Result<(), Self::Error>;
    /// Begin a path.
    fn begin_array(&mut self) -> Result<(), Self::Error>;
    /// End a path.
    fn end_array(&mut self) -> Result<(), Self::Error>;
    /// End a structural object.
    fn end_object(&mut self) -> Result<(), Self::Error>;
    /// Admit and retain a traversal frame before its stack grows.
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Self::Error>;
}

struct CanonicalNode<'w, 's, S: quire_canonical::Sink + ?Sized>(
    &'w mut quire_canonical::Writer<'s, S>,
);
impl<S: quire_canonical::Sink + ?Sized> NodeIdentitySink for CanonicalNode<'_, '_, S> {
    type Error = quire_canonical::Error;
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn begin_object(&mut self) -> Result<(), Self::Error> {
        self.0.begin_object()
    }
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error> {
        self.0.name(name)
    }
    fn ordinal(&mut self, ordinal: u64) -> Result<(), Self::Error> {
        self.0.integer(i128::from(ordinal))
    }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> {
        self.0.string(text)
    }
    fn begin_array(&mut self) -> Result<(), Self::Error> {
        self.0.begin_array()
    }
    fn end_array(&mut self) -> Result<(), Self::Error> {
        self.0.end_array()
    }
    fn end_object(&mut self) -> Result<(), Self::Error> {
        self.0.end_object()
    }
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Self::Error> {
        self.0.reserve_list(stack, 1)?;
        stack.push(path);
        Ok(())
    }
}

fn project_node_head<S: NodeIdentitySink>(
    package: &str,
    ordinal: u64,
    sink: &mut S,
) -> Result<(), S::Error> {
    sink.visit()?;
    sink.begin_object()?;
    sink.name("ordinal")?;
    sink.ordinal(ordinal)?;
    sink.name("package")?;
    sink.text(package)?;
    sink.name("path")?;
    sink.begin_array()
}

/// A validated structural node identity with a fixed borrowed text path.
/// It uses the same canonical projection as [`encode_node_key`] without
/// constructing an owned node identity, text or JSON tree.
#[derive(Clone, Copy, Debug)]
pub struct TextNodeIdentity<'a, const N: usize> {
    package: &'a str,
    path: [&'a str; N],
    ordinal: u64,
}

impl<'a, const N: usize> TextNodeIdentity<'a, N> {
    /// Validate the same package, nonempty path and ordinal constraints as
    /// [`NodeKey::new`], without allocating. Path text is opaque.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey::EmptyPackage`], [`InvalidNodeKey::EmptyPath`] or
    /// [`InvalidNodeKey::OrdinalOutOfRange`] for invalid structural input.
    pub fn new(package: &'a str, path: [&'a str; N], ordinal: u64) -> Result<Self, InvalidNodeKey> {
        if package.is_empty() {
            return Err(InvalidNodeKey::EmptyPackage);
        }
        if N == 0 {
            return Err(InvalidNodeKey::EmptyPath);
        }
        if ordinal > quire_exact::MAX_NODE_ORDINAL {
            return Err(InvalidNodeKey::OrdinalOutOfRange);
        }
        Ok(Self {
            package,
            path,
            ordinal,
        })
    }

    /// Emit this borrowed identity through the shared structural projection.
    ///
    /// # Errors
    /// The sink's admission or encoding refusal.
    pub fn project<S: NodeIdentitySink>(&self, sink: &mut S) -> Result<(), S::Error> {
        project_node_head(self.package, self.ordinal, sink)?;
        for segment in self.path {
            sink.visit()?;
            sink.text(segment)?;
        }
        sink.end_array()?;
        sink.end_object()
    }

    /// The owning package text.
    pub fn package(&self) -> &'a str {
        self.package
    }

    /// The direct opaque path segments.
    pub fn path(&self) -> &[&'a str; N] {
        &self.path
    }
}

impl<const N: usize> quire_canonical::Encode for TextNodeIdentity<'_, N> {
    fn encode_into<S: quire_canonical::Sink + ?Sized>(
        &self,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        self.project(&mut CanonicalNode(writer))
    }
}

/// Emit a structural node identity directly into a canonical writer.
///
/// The writer admits its own storage and this traversal's stack before growth.
/// No intermediate identity text or JSON tree is retained.
///
/// # Errors
///
/// The writer's byte, reservation, allocation or scalar refusal.
pub fn encode_node_key<S: quire_canonical::Sink + ?Sized>(
    key: &NodeKey,
    writer: &mut quire_canonical::Writer<'_, S>,
) -> Result<(), quire_canonical::Error> {
    project_node_key(key, &mut CanonicalNode(writer))
}

/// Emit a node identity through the same structural recipe as the canonical
/// writer, admitting traversal and storage through the supplied sink.
///
/// # Errors
/// The sink's admission or encoding refusal.
pub fn project_node_key<S: NodeIdentitySink>(key: &NodeKey, sink: &mut S) -> Result<(), S::Error> {
    let mut stack = Vec::new();
    let root = key.view();
    project_node_head(root.package(), root.ordinal(), sink)?;
    sink.push_path(&mut stack, root.path())?;
    while let Some(path) = stack.last_mut() {
        sink.visit()?;
        match path.next() {
            Some(PathSegmentView::Text(text)) => sink.text(text)?,
            Some(PathSegmentView::Node(view)) => {
                project_node_head(view.package(), view.ordinal(), sink)?;
                sink.push_path(&mut stack, view.path())?;
            }
            None => {
                sink.end_array()?;
                sink.end_object()?;
                stack.pop();
            }
        }
    }
    Ok(())
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
    struct Unbounded;
    impl quire_exact::Reservation for Unbounded {
        fn reserve(&mut self, _: usize) -> Result<(), quire_exact::ReservationDenied> { Ok(()) }
    }
    impl quire_exact::IntegerDecimalBudget for Unbounded {
        fn work(&mut self, _: u64) -> Result<(), quire_exact::ReservationDenied> { Ok(()) }
    }
    node_key_from_text_with_budget(text, &mut Unbounded)
}

/// Parse the shared structural identity with caller admission before each
/// traversal and actual string, stack, path and Exact construction allocation.
/// A denied caller retains its precise work/storage/cancellation cause.
///
/// # Errors
/// [`NodeWireError::Denied`] for caller denial; the other variants retain the
/// legacy parser's shape, construction and native allocation distinctions.
#[ix_trace_rs::string_edge]
pub fn node_key_from_text_with_budget(
    text: &str,
    budget: &mut dyn quire_exact::IntegerDecimalBudget,
) -> Result<NodeKey, NodeWireError> {
    fn push<T>(vec: &mut Vec<T>, item: T, budget: &mut dyn quire_exact::IntegerDecimalBudget) -> Result<(), NodeWireError> {
        if vec.len() == vec.capacity() {
            let capacity = vec.capacity().checked_mul(2).unwrap_or(usize::MAX).max(4);
            let bytes = capacity.checked_mul(core::mem::size_of::<T>()).ok_or(NodeWireError::Allocation)?;
            budget.reserve(bytes).map_err(|_| NodeWireError::Denied)?;
            vec.try_reserve_exact(capacity - vec.len()).map_err(|_| NodeWireError::Allocation)?;
        }
        vec.push(item);
        Ok(())
    }
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
    fn string(text: &str, at: &mut usize, budget: &mut dyn quire_exact::IntegerDecimalBudget) -> Result<String, NodeWireError> {
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
        let raw = text.get(start + 1..end).ok_or(NodeWireError::Shape)?;
        budget.work(u64::try_from(raw.len()).map_err(|_| NodeWireError::Allocation)?).map_err(|_| NodeWireError::Denied)?;
        budget.reserve(raw.len()).map_err(|_| NodeWireError::Denied)?;
        let mut decoded = String::new();
        decoded.try_reserve_exact(raw.len()).map_err(|_| NodeWireError::Allocation)?;
        let mut chars = raw.chars();
        fn hex(chars: &mut core::str::Chars<'_>) -> Result<u32, NodeWireError> {
            let mut value = 0;
            for _ in 0..4 {
                value = value * 16 + chars.next().and_then(|c| c.to_digit(16)).ok_or(NodeWireError::Shape)?;
            }
            Ok(value)
        }
        while let Some(next) = chars.next() {
            let next = if next == '\\' {
                match chars.next().ok_or(NodeWireError::Shape)? {
                    '"' => '"', '\\' => '\\', '/' => '/',
                    'b' => '\u{8}', 'f' => '\u{c}', 'n' => '\n', 'r' => '\r', 't' => '\t',
                    'u' => {
                        let high = hex(&mut chars)?;
                        let scalar = if (0xd800..=0xdbff).contains(&high) {
                            if chars.next() != Some('\\') || chars.next() != Some('u') { return Err(NodeWireError::Shape); }
                            let low = hex(&mut chars)?;
                            if !(0xdc00..=0xdfff).contains(&low) { return Err(NodeWireError::Shape); }
                            0x10000 + ((high - 0xd800) << 10) + low - 0xdc00
                        } else { high };
                        char::from_u32(scalar).ok_or(NodeWireError::Shape)?
                    }
                    _ => return Err(NodeWireError::Shape),
                }
            } else {
                if next <= '\u{1f}' || next == '"' { return Err(NodeWireError::Shape); }
                next
            };
            decoded.push(next);
        }
        Ok(decoded)
    }
    let bytes = text.as_bytes();
    budget.work(u64::try_from(bytes.len()).map_err(|_| NodeWireError::Allocation)?).map_err(|_| NodeWireError::Denied)?;
    let mut at = 0;
    expect(bytes, &mut at, b'{')?;
    let mut stack = Vec::new();
    push(&mut stack, frame(), budget)?;
    loop {
        budget.work(1).map_err(|_| NodeWireError::Denied)?;
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
                let work = done.segments.iter().fold(package.len(), |work, segment| work.saturating_add(match segment { PathSegment::Text(text) => text.len(), PathSegment::Node(_) => 1 }));
                budget.work(u64::try_from(work).map_err(|_| NodeWireError::Allocation)?).map_err(|_| NodeWireError::Denied)?;
                let key = NodeKey::try_new_with(package, done.segments, ordinal, &mut |bytes| budget.reserve(bytes))
                    .map_err(|error| match error {
                        quire_exact::NodeKeyConstructionError::Policy(_) => NodeWireError::Denied,
                        quire_exact::NodeKeyConstructionError::Construction(error) => NodeWireError::Key(error),
                    })?;
                match stack.last_mut() {
                    Some(parent) => push(&mut parent.segments, PathSegment::Node(key), budget)?,
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
                let name = string(text, &mut at, budget)?;
                expect(bytes, &mut at, b':')?;
                skip(bytes, &mut at);
                match name.as_str() {
                    "package" if top.package.is_none() => {
                        top.package =
                            Some(string(text, &mut at, budget).map_err(|error| if error == NodeWireError::Denied { error } else { NodeWireError::Package })?);
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
                    let name = string(text, &mut at, budget).map_err(|error| if error == NodeWireError::Denied { error } else { NodeWireError::Path })?;
                    push(&mut top.segments, PathSegment::Text(name), budget)?;
                    top.state = State::PathAfter;
                }
                Some(b'{') => {
                    at += 1;
                    top.state = State::PathAfter;
                    push(&mut stack, frame(), budget)?;
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

    #[ix_trace_rs::trace("TC-1406", "FR-201-AC-50")]
    #[test]
    fn text_parser_admits_real_blocks_before_allocating_and_preserves_caller_cancellation() {
        struct Budget {
            remaining: u64,
            requests: Vec<usize>,
            cancel: quire_exact::Cancel,
            cause: Option<quire_exact::CancelCause>,
        }
        impl quire_exact::Reservation for Budget {
            fn reserve(&mut self, bytes: usize) -> Result<(), quire_exact::ReservationDenied> {
                self.requests.push(bytes);
                let bytes = u64::try_from(bytes).map_err(|_| quire_exact::ReservationDenied)?;
                self.remaining = self.remaining.checked_sub(bytes).ok_or(quire_exact::ReservationDenied)?;
                Ok(())
            }
        }
        impl quire_exact::IntegerDecimalBudget for Budget {
            fn work(&mut self, _: u64) -> Result<(), quire_exact::ReservationDenied> {
                if let Some(cause) = self.cause { self.cancel.cancel(cause); }
                self.cancel.poll();
                if self.cancel.tripped().is_some() { Err(quire_exact::ReservationDenied) } else { Ok(()) }
            }
        }
        let budget = |remaining, cause| Budget { remaining, requests: Vec::new(), cancel: quire_exact::Cancel::new(), cause };
        let key = nested(1024);
        let text = node_key_to_text(&key).unwrap();
        let mut complete = budget(u64::MAX, None);
        assert_eq!(node_key_from_text_with_budget(&text, &mut complete).unwrap(), key);
        let required = u64::MAX - complete.remaining;
        let mut short = budget(required - 1, None);
        assert_eq!(node_key_from_text_with_budget(&text, &mut short), Err(NodeWireError::Denied));
        assert_eq!(short.requests.last(), complete.requests.last());
        for cause in [quire_exact::CancelCause::Requested, quire_exact::CancelCause::Deadline] {
            let mut cancelled = budget(u64::MAX, Some(cause));
            assert_eq!(node_key_from_text_with_budget(&text, &mut cancelled), Err(NodeWireError::Denied));
            assert_eq!(cancelled.cancel.tripped(), Some(cause));
            assert!(cancelled.requests.is_empty(), "the actual text-work admission precedes every parser allocation");
        }
        let escaped = r#"{"package":"p\u006bg","path":["\ud83d\ude00","\"\\\b\f\n\r\t\/"],"ordinal":0}"#;
        let value: Value = serde_json::from_str(escaped).unwrap();
        assert_eq!(node_key_from_text(escaped).unwrap(), node_key_from_json(&value).unwrap());
    }

    #[ix_trace_rs::trace("TC-728", "FR-259-AC-1")]
    #[test]
    fn borrowed_text_node_identity_keeps_literal_bytes_and_shape_validation() {
        let view = TextNodeIdentity::new("pkg", ["model", "object_type", "node\"\\\n"], 0)
            .expect("valid borrowed node identity");
        let bytes = quire_canonical::to_vec(&view, quire_canonical::Limits::new(1024))
            .expect("borrowed identity encodes");
        assert_eq!(
            bytes,
            br#"{"ordinal":0,"package":"pkg","path":["model","object_type","node\"\\\n"]}"#
        );
        assert!(matches!(
            TextNodeIdentity::new("", ["p"], 0),
            Err(InvalidNodeKey::EmptyPackage)
        ));
        assert!(matches!(
            TextNodeIdentity::new("pkg", [], 0),
            Err(InvalidNodeKey::EmptyPath)
        ));
        assert!(matches!(
            TextNodeIdentity::new("pkg", ["p"], quire_exact::MAX_NODE_ORDINAL + 1),
            Err(InvalidNodeKey::OrdinalOutOfRange)
        ));
        let upper = TextNodeIdentity::new("pkg", ["p"], quire_exact::MAX_NODE_ORDINAL)
            .expect("maximum exact ordinal");
        assert_eq!(
            quire_canonical::to_vec(&upper, quire_canonical::Limits::new(1024)).unwrap(),
            br#"{"ordinal":9007199254740991,"package":"pkg","path":["p"]}"#,
        );
    }

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
