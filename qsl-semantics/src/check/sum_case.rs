// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `SumCase` family's S3 check of `case` over a declared union
//! (FR-318, ADR-012 §16.5).
//!
//! A union is a composite declaration of the package's type environment
//! (FR-316), so the scrutinee's static type names it by its declaration key.
//! [`CaseBuilder`] is the staged builder of ADR-012 §4.2: the typer submits
//! the typed scrutinee, then each arm head in source order, then asks for
//! `finish`. It refuses at most once: `finish` reads only the recorded arm
//! heads, never an arm body, and runs QSpec FR-146's four obligations in
//! FR-146's order.

use std::collections::BTreeSet;

use quire_exact::{NodeKey, ValueType};
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment, UnionMemberDeclaration};
use quire_semantic_value::location::Location;

use super::ir::{CaseArm, NodeId, Slot};
use super::refusal::{CheckCause, CheckRefusal, KeyFault};

/// Which of QSpec FR-146's exhaustiveness obligations a `case` failed, in
/// FR-146's order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExhaustivenessObligation {
    /// Two arms name the same member of the union.
    DuplicateArm,
    /// An arm's name resolves to no member of the union, including a member
    /// of another union.
    UnknownMember,
    /// An arm binds a number of identifiers other than its member's payload
    /// arity.
    ArmArity,
    /// The arms name a strict subset of the union's members.
    MissingArm,
}

impl ExhaustivenessObligation {
    /// The obligation's FR-146 spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateArm => "duplicate-arm",
            Self::UnknownMember => "unknown-member",
            Self::ArmArity => "arm-arity",
            Self::MissingArm => "missing-arm",
        }
    }
}

/// The `SumCase` family's own refusal cause (FR-318): the one failed
/// exhaustiveness obligation of a `case`. The scrutinee, arm-body and
/// construction refusals are `ill_typed`/`type-mismatch` and share
/// [`CheckCause::IllTyped`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SumCaseCause {
    /// `undefined_expression`/`unproved-exhaustiveness`.
    UnprovedExhaustiveness {
        /// The scrutinee union's declaration identity.
        union: NodeKey,
        /// The `case`'s arm set: each arm's member name as written, in
        /// source order.
        arms: Vec<String>,
        /// The first failed obligation.
        obligation: ExhaustivenessObligation,
    },
}

impl SumCaseCause {
    /// The catalog code every variant maps to.
    pub const fn code(&self) -> qsl_foundation::diagnostic::Code {
        match self {
            Self::UnprovedExhaustiveness { .. } => {
                qsl_foundation::diagnostic::Code::UndefinedExpression
            }
        }
    }

    /// The closed cause tag.
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::UnprovedExhaustiveness { .. } => "unproved-exhaustiveness",
        }
    }
}

/// A union declaration read from the type environment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnionRef<'a> {
    /// The union's declaration identity.
    pub(crate) declaration: NodeKey,
    /// The declared name, which a qualified arm head spells as `U::m`.
    name: &'a str,
    /// The members in declared order.
    members: &'a [UnionMemberDeclaration],
}

impl<'a> UnionRef<'a> {
    /// The union a `case` scrutinee of static type `scrutinee` ranges over,
    /// or `None` when the type is no declared union.
    pub(crate) fn of(types: &'a TypeEnvironment, scrutinee: &ValueType) -> Option<Self> {
        let ValueType::Composite(declaration) = scrutinee else {
            return None;
        };
        let composite = types.composite(*declaration)?;
        let CompositeShape::Union(members) = composite.shape() else {
            return None;
        };
        Some(Self {
            declaration: *declaration,
            name: composite.name(),
            members,
        })
    }

    /// The union declared as `declaration`, or `None` when that is no union.
    pub(crate) fn declared(types: &'a TypeEnvironment, declaration: NodeKey) -> Option<Self> {
        Self::of(types, &ValueType::Composite(declaration))
    }

