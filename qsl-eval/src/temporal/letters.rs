// SPDX-License-Identifier: AGPL-3.0-or-later
//! The seam between the temporal evaluator and the one clause evaluator
//! (FR-107): the evaluator owns the temporal meaning of a trace, and a
//! [`TemporalLetters`] reads each atom of the clause at a position.

use qsl_foundation::UndefinedRecord;
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Incomplete, Meter};

use super::position::TemporalPosition;
use super::window::InsufficientData;

/// Why an atom has no value at a position (ADR-018 UE-2): the kernel's
/// undefined outcome, a family's `UndefinedRecord`, or a window aggregate
/// that found too few values (ADR-024 WA-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UndefinedCause {
    /// The kernel `Outcome::Undefined`, for example division by zero.
    Kernel(quire_exact::Undefined),
    /// A family's `FamilyResult::Undefined` record.
    Family(UndefinedRecord),
    /// A window aggregate held fewer values than its `min_count`.
    InsufficientData(InsufficientData),
}

/// The value of one atom at one position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomReading {
    /// The atom is true.
    True,
    /// The atom is false.
    False,
    /// The atom is undefined at the position (ADR-018 UE-1).
    Undefined(UndefinedCause),
}

/// Reads the clause's letter, the value of each atom of the clause at a
/// position (ADR-018 UE-1).
///
/// Every method charges the caller's `meter` for its own work; the evaluator
/// charges one work unit for the read itself. An exhausted meter is
/// returned as `Err(Incomplete)`, never as a reading.
pub trait TemporalLetters {
    /// The `holds` atom `atom` at the represented `position`.
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete>;

    /// The `when` condition of an `on each` activation at the represented
    /// `position`. Called only for a clause whose activation has one.
    fn activation(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete>;
}
