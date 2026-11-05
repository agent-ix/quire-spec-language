// SPDX-License-Identifier: AGPL-3.0-or-later
//! The seam between the temporal evaluator and the one clause evaluator
//! (FR-107): the evaluator owns the temporal meaning of a trace, and a
//! [`TemporalLetters`] reads each atom of the clause at a position.

use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Incomplete, Meter};
use quire_semantic_value::catalog::{InternalFault, UndefinedRecord};

use super::finite_window::Valuation;
use super::position::TemporalPosition;
use super::window::{InsufficientData, WindowLimitReached, WindowRefusal};

/// What reading the captures of an activation gave (FR-337 "Capture
/// observation").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureReading {
    /// Every capture has a value for the activation: the clause has none, or
    /// each initializer is defined.
    Fixed,
    /// The capture `capture` has no value at the activation: its initializer
    /// is undefined.
    Undefined {
        /// The capture's name.
        capture: String,
        /// Why the initializer is undefined.
        cause: UndefinedCause,
    },
}

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
    /// The S6a evaluation of a condition at the requested position was a
    /// refusal (FR-327 "Refused condition"). The letter keeps the refusal
    /// whole; the evaluator stops reading at that position and returns
    /// [`TemporalRefusal::ConditionRefused`](super::TemporalRefusal) there,
    /// unless an activation before it evaluates false or an atom before it
    /// is undefined.
    Refused,
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

    /// The valuation of the `holds` atom `atom` at the represented
    /// `position` under the finite-window profile (FR-501 "Verdicts"), which
    /// distinguishes a valuation absent from the observation and two
    /// differing admitted valuations from a defined one. The default reads
    /// [`Self::atom`], so letters that never miss a valuation need not
    /// override it.
    fn valuation(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<Valuation, LetterStop> {
        self.atom(atom, position, meter).map(Valuation::from)
    }

    /// Read an atom while exposing one charge seam per aggregate window
    /// position. Ordinary atoms use `valuation` without calling the seam.
    fn valuation_with_window_visits(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
        _visit: &mut dyn FnMut(&mut Meter) -> Result<(), LetterStop>,
    ) -> Result<Valuation, LetterStop> {
        self.valuation(atom, position, meter)
    }

    /// The atom `atom` stands for under the finite-window profile: nodes that
    /// are the same atom (the same `holds` condition) return one value, a node
    /// index, and share one valuation at a position (QSpec FR-160). The
    /// default makes every node its own atom.
    fn atom_class(&self, atom: FormulaNodeId) -> usize {
        atom.index()
    }

    /// The `when` condition of an `on each` activation at the represented
    /// `position`. Called only for a clause whose activation has one.
    fn activation(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop>;

    /// FR-337 "Capture observation": fix the clause's captures, in source
    /// order, at the represented `position`, where an instance of the clause
    /// activates: the evaluator calls it at position 0 of an `on origin`
    /// clause and at each position an `on each` clause activates (where its
    /// guard, when it has one, is true), once per activation. A clause with
    /// no capture is never asked. The default fails closed: letters that do
    /// not read captures stop with a fault rather than treat a capture as
    /// defined.
    fn captures(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        Err(LetterStop::Fault(InternalFault::new(
            "temporal.letters",
            "letters-of-a-captured-clause-read-its-captures",
        )))
    }

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