    /// The position of the member named exactly `member`.
    pub(crate) fn position_of(&self, member: &str) -> Option<usize> {
        self.members
            .iter()
            .position(|declared| declared.identifier().as_str() == member)
    }

    /// The position of the member an arm head spells, bare `m` or qualified
    /// `U::m`. A qualifier other than this union's name names no member.
    fn member(&self, spelled: &str) -> Option<usize> {
        let member = match spelled.rsplit_once("::") {
            Some((qualifier, member)) if qualifier == self.name => member,
            Some(_) => return None,
            None => spelled,
        };
        self.members
            .iter()
            .position(|declared| declared.identifier().as_str() == member)
    }

    /// The payload types of the member at `position`, in declared order.
    pub(crate) fn payload(&self, position: usize) -> &'a [ValueType] {
        self.members
            .get(position)
            .map_or(&[], UnionMemberDeclaration::positions)
    }
}

/// One member of a declared union, named by a construction `U::m` or
/// `U::m(a, b)` (FR-317): the `SumCase` family's side of the call-target
/// seam.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnionMember<'a> {
    declaration: NodeKey,
    /// The member's position in the union's declared member order.
    position: usize,
    /// The member's payload types, in declared position order.
    payload: &'a [ValueType],
}

impl<'a> UnionMember<'a> {
    /// The member at `position` of `union`, or `None` when it has none.
    pub(crate) fn of(union: UnionRef<'a>, position: usize) -> Option<Self> {
        Some(Self {
            declaration: union.declaration,
            position,
            payload: union.members.get(position)?.positions(),
        })
    }

    /// The type payload argument `index` is checked against.
    pub(crate) fn parameter(&self, index: usize) -> Option<&'a ValueType> {
        self.payload.get(index)
    }

    /// Admit the shape of a construction at `location`: a nullary member is
    /// written bare (or with no arguments) and a member that declares a
    /// payload is called with exactly its arity (QSpec FR-143). `supplied` is
    /// the call's argument count, `None` for a bare name.
    pub(crate) fn admit_shape(
        &self,
        supplied: Option<usize>,
        location: &Location,
    ) -> Result<(), CheckRefusal> {
        let admitted =
            supplied.map_or(self.payload.is_empty(), |count| count == self.payload.len());
        if admitted {
            Ok(())
        } else {
            Err(CheckRefusal::ill_typed(
                location,
                quire_exact::IllTypedCause::TypeMismatch,
            ))
        }
    }

    /// The construction node over its checked payload `arguments`.
    pub(crate) fn finish(self, arguments: Vec<NodeId>, location: &Location) -> super::ir::Node {
        super::ir::Node {
            kind: super::ir::NodeKind::UnionValue {
                declaration: self.declaration,
                member: self.position,
                arguments,
            },
            value_type: ValueType::Composite(self.declaration),
            location: location.clone(),
        }
    }
}

/// What an arm's head resolved to. Recording a head refuses nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArmHead {
    /// The name resolves to no member of the union.
    Unknown,
    /// The name resolves to a member, but the arm binds a different number of
    /// identifiers than the member's payload arity; the binders stay untyped.
    Arity(usize),
    /// The name resolves to a member and the binder count matches.
    Resolved(usize),
}

impl ArmHead {
    fn member(self) -> Option<usize> {
        match self {
            Self::Unknown => None,
            Self::Arity(member) | Self::Resolved(member) => Some(member),
        }
    }
}

/// The staged builder of one `case` (ADR-012 §4.2): the union, the arm
/// heads recorded so far and the arms whose bodies were checked.
pub(crate) struct CaseBuilder<'a> {
    union: UnionRef<'a>,
    scrutinee: NodeId,
    /// Each arm's name as written, in source order.
    names: Vec<String>,
    heads: Vec<ArmHead>,
    /// The checked arms, in source order.
    arms: Vec<CaseArm>,
}

