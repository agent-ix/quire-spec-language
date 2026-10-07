// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070 "Witness value text": the wire form of a [`WitnessValue`] in a
//! witness transcript entry, written and read through `quire-canonical`.
//!
//! The value text is the RFC 8785 (JCS) encoding of the value's QSpec FR-181
//! typed canonical form, and the entry carries it with exactly four
//! characters replaced (`%`, `;`, `<` and `>`), so each value has one escaped
//! form. Both directions run on an explicit heap stack: a value of any depth
//! encodes and decodes on a small native stack, and only the entry's byte
//! length bounds it.

use std::collections::{HashMap, HashSet};

use qsl_foundation::digest::WireNodeId;
use quire_canonical::{read, to_vec, Error as CanonicalError, Limits, Node, NodeRef, Writer};
use quire_exact::Integer;

use super::value::{QuantityMagnitude, WitnessField, WitnessSlot, WitnessValue};

/// Why a value text did not encode.
#[derive(Debug, thiserror::Error)]
#[error("a witness value does not encode: {0}")]
pub struct ValueTextError(#[from] CanonicalError);

/// What is wrong with a witness entry that did not decode (FR-070 "Decode").
/// It tells the refusals apart without carrying any of the entry's content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryFault {
    /// A `Boolean` or integer entry that is not `0`/`1` or a decimal `i128`.
    Scalar,
    /// A `%` that does not begin one of the four escapes, an escape in
    /// lowercase hex, or a raw `<`, `>` or `;` in the escaped text.
    Escape,
    /// The unescaped text is not exactly one JSON value.
    Json,
    /// The value is not its own JCS encoding.
    NotJcs,
    /// A `type` tag outside the table, or a shape with a missing, extra or
    /// mistyped member.
    Shape,
    /// A decimal string, rational, `bits` string or `identity` not in the
    /// form's spelling.
    Spelling,
    /// `{"type":"null"}` anywhere but as a record field's slot.
    Null,
    /// `set` or `bag` elements out of ascending order, or two elements with
    /// equal JCS bytes in a `set` or an `ordered-set`.
    Elements,
}

// ---------------------------------------------------------------------------
// Encoding.
// ---------------------------------------------------------------------------

/// The four bytes the entry replaces.
fn is_escaped(byte: u8) -> bool {
    matches!(byte, b'%' | b';' | b'<' | b'>')
}

/// One step of the encoder.
enum Task<'v> {
    Value(&'v WitnessValue),
    Slot(&'v WitnessSlot),
    Name(&'static str),
    Str(&'v str),
    Owned(String),
    Bool(bool),
    Null,
    BeginObject,
    EndObject,
    BeginArray,
    EndArray,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn push_in_order<'v>(tasks: &mut Vec<Task<'v>>, steps: Vec<Task<'v>>) {
    tasks.extend(steps.into_iter().rev());
}

/// `members` as an object: its member names, each with the task that writes
/// its value. The writer orders the members, so the order here is free.
fn object<'v>(members: Vec<(&'static str, Task<'v>)>, extra: Vec<Task<'v>>) -> Vec<Task<'v>> {
    let mut steps = vec![Task::BeginObject];
    for (name, value) in members {
        steps.push(Task::Name(name));
        steps.push(value);
    }
    steps.extend(extra);
    steps.push(Task::EndObject);
    steps
}

fn tag(name: &'static str) -> (&'static str, Task<'static>) {
    ("type", Task::Str(name))
}

fn text_of(value: &Integer) -> Task<'static> {
    Task::Owned(value.to_string())
}

/// One value's shell: the JCS bytes of the value with each child written as
/// a placeholder number `k`, and the children in placeholder order.
struct Shell<'v> {
    bytes: Vec<u8>,
    children: Vec<&'v WitnessValue>,
}

