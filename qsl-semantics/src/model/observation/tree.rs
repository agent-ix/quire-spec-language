// SPDX-License-Identifier: AGPL-3.0-or-later
//! The observation reader's view of a document read by `quire-canonical`'s
//! shared reader (FR-261): `quire_canonical::NodeRef` with the accessors the
//! document checks use, and the FR-106 value form read from it on an
//! explicit heap stack.
//!
//! The reader's arena keeps object members in the document's own order,
//! which FR-106's admission-order rule ("naming the first [unknown member]
//! in document order") needs, and refuses a repeated member name, so a
//! member is found by name alone. Nothing here recurses with the document's
//! depth.

use quire_canonical::{Node, NodeRef};

use super::{RawNode, SelectedObject, SnapshotValue};

/// One value of a document read by `quire-canonical`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Json<'d>(NodeRef<'d>);

/// The members of one object of a document, in document order.
#[derive(Clone, Copy, Debug)]
pub(super) struct Object<'d>(NodeRef<'d>);

impl<'d> Json<'d> {
    /// The document's top-level value.
    pub(super) fn of(node: NodeRef<'d>) -> Self {
        Self(node)
    }

    /// This value's members, or `None` when it is not an object.
    pub(super) fn as_object(self) -> Option<Object<'d>> {
        matches!(self.0.node(), Node::Object(_)).then_some(Object(self.0))
    }

    /// This value's elements, or `None` when it is not an array.
    pub(super) fn as_array(self) -> Option<impl ExactSizeIterator<Item = Json<'d>>> {
        match self.0.node() {
            Node::Array(items) => Some(items.map(Json)),
            _ => None,
        }
    }

    /// This value as a string, or `None` when it is not one.
    pub(super) fn as_str(self) -> Option<&'d str> {
        match self.0.node() {
            Node::String(text) => Some(text),
            _ => None,
        }
    }

    /// This value as a bool, or `None` when it is not one.
    pub(super) fn as_bool(self) -> Option<bool> {
        match self.0.node() {
            Node::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Whether this value is JSON `null`.
    pub(super) fn is_null(self) -> bool {
        matches!(self.0.node(), Node::Null)
    }
}

impl<'d> Object<'d> {
    /// The members, in document order. `Json::as_object` is the only
    /// constructor, so this is never empty for want of an object.
    fn members(self) -> impl Iterator<Item = (&'d str, NodeRef<'d>)> {
        match self.0.node() {
            Node::Object(members) => Some(members),
            _ => None,
        }
        .into_iter()
        .flatten()
    }

    /// The value of member `name`.
    pub(super) fn member(self, name: &str) -> Option<Json<'d>> {
        self.0.get(name).map(Json)
    }

    /// Whether a member named `name` is present.
    pub(super) fn contains_key(self, name: &str) -> bool {
        self.member(name).is_some()
    }

    /// The members as `(name, value)`, in document order.
    pub(super) fn iter(self) -> impl Iterator<Item = (&'d str, Json<'d>)> {
        self.members().map(|(name, value)| (name, Json(value)))
    }

    /// The member names, in document order.
    pub(super) fn keys(self) -> impl Iterator<Item = &'d str> {
        self.iter().map(|(name, _)| name)
    }
}

/// Push `node` into the arena and return its index.
fn push(nodes: &mut Vec<RawNode>, node: RawNode) -> usize {
    nodes.push(node);
    nodes.len() - 1
}

/// The charge `read_raw_value` makes for each value form it reads: it runs
/// once per form, as the tree is walked, so a document is refused at the
/// first form past a limit and the walk never outruns it.
type Charge<'c, E> = &'c mut dyn FnMut() -> Result<(), E>;

