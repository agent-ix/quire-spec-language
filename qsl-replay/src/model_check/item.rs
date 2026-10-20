// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 Inputs and FR-338 Inputs: the item an EN-1 run decides and the
//! core's closure checker checks, and the canonical safety translation of
//! each of its instances (ADR-018 LA-2).
//!
//! A TP-1 clause binds one instance per key of each population of the
//! subject that admits its `over` type, derived from the subject's universes
//! at request time (FR-125 Instancing). Each instance has its own
//! translation, under a key context that binds the typed reference of its
//! key: two admitting populations that hold the same key give two instances
//! with one key context. The deadlock-freedom item has one instance with the
//! binding `[]`.
//!
//! A clause's key contexts name its identity, which the item does not carry:
//! [`ClauseIdentity::resolve`] reads it from the request's package
//! (FR-337-AC-5), so no caller names an occurrence or an `over` node.

use qsl_eval::automaton::{
    ContextBinding, SafetyTranslation, StateContext, TranslationRefusal, MAX_ATOMS,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::NotSimulated;
use qsl_foundation::InternalFault;
use qsl_semantics::check::CheckedTemporalClause;
use quire_exact::{Cancel, Value, ValueType};

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

/// One instance of an item: the bound population key, `None` for the
/// deadlock-freedom item, and the translation under its context.
#[derive(Clone, Debug)]
pub struct Instance {
    /// The population key the instance binds.
    pub binding: Option<ModelInstance>,
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
    /// FR-125: a population that admits the `over` type has no universe
    /// (`RequiresBound`), or that check could not classify it.
    #[error("the clause's instances are not finite: {0:?}")]
    Unbounded(NotSimulated),
    /// An invariant of the subject broke.
    #[error("instancing broke invariant {0:?}")]
    Fault(InternalFault),
}

impl ModelCheckItem<'_> {
    /// Each instance of the item with its translation, in FR-125's
    /// canonical instance order, derived from `system`'s universes. A
    /// clause's identity is resolved from `subject`'s package; `system` is
    /// `subject`'s admitted model system.
    ///
    /// # Errors
    ///
    /// [`ItemRefusal`] when an admitting population has no universe, the
    /// package gives the clause no root, or the item has no canonical
    /// translation.
    pub fn instances(
        &self,
        subject: &ModelSubject<'_>,
        system: &ModelSystem<'_>,
        cancel: &Cancel,
    ) -> Result<Vec<Instance>, ItemRefusal> {
        match self {
            Self::DeadlockFreedom => Ok(vec![Instance {
                binding: None,
                translation: SafetyTranslation::deadlock_freedom()
                    .map_err(|_| ItemRefusal::DeadlockContext)?,
            }]),
            Self::Clause { clause, .. } => {
                let identity = ClauseIdentity::resolve(
                    subject.simulation.package,
                    subject.simulation.packages,
                    clause,
                    cancel,
                )?;
                let over = &clause.over().1;
                let bindings = system.instances(over).map_err(ItemRefusal::Unbounded)?;
                if bindings.len() > MAX_ATOMS {
                    return Err(ItemRefusal::TooManyBindings {
                        count: bindings.len(),
                        max: MAX_ATOMS,
                    });
                }
                bindings
                    .into_iter()
                    .map(|instance| {
                        let binding = match (identity.over(), over) {
                            (Some(parameter), ValueType::Reference(object_type)) => {
                                let reference = system
                                    .bound_reference(*object_type, &instance)
                                    .map_err(ItemRefusal::Fault)?;
                                vec![ContextBinding {
                                    parameter,
                                    value: Value::Reference(reference),
                                }]
                            }
                            _ => Vec::new(),
                        };
                        let context =
                            StateContext::clause(clause, identity.occurrence().clone(), binding);
                        let translation = SafetyTranslation::reachable_invariant(clause, &context)?;
                        Ok(Instance {
                            binding: Some(instance),
                            translation,
                        })
                    })
                    .collect()
            }
        }
    }
}
