// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 Inputs and FR-338 Inputs: the item an EN-1 run decides and the
//! core's closure checker checks, and the canonical safety translation of
//! each of its instances (ADR-018 LA-2).
//!
//! A TP-1 clause binds one instance per key of each population of the
//! subject that admits its `over` type, derived from the subject's universes
//! at request time (FR-125 Instancing); each instance is one concrete
//! binding, so it has its own key context and its own translation. The deadlock-freedom item has one
//! instance with the binding `[]`.
//!
//! The clause's checked occurrence and `over` parameter node are part of the
//! item because S4 emits no temporal clause node yet (FR-337): the checker
//! cannot resolve them in the recompiled package, so the request names them.

use qsl_eval::automaton::{
    ContextBinding, SafetyTranslation, StateContext, TranslationRefusal, MAX_ATOMS,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::NotSimulated;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::InternalFault;
use qsl_semantics::check::CheckedTemporalClause;
use quire_exact::{Value, ValueType};

/// A clause's identity in its package: the checked occurrence and the node of
/// its `over` parameter, `None` for a clause with no `over`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseIdentity {
    /// The clause's O-07 occurrence: node, role and ordinal.
    pub occurrence: OccurrenceKey,
    /// The `over` parameter's checked node.
    pub over: Option<WireNodeId>,
}

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
        /// The clause's identity, which every key context names.
        identity: &'a ClauseIdentity,
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
    /// canonical instance order, derived from `system`'s universes.
    ///
    /// # Errors
    ///
    /// [`ItemRefusal`] when an admitting population has no universe, or the
    /// item has no canonical translation.
    pub fn instances(&self, system: &ModelSystem<'_>) -> Result<Vec<Instance>, ItemRefusal> {
        match self {
            Self::DeadlockFreedom => Ok(vec![Instance {
                binding: None,
                translation: SafetyTranslation::deadlock_freedom()
                    .map_err(|_| ItemRefusal::DeadlockContext)?,
            }]),
            Self::Clause {
                clause, identity, ..
            } => {
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
                        let binding = match (identity.over, over) {
                            (Some(parameter), ValueType::Reference(object_type)) => {
                                let reference = system
                                    .bound_reference(*object_type, &instance)
                                    .ok_or_else(|| {
                                        ItemRefusal::Fault(InternalFault::new(
                                            "model-check-item",
                                            "over-type-declared",
                                        ))
                                    })?;
                                vec![ContextBinding {
                                    parameter,
                                    value: Value::Reference(reference),
                                }]
                            }
                            _ => Vec::new(),
                        };
                        let context =
                            StateContext::clause(clause, identity.occurrence.clone(), binding);
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
