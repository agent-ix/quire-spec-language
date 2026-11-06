// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-043-AC-15, AC-16: valuation evidence on the checked-package route.
//!
//! Each row binds six facts: the declaration occurrence it values, the trace
//! position, the anchor and capture instance of the activated instance, the
//! observation owner's revision, and the checked `holds` operand. Rows that
//! agree on all six remain separate rows in admission order, so the retained
//! evidence is a sequence and never a map keyed by position and leaf. QSL
//! compares the anchor, capture instance and revision as opaque values and
//! does not authenticate the observation owner.

use qsl_package::checked_views::{ClauseOccurrence, TemporalSubjectView};
use quire_contract_model::CheckedNodeId;

use super::result::Refusal;
use super::trace::Trace;

/// One admitted `holds` valuation and the six facts it is bound to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValuationRow {
    /// The declaration occurrence the row values (FR-515 occurrence key).
    pub subject: ClauseOccurrence,
    /// The valued position: its index among the trace's admitted positions.
    pub position: usize,
    /// The activation anchor of the instance the row is evaluated for.
    pub anchor: String,
    /// The identity of that instance's immutable capture environment.
    pub capture_instance: String,
    /// The observation owner's opaque revision reference for the row.
    pub revision: String,
    /// The checked `holds` operand the row values.
    pub operand: CheckedNodeId,
    /// The Boolean value.
    pub value: bool,
}

/// The activated instance a set of rows is evaluated for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValuationInstance<'a> {
    /// The instance's activation anchor.
    pub anchor: &'a str,
    /// The identity of the instance's capture environment.
    pub capture_instance: &'a str,
}

/// Which check a refused valuation row failed (FR-043-AC-16).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValuationCause {
    /// The subject is no occurrence the declaration holds.
    OccurrenceNotDeclared,
    /// The operand is no `holds` operand of the checked formula.
    OperandNotInFormula,
    /// The position is not an admitted position of the trace.
    PositionNotAdmitted,
    /// The anchor is not the evaluated instance's anchor.
    AnchorMismatch,
    /// The capture instance is not the evaluated instance's.
    CaptureInstanceMismatch,
    /// The revision is not the evaluation's revision reference.
    RevisionMismatch,
}

/// A valuation the evaluation required and the evidence does not establish:
/// no row, or rows that disagree. It is incompleteness, never `false`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("no established valuation of the operand at position {position}")]
pub struct MissingValuation {
    /// The occurrence the valuation was required for.
    pub subject: ClauseOccurrence,
    /// The position.
    pub position: usize,
    /// The operand.
    pub operand: CheckedNodeId,
}

/// The admitted valuation rows, every one retained in admission order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValuationEvidence {
    rows: Vec<ValuationRow>,
}

impl ValuationEvidence {
    /// Admit `rows` against the checked `view`, the `trace`, the activated
    /// `instance` and the evaluation's `revision` reference. The first row
    /// that fails a check refuses the whole set, names its index in
    /// `Refusal::Valuation`, and yields no evidence.
    ///
    /// # Errors
    ///
    /// [`Refusal::Valuation`] with the failed [`ValuationCause`].
    pub fn admit(
        view: &TemporalSubjectView<'_>,
        trace: &Trace,
        instance: ValuationInstance<'_>,
        revision: &str,
        rows: Vec<ValuationRow>,
    ) -> Result<Self, Refusal> {
        for (index, row) in rows.iter().enumerate() {
            let cause = if view.arena_of(&row.subject).is_err() {
                Some(ValuationCause::OccurrenceNotDeclared)
            } else if !view.predicate_leaves().contains(&row.operand) {
                Some(ValuationCause::OperandNotInFormula)
            } else if row.position >= trace.positions.len() {
                Some(ValuationCause::PositionNotAdmitted)
            } else if row.anchor != instance.anchor {
                Some(ValuationCause::AnchorMismatch)
            } else if row.capture_instance != instance.capture_instance {
                Some(ValuationCause::CaptureInstanceMismatch)
            } else if row.revision != revision {
                Some(ValuationCause::RevisionMismatch)
            } else {
                None
            };
            if let Some(cause) = cause {
                return Err(Refusal::Valuation { cause, row: index });
            }
        }
        Ok(Self { rows })
    }

    /// Every admitted row, in admission order, duplicates included.
    pub fn rows(&self) -> &[ValuationRow] {
        &self.rows
    }

    /// The Boolean the rows establish for `operand` at `position` under
    /// `subject`: the value every such row agrees on.
    ///
    /// # Errors
    ///
    /// [`MissingValuation`] when no row values it, or the rows disagree.
    pub fn value_of(
        &self,
        subject: &ClauseOccurrence,
        position: usize,
        operand: &CheckedNodeId,
    ) -> Result<bool, MissingValuation> {
        let mut values = self
            .rows
            .iter()
            .filter(|row| {
                row.subject == *subject && row.position == position && row.operand == *operand
            })
            .map(|row| row.value);
        let missing = || MissingValuation {
            subject: subject.clone(),
            position,
            operand: operand.clone(),
        };
        let first = values.next().ok_or_else(missing)?;
        if values.all(|value| value == first) {
            Ok(first)
        } else {
            Err(missing())
        }
    }
}
