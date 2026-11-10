// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "State, state key and anchor" and "Applications and transition
//! identities": QSpec FR-181's typed canonical form of a model state, of a
//! value and of a transition identity.
//!
//! A value, key or identity is built as an owned [`Enc`] tree and written
//! through `quire-canonical`'s event writer, or by [`Canonicalizer`] straight
//! to the same RFC 8785 bytes. The tree's depth follows the nesting of the
//! declared value types, which a checked package bounds, so building and
//! writing it recurse once per declared nesting level and never per element.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};

use qsl_foundation::node_wire::node_key_from_json;
use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::{
    EffectiveId, FieldValue, NodeKey, ObjectReference, OriginalDeclarationKey, PathSegmentView,
    UniverseId, Value,
};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};
use serde_json::Value as Json;

use qsl_semantics::check::VariantTypeDeclaration;
use qsl_semantics::model::state::{ModelState, StateField, StateModel};
use quire_checked::key::DeclarationKey;

mod projection;

/// One JSON value in FR-181's typed canonical form, before it is written.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Enc {
    Null,
    Bool(bool),
    Str(String),
    Array(Vec<Enc>),
    /// A structural node identity, written as its `{package, path, ordinal}`
    /// object.
    Node(NodeKey),
    /// A structural effective declaration identity, written as FR-201's
    /// recursive `{owner_effective_type, original}` record.
    Effective(EffectiveId),
    /// Members in any order: the writer sorts them (RFC 8785 section 3.2.3).
    Object(Vec<(&'static str, Enc)>),
    /// A value many members and keys share, with its RFC 8785 bytes found
    /// once when it was made ([`Enc::shared`]): written as the value it
    /// holds, and by [`Canonicalizer`] as those bytes.
    Shared(Arc<SharedEnc>),
    /// A declaration's identity text ([`declaration_text`]) as a string.
    Declaration(DeclarationKey),
    /// FR-181's `map`, `{"type":"map","entries":[{"key":..,"value":..}, ...]}`,
    /// its entries in any order: they are written ascending by the key's
    /// RFC 8785 bytes, entries with equal key bytes in the order held.
    Map(Vec<(Enc, Enc)>),
    /// A value FR-181's typed canonical form does not yet cover here; it has
    /// no encoding, and writing it refuses with this description.
    Unencodable(&'static str),
}

impl Encode for Enc {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        match self {
            Self::Null => writer.null(),
            Self::Bool(value) => writer.bool(*value),
            Self::Str(value) => writer.string(value),
            Self::Array(items) => {
                writer.begin_array()?;
                for item in items {
                    item.encode_into(writer)?;
                }
                writer.end_array()
            }
            Self::Node(key) => BorrowedIdentity::Node(key).encode_into(writer),
            Self::Effective(id) => BorrowedIdentity::Effective(id).encode_into(writer),
            Self::Shared(shared) => shared.value.encode_into(writer),
            Self::Declaration(key) => BorrowedIdentity::Declaration(key).encode_into(writer),
            Self::Object(members) => {
                writer.begin_object()?;
                for (name, value) in members {
                    writer.name(name)?;
                    value.encode_into(writer)?;
                }
                writer.end_object()
            }
            Self::Map(entries) => {
                // A key with no encoding sorts first, so writing it refuses
                // before any other entry is written.
                let limits = Limits::new(u64::MAX);
                let mut keyed: Vec<(Vec<u8>, &Enc, &Enc)> = entries
                    .iter()
                    .map(|(key, value)| {
                        let bytes = quire_canonical::to_vec(key, limits).unwrap_or_default();
                        (bytes, key, value)
                    })
                    .collect();
                keyed.sort_by(|left, right| left.0.cmp(&right.0));
                writer.begin_object()?;
                writer.name("entries")?;
                writer.begin_array()?;
                for (_, key, value) in keyed {
                    writer.begin_object()?;
                    writer.name("key")?;
                    key.encode_into(writer)?;
                    writer.name("value")?;
                    value.encode_into(writer)?;
                    writer.end_object()?;
                }
                writer.end_array()?;
                writer.name("type")?;
                writer.string("map")?;
                writer.end_object()
            }
            Self::Unencodable(what) => Err(unencodable(what)),
        }
    }
}

/// What a declaration identity whose text does not encode is.
const UNENCODABLE_DECLARATION: &str = "a declaration identity";

/// The refusal of a value FR-181's typed canonical form does not cover.
fn unencodable(what: &str) -> Error {
    Error::Serialize(format!(
        "{what} has no typed canonical form in a model state"
    ))
}

/// The RFC 8785 bytes of [`Enc`] trees, written straight into one buffer.
///
/// A tree is written directly when it can be: the members of an object are
/// written in the order held and then arranged by name, a map's entries by
/// key bytes, and a string with nothing to escape is its UTF-8 bytes in
/// quotes (RFC 8785 section 3.2.2.2); any other string, and each node,
/// effective and declaration identity, is encoded by `quire-canonical`.
/// Those bytes are the ones `quire_canonical::to_vec` gives for the tree.
/// Whenever the tree has no encoding or its bytes exceed the caller's bound,
/// `quire_canonical::to_vec` writes it instead, so every refusal is
/// `quire-canonical`'s own. An identity's bytes are encoded once per
/// canonicalizer and reused, so the keys of one system encode each location
/// or reference they name once.
///
/// It is `pub` inside this crate-private module so that
/// [`MemoryModel::key_bytes`](crate::simulation::protocol::MemoryModel::key_bytes)
/// can take it while no other crate can name it, and so cannot override
/// that method.
#[derive(Default)]
pub struct Canonicalizer {
    nodes: RwLock<HashMap<NodeKey, Arc<[u8]>>>,
    effective: RwLock<HashMap<EffectiveId, Arc<[u8]>>>,
    declarations: RwLock<HashMap<DeclarationKey, Arc<[u8]>>>,
}

/// The canonical bytes written so far, against the caller's byte bound.
///
/// Every write returns `None` when the direct path stops: the tree has no
/// encoding, or the bytes already exceed the bound, which the whole tree's
/// bytes then exceed too. `quire_canonical::to_vec` then decides.
struct Out {
    bytes: Vec<u8>,
    bound: u64,
}

impl Out {
    fn new(bound: u64) -> Self {
        Self {
            bytes: Vec::new(),
            bound,
        }
    }

    /// Append `bytes`, stopping past the bound.
    fn push(&mut self, bytes: &[u8]) -> Option<()> {
        let length = u64::try_from(self.bytes.len().checked_add(bytes.len())?).ok()?;
        if length > self.bound {
            return None;
        }
        self.bytes.try_reserve(bytes.len()).ok()?;
        self.bytes.extend_from_slice(bytes);
        Some(())
    }

    /// The bytes written, when `quire-canonical`'s writer gives them too:
    /// it buffers an object's bytes behind 32-bit offsets, so bytes past
    /// `u32::MAX` are left to it.
    fn finish(self) -> Option<Vec<u8>> {
        u32::try_from(self.bytes.len()).ok()?;
        Some(self.bytes)
    }
}

/// One member's value: a typed canonical form, or RFC 8785 bytes already
/// written.
enum EncodedMember<'m> {
    Enc(&'m Enc),
    Bytes(&'m [u8]),
}

/// One `quire-canonical` string event's value.
struct Text<'t>(&'t str);

impl Encode for Text<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.string(self.0)
    }
}

/// RFC 8785 section 3.2.3: member names compare by UTF-16 code units.
fn cmp_names(left: &str, right: &str) -> std::cmp::Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

impl Canonicalizer {
    /// `value`'s RFC 8785 bytes under `limits`: the result
    /// `quire_canonical::to_vec(value, limits)` gives.
    ///
    /// # Errors
    ///
    /// The refusal `quire_canonical::to_vec(value, limits)` gives.
    pub(crate) fn to_vec(&self, value: &Enc, limits: Limits) -> Result<Vec<u8>, Error> {
        match self.written(value, limits) {
            Some(bytes) => Ok(bytes),
            None => quire_canonical::to_vec(value, limits),
        }
    }

