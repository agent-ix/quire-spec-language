// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-249 (QSpec FR-419): S3's check of the `release S.t on M::o;`
//! declarations of a unit.
//!
//! A declaration names a task set of the unit, a task of that set and an
//! operation of a model. Each name that does not resolve is refused
//! `missing_declaration`/`missing-name` (an operation of several declaring
//! types is `ambiguous_declaration`/`ambiguous-name`), every one reported. An
//! operation may release several tasks, each at most once: a second
//! declaration of one task on one operation is refused
//! `conflicting_declaration`/`duplicate-declaration`. A checked declaration is
//! recorded as a [`Release`] on the unit's [`CheckedTaskSets`], the record
//! QSL FR-249's lowering reads. This stage lowers nothing and settles no
//! `StopwatchRequired`.

use qsl_foundation::Span;

use super::{CheckedTaskSets, TaskSetDeclaration};
use crate::check::probabilistic::{ProbabilisticInput, ProbabilisticOperation, Unresolved};
use crate::check::refusal::TemporalCause;

/// A checked `release S.t on M::o;`: each step of the operation releases one
/// job of the task (QSL FR-249).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Release {
    task_set: String,
    task: String,
    operation: ProbabilisticOperation,
    span: Span,
}

impl Release {
    /// The task set the released task belongs to.
    pub fn task_set(&self) -> &str {
        &self.task_set
    }

    /// The released task.
    pub fn task(&self) -> &str {
        &self.task
    }

    /// The operation whose every step releases the task.
    pub fn operation(&self) -> &ProbabilisticOperation {
        &self.operation
    }

    /// The span of the declaration.
    pub fn span(&self) -> Span {
        self.span
    }
}

/// Check the unit's `release` declarations against its task sets and the
/// operations `input` resolves, recording each valid one in `model`.
pub(in crate::check) fn check_releases(
    declarations: &[TaskSetDeclaration],
    input: &ProbabilisticInput<'_>,
    model: &mut CheckedTaskSets,
) -> Vec<TemporalCause> {
    let mut causes = Vec::new();
    for declaration in declarations {
        let TaskSetDeclaration::Release(form) = declaration else {
            continue;
        };
        // A task set name resolves to the first task set of the name, as a
        // claim's does.
        let set = declarations.iter().find_map(|candidate| match candidate {
            TaskSetDeclaration::Taskset(set) if set.name.name == form.task_set.name => Some(set),
            _ => None,
        });
        let mut resolved = true;
        match set {
            None => {
                resolved = false;
                causes.push(TemporalCause::UnknownReleaseTaskSet {
                    name: form.task_set.name.clone(),
                    operation: form.operation.name.clone(),
                    span: form.task_set.span,
                });
            }
            Some(set) => {
                if !set
                    .tasks
                    .iter()
                    .any(|task| task.name.name == form.task.name)
                {
                    resolved = false;
                    causes.push(TemporalCause::UnknownReleaseTask {
                        task_set: form.task_set.name.clone(),
                        task: form.task.name.clone(),
                        span: form.task.span,
                    });
                }
            }
        }
        let operation = match input.operation(&form.operation, None) {
            Ok((operation, _)) => operation,
            Err(fault) => {
                causes.push(match fault {
                    Unresolved::Missing => TemporalCause::UnresolvedReleaseOperation {
                        name: form.operation.name.clone(),
                        span: form.operation.span,
                    },
                    Unresolved::Ambiguous => TemporalCause::AmbiguousReleaseOperation {
                        name: form.operation.name.clone(),
                        span: form.operation.span,
                    },
                });
                continue;
            }
        };
        if !resolved {
            continue;
        }
        let first = model.releases.iter().find(|earlier| {
            earlier.operation.key() == operation.key()
                && earlier.task_set == form.task_set.name
                && earlier.task == form.task.name
        });
        if let Some(first) = first {
            causes.push(TemporalCause::DuplicateRelease {
                task_set: form.task_set.name.clone(),
                task: form.task.name.clone(),
                operation: form.operation.name.clone(),
                first: first.span,
                span: form.span,
            });
            continue;
        }
        model.releases.push(Release {
            task_set: form.task_set.name.clone(),
            task: form.task.name.clone(),
            operation,
            span: form.span,
        });
    }
    causes
}
