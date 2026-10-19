// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218's unit-wide rules on protocol references and the protocols they
//! name: a second reference to one target refuses, and each protocol is
//! read for the root of its `run` tree, whose keyword is its node's
//! `profile_operator`.

use std::collections::BTreeMap;

use qsl_forms::{ProtocolDeclarationForm, ProtocolNodeKind};
use quire_semantic_value::location::Location;

use super::ir::{CheckedBody, NodeKind};
use super::lowering::ProtocolInput;
use super::refusal::{CheckCause, CheckRefusal, ProtocolReferenceCause};

/// The refusal of every reference to a target already referred to earlier in
/// the unit, at the later reference, naming the first. `bodies` are the
/// unit's checked bodies in declaration order; a body's references are read
/// in the order it evaluates them.
///
/// The carrier is keyed by its body alone (`{term: "reference", target}`)
/// and IR admits one `expression` occurrence per carrier, so two references
/// to one target cannot both be written.
pub(super) fn repeated<'b>(bodies: impl IntoIterator<Item = &'b CheckedBody>) -> Vec<CheckRefusal> {
    let mut first: BTreeMap<usize, Location> = BTreeMap::new();
    let mut refusals = Vec::new();
    for body in bodies {
        for node in body.root().descendants() {
            let NodeKind::ProtocolReference { protocol } = node.kind() else {
                continue;
            };
            match first.get(protocol) {
                None => {
                    first.insert(*protocol, node.location().clone());
                }
                Some(earlier) => refusals.push(CheckRefusal {
                    location: node.location().clone(),
                    cause: CheckCause::ProtocolReference(Box::new(
                        ProtocolReferenceCause::Repeated {
                            first: earlier.clone(),
                        },
                    )),
                }),
            }
        }
    }
    refusals
}

/// Each protocol of `protocols` as lowering reads it.
pub(super) fn inputs(protocols: &[ProtocolDeclarationForm]) -> Vec<ProtocolInput> {
    protocols
        .iter()
        .map(|protocol| {
            let root = run_root(protocol);
            ProtocolInput {
                name: protocol.name.name.clone(),
                root: root.label(),
                operator: operator(root),
            }
        })
        .collect()
}

/// The kind of the `run` tree's root: the first top-level declaration that
/// is neither the `finish` nor a `compensate` template. The grammar's
/// `Control` is a structural control, an event node or a `commit`.
fn run_root(protocol: &ProtocolDeclarationForm) -> ProtocolNodeKind {
    protocol
        .declarations
        .iter()
        .filter(|declaration| declaration.scope.is_none())
        .map(|declaration| declaration.kind)
        .find(|kind| {
            !matches!(
                kind,
                ProtocolNodeKind::Finish | ProtocolNodeKind::CompensateTemplate
            )
        })
        .unwrap_or(ProtocolNodeKind::Finish)
}

/// The `profile_operator` a root names: one of FR-052's bounded control
/// constructs. An `await`, a `check`, an event node and a `commit` name
/// none, and QSpec FR-322-AC-22 defines no other.
fn operator(root: ProtocolNodeKind) -> Option<&'static str> {
    match root {
        ProtocolNodeKind::Sequence
        | ProtocolNodeKind::Choice
        | ProtocolNodeKind::Parallel
        | ProtocolNodeKind::Repeat => Some(root.label()),
        ProtocolNodeKind::Await
        | ProtocolNodeKind::Check
        | ProtocolNodeKind::Fence
        | ProtocolNodeKind::Send
        | ProtocolNodeKind::Receive
        | ProtocolNodeKind::Attempt
        | ProtocolNodeKind::Effect
        | ProtocolNodeKind::Event
        | ProtocolNodeKind::Commit
        | ProtocolNodeKind::Finish
        | ProtocolNodeKind::CompensateTemplate
        | ProtocolNodeKind::Branch
        | ProtocolNodeKind::Case => None,
    }
}