    /// `value`'s RFC 8785 bytes under `limits`, when they can be written
    /// directly: the bytes `quire_canonical::to_vec(value, limits)` gives.
    ///
    /// `None` when `value` has no encoding or exceeds `limits`:
    /// `quire_canonical::to_vec` then decides the refusal.
    pub(crate) fn written(&self, value: &Enc, limits: Limits) -> Option<Vec<u8>> {
        let mut out = Out::new(limits.max_bytes());
        self.write(value, &mut out)?;
        out.finish()
    }

    /// The RFC 8785 bytes of the object of `members` and the member `name`,
    /// whose value is the RFC 8785 bytes `bytes`, when they can be written
    /// directly within `limits`.
    ///
    /// `None` when the object has no encoding or exceeds `limits`: the
    /// caller's own encoding of it then decides the refusal.
    pub(crate) fn object_with(
        &self,
        members: &[(&'static str, Enc)],
        name: &'static str,
        bytes: &[u8],
        limits: Limits,
    ) -> Option<Vec<u8>> {
        let mut out = Out::new(limits.max_bytes());
        let members = members
            .iter()
            .map(|(name, value)| (*name, EncodedMember::Enc(value)))
            .chain(std::iter::once((name, EncodedMember::Bytes(bytes))));
        self.object(members, &mut out)?;
        out.finish()
    }

    fn write(&self, value: &Enc, out: &mut Out) -> Option<()> {
        match value {
            Enc::Null => out.push(b"null"),
            Enc::Bool(true) => out.push(b"true"),
            Enc::Bool(false) => out.push(b"false"),
            Enc::Str(text) => Self::string(text, out),
            Enc::Array(items) => {
                out.push(b"[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(b",")?;
                    }
                    self.write(item, out)?;
                }
                out.push(b"]")
            }
            Enc::Node(key) => Self::identity(&self.nodes, key, &BorrowedIdentity::Node(key), out),
            Enc::Effective(id) => {
                Self::identity(&self.effective, id, &BorrowedIdentity::Effective(id), out)
            }
            Enc::Declaration(key) => Self::identity(
                &self.declarations,
                key,
                &BorrowedIdentity::Declaration(key),
                out,
            ),
            Enc::Shared(shared) => match &shared.bytes {
                Some(bytes) => out.push(bytes),
                None => self.write(&shared.value, out),
            },
            Enc::Object(members) => self.object(
                members
                    .iter()
                    .map(|(name, value)| (*name, EncodedMember::Enc(value))),
                out,
            ),
            Enc::Map(entries) => self.map(entries, out),
            Enc::Unencodable(_) => None,
        }
    }

    fn string(text: &str, out: &mut Out) -> Option<()> {
        if text
            .bytes()
            .all(|byte| byte != b'"' && byte != b'\\' && byte >= 0x20)
        {
            out.push(b"\"")?;
            out.push(text.as_bytes())?;
            out.push(b"\"")
        } else {
            out.push(&quire_canonical::to_vec(&Text(text), Limits::new(out.bound)).ok()?)
        }
    }

    /// An identity's bytes, encoded on first use and reused after.
    fn identity<K: Clone + Eq + std::hash::Hash, E: Encode>(
        memo: &RwLock<HashMap<K, Arc<[u8]>>>,
        key: &K,
        leaf: &E,
        out: &mut Out,
    ) -> Option<()> {
        let known = memo
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(key)
            .cloned();
        let bytes = match known {
            Some(bytes) => bytes,
            None => {
                let bytes: Arc<[u8]> = quire_canonical::to_vec(leaf, Limits::new(u64::MAX))
                    .ok()?
                    .into();
                memo.write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(key.clone(), Arc::clone(&bytes));
                bytes
            }
        };
        out.push(&bytes)
    }

    /// The members written in the order held, then arranged by name.
    fn object<'m>(
        &self,
        members: impl Iterator<Item = (&'static str, EncodedMember<'m>)>,
        out: &mut Out,
    ) -> Option<()> {
        out.push(b"{")?;
        let start = out.bytes.len();
        let mut spans = Vec::with_capacity(members.size_hint().0);
        for (index, (name, value)) in members.enumerate() {
            if index > 0 {
                out.push(b",")?;
            }
            let from = out.bytes.len();
            Self::string(name, out)?;
            out.push(b":")?;
            match value {
                EncodedMember::Enc(value) => self.write(value, out)?,
                EncodedMember::Bytes(bytes) => out.push(bytes)?,
            }
            spans.push((name, from, out.bytes.len()));
        }
        if !spans
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if cmp_names(left.0, right.0).is_lt()))
        {
            spans.sort_by(|left, right| cmp_names(left.0, right.0));
            // Two members with one name have no encoding.
            if spans
                .windows(2)
                .any(|pair| matches!(pair, [left, right] if left.0 == right.0))
            {
                return None;
            }
            let written = out.bytes.split_off(start);
            for (index, (_, from, to)) in spans.iter().enumerate() {
                if index > 0 {
                    out.push(b",")?;
                }
                out.push(written.get(from - start..to - start)?)?;
            }
        }
        out.push(b"}")
    }

    /// FR-181's `map`: every key's bytes first, then the entries ascending
    /// by them.
    fn map(&self, entries: &[(Enc, Enc)], out: &mut Out) -> Option<()> {
        let mut keyed = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            // A key's bytes are part of the map's, so they too stay within
            // the bound.
            let mut bytes = Out::new(out.bound);
            self.write(key, &mut bytes)?;
            keyed.push((bytes.bytes, value));
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        out.push(b"{\"entries\":[")?;
        for (index, (key, value)) in keyed.iter().enumerate() {
            out.push(if index > 0 {
                b",{\"key\":"
            } else {
                b"{\"key\":"
            })?;
            out.push(key)?;
            out.push(b",\"value\":")?;
            self.write(value, out)?;
            out.push(b"}")?;
        }
        out.push(b"],\"type\":\"map\"}")
    }
}

/// The value of an [`Enc::Shared`] and its RFC 8785 bytes, `None` when it
/// has no encoding (writing it then refuses as the value does).
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct SharedEnc {
    value: Enc,
    bytes: Option<Vec<u8>>,
}

impl Enc {
    /// `value` as an [`Enc::Shared`], its bytes written once now.
    pub(crate) fn shared(value: Enc) -> Self {
        let bytes = Canonicalizer::default()
            .to_vec(&value, Limits::new(u64::MAX))
            .ok();
        Self::Shared(Arc::new(SharedEnc { value, bytes }))
    }

    pub(crate) fn text(value: impl Into<String>) -> Self {
        Self::Str(value.into())
    }

    /// `{"type": tag, ...members}`.
    pub(crate) fn tagged(tag: &str, mut members: Vec<(&'static str, Enc)>) -> Self {
        members.push(("type", Self::text(tag)));
        Self::Object(members)
    }

    /// This value's canonical RFC 8785 text.
    ///
    /// # Errors
    ///
    /// [`Error`] when the tree holds an unencodable value or exceeds `limits`.
    pub(crate) fn canonical_text(&self, limits: Limits) -> Result<String, Error> {
        let bytes = quire_canonical::to_vec(self, limits)?;
        String::from_utf8(bytes).map_err(|_| Error::Serialize("canonical text is UTF-8".into()))
    }
}

/// The state key an engine encodes: FR-181's `simulation-state` envelope.
pub struct ModelKey(pub(super) Enc);

impl Encode for ModelKey {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.0.encode_into(writer)
    }
}

/// The identity leaves shared by the retained tree and its canonical cache.
enum BorrowedIdentity<'a> {
    Node(&'a NodeKey),
    Effective(&'a EffectiveId),
    Declaration(&'a DeclarationKey),
}

impl Encode for BorrowedIdentity<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        match self {
            Self::Node(key) => qsl_foundation::node_wire::encode_node_key(key, writer),
            Self::Effective(id) => write_effective(writer, id),
            Self::Declaration(key) => match declaration_text(key, Limits::new(u64::MAX)) {
                Ok(text) => writer.string(&text),
                Err(_) => Err(unencodable(UNENCODABLE_DECLARATION)),
            },
        }
    }
}

/// The one declaration-record projection, used as an owned tree or borrowed
/// canonical events before its text is embedded as a string value.
fn declaration_members<'a>(key: DeclarationRecord<'a>) -> [(&'static str, &'a str); 2] {
    [("node", key.node), ("package", key.package)]
}