/// Encode `value`'s own shell. A child (`Task::Value` below the root) is not
/// expanded: it is written as the bare number `k`, its position in
/// `Shell::children`. The shell holds no other bare number (every scalar the
/// forms carry is a string, a boolean or `null`), so [`splice`] finds each
/// placeholder by position, outside any string.
fn encode_shell<'v>(
    value: &'v WitnessValue,
    order: Option<&[usize]>,
) -> Result<Shell<'v>, CanonicalError> {
    let mut bytes = Vec::new();
    let mut children = Vec::new();
    {
        let mut writer = Writer::new(&mut bytes, Limits::new(u64::MAX));
        let mut tasks = value_steps(value, order);
        tasks.reverse();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Name(name) => writer.name(name)?,
                Task::Str(text) => writer.string(text)?,
                Task::Owned(text) => writer.string(&text)?,
                Task::Bool(flag) => writer.bool(flag)?,
                Task::Null => writer.null()?,
                Task::BeginObject => writer.begin_object()?,
                Task::EndObject => writer.end_object()?,
                Task::BeginArray => writer.begin_array()?,
                Task::EndArray => writer.end_array()?,
                Task::Slot(slot) => match slot {
                    WitnessSlot::Present(child) => tasks.push(Task::Value(child)),
                    WitnessSlot::Absent => tasks.push(Task::Null),
                    WitnessSlot::Null => {
                        push_in_order(&mut tasks, object(vec![tag("null")], Vec::new()));
                    }
                },
                Task::Value(child) => {
                    writer.integer(i128::try_from(children.len()).map_err(|_| {
                        CanonicalError::Internal {
                            invariant: "a value has fewer children than i128 holds",
                        }
                    })?)?;
                    children.push(child);
                }
            }
        }
        writer.finish()?;
    }
    Ok(Shell { bytes, children })
}

/// `shell` with each placeholder number replaced by the matching entry of
/// `parts`, found by scanning the shell outside its strings.
fn splice(shell: &[u8], parts: &mut [Vec<u8>]) -> Result<Vec<u8>, CanonicalError> {
    let invariant = |invariant| CanonicalError::Internal { invariant };
    let total: usize = parts.iter().map(Vec::len).sum();
    let mut out = Vec::with_capacity(shell.len() + total);
    let mut position = 0;
    let mut in_string = false;
    while let Some(&byte) = shell.get(position) {
        if in_string {
            out.push(byte);
            match byte {
                b'\\' => {
                    position += 1;
                    out.push(
                        *shell
                            .get(position)
                            .ok_or(invariant("a string escape ends"))?,
                    );
                }
                b'"' => in_string = false,
                _ => {}
            }
            position += 1;
        } else if byte.is_ascii_digit() {
            let mut index = 0_usize;
            while let Some(digit) = shell.get(position).filter(|digit| digit.is_ascii_digit()) {
                index = index * 10 + usize::from(digit - b'0');
                position += 1;
            }
            let part = parts
                .get_mut(index)
                .ok_or(invariant("a placeholder names a child"))?;
            out.append(part);
        } else {
            in_string = byte == b'"';
            out.push(byte);
            position += 1;
        }
    }
    Ok(out)
}

/// The JCS bytes of `root`, built bottom-up: each value's shell is encoded
/// once, and its parent splices the finished bytes in, so no value is encoded
/// more than once at any nesting depth. Set and bag elements are ordered by
/// those bytes. Also returns the bytes the encoder wrote, which is the sum of
/// every value's shell.
fn encode_bottom_up(root: &WitnessValue) -> Result<(Vec<u8>, u64), CanonicalError> {
    let mut preorder = Vec::new();
    let mut pending = vec![root];
    while let Some(value) = pending.pop() {
        preorder.push(value);
        pending.extend(value.children());
    }
    let mut finished: HashMap<*const WitnessValue, Vec<u8>> = HashMap::new();
    let mut encoded = 0_u64;
    let missing = || CanonicalError::Internal {
        invariant: "a child is finished before its parent",
    };
    for value in preorder.into_iter().rev() {
        let order = match value {
            WitnessValue::Set(elements) | WitnessValue::Bag(elements) => {
                let mut keyed = Vec::with_capacity(elements.len());
                for (index, element) in elements.iter().enumerate() {
                    let bytes = finished
                        .get(&std::ptr::from_ref(element))
                        .ok_or_else(missing)?;
                    keyed.push((bytes, index));
                }
                keyed.sort();
                Some(
                    keyed
                        .into_iter()
                        .map(|(_, index)| index)
                        .collect::<Vec<_>>(),
                )
            }
            _ => None,
        };
        let shell = encode_shell(value, order.as_deref())?;
        encoded = encoded.saturating_add(u64::try_from(shell.bytes.len()).unwrap_or(u64::MAX));
        let mut parts = Vec::with_capacity(shell.children.len());
        for child in &shell.children {
            parts.push(
                finished
                    .remove(&std::ptr::from_ref(*child))
                    .ok_or_else(missing)?,
            );
        }
        let bytes = splice(&shell.bytes, &mut parts)?;
        finished.insert(std::ptr::from_ref(value), bytes);
    }
    let bytes = finished
        .remove(&std::ptr::from_ref(root))
        .ok_or_else(missing)?;
    Ok((bytes, encoded))
}

