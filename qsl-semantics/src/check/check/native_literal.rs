// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091 "Native scalar literals": the calls `UUID("...")` and `Timestamp(n)`
//! check to a `UUID` or `Timestamp` literal.
//!
//! The two spellings are ordinary calls to S2; here the native literal
//! constructor of that name is one candidate beside the unit's declared
//! functions and tuples of the same name, and exactly one candidate is
//! required. The quoted `String` literal needs no resolution and is typed in
//! the typing loop.

use quire_exact::{Integer, Timestamp, Uuid, Value, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::{mismatch, node, Typer};
use crate::check::ir::{CheckedLiteral, Node, NodeKind};
use crate::check::refusal::{CheckCause, CheckRefusal};
use crate::check::type_form::{IntegerOutsideI128, IntegerSite};
use qsl_forms::{ExprNode, ExprRef};

/// The native types that have a call-shaped literal.
#[derive(Clone, Copy)]
pub(super) enum NativeConstructor {
    /// `UUID("...")`.
    Uuid,
    /// `Timestamp(n)`.
    Timestamp,
}

impl NativeConstructor {
    /// The constructor an unqualified call of `name` spells, if any.
    #[qsl_attrs::string_edge]
    pub(super) fn named(name: &str) -> Option<Self> {
        match name {
            "UUID" => Some(Self::Uuid),
            "Timestamp" => Some(Self::Timestamp),
            _ => None,
        }
    }
}

impl Typer<'_> {
    /// The checked literal of the call `call` of the native constructor
    /// `constructor`, written `name(arguments)`.
    pub(super) fn native_literal(
        &self,
        constructor: NativeConstructor,
        name: &str,
        call: ExprRef<'_>,
        arguments: &[qsl_forms::ExprId],
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        self.single_candidate(name, location)?;
        let [argument] = arguments else {
            return Err(mismatch(location));
        };
        let argument = call.at(*argument);
        let argument_location = location.child(0);
        match constructor {
            NativeConstructor::Uuid => {
                let ExprNode::String(text) = argument.node() else {
                    return Err(mismatch(&argument_location));
                };
                let uuid = Uuid::from_canonical_text(text).map_err(|_| CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::MalformedUuidLiteral {
                        argument: text.clone(),
                    },
                })?;
                Ok(node(
                    NodeKind::Literal(CheckedLiteral(Value::Uuid(uuid))),
                    ValueType::Uuid,
                    location,
                ))
            }
            NativeConstructor::Timestamp => {
                // `(magnitude, negated, where the magnitude literal sits)`.
                let (magnitude, negated, magnitude_location) = match argument.node() {
                    ExprNode::Integer(count) => (count, false, argument_location),
                    ExprNode::Negate(operand) => match argument.at(*operand).node() {
                        ExprNode::Integer(count) => (count, true, argument_location.child(0)),
                        _ => return Err(mismatch(&argument_location)),
                    },
                    _ => return Err(mismatch(&argument_location)),
                };
                // A count outside `i128` is refused as the positive literal
                // the argument writes, as FR-091's integer section states.
                if let Some(outside) = IntegerOutsideI128::of(IntegerSite::Literal, magnitude) {
                    return Err(CheckRefusal {
                        location: magnitude_location,
                        cause: CheckCause::IntegerOutsideI128(outside),
                    });
                }
                let count = if negated && *magnitude != Integer::from(0_i64) {
                    format!("-{magnitude}")
                } else {
                    magnitude.to_string()
                };
                let timestamp =
                    Timestamp::from_canonical_text(&count).map_err(|_| mismatch(location))?;
                Ok(node(
                    NodeKind::Literal(CheckedLiteral(Value::Timestamp(timestamp))),
                    ValueType::Timestamp,
                    location,
                ))
            }
        }
    }

    /// Refuse `ambiguous_declaration`/`ambiguous-name` when the unit also
    /// declares a function or a type form named `name`: the native
    /// constructor is a second candidate with no span.
    fn single_candidate(&self, name: &str, location: &Location) -> Result<(), CheckRefusal> {
        if let Some(index) = self.signatures().position(name) {
            return Err(CheckRefusal {
                location: location.clone(),
                cause: CheckCause::AmbiguousName {
                    name: name.to_owned(),
                    loci: vec![
                        location.clone(),
                        Location::root(Origin::Body {
                            function: name.to_owned(),
                            index,
                        }),
                    ],
                },
            });
        }
        match self.type_named(name, location) {
            Err(
                refusal @ CheckRefusal {
                    cause: CheckCause::AmbiguousName { .. },
                    ..
                },
            ) => Err(refusal),
            _ => Ok(()),
        }
    }
}
