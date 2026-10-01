// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 enumeration values at run time (ADR-011 §6.1 layer SV): an
//! admitted member as an [`EnumValue`], the checked `VariantId` ->
//! [`EnumValue`] index ([`EnumMemberIndex`]), and the FR-141 comparison
//! schedule ([`compare_enum`]). Declaration admission and node identity
//! (preimages, digests) stay in `qsl-semantics`' `value::enumeration`.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use crate::stop::{outcome_from_stop, Stop};
use quire_exact::{
    Charge, ChargePoint, ComparisonOperator, IllTyped, IllTypedCause, LimitKind, Meter, NodeKey,
    Outcome, VariantId,
};

/// An enumeration value: (declaration identity, member identity).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumValue {
    declaration: NodeKey,
    member: NodeKey,
    ordered: bool,
    position: usize,
    case: Box<str>,
}

impl EnumValue {
    /// The member `member` of the enum declaration `declaration`, at
    /// zero-based canonical-list `position`, named `case`, of an `ordered`
    /// declaration or not. Trusted: the caller has admitted the member
    /// against its declaration (`qsl-semantics`' `EnumDeclaration::
    /// admit_member` is the checked path); nothing is re-verified here.
    pub fn admitted(
        declaration: NodeKey,
        member: NodeKey,
        ordered: bool,
        position: usize,
        case: Box<str>,
    ) -> Self {
        Self {
            declaration,
            member,
            ordered,
            position,
            case,
        }
    }

    /// Declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// Member node key.
    pub fn member(&self) -> NodeKey {
        self.member
    }

    /// Zero-based FR-141 canonical-list position of the member: the same
    /// value ADR-013 O-14/OQ-D's `EnumMember::rank` carries once a caller
    /// pairs it with this member's [`Self::variant`]. Widened from
    /// `pub(crate)` (this change): an external caller building an
    /// `EnumShape`/`Value::Enum` from an admitted [`EnumValue`] -- exactly
    /// what `check::check::Typer::name` and `check::EnumBinding::shape` do
    /// inside this crate -- needs it too.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Whether the declaration is an `ordered enum`.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// The case identifier.
    pub fn case(&self) -> &str {
        &self.case
    }

    /// This member's kernel `VariantId` (ADR-013 O-14, OQ-F ruling): the same
    /// `quire.checked-semantic-node/v1` bytes as [`Self::member`], retyped.
    /// [`Self::member`]'s bytes are already the OQ-F-ruled `quire.enum-
    /// member-node/v1` preimage digest -- verified against that exact
    /// preimage at [`EnumDeclaration::admit_member`] -- so this needs no
    /// fresh computation, only the kernel's own opaque wrapper.
    pub fn variant(&self) -> VariantId {
        VariantId::from_digest(*self.member.as_bytes())
    }
}

/// ADR-013 T-6 (last sentence): the checked `VariantId` -> [`EnumValue`]
/// index, built once as `check` admits each enum member and consulted
/// wherever an evaluated kernel `Value::Enum` (a bare `VariantId` and rank,
/// ADR-013 O-14/OQ-D) needs its declaration, ordered flag, position or case
/// name back. The kernel is a leaf and carries none of this (`quire_exact::
/// value`'s own module doc); the FR-141 enum-specific `=`/ordering schedule
/// ([`compare_enum`], `value::declaration::CheckedEquality`'s `Enum`
/// schedule, and `value::expression::evaluate`'s `OrderedKind::Enums` arm)
/// resolves a `VariantId` back to its full [`EnumValue`] through this index
/// rather than the kernel ever holding one.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
///
/// The table is shared behind an `Arc`: a clone is a reference
/// count, so every checked equality over one enum can hold that enum's
/// table without copying it.
pub struct EnumMemberIndex(Arc<BTreeMap<VariantId, EnumValue>>);

impl EnumMemberIndex {
    /// Record one admitted member, keyed by its own `VariantId`. The last
    /// write for a given `VariantId` wins; two structurally identical
    /// members (same declaration, same case) always share one `VariantId`
    /// (content-addressed, ADR-013 O-04), so recording either is equivalent.
    pub fn record(&mut self, member: EnumValue) {
        Arc::make_mut(&mut self.0).insert(member.variant(), member);
    }

    /// The full checked member `variant` names, or `None` when it was never
    /// recorded -- never re-derived by any other means (R-05).
    pub fn resolve(&self, variant: VariantId) -> Option<&EnumValue> {
        self.0.get(&variant)
    }

    /// The sub-index holding only the entries named by `variants`, silently
    /// skipping any this index never recorded. `value::declaration::
    /// CheckedEquality` (SR-511 M2) uses this to retain, inside a checked
    /// `Enum`-scheduled equality node, only the compared operands' own enum
    /// declaration -- an `EnumShape::variants()` iterator -- rather than a
    /// clone of the whole package's enum-member index.
    pub fn filtered(&self, variants: impl IntoIterator<Item = VariantId>) -> Self {
        let mut filtered = Self::default();
        for variant in variants {
            if let Some(member) = self.resolve(variant) {
                filtered.record(member.clone());
            }
        }
        filtered
    }
}

/// Compare two enum values. Members of different declarations, and ordering
/// requests over an unordered declaration, are ill-typed and consume nothing.
pub fn compare_enum(
    operator: ComparisonOperator,
    left: &EnumValue,
    right: &EnumValue,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    if left.declaration != right.declaration {
        return Err(IllTyped {
            cause: IllTypedCause::DistinctEnumDeclarations,
        });
    }
    if operator.is_ordering() && !left.ordered {
        return Err(IllTyped {
            cause: IllTypedCause::UnorderedEnumOrdering,
        });
    }
    Ok(outcome_from_stop(compare(operator, left, right, meter)))
}

fn compare(
    operator: ComparisonOperator,
    left: &EnumValue,
    right: &EnumValue,
    meter: &mut Meter,
) -> Result<bool, Stop> {
    for read in 1..=2 {
        meter.charge(
            Charge::new(ChargePoint::EnumIdentityRead).size(LimitKind::ValueOccurrences, read),
        )?;
    }
    let ordering = if operator.is_ordering() {
        left.position.cmp(&right.position)
    } else {
        left.member.cmp(&right.member)
    };
    meter.charge(Charge::new(ChargePoint::EnumResultRetain).results(1))?;
    Ok(operator.holds(ordering))
}