/// Read one FR-106 value form from `json`, or `Ok(None)` when it is not one
/// of the six tagged shapes: a wire-reading edge (ADR-012 §9), converting
/// each value's tag to a closed form once, here. The tree is walked from an
/// explicit heap stack, so a value of any depth reads in constant native
/// stack; `charge` runs once per value form read (`observation.values`),
/// and its refusal stops the read.
///
/// # Errors
///
/// Whatever `charge` returns.
#[qsl_attrs::string_edge]
pub(super) fn read_raw_value<E>(
    json: Json<'_>,
    charge: Charge<'_, E>,
) -> Result<Option<SnapshotValue>, E> {
    /// A form whose payload is still being read.
    enum Open {
        /// `{"present": <value>}`, waiting for its payload.
        Present,
        /// `{"sequence": [...]}`, waiting for `remaining` more items.
        Sequence { remaining: usize, items: Vec<usize> },
    }
    // The arena every form is pushed into, as each completes.
    let mut nodes: Vec<RawNode> = Vec::new();
    // Each open form, outermost first.
    let mut open: Vec<Open> = Vec::new();
    // Payloads still to read: the form to read next is last.
    let mut pending: Vec<Json<'_>> = vec![json];
    // The node of the form that just completed.
    let mut finished: Option<usize> = None;
    loop {
        // Close every form that has all its payloads.
        while let Some(done) = finished.take() {
            match open.pop() {
                None => return Ok(Some(SnapshotValue::from_arena(nodes, done))),
                Some(Open::Present) => {
                    nodes.push(RawNode::Present(done));
                    finished = Some(nodes.len() - 1);
                }
                Some(Open::Sequence {
                    remaining,
                    mut items,
                }) => {
                    items.push(done);
                    if remaining == 1 {
                        nodes.push(RawNode::Sequence(items));
                        finished = Some(nodes.len() - 1);
                    } else {
                        open.push(Open::Sequence {
                            remaining: remaining - 1,
                            items,
                        });
                    }
                }
            }
        }
        let Some(next) = pending.pop() else {
            return Ok(None);
        };
        charge()?;
        let Some(object) = next.as_object() else {
            return Ok(None);
        };
        let mut members = object.iter();
        let (Some((tag, payload)), None) = (members.next(), members.next()) else {
            return Ok(None);
        };
        match tag {
            "boolean" => match payload.as_bool() {
                Some(value) => finished = Some(push(&mut nodes, RawNode::Boolean(value))),
                None => return Ok(None),
            },
            "integer" => match payload.as_str() {
                Some(spelling) => {
                    finished = Some(push(&mut nodes, RawNode::Integer(spelling.to_owned())));
                }
                None => return Ok(None),
            },
            // FR-106 line 107 spells this tag's payload `{}`, not any value
            // (SR-750 FND-013): an object with any member, or a non-object
            // payload, is not this shape at all.
            "absent" => match payload.as_object() {
                Some(empty) if empty.iter().next().is_none() => {
                    finished = Some(push(&mut nodes, RawNode::Absent));
                }
                _ => return Ok(None),
            },
            "present" => {
                open.push(Open::Present);
                pending.push(payload);
            }
            "reference" => {
                let Some(reference) = payload.as_object() else {
                    return Ok(None);
                };
                let (Some(population), Some(key)) = (
                    reference.member("population").and_then(Json::as_str),
                    reference.member("key").and_then(Json::as_str),
                ) else {
                    return Ok(None);
                };
                finished = Some(push(
                    &mut nodes,
                    RawNode::Reference(SelectedObject {
                        population: population.to_owned(),
                        key: key.to_owned(),
                    }),
                ));
            }
            "sequence" => {
                let Some(items) = payload.as_array() else {
                    return Ok(None);
                };
                let count = items.len();
                if count == 0 {
                    finished = Some(push(&mut nodes, RawNode::Sequence(Vec::new())));
                } else {
                    open.push(Open::Sequence {
                        remaining: count,
                        items: Vec::with_capacity(count),
                    });
                    // Pushed last-first, so the first item reads first.
                    let elements: Vec<Json<'_>> = items.collect();
                    pending.extend(elements.into_iter().rev());
                }
            }
            _ => return Ok(None),
        }
    }
}