#[derive(Clone, Copy)]
struct DeclarationRecord<'a> {
    package: &'a str,
    node: &'a str,
}

impl<'a> DeclarationRecord<'a> {
    fn of_key(key: &'a DeclarationKey) -> Self {
        Self {
            package: &key.package,
            node: &key.node,
        }
    }

    fn of_node(key: &'a NodeKey) -> Option<Self> {
        let view = key.view();
        match view.path().last()? {
            PathSegmentView::Text(node) => Some(Self {
                package: view.package(),
                node,
            }),
            PathSegmentView::Node(_) => None,
        }
    }

    fn text(self, limits: Limits) -> Result<String, Error> {
        let bytes = quire_canonical::to_vec(&self, limits)?;
        String::from_utf8(bytes).map_err(|_| Error::Serialize("canonical text is UTF-8".into()))
    }
}

impl Encode for DeclarationRecord<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        for (name, value) in declaration_members(*self) {
            writer.name(name)?;
            writer.string(value)?;
        }
        writer.end_object()
    }
}

/// Write `id` as FR-201's recursive record, outermost owner last, one loop
/// iteration per level: the writer's own nesting is the only depth.
fn write_effective<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    id: &EffectiveId,
) -> Result<(), Error> {
    let chain = id.owner_chain();
    for original in chain.iter().rev() {
        writer.begin_object()?;
        writer.name("original")?;
        writer.begin_object()?;
        writer.name("node")?;
        writer.string(original.node())?;
        writer.name("package")?;
        writer.string(original.package())?;
        writer.end_object()?;
        writer.name("owner_effective_type")?;
    }
    writer.null()?;
    for _ in chain {
        writer.end_object()?;
    }
    Ok(())
}

/// A declaration's identity text: the RFC 8785 text of its
/// `$defs.DeclarationKey` preimage `{package, node}` (FR-120 "Identities"),
/// the same text `qsl_semantics::model::key::declaration_identity_text`
/// gives, which checked clauses lower operation labels to.
pub(crate) fn declaration_enc(key: &DeclarationKey) -> Enc {
    Enc::Object(
        declaration_members(DeclarationRecord::of_key(key))
            .into_iter()
            .map(|(name, value)| (name, Enc::text(value)))
            .collect(),
    )
}

/// The declaration identity text of `key`.
pub(super) fn declaration_text(key: &DeclarationKey, limits: Limits) -> Result<String, Error> {
    DeclarationRecord::of_key(key).text(limits)
}

/// FR-181's reference triple.
pub(crate) fn reference_enc(reference: &ObjectReference) -> Enc {
    projection::reference(reference)
}

/// FR-181's reference triple as RFC 8785 text: the encoding FR-196 names for
/// the `binding` member of a probability product key.
///
/// # Errors
///
/// [`Error`] when the reference is unencodable.
pub fn reference_canonical_text(reference: &ObjectReference) -> Result<String, Error> {
    reference_enc(reference).canonical_text(Limits::new(u64::MAX))
}

/// Emit the same FR-181 reference triple from borrowed identities.
/// All traversal and canonical storage is admitted by the supplied writer.
///
/// # Errors
///
/// The writer's byte, storage, allocation or encoding refusal.
pub fn encode_reference<S: Sink + ?Sized>(
    reference: &ObjectReference,
    writer: &mut Writer<'_, S>,
) -> Result<(), Error> {
    projection::write_reference(reference, writer)
}

/// The UnitId text: the RFC 8785 text of its structural identity record.
fn unit_enc(unit: &quire_exact::UnitId) -> Enc {
    identity_text(unit_identity_enc(unit), "a unit identity")
}

fn identity_text(value: Enc, what: &'static str) -> Enc {
    value
        .canonical_text(Limits::new(super::identity_limits().input_bytes))
        .map_or(Enc::Unencodable(what), Enc::text)
}

/// FR-088's UnitId identity record: `{declared: <NodeId>}` or the ordered
/// compound terms.
fn unit_identity_enc(unit: &quire_exact::UnitId) -> Enc {
    projection::unit_identity(unit)
}

/// A checked type node's producer declaration identity, `{package, node}`.
/// Type-environment keys are minted with the declaration's opaque producer
/// node as the final segment of their structural path.
#[cfg(test)]
fn declaration_name(key: &NodeKey) -> Enc {
    DeclarationRecord::of_node(key)
        .and_then(|record| {
            record
                .text(Limits::new(super::identity_limits().input_bytes))
                .ok()
        })
        .map_or(Enc::Unencodable("a declaration identity"), Enc::Str)
}

/// The first JSON value of `text` and the text after it.
fn take_json(text: &str) -> Option<(Json, &str)> {
    let mut values = serde_json::Deserializer::from_str(text).into_iter::<Json>();
    let value = values.next()?.ok()?;
    let rest = &text[values.byte_offset()..];
    Some((value, rest))
}