impl<'a> CaseBuilder<'a> {
    /// A builder for a `case` over `union` whose scrutinee is the typed node
    /// `scrutinee`.
    pub(crate) fn new(union: UnionRef<'a>, scrutinee: NodeId) -> Self {
        Self {
            union,
            scrutinee,
            names: Vec::new(),
            heads: Vec::new(),
            arms: Vec::new(),
        }
    }

    /// Record the next arm's head: `spelled` is the member name as written
    /// and `binders` the number of identifiers the arm binds.
    pub(crate) fn head(&mut self, spelled: &str, binders: usize) -> ArmHead {
        let head = match self.union.member(spelled) {
            None => ArmHead::Unknown,
            Some(member) if self.union.payload(member).len() == binders => {
                ArmHead::Resolved(member)
            }
            Some(member) => ArmHead::Arity(member),
        };
        self.names.push(spelled.to_owned());
        self.heads.push(head);
        head
    }

    /// The payload types an arm of `member` binds positionally.
    pub(crate) fn payload(&self, member: usize) -> &'a [ValueType] {
        self.union.payload(member)
    }

    /// Record the checked arm of a resolved head.
    pub(crate) fn arm(&mut self, member: usize, binders: Vec<Slot>, body: NodeId) {
        self.arms.push(CaseArm {
            member,
            binders,
            body,
        });
    }

    /// Run the four FR-146 obligations over the recorded heads, in FR-146's
    /// order, and return the first failure; or, when none fails, the
    /// scrutinee, the union and the arms in the union's declared member
    /// order. `case` is the `case` expression's location; an arm's location
    /// is the `case`'s child `1 + arm index`.
    pub(crate) fn finish(
        self,
        case: &Location,
    ) -> Result<(NodeId, NodeKey, Vec<CaseArm>), CheckRefusal> {
        let arm_at = |index: usize| case.child(1 + index);
        let mut failure = None;
        let mut seen = BTreeSet::new();
        if let Some(index) = self
            .heads
            .iter()
            .position(|head| head.member().is_some_and(|member| !seen.insert(member)))
        {
            failure = Some((ExhaustivenessObligation::DuplicateArm, arm_at(index)));
        } else if let Some(index) = self.heads.iter().position(|head| *head == ArmHead::Unknown) {
            failure = Some((ExhaustivenessObligation::UnknownMember, arm_at(index)));
        } else if let Some(index) = self
            .heads
            .iter()
            .position(|head| matches!(head, ArmHead::Arity(_)))
        {
            failure = Some((ExhaustivenessObligation::ArmArity, arm_at(index)));
        } else if seen.len() < self.union.members.len() {
            failure = Some((ExhaustivenessObligation::MissingArm, case.clone()));
        }
        if let Some((obligation, location)) = failure {
            return Err(CheckRefusal {
                location,
                cause: CheckCause::SumCase(Box::new(SumCaseCause::UnprovedExhaustiveness {
                    union: self.union.declaration,
                    arms: self.names,
                    obligation,
                })),
            });
        }
        // Every head resolved, no member repeats and none is missing, so each
        // member has exactly one checked arm.
        let mut arms = self.arms;
        arms.sort_by_key(|arm| arm.member);
        if arms.len() != self.union.members.len() {
            return Err(fault(case, "case-arm-count"));
        }
        Ok((self.scrutinee, self.union.declaration, arms))
    }
}

/// A broken invariant of the builder, never a refusal of the input.
fn fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
            qsl_foundation::InternalFault::new("S3", invariant),
        ))),
    }
}

/// The honest refusal of a union declaration, construction or `case` that
/// reaches the FR-093 lowering, which does not yet key them: the checked
/// graph holds them, but no checked package can be built from it until the
/// union lowering (FR-319, FR-320) exists.
pub(crate) fn unlowered(location: &Location) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::UnsupportedFeature {
            loci: vec![location.clone()],
        },
    }
}

#[cfg(test)]
mod tests;