/// An array of `elements`, each written as a value.
fn array<'v>(elements: Vec<&'v WitnessValue>) -> Vec<Task<'v>> {
    let mut steps = vec![Task::BeginArray];
    steps.extend(elements.into_iter().map(Task::Value));
    steps.push(Task::EndArray);
    steps
}

/// An object holding `members`, then the member `name` whose value is the
/// array steps `elements`.
fn with_array<'v>(
    members: Vec<(&'static str, Task<'v>)>,
    name: &'static str,
    elements: Vec<Task<'v>>,
) -> Vec<Task<'v>> {
    let mut steps = vec![Task::BeginObject];
    for (member, task) in members {
        steps.push(Task::Name(member));
        steps.push(task);
    }
    steps.push(Task::Name(name));
    steps.extend(elements);
    steps.push(Task::EndObject);
    steps
}

/// The steps that write one value.
fn value_steps<'v>(value: &'v WitnessValue, order: Option<&[usize]>) -> Vec<Task<'v>> {
    match value {
        WitnessValue::Boolean(flag) => {
            object(vec![tag("boolean"), ("value", Task::Bool(*flag))], vec![])
        }
        WitnessValue::Integer(number) => object(
            vec![tag("integer"), ("value", Task::Owned(number.to_string()))],
            vec![],
        ),
        WitnessValue::ExactInteger(number) => {
            object(vec![tag("integer"), ("value", text_of(number))], vec![])
        }
        WitnessValue::Enum {
            declaration,
            member,
        } => object(
            vec![
                tag("enum"),
                ("name", Task::Owned(declaration.to_string())),
                ("member", Task::Str(member)),
            ],
            vec![],
        ),
        WitnessValue::Text(text) => object(vec![tag("text"), ("value", Task::Str(text))], vec![]),
        WitnessValue::Rational {
            numerator,
            denominator,
        } => object(
            vec![
                tag("rational"),
                ("numerator", text_of(numerator)),
                ("denominator", text_of(denominator)),
            ],
            vec![],
        ),
        WitnessValue::Decimal { coefficient, scale } => object(
            vec![
                tag("decimal"),
                ("coefficient", text_of(coefficient)),
                ("scale", text_of(scale)),
            ],
            vec![],
        ),
        WitnessValue::Float32(bits) => object(
            vec![tag("float32"), ("bits", Task::Owned(format!("{bits:08x}")))],
            vec![],
        ),
        WitnessValue::Float64(bits) => object(
            vec![
                tag("float64"),
                ("bits", Task::Owned(format!("{bits:016x}"))),
            ],
            vec![],
        ),
        WitnessValue::Quantity { magnitude, unit } => {
            let mut members = match magnitude {
                QuantityMagnitude::Rational {
                    numerator,
                    denominator,
                } => vec![
                    tag("rational"),
                    ("numerator", text_of(numerator)),
                    ("denominator", text_of(denominator)),
                ],
                QuantityMagnitude::Decimal { coefficient, scale } => vec![
                    tag("decimal"),
                    ("coefficient", text_of(coefficient)),
                    ("scale", text_of(scale)),
                ],
            };
            members.push(("unit", Task::Owned(hex(unit))));
            object(members, vec![])
        }
        WitnessValue::Reference {
            universe,
            object_type,
            identity,
        } => object(
            vec![
                tag("reference"),
                ("universe", Task::Owned(hex(universe))),
                ("object_type", Task::Owned(hex(object_type))),
                ("identity", Task::Owned(hex(identity))),
            ],
            vec![],
        ),
        WitnessValue::Option(None) => object(vec![tag("option"), ("value", Task::Null)], vec![]),
        WitnessValue::Option(Some(inner)) => {
            object(vec![tag("option"), ("value", Task::Value(inner))], vec![])
        }
        WitnessValue::Record {
            declaration,
            fields,
        } => {
            let mut elements = vec![Task::BeginArray];
            for field in fields {
                elements.extend(object(
                    vec![
                        ("name", Task::Str(&field.name)),
                        ("value", Task::Slot(&field.slot)),
                    ],
                    vec![],
                ));
            }
            elements.push(Task::EndArray);
            with_array(
                vec![
                    tag("record"),
                    ("name", Task::Owned(declaration.to_string())),
                ],
                "fields",
                elements,
            )
        }
        WitnessValue::Tuple {
            declaration,
            components,
        } => with_array(
            vec![tag("tuple"), ("name", Task::Owned(declaration.to_string()))],
            "components",
            array(components.iter().collect()),
        ),
        WitnessValue::Union {
            declaration,
            member,
            components,
        } => with_array(
            vec![
                tag("union"),
                ("name", Task::Owned(declaration.to_string())),
                ("member", Task::Str(member)),
            ],
            "components",
            array(components.iter().collect()),
        ),
        WitnessValue::Sequence(elements) => with_array(
            vec![tag("sequence")],
            "elements",
            array(elements.iter().collect()),
        ),
        WitnessValue::OrderedSet(elements) => with_array(
            vec![tag("ordered-set")],
            "elements",
            array(elements.iter().collect()),
        ),
        WitnessValue::Set(elements) | WitnessValue::Bag(elements) => {
            let name = if matches!(value, WitnessValue::Set(_)) {
                "set"
            } else {
                "bag"
            };
            // The element order is the sorted one computed for this node.
            let ordered: Vec<&WitnessValue> = match order {
                Some(order) => order
                    .iter()
                    .filter_map(|index| elements.get(*index))
                    .collect(),
                None => elements.iter().collect(),
            };
            with_array(vec![tag(name)], "elements", array(ordered))
        }
    }
}