/// The effective identity [`write_effective`] wrote at the start of `text`,
/// and the text after it. Reads one record level per loop iteration and
/// refuses any other shape.
#[ix_trace_rs::string_edge]
fn take_effective(text: &str) -> Option<(EffectiveId, &str)> {
    let mut rest = text;
    let mut originals = Vec::new();
    loop {
        rest = rest.strip_prefix(r#"{"original":"#)?;
        let (original, after) = take_json(rest)?;
        let members = original.as_object()?;
        if members.len() != 2 {
            return None;
        }
        originals.push(OriginalDeclarationKey::new(
            members.get("package")?.as_str()?,
            members.get("node")?.as_str()?,
        ));
        rest = after.strip_prefix(r#","owner_effective_type":"#)?;
        if let Some(after_null) = rest.strip_prefix("null") {
            rest = after_null;
            break;
        }
    }
    for _ in &originals {
        rest = rest.strip_prefix('}')?;
    }
    let mut id = None;
    for original in originals.into_iter().rev() {
        id = Some(EffectiveId::new(id, original));
    }
    Some((id?, rest))
}

/// The reference [`reference_enc`] wrote at the start of `text`, and the
/// text after it: its canonical text names the members in sorted order, so
/// the text is matched member by member.
#[ix_trace_rs::string_edge]
fn take_reference(text: &str) -> Option<(ObjectReference, &str)> {
    let rest = text.strip_prefix(r#"{"identity":"#)?;
    let (identity, rest) = take_json(rest)?;
    let rest = rest.strip_prefix(r#","object_type":"#)?;
    let (object_type, rest) = take_effective(rest)?;
    let rest = rest.strip_prefix(r#","type":"reference","universe":"#)?;
    let (universe, rest) = take_json(rest)?;
    let rest = rest.strip_prefix('}')?;
    Some((
        ObjectReference::new(
            UniverseId::new(node_key_from_json(&universe).ok()?),
            object_type,
            quire_exact::ObjectId::new(identity.as_str()?).ok()?,
        ),
        rest,
    ))
}

/// The reference [`reference_enc`] wrote as `text`.
/// `#[string_edge]`: reads the canonical text this module's own encoder wrote.
#[ix_trace_rs::string_edge]
pub(crate) fn parse_reference(text: &str) -> Option<ObjectReference> {
    match take_reference(text)? {
        (reference, "") => Some(reference),
        _ => None,
    }
}

/// A timed transition's `clock_place` argument split at the object it
/// names: the text before the object's reference, the reference, and the
/// text after it. `None` for any other text.
/// `#[string_edge]`: reads the canonical text this module's own encoder wrote.
#[ix_trace_rs::string_edge]
pub(super) fn split_clock_place(text: &str) -> Option<(&str, ObjectReference, &str)> {
    if !text.ends_with(r#","type":"clock_place"}"#) {
        return None;
    }
    let marker = r#","object":"#;
    let start = text.find(marker)? + marker.len();
    let (reference, after) = take_reference(&text[start..])?;
    Some((&text[..start], reference, after))
}

/// The reference a parameter's canonical text names: the text of a reference,
/// or of an option holding one. `Some(None)` is an absent option.
/// `#[string_edge]`: reads the canonical text this module's own encoder wrote.
#[ix_trace_rs::string_edge]
pub(super) fn parse_optional_reference(text: &str) -> Option<Option<ObjectReference>> {
    if let Some(inner) = text
        .strip_prefix(r#"{"type":"option","value":"#)
        .and_then(|rest| rest.strip_suffix('}'))
    {
        return if inner == "null" {
            Some(None)
        } else {
            parse_reference(inner).map(Some)
        };
    }
    parse_reference(text).map(Some)
}

/// The declarations a value's typed canonical form reads: the package's
/// record, tuple and union types and, where the caller has them, its
/// variant (enum) types.
#[derive(Clone, Copy)]
pub(crate) struct Typing<'t> {
    /// `None` only where a value is compared, not encoded: see
    /// [`values_equal`].
    types: Option<&'t TypeEnvironment>,
    variants: &'t [VariantTypeDeclaration],
    /// The element-count members of the pure-clock collections, which a
    /// record writes after its declared fields.
    clock_counts: &'t [super::ClockCount],
    /// The presence-bit members of the `Option<Clock>` fields, which a
    /// record writes as the field's option shell.
    clock_bits: &'t [(DeclarationKey, StateField)],
    /// The run's map-key names, when the caller keeps them.
    names: Option<&'t KeyNames>,
}

/// The names a run's state keys write as map keys, each with the RFC 8785
/// bytes its map sorts by: every object reference and population name, built
/// once per run rather than once per key. A reference and a population name
/// are functions of the run's model alone.
#[derive(Default)]
pub(crate) struct KeyNames {
    references: RwLock<BTreeMap<DeclarationKey, BTreeMap<String, KeyName>>>,
    populations: Mutex<BTreeMap<DeclarationKey, KeyName>>,
}

/// A map key's RFC 8785 bytes and form.
pub(crate) type KeyName = (Vec<u8>, Enc);

/// A map key with the bytes [`map_enc`] orders it by.
pub(crate) fn keyed_name(key: Enc) -> KeyName {
    (
        Canonicalizer::default()
            .to_vec(&key, Limits::new(u64::MAX))
            .unwrap_or_default(),
        key,
    )
}

impl KeyNames {
    /// The reference of the object `key` of type `type_identity` as a map
    /// key; `None` when the reference does not resolve.
    fn reference(
        &self,
        model: &StateModel,
        type_identity: &DeclarationKey,
        key: &str,
    ) -> Option<KeyName> {
        if let Some(found) = self
            .references
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(type_identity)
            .and_then(|objects| objects.get(key))
        {
            return Some(found.clone());
        }
        let reference = model.object_reference(type_identity, key).ok()?;
        let named = keyed_name(reference_enc(&reference));
        self.references
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(type_identity.clone())
            .or_default()
            .insert(key.to_owned(), named.clone());
        Some(named)
    }

    /// The population `population`'s name as a map key.
    fn population(&self, population: &DeclarationKey) -> KeyName {
        self.populations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(population.clone())
            .or_insert_with(|| keyed_name(population_name(population)))
            .clone()
    }
}

/// The map key a population's objects are written under.
fn population_name(population: &DeclarationKey) -> Enc {
    Enc::tagged("text", vec![("value", declaration_enc_text(population))])
}

impl<'t> Typing<'t> {
    /// No declarations: a record, tuple or enum value has no form.
    fn none() -> Self {
        Self {
            types: None,
            variants: &[],
            clock_counts: &[],
            clock_bits: &[],
            names: None,
        }
    }

    /// The record, tuple and union types of `types` and the variant types
    /// `variants`.
    pub(crate) fn with_variants(
        types: &'t TypeEnvironment,
        variants: &'t [VariantTypeDeclaration],
    ) -> Self {
        Self {
            types: Some(types),
            variants,
            clock_counts: &[],
            clock_bits: &[],
            names: None,
        }
    }

    /// These declarations with the element-count members `clock_counts`
    /// and the presence-bit members `clock_bits`.
    pub(crate) fn with_clock_members(
        self,
        clock_counts: &'t [super::ClockCount],
        clock_bits: &'t [(DeclarationKey, StateField)],
    ) -> Self {
        Self {
            clock_counts,
            clock_bits,
            ..self
        }
    }

    /// These declarations, writing the map-key names `names` keeps.
    pub(crate) fn with_names(self, names: &'t KeyNames) -> Self {
        Self {
            names: Some(names),
            ..self
        }
    }

    /// The present value of the record field `name` of the record value
    /// `value`; `None` when `value` is no record, the field is absent or
    /// its declaration is not in these types.
    pub(crate) fn component<'v>(&self, value: &'v Value, name: &str) -> Option<&'v Value> {
        let Value::Composite(composite) = value else {
            return None;
        };
        let Some(CompositeShape::Record(fields)) = self
            .types?
            .composite(composite.declaration().clone())
            .map(CompositeDeclaration::shape)
        else {
            return None;
        };
        let position = fields.iter().position(|field| field.name() == name)?;
        match composite.slots().get(position)? {
            FieldValue::Present(value) => Some(value),
            FieldValue::Absent | FieldValue::Null => None,
        }
    }
}

/// FR-181's typed canonical form of `value`, its records, tuples, unions and
/// enums read from `typing`.
pub(crate) fn value_enc(value: &Value, typing: Typing<'_>) -> Enc {
    value_enc_with(value, typing, &|_| None)
}

/// FR-120's typed canonical form of `value`, where a record value is
/// `{"type":"record","name":…,"fields":[{name, value}…]}` with `name` the
/// declaration identity text, one entry per declared field in
/// declaration order and an absent field `null`; a tuple is
/// `{"type":"tuple","name":…,"components":[…]}`; a union is
/// `{"type":"union","name":…,"member":…,"components":[…]}`; and an enum is
/// `{"type":"enum","name":…,"member":…}` with `name` the variant type's
/// declaration identity text and `member` the declared case.
pub(crate) fn typed_value_enc(
    value: &Value,
    types: &TypeEnvironment,
    variants: &[VariantTypeDeclaration],
) -> Enc {
    value_enc(value, Typing::with_variants(types, variants))
}

/// A reference's replacement in a key-free encoding (FR-152): `Some` for a
/// reference the encoding names by a placeholder, `None` to encode it.
pub(super) type Placeholder<'p> = dyn Fn(&ObjectReference) -> Option<Enc> + 'p;

/// FR-181's typed canonical form of `value`, as [`value_enc`], with each
/// reference `placeholder` replaces written as its replacement, recursively
/// through options, collections, records, tuples and unions.
pub(super) fn value_enc_with(
    value: &Value,
    typing: Typing<'_>,
    placeholder: &Placeholder<'_>,
) -> Enc {
    value_enc_with_decimal_mode(value, typing, placeholder, false)
}

fn value_enc_with_decimal_mode(
    value: &Value,
    typing: Typing<'_>,
    placeholder: &Placeholder<'_>,
    normalize_decimal: bool,
) -> Enc {
    projection::value(value, typing, placeholder, normalize_decimal)
}

/// Whether two values have the same typed canonical form. A value with no
/// form equals nothing. Two records, tuples or unions are compared
/// component by component, which is what their forms compare: one
/// declaration, one member, and the same component at each slot. Two enum
/// members are compared by variant and rank, which is what an enum's form
/// names; options and collections, which may hold any of these, element by
/// element.
pub(super) fn values_equal(left: &Value, right: &Value) -> bool {
    let slot_equal = |left: &FieldValue, right: &FieldValue| match (left, right) {
        (FieldValue::Present(left), FieldValue::Present(right)) => values_equal(left, right),
        (FieldValue::Absent | FieldValue::Null, FieldValue::Absent | FieldValue::Null) => true,
        _ => false,
    };
    match (left, right) {
        (Value::Composite(left), Value::Composite(right)) => {
            left.declaration() == right.declaration()
                && left.slots().len() == right.slots().len()
                && left
                    .slots()
                    .iter()
                    .zip(right.slots())
                    .all(|(left, right)| slot_equal(left, right))
        }
        (Value::Union(left), Value::Union(right)) => {
            left.declaration() == right.declaration()
                && left.member().identifier() == right.member().identifier()
                && left.payload().len() == right.payload().len()
                && left
                    .payload()
                    .iter()
                    .zip(right.payload())
                    .all(|(left, right)| values_equal(left, right))
        }
        (Value::Enum(left), Value::Enum(right)) => left == right,
        (Value::Option(left), Value::Option(right)) => {
            left.payload_type() == right.payload_type()
                && match (left.payload(), right.payload()) {
                    (Some(left), Some(right)) => values_equal(left, right),
                    (None, None) => true,
                    _ => false,
                }
        }
        (Value::Collection(left), Value::Collection(right)) => {
            left.collection_type() == right.collection_type()
                && left.elements().len() == right.elements().len()
                && left
                    .elements()
                    .iter()
                    .zip(right.elements())
                    .all(|(left, right)| values_equal(left, right))
        }
        (
            Value::Composite(_)
            | Value::Union(_)
            | Value::Enum(_)
            | Value::Option(_)
            | Value::Collection(_),
            _,
        )
        | (
            _,
            Value::Composite(_)
            | Value::Union(_)
            | Value::Enum(_)
            | Value::Option(_)
            | Value::Collection(_),
        ) => false,
        _ => {
            let (left, right) = (untyped_value_enc(left), untyped_value_enc(right));
            !matches!(
                (&left, &right),
                (Enc::Unencodable(_), _) | (_, Enc::Unencodable(_))
            ) && left == right
        }
    }
}

/// A value's typed canonical form where no package declarations are at
/// hand: every family but records, tuples and enums, which have none here.
pub(crate) fn untyped_value_enc(value: &Value) -> Enc {
    value_enc(value, Typing::none())
}

/// FR-181's `map`: `{"type":"map","entries":[{"key":..,"value":..}, ...]}`
/// with the entries ascending by the key's RFC 8785 bytes.
pub(crate) fn map_enc(entries: Vec<(Enc, Enc)>) -> Enc {
    Enc::Map(entries)
}

/// A map entry with the bytes [`map_enc`] orders it by.
fn keyed_entry(key: Enc, value: Enc) -> (Vec<u8>, Enc, Enc) {
    let (bytes, key) = keyed_name(key);
    (bytes, key, value)
}

/// [`map_enc`] of entries that carry their key's RFC 8785 bytes, so a caller
/// that writes one key in many maps encodes it once.
pub(crate) fn sorted_map_enc(mut keyed: Vec<(Vec<u8>, Enc, Enc)>) -> Enc {
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    Enc::tagged(
        "map",
        vec![(
            "entries",
            Enc::Array(
                keyed
                    .into_iter()
                    .map(|(_, key, value)| Enc::Object(vec![("key", key), ("value", value)]))
                    .collect(),
            ),
        )],
    )
}

/// The `semantic` member of a model state's key (FR-120 "State, state key
/// and anchor"): each population's declaration identity text, mapped to its
/// objects' reference triples, each mapped to a `record` of its declared
/// fields in declaration order. A state that lists relationship extents adds
/// one more entry, keyed by the text `relationships` (no population key is
/// text of that form), mapping each listed relationship's identity text to
/// its completeness and its instance pairs in ascending canonical order, so
/// two states differing only in their extents have different keys.
pub(crate) fn semantic_enc(model: &StateModel, state: &ModelState, typing: Typing<'_>) -> Enc {
    let fresh = KeyNames::default();
    let names = typing.names.unwrap_or(&fresh);
    let mut populations = Vec::new();
    for (population, objects) in &state.populations {
        let mut entries = Vec::new();
        for (key, object) in objects {
            let Some((bytes, reference)) = names.reference(model, &object.type_identity, key)
            else {
                return Enc::Unencodable("an object whose reference does not resolve");
            };
            let record = object_enc(model, object, typing, &|_| None);
            entries.push((bytes, reference, record));
        }
        let (bytes, name) = names.population(population);
        populations.push((bytes, name, sorted_map_enc(entries)));
    }
    if !state.extents.is_empty() {
        populations.push(keyed_entry(
            Enc::tagged("text", vec![("value", Enc::text("relationships"))]),
            extents_enc(state),
        ));
    }
    sorted_map_enc(populations)
}

/// The listed relationship extents of `state`: relationship identity text to
/// `{complete, instances}`, the instances as `{source, target}` pairs
/// ascending by canonical bytes, repeats kept (admission keeps a repeated
/// pair when neither end is unique).
fn extents_enc(state: &ModelState) -> Enc {
    let limits = Limits::new(u64::MAX);
    let entries = state
        .extents
        .iter()
        .map(|extent| {
            let mut pairs: Vec<(Vec<u8>, Enc)> = extent
                .instances
                .iter()
                .map(|(source, target)| {
                    let pair = Enc::Object(vec![
                        ("source", reference_enc(source)),
                        ("target", reference_enc(target)),
                    ]);
                    (
                        quire_canonical::to_vec(&pair, limits).unwrap_or_default(),
                        pair,
                    )
                })
                .collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            (
                Enc::tagged(
                    "text",
                    vec![("value", declaration_enc_text(&extent.relationship))],
                ),
                Enc::Object(vec![
                    ("complete", Enc::Bool(extent.complete)),
                    (
                        "instances",
                        Enc::Array(pairs.into_iter().map(|(_, pair)| pair).collect()),
                    ),
                ]),
            )
        })
        .collect();
    map_enc(entries)
}

/// One object's `record` in a state key (FR-120): its type name and its
/// declared fields in declaration order, each reference `placeholder`
/// replaces written as its replacement.
pub(super) fn object_enc(
    model: &StateModel,
    object: &qsl_semantics::model::state::StateObject,
    typing: Typing<'_>,
    placeholder: &Placeholder<'_>,
) -> Enc {
    let Ok(Some(fields)) = model.fields(&object.type_identity) else {
        return Enc::Unencodable("an object of an undeclared type");
    };
    let mut fields: Vec<Enc> = fields
        .iter()
        .map(|field| {
            // QSpec FR-181: a field declared `Option<T>` holds an option
            // value; an absent presence-optional field is a plain `null`
            // entry; a record, tuple, union or enum value takes its declared
            // form from `typing`.
            let value = object.fields.get(&field.key).map_or(Enc::Null, |value| {
                value_enc_with(value, typing, placeholder)
            });
            Enc::Object(vec![
                ("name", Enc::text(field.name.clone())),
                ("value", value),
            ])
        })
        .collect();
    // A collection of bare clocks holds one `Clock` tuple per element, the
    // shell FR-230 gives a present clock place, so two states that differ
    // in its element count have different keys.
    for (type_key, field, _, _) in typing.clock_counts {
        if *type_key != object.type_identity {
            continue;
        }
        let elements = match object.fields.get(&field.key) {
            Some(Value::Integer(count)) => (0..count.to_u64().unwrap_or(0))
                .map(|_| clock_shell_enc())
                .collect(),
            // FR-230-AC-42: a `Sequence<Option<Clock>>` writes each element's
            // option shell, `null` when absent and a `Clock` shell when
            // present.
            Some(Value::Collection(bits)) => bits
                .elements()
                .iter()
                .map(|bit| {
                    let shell = if matches!(bit, Value::Boolean(true)) {
                        clock_shell_enc()
                    } else {
                        Enc::Null
                    };
                    Enc::tagged("option", vec![("value", shell)])
                })
                .collect(),
            _ => continue,
        };
        fields.push(Enc::Object(vec![
            ("name", Enc::text(field.name.clone())),
            (
                "value",
                Enc::tagged("sequence", vec![("elements", Enc::Array(elements))]),
            ),
        ]));
    }
    // FR-230 "Presence bit": an `Option<Clock>` field writes its option
    // shell, `null` when absent and a `Clock` shell when present.
    for (type_key, field) in typing.clock_bits {
        if *type_key != object.type_identity {
            continue;
        }
        let present = matches!(object.fields.get(&field.key), Some(Value::Boolean(true)));
        let shell = if present {
            clock_shell_enc()
        } else {
            Enc::Null
        };
        fields.push(Enc::Object(vec![
            ("name", Enc::text(field.name.clone())),
            ("value", Enc::tagged("option", vec![("value", shell)])),
        ]));
    }
    Enc::tagged(
        "record",
        vec![
            ("name", declaration_enc_text(&object.type_identity)),
            ("fields", Enc::Array(fields)),
        ],
    )
}

/// A present clock place's shell, FR-230's
/// `{"type":"tuple","name":"Clock","components":[]}`.
pub(crate) fn clock_shell_enc() -> Enc {
    Enc::tagged(
        "tuple",
        vec![
            ("name", Enc::text("Clock")),
            ("components", Enc::Array(Vec::new())),
        ],
    )
}

/// A declaration's identity text as a string value.
fn declaration_enc_text(key: &DeclarationKey) -> Enc {
    Enc::Declaration(key.clone())
}

/// The empty `map`.
fn empty_map() -> Enc {
    map_enc(Vec::new())
}

/// FR-181's state key for a model state (FR-120 "State, state key and
/// anchor"): the five members a model state does not use are empty maps.
pub(super) fn model_key(model: &StateModel, state: &ModelState, typing: Typing<'_>) -> ModelKey {
    ModelKey(Enc::tagged(
        "simulation-state",
        vec![
            ("semantic", semantic_enc(model, state, typing)),
            ("control", empty_map()),
            ("queues", empty_map()),
            ("roles", empty_map()),
            ("observations", empty_map()),
            ("bounds", empty_map()),
            ("memory", empty_map()),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{
        from_admitted, CollectionKind, CollectionType, Integer, Quantity, Rational, Timestamp,
        UnitId, Uuid, ValueType,
    };

    use super::*;

    /// FR-181's integer form is an exact decimal string.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn an_integer_encodes_as_a_tagged_decimal_string() {
        let text = untyped_value_enc(&Value::Integer(Integer::from(-12_i64)))
            .canonical_text(Limits::new(1024))
            .expect("an integer encodes");
        assert_eq!(text, r#"{"type":"integer","value":"-12"}"#);
    }

    /// FR-181-AC-19: UUID and Timestamp use their distinct typed forms.
    #[trace("TC-1195", "FR-120-AC-16")]
    #[test]
    fn uuid_and_timestamp_encode_in_their_typed_forms() {
        let uuid = Uuid::from_canonical_text("00112233-4455-6677-8899-aabbccddeeff")
            .expect("canonical UUID");
        let timestamp = Timestamp::from_canonical_text("-1").expect("canonical timestamp");
        let typing = Typing::none();

        assert_eq!(
            value_enc(&Value::Uuid(uuid), typing)
                .canonical_text(Limits::new(1024))
                .expect("a UUID encodes"),
            r#"{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeeff"}"#
        );
        assert_eq!(
            value_enc(&Value::Timestamp(timestamp), typing)
                .canonical_text(Limits::new(1024))
                .expect("a Timestamp encodes"),
            r#"{"nanoseconds":"-1","type":"timestamp"}"#
        );
    }

    /// FR-181-AC-19/20: UUID and Timestamp set members use their typed forms
    /// and sets order those forms by their encoded JCS bytes.
    #[trace("TC-1195", "FR-120-AC-17")]
    #[test]
    fn uuid_and_timestamp_sets_encode_in_canonical_byte_order() {
        let uuid = |value| Value::Uuid(Uuid::from_canonical_text(value).expect("canonical UUID"));
        let timestamp = |value| {
            Value::Timestamp(Timestamp::from_canonical_text(value).expect("canonical timestamp"))
        };
        let ids = from_admitted(
            CollectionType::new(CollectionKind::Set, ValueType::Uuid, None),
            vec![
                uuid("00112233-4455-6677-8899-aabbccddeeff"),
                uuid("00112233-4455-6677-8899-aabbccddeefe"),
            ],
        );
        let times = from_admitted(
            CollectionType::new(CollectionKind::Set, ValueType::Timestamp, None),
            vec![timestamp("2"), timestamp("10")],
        );
        let typing = Typing::none();

        assert_eq!(
            value_enc(&ids, typing)
                .canonical_text(Limits::new(1024))
                .expect("the UUID set encodes"),
            r#"{"elements":[{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeefe"},{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeeff"}],"type":"set"}"#
        );
        assert_eq!(
            value_enc(&times, typing)
                .canonical_text(Limits::new(1024))
                .expect("the Timestamp set encodes"),
            r#"{"elements":[{"nanoseconds":"10","type":"timestamp"},{"nanoseconds":"2","type":"timestamp"}],"type":"set"}"#
        );
    }

    /// FR-120: a unit identity is written as its UnitId text.
    #[test]
    fn a_unit_encodes_as_its_identity_text() {
        let unit = UnitId::compound(vec![
            (crate::test_ids::node_key([1]), Integer::from(1_i64)),
            (crate::test_ids::node_key([2]), Integer::from(-2_i64)),
        ]);
        let text = Enc::Object(vec![("unit", unit_enc(&unit))])
            .canonical_text(Limits::new(1024))
            .expect("a unit encodes");
        let identity_text = unit_identity_enc(&unit)
            .canonical_text(Limits::new(1024))
            .expect("the UnitId identity has canonical text");
        assert_eq!(
            text,
            format!(
                r#"{{"unit":{}}}"#,
                serde_json::to_string(&identity_text).expect("unit text serializes")
            )
        );
    }

    fn declaration_json(seed: u8) -> String {
        serde_json::to_string(&format!(r#"{{"node":"[{seed}]","package":"fixture"}}"#))
            .expect("DeclarationKey identity text is a JSON string")
    }

    #[test]
    fn a_type_node_name_is_the_declaration_key_identity_text() {
        assert_eq!(
            declaration_name(&crate::test_ids::node_key([0x11]))
                .canonical_text(Limits::new(1024))
                .expect("a declaration name encodes"),
            declaration_json(0x11)
        );
    }

    /// An effective identity owned through `depth` declarations.
    fn owned_chain(depth: usize) -> EffectiveId {
        let mut id = None;
        for level in 0..=depth {
            id = Some(EffectiveId::new(
                id,
                OriginalDeclarationKey::new("fixture", format!("Level{level}")),
            ));
        }
        id.expect("a chain has a root")
    }

    /// FR-201: a deep owner chain writes and reads back one loop iteration per
    /// level, and the reference it travels in round-trips.
    #[test]
    fn a_deep_owner_chain_round_trips_through_a_reference() {
        let reference = ObjectReference::new(
            crate::test_ids::universe_id([1]),
            owned_chain(5000),
            quire_exact::ObjectId::new("object").expect("an object identity"),
        );
        let text = reference_enc(&reference)
            .canonical_text(Limits::new(u64::MAX))
            .expect("the reference encodes");
        assert_eq!(parse_reference(&text), Some(reference));
    }

    #[trace("FR-196-AC-6")]
    #[test]
    fn borrowed_reference_encoding_preserves_the_owner_schema() {
        let reference = ObjectReference::new(
            crate::test_ids::universe_id([1]),
            owned_chain(2),
            quire_exact::ObjectId::new("object").unwrap(),
        );
        struct Borrowed<'a>(&'a ObjectReference);
        impl Encode for Borrowed<'_> {
            fn encode_into<S: Sink + ?Sized>(
                &self,
                writer: &mut Writer<'_, S>,
            ) -> Result<(), Error> {
                encode_reference(self.0, writer)
            }
        }
        let expected = reference_canonical_text(&reference).unwrap();
        assert_eq!(
            quire_canonical::to_vec(&Borrowed(&reference), Limits::new(expected.len() as u64))
                .unwrap(),
            expected.as_bytes()
        );
        assert!(matches!(
            quire_canonical::to_vec(
                &Borrowed(&reference),
                Limits::new(expected.len() as u64 - 1)
            ),
            Err(Error::Limit(_))
        ));
    }

    /// FR-201: a reference text whose effective identity is malformed has no
    /// reading.
    #[test]
    fn a_malformed_effective_identity_is_refused() {
        let reference = ObjectReference::new(
            crate::test_ids::universe_id([1]),
            owned_chain(2),
            quire_exact::ObjectId::new("object").expect("an object identity"),
        );
        let text = reference_enc(&reference)
            .canonical_text(Limits::new(1 << 20))
            .expect("the reference encodes");
        // One closing brace short, an extra member, and a foreign shape.
        let short = text.replacen("null}}}", "null}}", 1);
        assert!(parse_reference(&short).is_none());
        let extra = text.replacen(r#"{"node":"#, r#"{"extra":1,"node":"#, 1);
        assert!(parse_reference(&extra).is_none());
        assert!(parse_reference(r#"{"identity":"o","object_type":7}"#).is_none());
    }

    /// The declarations of FR-120-AC-16's `Tag { sku: Integer, note?:
    /// Boolean }` (node `11…`), `Pair(Boolean, Integer)` (`22…`) and `Shape`
    /// with the nullary member `none` and the member `tagged(Tag)` (`33…`),
    /// and the values `Tag { sku }`, `Pair(true, -3)` and `none`.
    fn declared() -> (TypeEnvironment, impl Fn(i64) -> Value, Value, Value) {
        use quire_exact::{from_admitted_slots, Presence, UnionMember, UnionValue};
        use quire_semantic_value::declaration::{
            CompositeDeclaration, FieldDeclaration, UnionMemberDeclaration,
        };
        let (tag, pair, shape) = (
            crate::test_ids::node_key([0x11]),
            crate::test_ids::node_key([0x22]),
            crate::test_ids::node_key([0x33]),
        );
        let none = UnionMember::from_admitted(shape.clone(), "none").expect("a member name");
        let types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    tag.clone(),
                    "Tag",
                    CompositeShape::Record(vec![
                        FieldDeclaration::new("sku", ValueType::Integer, Presence::Required),
                        FieldDeclaration::new("note", ValueType::Boolean, Presence::Optional),
                    ]),
                ),
                CompositeDeclaration::new(
                    pair.clone(),
                    "Pair",
                    CompositeShape::Tuple(vec![ValueType::Boolean, ValueType::Integer]),
                ),
                CompositeDeclaration::new(
                    shape.clone(),
                    "Shape",
                    CompositeShape::Union(vec![UnionMemberDeclaration::from_verified(
                        none.clone(),
                        vec![],
                    )]),
                ),
            ],
            [],
        )
        .expect("the declarations admit");
        let tag_value = move |sku: i64| {
            from_admitted_slots(
                tag.clone(),
                vec![
                    FieldValue::Present(Value::Integer(Integer::from(sku))),
                    FieldValue::Absent,
                ]
                .into_boxed_slice(),
            )
        };
        let pair_value = from_admitted_slots(
            pair,
            vec![
                FieldValue::Present(Value::Boolean(true)),
                FieldValue::Present(Value::Integer(Integer::from(-3_i64))),
            ]
            .into_boxed_slice(),
        );
        let none_value = UnionValue::from_admitted(none, vec![]);
        (types, tag_value, pair_value, none_value)
    }

    /// FR-120-AC-16 (TC-1195): declaration names use JCS DeclarationKey text.
    #[trace("TC-1195", "FR-120-AC-16")]
    #[test]
    fn a_record_tuple_and_union_encode_in_their_declared_forms() {
        let (types, tag, pair, none) = declared();
        let text = |value: &Value| {
            value_enc(value, Typing::with_variants(&types, &[]))
                .canonical_text(Limits::new(4096))
                .expect("the value encodes")
        };
        let (tag_name, pair_name, shape_name) = (
            declaration_json(0x11),
            declaration_json(0x22),
            declaration_json(0x33),
        );
        assert_eq!(
            text(&tag(7)),
            format!(
                r#"{{"fields":[{{"name":"sku","value":{{"type":"integer","value":"7"}}}},{{"name":"note","value":null}}],"name":{tag_name},"type":"record"}}"#
            )
        );
        assert_eq!(
            text(&pair),
            format!(
                r#"{{"components":[{{"type":"boolean","value":true}},{{"type":"integer","value":"-3"}}],"name":{pair_name},"type":"tuple"}}"#
            )
        );
        assert_eq!(
            text(&none),
            format!(r#"{{"components":[],"member":"none","name":{shape_name},"type":"union"}}"#)
        );
        // Without the declarations a record has no form; a union keeps its.
        assert!(matches!(untyped_value_enc(&tag(7)), Enc::Unencodable(_)));
    }

    /// FR-120-AC-16 (TC-1195): the declared forms nest through an option and
    /// a set, whose records are ordered by their encoded bytes; equality
    /// compares records component by component.
    #[trace("TC-1195")]
    #[test]
    fn declared_forms_nest_through_options_and_sets() {
        let (types, tag, _, _) = declared();
        let typing = Typing::with_variants(&types, &[]);
        let record = |sku: &str| {
            format!(
                r#"{{"fields":[{{"name":"sku","value":{{"type":"integer","value":"{sku}"}}}},{{"name":"note","value":null}}],"name":{},"type":"record"}}"#,
                declaration_json(0x11)
            )
        };
        let tag_type = ValueType::Composite(crate::test_ids::node_key([0x11]));
        let option = quire_exact::OptionValue::from_admitted(tag_type.clone(), Some(tag(7)));
        assert_eq!(
            value_enc(&option, typing)
                .canonical_text(Limits::new(4096))
                .expect("the option encodes"),
            format!(r#"{{"type":"option","value":{}}}"#, record("7"))
        );
        let set = from_admitted(
            CollectionType::new(CollectionKind::Set, tag_type.clone(), None),
            vec![tag(7), tag(10)],
        );
        assert_eq!(
            value_enc(&set, typing)
                .canonical_text(Limits::new(4096))
                .expect("the set encodes"),
            format!(
                r#"{{"elements":[{},{}],"type":"set"}}"#,
                record("10"),
                record("7")
            )
        );
        assert!(values_equal(&tag(7), &tag(7)));
        assert!(!values_equal(&tag(7), &tag(10)));
        // Options and collections of records, and enum members, compare
        // element by element and by variant and rank (SR-1646005).
        let other = quire_exact::OptionValue::from_admitted(tag_type.clone(), Some(tag(10)));
        assert!(values_equal(&option, &option.clone()));
        assert!(!values_equal(&option, &other));
        assert!(values_equal(&set, &set.clone()));
        let member = |rank| {
            Value::Enum(quire_exact::EnumMember::new(
                crate::test_ids::variant_id([0xfd]),
                rank,
            ))
        };
        assert!(values_equal(&member(0), &member(0)));
        assert!(!values_equal(&member(0), &member(1)));
    }

    /// FR-120-AC-16 / FR-189-AC-15: a quantity keeps its exact magnitude and
    /// declared unit identity in the rational wire form.
    #[trace("TC-1195", "FR-120-AC-16")]
    #[test]
    fn a_quantity_encodes_its_rational_magnitude_and_unit() {
        let unit = UnitId::declared(crate::test_ids::node_key([0xab]));
        let quantity = Value::Quantity(Quantity::new(
            Rational::new(Integer::from(3_i64), Integer::from(2_i64)).expect("nonzero"),
            unit.clone(),
        ));
        let text = value_enc(&quantity, Typing::none())
            .canonical_text(Limits::new(1024))
            .expect("a quantity encodes");
        assert_eq!(
            text,
            format!(
                r#"{{"denominator":"2","numerator":"3","type":"rational","unit":{}}}"#,
                serde_json::to_string(
                    &unit_identity_enc(&unit)
                        .canonical_text(Limits::new(1024))
                        .expect("unit identity text")
                )
                .expect("unit text serializes")
            )
        );
    }

    /// FR-120-AC-16/17 (TC-1195): a retained decimal keeps its representation,
    /// while set and bag elements use the normalized representation.
    #[trace("TC-1195", "FR-120-AC-16", "FR-120-AC-17")]
    #[test]
    fn decimal_values_keep_or_normalize_representation_by_context() {
        use quire_exact::{Decimal, Integer};

        let decimal = Value::Decimal(Decimal::new(Integer::from(150_i64), 2));
        let typing = Typing::none();
        let retained = value_enc(&decimal, typing)
            .canonical_text(Limits::new(1024))
            .expect("a decimal encodes");
        assert_eq!(
            retained,
            r#"{"coefficient":"150","scale":"2","type":"decimal"}"#
        );

        let decimal_type = ValueType::Decimal(
            quire_exact::DecimalType::new(
                Integer::from(0_i64),
                Integer::from(1000_i64),
                0,
                2,
                quire_exact::RoundingMode::Exact,
            )
            .expect("valid decimal type"),
        );
        for kind in [CollectionKind::Set, CollectionKind::Bag] {
            let collection = from_admitted(
                CollectionType::new(kind, decimal_type.clone(), None),
                vec![decimal.clone()],
            );
            let text = value_enc(&collection, typing)
                .canonical_text(Limits::new(1024))
                .expect("a decimal collection encodes");
            let type_name = match kind {
                CollectionKind::Set => "set",
                CollectionKind::Bag => "bag",
                _ => unreachable!(),
            };
            assert_eq!(
                text,
                format!(
                    r#"{{"elements":[{{"coefficient":"15","scale":"1","type":"decimal"}}],"type":"{type_name}"}}"#
                )
            );
        }

        let option_type = ValueType::option(decimal_type.clone());
        let option = quire_exact::OptionValue::from_admitted(decimal_type, Some(decimal));
        let nested = from_admitted(
            CollectionType::new(CollectionKind::Set, option_type, None),
            vec![option],
        );
        assert_eq!(
            value_enc(&nested, typing)
                .canonical_text(Limits::new(1024))
                .expect("a nested decimal set encodes"),
            r#"{"elements":[{"type":"option","value":{"coefficient":"15","scale":"1","type":"decimal"}}],"type":"set"}"#
        );
    }

    /// A value with no form refuses to write rather than writing a guess.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn a_value_without_a_form_refuses_to_encode() {
        let text = Enc::Unencodable("a union value").canonical_text(Limits::new(1024));
        assert!(matches!(text, Err(Error::Serialize(_))));
    }

    /// FR-120-AC-18 (TC-1195): a Population handle has no typed canonical form.
    #[trace("TC-1195", "FR-120-AC-18")]
    #[test]
    fn a_population_value_keeps_its_typed_refusal() {
        let value = Value::Population(quire_exact::PopulationId::new(crate::test_ids::node_key([
            1,
        ])));
        assert!(matches!(
            value_enc(&value, Typing::none()).canonical_text(Limits::new(1024)),
            Err(Error::Serialize(_))
        ));
    }

    /// QSpec FR-181: a field declared `Option<T>` is an option value, none
    /// `{"type":"option","value":null}` and some wrapping the encoded payload.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn an_option_value_encodes_as_an_option_object() {
        let int = ValueType::Int(
            quire_exact::IntegerInterval::new(Integer::from(0_i64), Integer::from(2_i64))
                .expect("nonempty"),
        );
        let none = quire_exact::OptionValue::none(int.clone());
        let some = quire_exact::OptionValue::from_admitted(
            int,
            Some(Value::Integer(Integer::from(1_i64))),
        );
        let text = |value: &Value| {
            untyped_value_enc(value)
                .canonical_text(Limits::new(1024))
                .expect("an option encodes")
        };
        assert_eq!(text(&none), r#"{"type":"option","value":null}"#);
        assert_eq!(
            text(&some),
            r#"{"type":"option","value":{"type":"integer","value":"1"}}"#
        );
    }

    /// The direct writer's result for `value` under a `bound`-byte limit,
    /// bytes or refusal, against `quire_canonical::to_vec`'s.
    fn assert_written_as_encoded(canonical: &Canonicalizer, value: &Enc, bound: u64) {
        let limits = Limits::new(bound);
        let written = canonical.to_vec(value, limits);
        let encoded = quire_canonical::to_vec(value, limits);
        assert_eq!(
            format!("{written:?}"),
            format!("{encoded:?}"),
            "{value:?} under {bound} bytes"
        );
    }

    /// FR-101-AC-2 and FR-101-AC-11: [`Canonicalizer`] writes the bytes and
    /// the refusals `quire-canonical` gives, at every byte bound, for strings
    /// that escape and that do not, member names held in any order or
    /// twice, maps whose keys or values do not encode, shared values, and
    /// node, effective and declaration identities.
    #[trace("TC-453", "FR-101-AC-2", "FR-101-AC-11")]
    #[test]
    fn the_direct_writer_writes_the_encoded_bytes_and_refusals() {
        let declaration = DeclarationKey {
            package: "test/sym".to_owned(),
            node: "ix://test/sym/counters".to_owned(),
        };
        let strings = [
            "",
            "plain",
            "quote \" and \\ backslash",
            "line\nfeed",
            "unit\u{1f}separator",
            "delete\u{7f}",
            "na\u{ef}ve \u{e9}t\u{e9}",
            "supplementary \u{1f600} plane",
        ];
        let mut trees: Vec<Enc> = strings.iter().map(|text| Enc::text(*text)).collect();
        trees.extend([
            Enc::Null,
            Enc::Bool(true),
            Enc::Array(vec![Enc::Bool(false), Enc::Null, Enc::text("x")]),
            Enc::Object(Vec::new()),
            Enc::Object(vec![("b", Enc::text("x")), ("a", Enc::text("y"))]),
            Enc::Object(vec![("a", Enc::text("x")), ("b", Enc::text("y"))]),
            Enc::Object(vec![
                ("\u{e9}", Enc::Null),
                ("\u{1f600}", Enc::Null),
                ("z", Enc::Null),
            ]),
            Enc::Object(vec![("a", Enc::Null), ("b", Enc::Null), ("a", Enc::Null)]),
            Enc::Object(vec![("b", Enc::Unencodable("a b")), ("a", Enc::text("y"))]),
            Enc::Object(vec![
                ("b", Enc::Object(vec![("d", Enc::Null), ("c", Enc::Null)])),
                (
                    "a",
                    Enc::Array(vec![Enc::Object(vec![("f", Enc::Null), ("e", Enc::Null)])]),
                ),
            ]),
            Enc::Map(Vec::new()),
            Enc::Map(vec![
                (Enc::text("b"), Enc::Null),
                (Enc::text("a"), Enc::text("q\"")),
                (Enc::text("b"), Enc::Bool(true)),
            ]),
            Enc::Map(vec![
                (Enc::text("a"), Enc::Null),
                (Enc::Unencodable("a key"), Enc::Null),
            ]),
            Enc::Map(vec![(Enc::text("a"), Enc::Unencodable("a value"))]),
            Enc::shared(Enc::Object(vec![("k", Enc::text("v")), ("j", Enc::Null)])),
            Enc::shared(Enc::Unencodable("a shared value")),
            Enc::Array(vec![Enc::shared(Enc::Unencodable("a shared value"))]),
            Enc::Node(crate::test_ids::node_key([7])),
            Enc::Effective(crate::test_ids::effective_id([8])),
            Enc::Declaration(declaration.clone()),
            Enc::Object(vec![
                ("node", Enc::Node(crate::test_ids::node_key([7]))),
                (
                    "effective",
                    Enc::Effective(crate::test_ids::effective_id([8])),
                ),
                ("declaration", Enc::Declaration(declaration)),
            ]),
            Enc::Unencodable("a value"),
        ]);
        // One canonicalizer across every tree and bound, so identities are
        // written from its caches after their first use.
        let canonical = Canonicalizer::default();
        for tree in &trees {
            let length = quire_canonical::to_vec(tree, Limits::new(u64::MAX)).map_or(64, |bytes| {
                u64::try_from(bytes.len()).expect("a length fits u64")
            });
            for bound in (0..=length.saturating_add(1)).chain([u64::MAX]) {
                assert_written_as_encoded(&canonical, tree, bound);
            }
        }
    }

    /// FR-101-AC-11: an object written beside bytes the caller wrote is
    /// left to the caller's encoding whenever it has no encoding or exceeds
    /// the bound, and is otherwise the object's RFC 8785 bytes.
    #[trace("TC-453", "FR-101-AC-11")]
    #[test]
    fn an_object_beside_written_bytes_is_written_only_when_it_encodes_within_the_bound() {
        let canonical = Canonicalizer::default();
        let members = [
            ("type", Enc::text("state")),
            ("control", Enc::Map(Vec::new())),
        ];
        let memory = br#"{"a":null}"#;
        let expected =
            br#"{"control":{"entries":[],"type":"map"},"memory":{"a":null},"type":"state"}"#;
        let bound = u64::try_from(expected.len()).expect("a length fits u64");
        assert_eq!(
            canonical.object_with(&members, "memory", memory, Limits::new(bound)),
            Some(expected.to_vec())
        );
        assert_eq!(
            canonical.object_with(&members, "memory", memory, Limits::new(bound - 1)),
            None
        );
        assert_eq!(
            canonical.object_with(&members, "type", memory, Limits::new(u64::MAX)),
            None
        );
        let unencodable = [("type", Enc::Unencodable("a rest point"))];
        assert_eq!(
            canonical.object_with(&unencodable, "memory", memory, Limits::new(u64::MAX)),
            None
        );
    }
}
