// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 Inputs and FR-338 Inputs: the item an EN-1 run decides and the
//! core's closure checker checks, and the canonical safety translation of
//! each of its instances (ADR-018 LA-2).
//!
//! A TP-1 clause binds one instance per object of its `over` population
//! (FR-125); each instance is one concrete binding, so it has its own key
//! context and its own translation. The deadlock-freedom item has one
//! instance with the binding `[]`.
//!
//! A clause's key contexts name its identity, which the item does not carry:
//! [`ClauseIdentity::resolve`] reads it from the request's package
//! (FR-337-AC-5), so no caller names an occurrence or an `over` node.

use qsl_eval::automaton::{
    ContextBinding, SafetyTranslation, StateContext, TranslationRefusal, MAX_ATOMS,
};
use qsl_eval::model_subject::{ModelLetters, ModelSubject};
use qsl_semantics::check::CheckedTemporalClause;
use quire_exact::{Cancel, ObjectReference, Value};

use crate::model_check::identity::{ClauseIdentity, ClauseRootRefusal};

/// What an EN-1 run decides (FR-126 Inputs) and a certificate request
/// checks (FR-338 Inputs).
#[derive(Clone, Copy)]
pub enum ModelCheckItem<'a> {
    /// An authored clause with its property form, its atoms read through
    /// `letters`.
    Clause {
        /// The checked clause.
        clause: &'a CheckedTemporalClause,
        /// The seam to the one clause evaluator for its `holds` atoms.
        letters: &'a dyn ModelLetters,
    },
    /// FR-124's derived deadlock-freedom item.
    DeadlockFreedom,
}

/// One instance of an item: the bound object, `None` for the
/// deadlock-freedom item, and the translation under its context.
#[derive(Clone, Debug)]
pub struct Instance {
    /// The object the instance binds.
    pub binding: Option<ObjectReference>,
    /// The instance's canonical safety translation.
    pub translation: SafetyTranslation,
}

/// Why an item has no canonical translation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ItemRefusal {
    /// The clause is not a TP-1 `always holds(A)` under infinite-trace.
    #[error(transparent)]
    Translation(#[from] TranslationRefusal),
    /// The clause binds more instances than one letter holds atoms.
    #[error("the clause binds {count} instances; a letter holds at most {max}")]
    TooManyBindings {
        /// The number of instances.
        count: usize,
        /// The atoms one letter holds.
        max: usize,
    },
    /// The package gives the clause no root (`clause-root-unresolved`).
    #[error(transparent)]
    ClauseRoot(#[from] ClauseRootRefusal),
    /// The deadlock-freedom item's context did not encode.
    #[error("the deadlock-freedom context does not encode")]
    DeadlockContext,
}

impl ModelCheckItem<'_> {
    /// Each instance of the item with its translation, in binding order. A
    /// clause's identity is resolved from `subject`'s package.
    ///
    /// # Errors
    ///
    /// [`ItemRefusal`] when the item has no canonical translation, or the
    /// package gives the clause no root.
    pub fn instances(
        &self,
        subject: &ModelSubject<'_>,
        cancel: &Cancel,
    ) -> Result<Vec<Instance>, ItemRefusal> {
        match self {
            Self::DeadlockFreedom => Ok(vec![Instance {
                binding: None,
                translation: SafetyTranslation::deadlock_freedom()
                    .map_err(|_| ItemRefusal::DeadlockContext)?,
            }]),
            Self::Clause { clause, letters } => {
                let identity = ClauseIdentity::resolve(
                    subject.simulation.package,
                    subject.simulation.packages,
                    clause,
                    cancel,
                )?;
                let bindings = letters.bindings();
                if bindings.len() > MAX_ATOMS {
                    return Err(ItemRefusal::TooManyBindings {
                        count: bindings.len(),
                        max: MAX_ATOMS,
                    });
                }
                bindings
                    .into_iter()
                    .map(|object| {
                        let binding = identity
                            .over()
                            .map(|parameter| ContextBinding {
                                parameter,
                                value: Value::Reference(object.clone()),
                            })
                            .into_iter()
                            .collect();
                        let context =
                            StateContext::clause(clause, identity.occurrence().clone(), binding);
                        let translation = SafetyTranslation::reachable_invariant(clause, &context)?;
                        Ok(Instance {
                            binding: Some(object),
                            translation,
                        })
                    })
                    .collect()
            }
        }
    }
}
