// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218's unit-wide rules on protocol references and the protocols they
//! name: a second reference to one target refuses, and each protocol is
//! lowered under the keyword of its `run` tree's root control.

use std::collections::BTreeMap;

use qsl_forms::{ProtocolDeclarationForm, ProtocolNodeKind};
use quire_semantic_value::location::{Location, Origin};

use super::ir::{CheckedBody, NodeKind};
use super::lowering::{declaration_origin, ProtocolInput};
use super::refusal::{CheckCause, CheckRefusal, KeyFault, ProtocolReferenceCause};

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

/// Each protocol of `protocols` as lowering reads it. A protocol with no
/// `run` control cannot reach here (the grammar requires one), and is an
/// internal fault if it does.
pub(super) fn inputs(
    protocols: &[ProtocolDeclarationForm],
) -> Result<Vec<ProtocolInput>, CheckRefusal> {
    protocols
        .iter()
        .map(|protocol| {
            let operator = run_control(protocol).ok_or_else(|| CheckRefusal {
                location: Location::root(Origin::TypeDeclaration {
                    name: declaration_origin(&protocol.name.name),
                }),
                cause: CheckCause::InternalFault(Box::new(KeyFault::ProtocolWithoutControl(
                    protocol.name.name.clone(),
                ))),
            })?;
            Ok(ProtocolInput {
                name: protocol.name.name.clone(),
                operator,
            })
        })
        .collect()
}

/// The keyword of the `run` tree's root control: the first top-level
/// declaration that is a control.
fn run_control(protocol: &ProtocolDeclarationForm) -> Option<&'static str> {
    protocol
        .declarations
        .iter()
        .filter(|declaration| declaration.scope.is_none())
        .find_map(|declaration| match declaration.kind {
            ProtocolNodeKind::Sequence
            | ProtocolNodeKind::Choice
            | ProtocolNodeKind::Parallel
            | ProtocolNodeKind::Repeat
            | ProtocolNodeKind::Await
            | ProtocolNodeKind::Check => Some(declaration.kind.label()),
            ProtocolNodeKind::Send
            | ProtocolNodeKind::Receive
            | ProtocolNodeKind::Attempt
            | ProtocolNodeKind::Effect
            | ProtocolNodeKind::Event
            | ProtocolNodeKind::Commit
            | ProtocolNodeKind::Finish
            | ProtocolNodeKind::CompensateTemplate
            | ProtocolNodeKind::Branch
            | ProtocolNodeKind::Case => None,
        })
}
