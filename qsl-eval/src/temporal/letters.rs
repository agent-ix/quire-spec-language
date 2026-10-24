// SPDX-License-Identifier: AGPL-3.0-or-later
//! The seam between the temporal evaluator and the one clause evaluator
//! (FR-107): the evaluator owns the temporal meaning of a trace, and a
//! [`TemporalLetters`] reads each atom of the clause at a position.

use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Incomplete, Meter};
use quire_semantic_value::catalog::{InternalFault, UndefinedRecord};

use super::position::TemporalPosition;
use super::window::{InsufficientData, WindowLimitReached, WindowRefusal};

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

/// Why a letter cannot give a reading: the run stops, with no truth value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LetterStop {
    /// The work budget ran out (ADR-014 TR-5).
    Incomplete(Incomplete),
    /// A window aggregate outgrew `max_window_values` (ADR-018 V-7).
    WindowLimit(WindowLimitReached),
    /// A window cannot be read on the trace.
    Window(WindowRefusal),
    /// An invariant of the letter broke.
    Fault(InternalFault),
}

impl From<Incomplete> for LetterStop {
    fn from(incomplete: Incomplete) -> Self {
        Self::Incomplete(incomplete)
    }
}

impl From<WindowRefusal> for LetterStop {
    fn from(refusal: WindowRefusal) -> Self {
        Self::Window(refusal)
    }
}

impl From<InternalFault> for LetterStop {
    fn from(fault: InternalFault) -> Self {
        Self::Fault(fault)
    }
}

/// Reads the clause's letter, the value of each atom of the clause at a
/// position (ADR-018 UE-1).
///
/// Every method charges the caller's `meter` for its own work; the evaluator
/// charges one work unit for the read itself. An exhausted meter is
/// returned as `Err(LetterStop::Incomplete)`, never as a reading.
pub trait TemporalLetters {
    /// The `holds` atom `atom` at the represented `position`.
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop>;

    /// The `when` condition of an `on each` activation at the represented
    /// `position`. Called only for a clause whose activation has one.
    fn activation(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop>;

    /// Whether the step into the represented `position` counts for interval
    /// distance (ADR-027 PB-3, QSpec FR-431). Every step of a model subject
    /// does, which is ADR-018's reading, so a letter of a protocol subject
    /// alone answers otherwise.
    fn counts_step(&self, _position: TemporalPosition) -> bool {
        true
    }

    /// Whether the step that closes a lasso's loop, from its last position
    /// back to its entry, counts. Asked only of a lasso.
    fn counts_loop_edge(&self) -> bool {
        true
    }
}
