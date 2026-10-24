// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-155 "Outputs": the locations a transition reads and writes, its
//! footprint, and the independence of two transitions that follows from
//! footprints that do not meet.
//!
//! The derivation of a model transition's footprint and its enforcement are
//! [`model`](super::model)'s; this module holds the vocabulary the engine
//! shares with every system that offers partial-order reduction.

use std::collections::BTreeSet;

use qsl_semantics::model::key::DeclarationKey;
use quire_exact::ObjectReference;

use crate::simulation::protocol::ProtocolLocation;

/// One location: a model location (QSpec FR-383 "Locations"), or a
/// protocol location (ADR-027 §5).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Location {
    /// A field of one object.
    Field {
        /// The object.
        object: ObjectReference,
        /// The field's declaration key.
        field: DeclarationKey,
    },
    /// The field on any object.
    AnyField {
        /// The field's declaration key.
        field: DeclarationKey,
    },
    /// Whether one object is a member of a population.
    Membership {
        /// The population's declaration key.
        population: DeclarationKey,
        /// The object.
        object: ObjectReference,
    },
    /// Whether any object is a member of a population.
    AnyMembership {
        /// The population's declaration key.
        population: DeclarationKey,
    },
    /// A location of a protocol state (ADR-027 §5, FR-216).
    Protocol(ProtocolLocation),
}

impl Location {
    /// Whether a write to this location can change what a read of `read`
    /// observes, by the cells each location stands for. A written membership
    /// (a creation or a deletion) changes the membership cell and every field
    /// cell of its object, so it meets both. A written field changes only its
    /// own field cell and never whether an object is a member, so a field
    /// location written does not meet a membership location read.
    pub fn affects(&self, read: &Self) -> bool {
        let field_write = matches!(self, Self::Field { .. } | Self::AnyField { .. });
        let membership_read = matches!(read, Self::Membership { .. } | Self::AnyMembership { .. });
        !(field_write && membership_read) && self.meets(read)
    }

    /// Whether the two locations share a cell of the state.
    ///
    /// Two field locations meet when they name the same field and, for two
    /// resolved objects, the same object. Two membership locations meet when
    /// they name the same population and, for two resolved objects, the same
    /// object. A membership meets a field of the object it is about. A
    /// location does not carry the member type of its population, so a
    /// membership of an unresolved object, or of any object, is taken to meet
    /// every field location that does not name a different object: a
    /// dependence the member types would exclude is kept, never an
    /// independence they would deny. The cost is precision: a creation or a
    /// deletion of any object depends on every transition that reads or
    /// writes a field, even of an unrelated type.
    ///
    /// A protocol location meets only a protocol location, as
    /// [`ProtocolLocation::meets`] states.
    pub fn meets(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Protocol(location), Self::Protocol(other)) => location.meets(other),
            (Self::Protocol(_), _) | (_, Self::Protocol(_)) => false,
            (
                Self::Field { object, field },
                Self::Field {
                    object: other_object,
                    field: other_field,
                },
            ) => object == other_object && field == other_field,
            (Self::Field { field, .. }, Self::AnyField { field: other_field })
            | (
                Self::AnyField { field },
                Self::Field {
                    field: other_field, ..
                },
            )
            | (Self::AnyField { field }, Self::AnyField { field: other_field }) => {
                field == other_field
            }
            (
                Self::Membership { population, object },
                Self::Membership {
                    population: other_population,
                    object: other_object,
                },
            ) => population == other_population && object == other_object,
            (Self::Membership { population, .. }, Self::AnyMembership { population: other })
            | (
                Self::AnyMembership { population },
                Self::Membership {
                    population: other, ..
                },
            )
            | (Self::AnyMembership { population }, Self::AnyMembership { population: other }) => {
                population == other
            }
            (Self::Membership { object, .. }, Self::Field { object: holder, .. })
            | (Self::Field { object: holder, .. }, Self::Membership { object, .. }) => {
                object == holder
            }
            (Self::Membership { .. }, Self::AnyField { .. })
            | (Self::AnyField { .. }, Self::Membership { .. })
            | (Self::AnyMembership { .. }, Self::Field { .. } | Self::AnyField { .. })
            | (Self::Field { .. } | Self::AnyField { .. }, Self::AnyMembership { .. }) => true,
        }
    }
}

/// What one transition reads, writes and needs to be enabled.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Footprint {
    /// `R(t)`: the locations the transition's enabledness and post-state
    /// read.
    pub reads: BTreeSet<Location>,
    /// `W(t)`: the locations the transition's frame lets it write.
    pub writes: BTreeSet<Location>,
    /// The locations the transition's enabledness reads: all of `reads` for a
    /// model transition.
    pub enabling: BTreeSet<Location>,
}

/// Whether some location of `writes` affects some location of `reads`.
pub fn any_affects(writes: &BTreeSet<Location>, reads: &BTreeSet<Location>) -> bool {
    writes
        .iter()
        .any(|written| reads.iter().any(|read| written.affects(read)))
}

/// Whether some location of `left` meets some location of `right`.
fn any_meet(left: &BTreeSet<Location>, right: &BTreeSet<Location>) -> bool {
    left.iter()
        .any(|written| right.iter().any(|other| written.meets(other)))
}

/// Whether `t` and `u` are independent (FR-155 "Independence"): neither
/// writes a location the other reads or writes.
pub fn independent(t: &Footprint, u: &Footprint) -> bool {
    !(any_affects(&t.writes, &u.reads)
        || any_meet(&t.writes, &u.writes)
        || any_affects(&u.writes, &t.reads))
}
