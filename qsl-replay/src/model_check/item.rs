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
    bounded_shape, translate, translate_counted, AutomatonError, AutomatonKind, BoundedTranslation,
    BuchiStates, ContextBinding, PropertyAutomaton, SafetyTranslation, StateContext, SubjectKind,
    TranslationRefusal, MAX_ATOMS,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::NotSimulated;
use qsl_semantics::check::CheckedTemporalClause;
use quire_exact::{Cancel, Value, ValueType};
use quire_semantic_value::catalog::InternalFault;

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
#[derive(Debug)]
pub struct Instance {
    /// The population key the instance binds.
    pub binding: Option<ModelInstance>,
    /// The instance's canonical translation.
    pub translation: Translation,
}

/// The canonical translation of one instance (FR-338): a safety item's, or a
/// bounded clause's.
#[derive(Debug)]
pub enum Translation {
    /// A TP-1 clause or the deadlock-freedom item.
    Safety(SafetyTranslation),
    /// A bounded `eventually[0,u]` or `always[0,u]` clause on origin (TP-2).
    Bounded(BoundedTranslation),
}

/// The TP-4 engine automaton and its FR-338 state-key context.
#[derive(Debug)]
pub struct LivenessTranslation {
    pub(crate) automaton: PropertyAutomaton,
    pub(crate) keys: BuchiStates,
}

impl Translation {
    /// Whether `octets` are a key of this translation's context, admitted
    /// before the key is used for membership.
    pub fn admits(&self, octets: &[u8]) -> bool {
        match self {
            Self::Safety(translation) => translation.admit(octets).is_ok(),
            Self::Bounded(translation) => translation.admit(octets).is_ok(),
        }
    }
}

/// A closure checker's instance, whose TP-4 state stays in the replay lane.
#[derive(Debug)]
pub(crate) struct ClosureInstance {
    pub(crate) binding: Option<ModelInstance>,
    pub(crate) translation: ClosureTranslation,
}

/// A closure checker's canonical translation, including TP-4's Büchi state
/// bridge. The producer's public `Translation` remains limited to the forms
/// it materializes in its certificate today.
#[derive(Debug)]
pub(crate) enum ClosureTranslation {
    Safety(SafetyTranslation),
    Bounded(BoundedTranslation),
    Liveness(LivenessTranslation),
}

impl ClosureTranslation {
    pub(crate) fn admits(&self, octets: &[u8]) -> bool {
        match self {
            Self::Safety(translation) => translation.admit(octets).is_ok(),
            Self::Bounded(translation) => translation.admit(octets).is_ok(),
            Self::Liveness(translation) => translation.keys.admit(octets).is_ok(),
        }
    }
}

/// Why an item has no canonical translation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ItemRefusal {
    /// The clause is neither a TP-1 `always holds(A)` under infinite-trace
    /// nor a bounded single-interval clause with a canonical translation.
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
    /// The requested TP-4 property automaton could not be materialized.
    #[error(transparent)]
    Automaton(#[from] AutomatonError),
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
        self.build_instances(
            subject,
            system,
            cancel,
            SubjectKind::Ordinary,
            u64::MAX,
            false,
        )?
        .into_iter()
        .map(|instance| {
            let translation = match instance.translation {
                ClosureTranslation::Safety(translation) => Translation::Safety(translation),
                ClosureTranslation::Bounded(translation) => Translation::Bounded(translation),
                ClosureTranslation::Liveness(_) => {
                    return Err(ItemRefusal::Fault(InternalFault::new(
                        "tp4-translation-not-produced",
                        "the model-check producer does not materialize TP-4 closure keys",
                    )))
                }
            };
            Ok(Instance {
                binding: instance.binding,
                translation,
            })
        })
        .collect()
    }

    /// Instances with a subject-specific TP-4 key algebra.
    pub(crate) fn instances_for(
        &self,
        subject: &ModelSubject<'_>,
        system: &ModelSystem<'_>,
        cancel: &Cancel,
        subject_kind: SubjectKind,
        max_automaton_states: u64,
    ) -> Result<Vec<ClosureInstance>, ItemRefusal> {
        self.build_instances(
            subject,
            system,
            cancel,
            subject_kind,
            max_automaton_states,
            true,
        )
    }

    fn build_instances(
        &self,
        subject: &ModelSubject<'_>,
        system: &ModelSystem<'_>,
        cancel: &Cancel,
        subject_kind: SubjectKind,
        max_automaton_states: u64,
        include_tp4: bool,
    ) -> Result<Vec<ClosureInstance>, ItemRefusal> {
        match self {
            Self::DeadlockFreedom => Ok(vec![ClosureInstance {
                binding: None,
                translation: ClosureTranslation::Safety(
                    SafetyTranslation::deadlock_freedom()
                        .map_err(|_| ItemRefusal::DeadlockContext)?,
                ),
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
                        let tp4_keys = include_tp4
                            .then(|| BuchiStates::clause(clause, &context, subject_kind).ok())
                            .flatten();
                        let translation = if bounded_shape(clause).is_some() {
                            ClosureTranslation::Bounded(BoundedTranslation::clause(
                                clause, &context,
                            )?)
                        } else if let Some(keys) = tp4_keys {
                            let automaton = match subject_kind {
                                SubjectKind::Ordinary => translate(clause, max_automaton_states),
                                SubjectKind::Protocol => {
                                    translate_counted(clause, max_automaton_states)
                                }
                            }
                            .map_err(ItemRefusal::Automaton)?;
                            if automaton.kind() == AutomatonKind::Liveness {
                                ClosureTranslation::Liveness(LivenessTranslation {
                                    automaton,
                                    keys,
                                })
                            } else {
                                // Counted protocol progress can lower an
                                // infinite-profile formula into the safety
                                // fragment. Keep the existing safety replay
                                // representation for those clauses.
                                ClosureTranslation::Safety(SafetyTranslation::reachable_invariant(
                                    clause, &context,
                                )?)
                            }
                        } else {
                            ClosureTranslation::Safety(SafetyTranslation::reachable_invariant(
                                clause, &context,
                            )?)
                        };
                        Ok(ClosureInstance {
                            binding: Some(instance),
                            translation,
                        })
                    })
                    .collect()
            }
        }
    }
}
