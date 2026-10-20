// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 EI-6 (FR-161 "Start", "Steps", "Length and limits"): turning a
//! path of a symmetry-reduced graph into a concrete path of the subject.
//!
//! [`Lift`] keeps `σ`, with the concrete state `σ(canonical state)`. It
//! starts at the first initial state in subject order whose canonical form
//! is the path's first state, with `σ = π0⁻¹` for the permutation `π0` the
//! canonicaliser returned for it. Each retained edge `(source, t, target,
//! π)` becomes the concrete step `σ(t)` to `σ ∘ π⁻¹ (target)`, and `σ`
//! becomes `σ ∘ π⁻¹`; a successor that was never canonicalised is the same
//! step with `π` the identity. Every concrete step is checked against an
//! expansion of the concrete source and counted against the run's
//! remaining `max_transitions`.
//!
//! Under the trivial group the path is already concrete: each step is
//! recorded as it is, with no expansion and no count, so an unreduced run's
//! counterexample is unchanged.

use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::{
    EncodingRefusal, ExpansionStop, Permutation, SymmetryGroup, TransitionSystem,
};
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{CounterexampleKind, ModelStep};
use quire_semantic_value::model_transition::ModelTransition;

use super::check::{fault, ModelCheckRefusal};

/// Why a path could not be concretised.
pub(super) enum LiftStop {
    /// An expansion or a permutation of a concrete state stopped.
    Expansion(ExpansionStop),
    /// The concrete steps exhausted what is left of `max_transitions`.
    Transitions,
    /// An identity has no canonical encoding, or an invariant broke.
    Refusal(ModelCheckRefusal),
}

impl From<EncodingRefusal> for LiftStop {
    fn from(refusal: EncodingRefusal) -> Self {
        Self::Refusal(ModelCheckRefusal::Encoding(refusal))
    }
}

impl From<ModelCheckRefusal> for LiftStop {
    fn from(refusal: ModelCheckRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

/// A path being concretised: `σ`, the concrete state it has reached and the
/// concrete steps so far.
pub(super) struct Lift<'s, 'a> {
    system: &'s ModelSystem<'a>,
    reduced: bool,
    sigma: Permutation,
    concrete: ModelSimState,
    digest: DigestRecord,
    steps: Vec<ModelStep>,
    /// The concrete steps still allowed (reduced paths only).
    budget: usize,
}

impl<'s, 'a> Lift<'s, 'a> {
    /// FR-161 "Start": the path from `root`, a state of the graph reduced by
    /// `group`, with the index of the subject's initial state it starts
    /// from. At most `budget` concrete steps may follow.
    ///
    /// # Errors
    ///
    /// [`LiftStop`] when the canonicaliser stops, or no initial state has
    /// `root`'s key.
    pub(super) fn start(
        system: &'s ModelSystem<'a>,
        group: &SymmetryGroup,
        root: &ModelSimState,
        budget: usize,
    ) -> Result<(usize, Self), LiftStop> {
        let reduced = !group.is_trivial();
        let (wanted, _) = system.state_key(root)?;
        for (index, initial) in system.initial().into_iter().enumerate() {
            let (canonical, permutation) = if reduced {
                system
                    .canonicalise(&initial, group)
                    .map_err(LiftStop::Expansion)?
            } else {
                (initial.clone(), Permutation::identity())
            };
            if system.state_key(&canonical)?.0 != wanted {
                continue;
            }
            let digest = system.state_key(&initial)?.1;
            return Ok((
                index,
                Self {
                    system,
                    reduced,
                    sigma: permutation.inverse(),
                    concrete: initial,
                    digest,
                    steps: Vec::new(),
                    budget,
                },
            ));
        }
        Err(fault("a-root-is-an-initial-state").into())
    }

    /// FR-161 "Steps": the retained edge labelled `transition` to the
    /// canonical `target`, canonicalised by `permutation`; a successor that
    /// was never canonicalised passes the identity.
    ///
    /// # Errors
    ///
    /// [`LiftStop`] when an expansion stops, the budget is spent, or the
    /// concrete step is not a step of the subject.
    pub(super) fn edge(
        &mut self,
        transition: &ModelTransition,
        target: &ModelSimState,
        permutation: &Permutation,
    ) -> Result<(), LiftStop> {
        if !self.reduced {
            self.digest = self.system.state_key(target)?.1;
            self.concrete = target.clone();
            self.steps.push(ModelStep {
                transition: transition.clone(),
                post_state: self.digest,
            });
            return Ok(());
        }
        if self.budget == 0 {
            return Err(LiftStop::Transitions);
        }
        self.budget -= 1;
        let step = self
            .system
            .permute_transition(transition, &self.sigma)
            .map_err(|_| fault("a-concretised-step-encodes"))?;
        let sigma = self.sigma.compose(&permutation.inverse());
        let next = self
            .system
            .permute(target, &sigma)
            .map_err(LiftStop::Expansion)?;
        let digest = self.system.state_key(&next)?.1;
        let expansion = self
            .system
            .successors(&self.concrete)
            .map_err(LiftStop::Expansion)?;
        let mut matched = false;
        for (candidate, state) in &expansion.successors {
            if *candidate == step && self.system.state_key(state)?.1 == digest {
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(fault("a-concretised-step-is-a-subject-step").into());
        }
        self.steps.push(ModelStep {
            transition: step,
            post_state: digest,
        });
        self.sigma = sigma;
        self.concrete = next;
        self.digest = digest;
        Ok(())
    }

    /// `σ`: the concrete state is `σ(canonical state)`.
    pub(super) fn sigma(&self) -> &Permutation {
        &self.sigma
    }

    /// The concrete state's digest.
    pub(super) fn digest(&self) -> DigestRecord {
        self.digest
    }

    /// The number of concrete steps so far.
    pub(super) fn len(&self) -> usize {
        self.steps.len()
    }

    /// The steps charged to the budget: every step of a reduced path, none
    /// of a concrete one.
    pub(super) fn charged(&self) -> usize {
        if self.reduced {
            self.steps.len()
        } else {
            0
        }
    }

    /// The concrete steps.
    pub(super) fn into_steps(self) -> Vec<ModelStep> {
        self.steps
    }

    /// ADR-021 RV-7: an undefined evaluation's site at the path's end,
    /// concretised: the concrete state's digest and, for a guard, `σ` of
    /// the guard, which is checked against the concrete state's own
    /// expansion as each step is. Nothing changes under the trivial group.
    ///
    /// # Errors
    ///
    /// [`LiftStop`] when the expansion stops or the concrete guard is not a
    /// guard of the subject.
    pub(super) fn site(&self, kind: &mut CounterexampleKind) -> Result<(), LiftStop> {
        let CounterexampleKind::UndefinedEvaluation(evaluation) = kind else {
            return Ok(());
        };
        if !self.reduced {
            return Ok(());
        }
        evaluation.site.state_key = self.digest;
        if let Some(guard) = &evaluation.site.transition {
            let guard = self
                .system
                .permute_transition(guard, &self.sigma)
                .map_err(|_| fault("a-concretised-guard-encodes"))?;
            let expansion = self
                .system
                .successors(&self.concrete)
                .map_err(LiftStop::Expansion)?;
            let found = expansion.findings.iter().any(|finding| {
                finding
                    .undefined_contract()
                    .is_some_and(|contract| contract.guard_of.as_ref() == Some(&guard))
            });
            if !found {
                return Err(fault("a-concretised-guard-is-a-subject-guard").into());
            }
            evaluation.site.transition = Some(guard);
        }
        Ok(())
    }
}