impl WitnessValue {
    /// This value's witness value text as a transcript entry carries it: its
    /// JCS encoding with `%`, `;`, `<` and `>` replaced by `%25`, `%3B`,
    /// `%3C` and `%3E`. Set and bag elements are written ascending by their
    /// own JCS bytes, so equal values always give the same text.
    pub fn to_value_text(&self) -> Result<String, ValueTextError> {
        let (bytes, _) = encode_bottom_up(self)?;
        let mut escaped = Vec::with_capacity(bytes.len());
        for byte in bytes {
            match byte {
                b'%' => escaped.extend_from_slice(b"%25"),
                b';' => escaped.extend_from_slice(b"%3B"),
                b'<' => escaped.extend_from_slice(b"%3C"),
                b'>' => escaped.extend_from_slice(b"%3E"),
                other => escaped.push(other),
            }
        }
        String::from_utf8(escaped).map_err(|_| {
            ValueTextError(CanonicalError::Internal {
                invariant: "JCS output is UTF-8",
            })
        })
    }

    /// The byte length of [`Self::to_value_text`]. A value that does not
    /// encode counts as `usize::MAX`, so the reader bound refuses it.
    pub fn value_text_len(&self) -> usize {
        encode_bottom_up(self).map_or(usize::MAX, |(bytes, _)| {
            let escaped: usize = bytes
                .iter()
                .map(|byte| if is_escaped(*byte) { 3 } else { 1 })
                .sum();
            escaped
        })
    }

    /// The value text `escaped`, read as one value (FR-070 "Decode").
    #[qsl_attrs::string_edge]
    pub(crate) fn from_value_text(escaped: &str) -> Result<Self, EntryFault> {
        let bytes = unescape(escaped)?;
        let document = read(&bytes, u64::MAX).map_err(|_| EntryFault::Json)?;
        let canonical =
            to_vec(&document.root(), Limits::new(u64::MAX)).map_err(|_| EntryFault::Json)?;
        if canonical != bytes {
            return Err(EntryFault::NotJcs);
        }
        let value = build(document.root())?;
        check_collections(&bytes)?;
        Ok(value)
    }
}

// ---------------------------------------------------------------------------
// Decoding.
// ---------------------------------------------------------------------------

/// Reverse the four replacements. A `%` that begins none of them, an escape
/// in lowercase hex, and a raw `;`, `<` or `>` (which no escaped text holds)
/// are refused.
fn unescape(text: &str) -> Result<Vec<u8>, EntryFault> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'%' => {
                let replaced = match bytes.get(index + 1..index + 3) {
                    Some(b"25") => b'%',
                    Some(b"3B") => b';',
                    Some(b"3C") => b'<',
                    Some(b"3E") => b'>',
                    _ => return Err(EntryFault::Escape),
                };
                out.push(replaced);
                index += 3;
            }
            b';' | b'<' | b'>' => return Err(EntryFault::Escape),
            other => {
                out.push(other);
                index += 1;
            }
        }
    }
    Ok(out)
}

/// The members of `node` when it is an object holding exactly `names`.
fn exact_object<'d>(
    node: NodeRef<'d>,
    names: &[&str],
) -> Result<Vec<(&'d str, NodeRef<'d>)>, EntryFault> {
    let Node::Object(members) = node.node() else {
        return Err(EntryFault::Shape);
    };
    let members: Vec<_> = members.collect();
    let exact = members.len() == names.len()
        && names
            .iter()
            .all(|name| members.iter().any(|(member, _)| member == name));
    if exact {
        Ok(members)
    } else {
        Err(EntryFault::Shape)
    }
}

