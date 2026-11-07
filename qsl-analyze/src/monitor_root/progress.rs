// SPDX-License-Identifier: AGPL-3.0-or-later
//! Component checks for independently admitted observation progress.
//!
//! These checks are not joint authority admission. Original node and claim
//! occurrence checks must also succeed before an admitted context is issued.

use qsl_semantics::check::{ModelTimeSource, TimeSourceKind};
use quire_observation::authority::{clock, progress, SubjectSelection};
use quire_observation::Identity;

/// An independently selected progress field that does not match.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(super) enum SelectionRefusal {
    #[error("progress belongs to another selected subject")]
    Subject,
    #[error("progress belongs to another selected clock or revision")]
    Clock,
    #[error("progress row names another observation source")]
    Source,
    #[error("progress row names another model")]
    Model,
    #[error("progress row names another time unit")]
    Unit,
    #[error("progress row names another time source")]
    TimeSource,
}

/// Check independent observation selections against the owner's actual value.
/// This borrows the original value; it never reconstructs progress authority.
pub(super) fn check_observation_selection(
    association: &progress::AssociationRef<'_>,
    subject: &SubjectSelection,
    clock: &clock::Selection,
) -> Result<(), SelectionRefusal> {
    if association.scope_identity() != subject.scope_identity.as_str()
        || association.population_identity() != subject.population_identity.as_str()
    {
        return Err(SelectionRefusal::Subject);
    }
    if association.clock_identity() != clock.clock_identity().as_str()
        || association.clock_revision() != clock.clock_revision().as_str()
    {
        return Err(SelectionRefusal::Clock);
    }
    Ok(())
}

/// Check only the non-node time-source fields of an asserted row against the
/// original checker facts. A successful component comparison does not admit
/// the row's structural node keys or its claim occurrence.
pub(super) fn check_time_source_fields(
    source: &ModelTimeSource,
    row: &progress::AssociationRow,
    observation_source: &Identity,
    clock: &clock::Selection,
) -> Result<(), SelectionRefusal> {
    if row.observation_source != observation_source.as_str() {
        return Err(SelectionRefusal::Source);
    }
    if row.observation_clock != clock.clock_identity().as_str()
        || row.observation_revision != clock.clock_revision().as_str()
    {
        return Err(SelectionRefusal::Clock);
    }
    if row.model != source.model {
        return Err(SelectionRefusal::Model);
    }
    if row.unit != source.unit {
        return Err(SelectionRefusal::Unit);
    }
    match (&source.source, &row.source) {
        (TimeSourceKind::Dense, progress::TimeSource::Dense) => Ok(()),
        (
            TimeSourceKind::Tick { operation, period },
            progress::TimeSource::Tick {
                operation: selected_operation,
                period_numerator,
                period_denominator,
                ..
            },
        ) if operation == selected_operation
            && period.numerator().to_string() == *period_numerator
            && period.denominator().to_string() == *period_denominator =>
        {
            Ok(())
        }
        _ => Err(SelectionRefusal::TimeSource),
    }
}

#[cfg(test)]
mod tests;
