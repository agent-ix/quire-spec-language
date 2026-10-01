// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 enumerations at run time (ADR-011 §6.1 layer SV): an admitted
//! declaration's structure ([`EnumDeclaration`]), an admitted member as an
//! [`EnumValue`], the checked `VariantId` -> [`EnumValue`] index
//! ([`EnumMemberIndex`]), and the FR-141 comparison schedule
//! ([`compare_enum`]). Node identity (preimages, digests, owner join and
//! stale-key refusal) stays in `qsl-semantics`' `value::enumeration`.

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::semantic_node::{InvalidSemanticGraph, SemanticGraphCause};
use crate::stop::{outcome_from_stop, Stop};
use quire_exact::{
    is_identifier, Charge, ChargePoint, ComparisonOperator, IllTyped, IllTypedCause, LimitKind,
    Meter, NodeKey, Outcome, VariantId,
};

/// Whether `members` is a schema member list: nonempty, distinct
/// identifiers. The one statement of the rule, shared by
/// [`EnumDeclaration::new`] and `qsl-semantics`' preimage readers.
pub fn is_member_list(members: &[String]) -> bool {
    let distinct: BTreeSet<_> = members.iter().collect();
    !members.is_empty()
        && distinct.len() == members.len()
        && members.iter().all(|case| is_identifier(case))
}

/// An admitted enum declaration node: its key, its ordering and its member
/// cases. This type checks the structural rules a declaration is compared
/// under. It does not mint or verify the key: SV takes the declaration key
/// the caller admitted as given (only the compiler's `check` mints and
/// verifies node keys, ADR-011 §6.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDeclaration {
    key: NodeKey,
    ordered: bool,
    members: Vec<String>,
}

impl EnumDeclaration {
    /// The declaration node `key` with its member cases, in declaration
    /// order for an `ordered enum` and sorted by case name otherwise
    /// (FR-141).
    ///
    /// An empty, repeated or non-identifier member list is
    /// `NonCanonicalPreimage`; an unordered declaration whose cases are not
    /// sorted is `UnsortedUnorderedMembers`. `key` is taken as given: it is
    /// the caller's admitted declaration key, not checked against the
    /// members.
    pub fn new(
        key: NodeKey,
        ordered: bool,
        members: Vec<String>,
    ) -> Result<Self, InvalidSemanticGraph> {
        if !is_member_list(&members) {
            return Err(InvalidSemanticGraph {
                cause: SemanticGraphCause::NonCanonicalPreimage,
            });
        }
        if !ordered && !members.is_sorted() {
            return Err(InvalidSemanticGraph {
                cause: SemanticGraphCause::UnsortedUnorderedMembers,
            });
        }
        Ok(Self {
            key,
            ordered,
            members,
        })
    }

    /// The declaration node key.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// Whether the declaration selects ordered semantics.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// Declaration-ordered (or case-sorted, if unordered) member cases.
    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// The value of member `case`, whose admitted member node key is
    /// `member`. A case outside the declaration is `UndeclaredCase`.
    ///
    /// The position and the ordered flag come from this declaration. The
    /// member key is taken as given: it is the caller's admitted
    /// `quire.enum-member-node/v1` key for `case`, and nothing here checks
    /// it against the case (SV mints and verifies no key). A wrong key gives
    /// a value whose `=` (member keys) and ordering (positions) disagree.
    pub fn member(&self, case: &str, member: NodeKey) -> Result<EnumValue, InvalidSemanticGraph> {
        let position = self
            .members
            .iter()
            .position(|declared| declared == case)
            .ok_or(InvalidSemanticGraph {
                cause: SemanticGraphCause::UndeclaredCase,
            })?;
        Ok(EnumValue {
            declaration: self.key,
            member,
            ordered: self.ordered,
            position,
            case: case.into(),
        })
    }
}

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
    /// `quire.checked-semantic-node/v1` bytes as [`Self::member`], retyped,
    /// with no fresh computation. It is the OQ-F `quire.enum-member-node/v1`
    /// identity exactly when the member key the value was built with is:
    /// `qsl-semantics`' `AdmittedEnumDeclaration::admit_member` verifies
    /// that against the preimage, while [`EnumDeclaration::member`] takes
    /// the key as given.
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
/// ([`compare_enum`], `declaration::CheckedEquality`'s `Enum`
/// schedule, and `value::expression::evaluate`'s `OrderedKind::Enums` arm)
/// resolves a `VariantId` back to its full [`EnumValue`] through this index
/// rather than the kernel ever holding one.
///
/// The table is shared behind an `Arc`: a clone is a reference
/// count, so every checked equality over one enum can hold that enum's
/// table without copying it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnumMemberIndex(Arc<BTreeMap<VariantId, EnumValue>>);

impl EnumMemberIndex {
    /// Record one admitted member, keyed by its own `VariantId`
    /// ([`EnumValue::variant`], its member key retyped). The last write for
    /// a given `VariantId` wins. When member keys are the verified
    /// content-addressed identities (ADR-013 O-04), two structurally
    /// identical members (same declaration, same case) share one
    /// `VariantId`, so recording either is equivalent; with keys taken as
    /// given, the index is keyed by whatever key the caller admitted.
    pub fn record(&mut self, member: EnumValue) {
        Arc::make_mut(&mut self.0).insert(member.variant(), member);
    }

    /// The full checked member `variant` names, or `None` when it was never
    /// recorded -- never re-derived by any other means (R-05).
    pub fn resolve(&self, variant: VariantId) -> Option<&EnumValue> {
        self.0.get(&variant)
    }

    /// The sub-index holding only the entries named by `variants`, silently
    /// skipping any this index never recorded. `declaration::
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn key(byte: u8) -> NodeKey {
        NodeKey::from_digest([byte; 32])
    }

    fn cases(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| String::from(*name)).collect()
    }

    fn refused(cause: SemanticGraphCause) -> Result<EnumDeclaration, InvalidSemanticGraph> {
        Err(InvalidSemanticGraph { cause })
    }

    #[test]
    fn a_declaration_refuses_a_malformed_member_list_before_an_unsorted_one() {
        for members in [
            vec![],
            cases(&["Ready", "Ready"]),
            cases(&["Ready", "9done"]),
            cases(&["b", "a", "a"]),
        ] {
            assert_eq!(
                EnumDeclaration::new(key(1), false, members),
                refused(SemanticGraphCause::NonCanonicalPreimage)
            );
        }
        assert_eq!(
            EnumDeclaration::new(key(1), false, cases(&["Ready", "Done"])),
            refused(SemanticGraphCause::UnsortedUnorderedMembers)
        );
        let ordered = EnumDeclaration::new(key(1), true, cases(&["Ready", "Done"])).unwrap();
        assert!(ordered.is_ordered());
        assert_eq!(ordered.key(), key(1));
        assert_eq!(ordered.members(), cases(&["Ready", "Done"]).as_slice());
    }

    #[test]
    fn a_member_carries_its_declaration_position_and_case() {
        let status = EnumDeclaration::new(key(1), true, cases(&["Ready", "Done"])).unwrap();
        let done = status.member("Done", key(3)).unwrap();
        assert_eq!(done.declaration(), key(1));
        assert_eq!(done.member(), key(3));
        assert_eq!(done.position(), 1);
        assert!(done.is_ordered());
        assert_eq!(done.case(), "Done");
        assert_eq!(
            status.member("Closed", key(4)).unwrap_err().cause,
            SemanticGraphCause::UndeclaredCase
        );
    }
}