fn member<'d>(members: &[(&str, NodeRef<'d>)], name: &str) -> Result<NodeRef<'d>, EntryFault> {
    members
        .iter()
        .find(|(member, _)| *member == name)
        .map(|(_, node)| *node)
        .ok_or(EntryFault::Shape)
}

fn string_member<'d>(members: &[(&str, NodeRef<'d>)], name: &str) -> Result<&'d str, EntryFault> {
    match member(members, name)?.node() {
        Node::String(text) => Ok(text),
        _ => Err(EntryFault::Shape),
    }
}

fn integer_member(members: &[(&str, NodeRef<'_>)], name: &str) -> Result<Integer, EntryFault> {
    string_member(members, name)?
        .parse::<Integer>()
        .map_err(|_| EntryFault::Spelling)
}

fn digest_member(members: &[(&str, NodeRef<'_>)], name: &str) -> Result<[u8; 32], EntryFault> {
    WireNodeId::from_hex(string_member(members, name)?)
        .map(|id| *id.as_bytes())
        .ok_or(EntryFault::Spelling)
}

fn declaration_member(
    members: &[(&str, NodeRef<'_>)],
    name: &str,
) -> Result<WireNodeId, EntryFault> {
    WireNodeId::from_hex(string_member(members, name)?).ok_or(EntryFault::Spelling)
}

/// Whether `text` is lowercase hex of exactly `digits` digits.
fn lower_hex(text: &str, digits: usize) -> bool {
    text.len() == digits
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn bits_member(members: &[(&str, NodeRef<'_>)], digits: usize) -> Result<u64, EntryFault> {
    let text = string_member(members, "bits")?;
    if !lower_hex(text, digits) {
        return Err(EntryFault::Spelling);
    }
    u64::from_str_radix(text, 16).map_err(|_| EntryFault::Spelling)
}

/// The rational `numerator`/`denominator` members: reduced, with a positive
/// denominator.
fn rational_members(members: &[(&str, NodeRef<'_>)]) -> Result<(Integer, Integer), EntryFault> {
    let numerator = integer_member(members, "numerator")?;
    let denominator = integer_member(members, "denominator")?;
    let reduced = !denominator.is_zero()
        && !denominator.is_negative()
        && numerator.gcd(&denominator) == Integer::one();
    if reduced {
        Ok((numerator, denominator))
    } else {
        Err(EntryFault::Spelling)
    }
}

/// An identity as lowercase hex of whole bytes.
fn identity_bytes(text: &str) -> Result<Vec<u8>, EntryFault> {
    if !text.len().is_multiple_of(2) || !lower_hex(text, text.len()) {
        return Err(EntryFault::Spelling);
    }
    (0..text.len())
        .step_by(2)
        .map(|start| {
            text.get(start..start + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or(EntryFault::Spelling)
        })
        .collect()
}

/// What a collection's elements must satisfy.
#[derive(Clone, Copy)]
enum CollectionKind {
    Sequence,
    OrderedSet,
    Set,
    Bag,
}

#[derive(Clone, Copy)]
enum SlotKind {
    Present,
    Absent,
    Null,
}

/// One pending step of [`build`].
enum Step<'d> {
    Visit(NodeRef<'d>),
    Option {
        present: bool,
    },
    Record {
        declaration: WireNodeId,
        fields: Vec<(String, SlotKind)>,
    },
    Tuple {
        declaration: WireNodeId,
        count: usize,
    },
    Union {
        declaration: WireNodeId,
        member: String,
        count: usize,
    },
    Collection {
        kind: CollectionKind,
        count: usize,
    },
}

fn array_of<'d>(node: NodeRef<'d>) -> Result<Vec<NodeRef<'d>>, EntryFault> {
    match node.node() {
        Node::Array(items) => Ok(items.collect()),
        _ => Err(EntryFault::Shape),
    }
}

/// A value's byte range in the text.
type Span = (usize, usize);

/// An open array or object while [`check_collections`] scans.
enum Open {
    Array {
        start: usize,
        items: Vec<Span>,
    },
    Object {
        start: usize,
        /// The key just read, until its value arrives.
        key: Option<Span>,
        /// Each member's key, value and, for an array, its items.
        members: Vec<(Span, Span, Option<Vec<Span>>)>,
    },
}

/// Check every `set`, `bag` and `ordered-set` in `bytes`: elements ascending
/// and distinct (`set`), ascending (`bag`) or distinct (`ordered-set`) by
/// their JCS bytes.
///
/// `bytes` is already known to be one canonical JCS document, so each
/// element's bytes are its own span of `bytes`: one scan finds every span,
/// and nothing is encoded again at any nesting depth. The scan runs on an
/// explicit stack.
fn check_collections(bytes: &[u8]) -> Result<(), EntryFault> {
    let mut stack: Vec<Open> = Vec::new();
    let mut position = 0;
    while let Some(&byte) = bytes.get(position) {
        match byte {
            b'{' => {
                stack.push(Open::Object {
                    start: position,
                    key: None,
                    members: Vec::new(),
                });
                position += 1;
            }
            b'[' => {
                stack.push(Open::Array {
                    start: position,
                    items: Vec::new(),
                });
                position += 1;
            }
            b'}' | b']' => {
                let closed = stack.pop().ok_or(EntryFault::Shape)?;
                position += 1;
                match closed {
                    Open::Array { start, items } => {
                        deliver(&mut stack, (start, position), Some(items))?;
                    }
                    Open::Object { start, members, .. } => {
                        check_object(bytes, &members)?;
                        deliver(&mut stack, (start, position), None)?;
                    }
                }
            }
            b',' | b':' => position += 1,
            b'"' => {
                let end = string_end(bytes, position)?;
                match stack.last_mut() {
                    Some(Open::Object { key, .. }) if key.is_none() => {
                        *key = Some((position + 1, end - 1));
                    }
                    _ => deliver(&mut stack, (position, end), None)?,
                }
                position = end;
            }
            _ => {
                let end = bytes
                    .get(position..)
                    .and_then(|rest| rest.iter().position(|b| matches!(b, b',' | b'}' | b']')))
                    .map_or(bytes.len(), |offset| position + offset);
                deliver(&mut stack, (position, end), None)?;
                position = end;
            }
        }
    }
    Ok(())
}

/// The end (one past the closing quote) of the string starting at `start`.
fn string_end(bytes: &[u8], start: usize) -> Result<usize, EntryFault> {
    let mut position = start + 1;
    while let Some(&byte) = bytes.get(position) {
        match byte {
            b'\\' => position += 2,
            b'"' => return Ok(position + 1),
            _ => position += 1,
        }
    }
    Err(EntryFault::Json)
}

/// Hand a finished value to the array or object holding it.
fn deliver(stack: &mut [Open], span: Span, items: Option<Vec<Span>>) -> Result<(), EntryFault> {
    match stack.last_mut() {
        Some(Open::Array { items: held, .. }) => held.push(span),
        Some(Open::Object { key, members, .. }) => {
            let key = key.take().ok_or(EntryFault::Shape)?;
            members.push((key, span, items));
        }
        None => {}
    }
    Ok(())
}

/// If the closed object is a collection, check its elements.
fn check_object(
    bytes: &[u8],
    members: &[(Span, Span, Option<Vec<Span>>)],
) -> Result<(), EntryFault> {
    let named = |name: &[u8]| {
        members
            .iter()
            .find(|((start, end), ..)| bytes.get(*start..*end) == Some(name))
    };
    let Some(((_, _), (start, end), _)) = named(b"type") else {
        return Ok(());
    };
    let kind = match bytes.get(*start..*end) {
        Some(b"\"set\"") => CollectionKind::Set,
        Some(b"\"bag\"") => CollectionKind::Bag,
        Some(b"\"ordered-set\"") => CollectionKind::OrderedSet,
        _ => return Ok(()),
    };
    let Some((_, _, Some(elements))) = named(b"elements") else {
        return Ok(());
    };
    // No order or distinctness check applies below two elements.
    if elements.len() < 2 {
        return Ok(());
    }
    let keys: Vec<&[u8]> = elements
        .iter()
        .map(|(start, end)| bytes.get(*start..*end).ok_or(EntryFault::Shape))
        .collect::<Result<_, _>>()?;
    let ordered = match kind {
        CollectionKind::Set => keys.windows(2).all(|pair| pair[0] < pair[1]),
        CollectionKind::Bag => keys.windows(2).all(|pair| pair[0] <= pair[1]),
        CollectionKind::OrderedSet => {
            let distinct: HashSet<&[u8]> = keys.iter().copied().collect();
            distinct.len() == keys.len()
        }
        CollectionKind::Sequence => true,
    };
    if ordered {
        Ok(())
    } else {
        Err(EntryFault::Elements)
    }
}

/// The value `root` holds, read on an explicit heap stack.
fn build(root: NodeRef<'_>) -> Result<WitnessValue, EntryFault> {
    let mut pending = vec![Step::Visit(root)];
    let mut values: Vec<WitnessValue> = Vec::new();
    let underflow = || EntryFault::Shape;
    while let Some(step) = pending.pop() {
        match step {
            Step::Visit(node) => visit(node, &mut pending, &mut values)?,
            Step::Option { present } => {
                let inner = if present {
                    Some(Box::new(values.pop().ok_or_else(underflow)?))
                } else {
                    None
                };
                values.push(WitnessValue::Option(inner));
            }
            Step::Record {
                declaration,
                fields,
            } => {
                let present = fields
                    .iter()
                    .filter(|(_, kind)| matches!(kind, SlotKind::Present))
                    .count();
                let start = values.len().checked_sub(present).ok_or_else(underflow)?;
                let mut built = values.split_off(start).into_iter();
                let fields = fields
                    .into_iter()
                    .map(|(name, kind)| {
                        let slot = match kind {
                            SlotKind::Present => WitnessSlot::Present(built.next()?),
                            SlotKind::Absent => WitnessSlot::Absent,
                            SlotKind::Null => WitnessSlot::Null,
                        };
                        Some(WitnessField { name, slot })
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(underflow)?;
                values.push(WitnessValue::Record {
                    declaration,
                    fields,
                });
            }
            Step::Tuple { declaration, count } => {
                let start = values.len().checked_sub(count).ok_or_else(underflow)?;
                let components = values.split_off(start);
                values.push(WitnessValue::Tuple {
                    declaration,
                    components,
                });
            }
            Step::Union {
                declaration,
                member,
                count,
            } => {
                let start = values.len().checked_sub(count).ok_or_else(underflow)?;
                let components = values.split_off(start);
                values.push(WitnessValue::Union {
                    declaration,
                    member,
                    components,
                });
            }
            Step::Collection { kind, count } => {
                let start = values.len().checked_sub(count).ok_or_else(underflow)?;
                let elements = values.split_off(start);
                values.push(match kind {
                    CollectionKind::Sequence => WitnessValue::Sequence(elements),
                    CollectionKind::OrderedSet => WitnessValue::OrderedSet(elements),
                    CollectionKind::Set => WitnessValue::Set(elements),
                    CollectionKind::Bag => WitnessValue::Bag(elements),
                });
            }
        }
    }
    match (values.pop(), values.is_empty()) {
        (Some(value), true) => Ok(value),
        _ => Err(EntryFault::Shape),
    }
}

/// Read the form at `node`: a leaf is pushed as a value, a composite pushes
/// its parts' visits and then the step that assembles them.
#[qsl_attrs::string_edge]
fn visit<'d>(
    node: NodeRef<'d>,
    pending: &mut Vec<Step<'d>>,
    values: &mut Vec<WitnessValue>,
) -> Result<(), EntryFault> {
    let Node::Object(first) = node.node() else {
        return Err(EntryFault::Shape);
    };
    let tag = first
        .clone()
        .find(|(name, _)| *name == "type")
        .map(|(_, tag)| tag)
        .ok_or(EntryFault::Shape)?;
    let Node::String(tag) = tag.node() else {
        return Err(EntryFault::Shape);
    };
    let has_unit = first.clone().any(|(name, _)| name == "unit");
    match tag {
        "boolean" => {
            let members = exact_object(node, &["type", "value"])?;
            let Node::Bool(flag) = member(&members, "value")?.node() else {
                return Err(EntryFault::Shape);
            };
            values.push(WitnessValue::Boolean(flag));
        }
        "integer" => {
            let members = exact_object(node, &["type", "value"])?;
            values.push(WitnessValue::ExactInteger(integer_member(
                &members, "value",
            )?));
        }
        "enum" => {
            let members = exact_object(node, &["type", "name", "member"])?;
            values.push(WitnessValue::Enum {
                declaration: declaration_member(&members, "name")?,
                member: string_member(&members, "member")?.to_owned(),
            });
        }
        "text" => {
            let members = exact_object(node, &["type", "value"])?;
            values.push(WitnessValue::Text(
                string_member(&members, "value")?.to_owned(),
            ));
        }
        "rational" | "decimal" => {
            let rational = tag == "rational";
            let magnitude_names: &[&str] = if rational {
                &["type", "numerator", "denominator"]
            } else {
                &["type", "coefficient", "scale"]
            };
            let names: Vec<&str> = magnitude_names
                .iter()
                .copied()
                .chain(has_unit.then_some("unit"))
                .collect();
            let members = exact_object(node, &names)?;
            let magnitude = if rational {
                let (numerator, denominator) = rational_members(&members)?;
                QuantityMagnitude::Rational {
                    numerator,
                    denominator,
                }
            } else {
                QuantityMagnitude::Decimal {
                    coefficient: integer_member(&members, "coefficient")?,
                    scale: integer_member(&members, "scale")?,
                }
            };
            values.push(if has_unit {
                WitnessValue::Quantity {
                    magnitude,
                    unit: digest_member(&members, "unit")?,
                }
            } else {
                match magnitude {
                    QuantityMagnitude::Rational {
                        numerator,
                        denominator,
                    } => WitnessValue::Rational {
                        numerator,
                        denominator,
                    },
                    QuantityMagnitude::Decimal { coefficient, scale } => {
                        WitnessValue::Decimal { coefficient, scale }
                    }
                }
            });
        }
        "float32" => {
            let members = exact_object(node, &["type", "bits"])?;
            let bits =
                u32::try_from(bits_member(&members, 8)?).map_err(|_| EntryFault::Spelling)?;
            values.push(WitnessValue::Float32(bits));
        }
        "float64" => {
            let members = exact_object(node, &["type", "bits"])?;
            values.push(WitnessValue::Float64(bits_member(&members, 16)?));
        }
        "reference" => {
            let members = exact_object(node, &["type", "universe", "object_type", "identity"])?;
            values.push(WitnessValue::Reference {
                universe: digest_member(&members, "universe")?,
                object_type: digest_member(&members, "object_type")?,
                identity: identity_bytes(string_member(&members, "identity")?)?,
            });
        }
        "option" => {
            let members = exact_object(node, &["type", "value"])?;
            let inner = member(&members, "value")?;
            let present = !matches!(inner.node(), Node::Null);
            pending.push(Step::Option { present });
            if present {
                pending.push(Step::Visit(inner));
            }
        }
        "record" => {
            let members = exact_object(node, &["type", "name", "fields"])?;
            let declaration = declaration_member(&members, "name")?;
            let mut fields = Vec::new();
            let mut present = Vec::new();
            for entry in array_of(member(&members, "fields")?)? {
                let parts = exact_object(entry, &["name", "value"])?;
                let name = string_member(&parts, "name")?.to_owned();
                let slot = member(&parts, "value")?;
                let kind = match slot.node() {
                    Node::Null => SlotKind::Absent,
                    Node::Object(_) if is_null_slot(slot)? => SlotKind::Null,
                    _ => {
                        present.push(slot);
                        SlotKind::Present
                    }
                };
                fields.push((name, kind));
            }
            pending.push(Step::Record {
                declaration,
                fields,
            });
            pending.extend(present.into_iter().rev().map(Step::Visit));
        }
        "tuple" => {
            let members = exact_object(node, &["type", "name", "components"])?;
            let declaration = declaration_member(&members, "name")?;
            let components = array_of(member(&members, "components")?)?;
            pending.push(Step::Tuple {
                declaration,
                count: components.len(),
            });
            pending.extend(components.into_iter().rev().map(Step::Visit));
        }
        "union" => {
            let members = exact_object(node, &["type", "name", "member", "components"])?;
            let declaration = declaration_member(&members, "name")?;
            let member_name = string_member(&members, "member")?.to_owned();
            let components = array_of(member(&members, "components")?)?;
            pending.push(Step::Union {
                declaration,
                member: member_name,
                count: components.len(),
            });
            pending.extend(components.into_iter().rev().map(Step::Visit));
        }
        "sequence" | "ordered-set" | "set" | "bag" => {
            let kind = match tag {
                "sequence" => CollectionKind::Sequence,
                "ordered-set" => CollectionKind::OrderedSet,
                "set" => CollectionKind::Set,
                _ => CollectionKind::Bag,
            };
            let members = exact_object(node, &["type", "elements"])?;
            let elements = array_of(member(&members, "elements")?)?;
            pending.push(Step::Collection {
                kind,
                count: elements.len(),
            });
            pending.extend(elements.into_iter().rev().map(Step::Visit));
        }
        "null" => return Err(EntryFault::Null),
        _ => return Err(EntryFault::Shape),
    }
    Ok(())
}

/// Whether the object `slot` is a `{"type":"null"}` slot rather than a value.
/// A `null`-tagged object with any other member is a shape fault.
#[qsl_attrs::string_edge]
fn is_null_slot(slot: NodeRef<'_>) -> Result<bool, EntryFault> {
    let Some(tag) = slot.get("type") else {
        return Ok(false);
    };
    if !matches!(tag.node(), Node::String("null")) {
        return Ok(false);
    }
    exact_object(slot, &["type"])?;
    Ok(true)
}

#[cfg(test)]
mod tests;
