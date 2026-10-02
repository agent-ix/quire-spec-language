// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092 "The group order", steps 1-2: a recursion-group member's full and
//! anonymous shapes and its targets.
//!
//! The one place node keying rewrites a preimage rather than builds it: the
//! member's RFC 8785 bytes are read back through `quire-canonical`'s reader,
//! and the tree is written again through its event API with each
//! `group_reference` placeholder's ordinal left out (collected in the order
//! the bytes carry them), and, for the anonymous shape, the top-level
//! `owner` left out and `declaration` written as `null`. Both walks run on
//! an explicit heap stack. This module hashes nothing: the caller hashes the
//! shape text, so no file both reads JSON and hashes (arch-lint
//! `canonical-encoder`).

use quire_canonical::{Document, Encode, Node, NodeRef, Sink, Writer};

use super::{canonical_bytes, NodeKeyRefusal};

/// One member's shapes, as RFC 8785 text, and its targets.
pub(super) struct MemberShapes {
    /// The full shape.
    pub(super) full: String,
    /// The anonymous shape: `declaration` `null`, no `owner`.
    pub(super) anonymous: String,
    /// Each placeholder's target position, in RFC 8785 order.
    pub(super) targets: Vec<usize>,
}

/// The shapes of the member whose preimage's RFC 8785 bytes are
/// `canonical`, its placeholders' ordinals naming positions below `count`.
pub(super) fn member_shapes(
    canonical: &[u8],
    count: usize,
) -> Result<MemberShapes, NodeKeyRefusal> {
    // These are this module's own canonical bytes, already in memory and
    // bounded by the encoder that wrote them, so the read sets no byte
    // limit of its own.
    let document =
        quire_canonical::read(canonical, u64::MAX).map_err(|error| NodeKeyRefusal::Encode {
            reason: error.to_string(),
        })?;
    let targets = targets(&document, count)?;
    let full = canonical_text(&Shape {
        document: &document,
        anonymous: false,
    })?;
    let anonymous = canonical_text(&Shape {
        document: &document,
        anonymous: true,
    })?;
    Ok(MemberShapes {
        full,
        anonymous,
        targets,
    })
}

/// Whether `node` is a `{term: "group_reference", ...}` placeholder.
fn is_placeholder(node: NodeRef<'_>) -> bool {
    node.get("term")
        .is_some_and(|term| matches!(term.node(), Node::String("group_reference")))
}

/// Every placeholder's ordinal, in RFC 8785 order: the canonical bytes
/// already carry their members in that order, so a depth-first walk in
/// document order meets the placeholders in it. Every ordinal is a member's
/// position, below `count`.
fn targets(document: &Document, count: usize) -> Result<Vec<usize>, NodeKeyRefusal> {
    let mut targets = Vec::new();
    let mut pending = vec![document.root()];
    while let Some(node) = pending.pop() {
        if is_placeholder(node) {
            let target = node
                .get("ordinal")
                .and_then(|ordinal| match ordinal.node() {
                    Node::Number(number) => number.text().parse::<usize>().ok(),
                    _ => None,
                })
                .filter(|target| *target < count)
                .ok_or(NodeKeyRefusal::InvalidGroup)?;
            targets.push(target);
            continue;
        }
        let children: Vec<NodeRef<'_>> = match node.node() {
            Node::Array(items) => items.collect(),
            Node::Object(members) => members.map(|(_, value)| value).collect(),
            Node::Null | Node::Bool(_) | Node::Number(_) | Node::String(_) => continue,
        };
        pending.extend(children.into_iter().rev());
    }
    Ok(targets)
}

/// A member's shape: its preimage tree with every placeholder's `ordinal`
/// left out and, when `anonymous`, the top-level `owner` left out and
/// `declaration` written as `null`.
struct Shape<'d> {
    document: &'d Document,
    anonymous: bool,
}

impl Encode for Shape<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        enum Task<'d> {
            Value(NodeRef<'d>),
            Name(&'d str),
            Null,
            EndArray,
            EndObject,
        }
        let root = self.document.root();
        let mut tasks = vec![Task::Value(root)];
        let mut at_root = true;
        while let Some(task) = tasks.pop() {
            match task {
                Task::Value(node) => {
                    let top = std::mem::replace(&mut at_root, false);
                    match node.node() {
                        Node::Object(members) => {
                            writer.begin_object()?;
                            tasks.push(Task::EndObject);
                            let placeholder = is_placeholder(node);
                            let members: Vec<(&str, NodeRef<'_>)> = members.collect();
                            for (name, value) in members.into_iter().rev() {
                                if placeholder && name == "ordinal" {
                                    continue;
                                }
                                if top && self.anonymous && name == "owner" {
                                    continue;
                                }
                                if top && self.anonymous && name == "declaration" {
                                    tasks.push(Task::Null);
                                } else {
                                    tasks.push(Task::Value(value));
                                }
                                tasks.push(Task::Name(name));
                            }
                        }
                        Node::Array(items) => {
                            writer.begin_array()?;
                            tasks.push(Task::EndArray);
                            let items: Vec<NodeRef<'_>> = items.collect();
                            tasks.extend(items.into_iter().rev().map(Task::Value));
                        }
                        Node::Null | Node::Bool(_) | Node::Number(_) | Node::String(_) => {
                            node.encode_into(writer)?;
                        }
                    }
                }
                Task::Name(name) => writer.name(name)?,
                Task::Null => writer.null()?,
                Task::EndArray => writer.end_array()?,
                Task::EndObject => writer.end_object()?,
            }
        }
        Ok(())
    }
}

/// `value`'s RFC 8785 text: [`canonical_bytes`], which are UTF-8.
fn canonical_text(value: &impl Encode) -> Result<String, NodeKeyRefusal> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| NodeKeyRefusal::Encode {
        reason: error.to_string(),
    })
}
