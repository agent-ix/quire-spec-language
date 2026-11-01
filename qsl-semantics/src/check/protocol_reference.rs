// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218's unit-wide rules on protocol references and the protocols they
//! name: a second reference to one target refuses, and each protocol is
//! paired with what S3 checked of it for lowering.

use quire_checked::node::NodeKind;
use std::collections::BTreeMap;

use qsl_forms::ProtocolDeclarationForm;
use quire_semantic_value::location::{Location, Origin};

use super::ir::CheckedBody;
use super::lowering::{declaration_origin, ProtocolInput};
use super::protocol_clause::CheckedProtocol;
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

/// Each protocol of `protocols` as lowering reads it, with what S3 checked
/// of it. `places[i]` is the position of the `i`-th protocol among
/// `checked`; a protocol S3 refused has none, and lowering reads none of
/// them, so one is a fault.
pub(super) fn inputs<'a>(
    protocols: &'a [ProtocolDeclarationForm],
    places: &[Option<usize>],
    checked: &'a [CheckedProtocol],
) -> Result<Vec<ProtocolInput<'a>>, CheckRefusal> {
    protocols
        .iter()
        .enumerate()
        .map(|(index, form)| {
            places
                .get(index)
                .copied()
                .flatten()
                .and_then(|place| checked.get(place))
                .map(|checked| ProtocolInput { form, checked })
                .ok_or_else(|| CheckRefusal {
                    location: Location::root(Origin::TypeDeclaration {
                        name: declaration_origin(&form.name.name),
                    }),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UncheckedProtocolFact)),
                })
        })
        .collect()
}
