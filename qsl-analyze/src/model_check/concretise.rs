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
//! counterexample is unchanged. A subject that offers no symmetry (a
//! protocol subject) is only ever lifted under the trivial group, so it
//! supplies no [`Permuter`].

use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::{
    EncodingRefusal, ExpansionStop, Permutation, SubjectSystem, SymmetryGroup, TransitionSystem,
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

/// What lifting a reduced path reads of a subject's symmetry (ADR-021
/// EI-6): the canonical form of a state, and the image of a state and of a
/// transition identity under a permutation.
pub(super) trait Permuter<State, Transition> {
    fn canonicalise(
        &self,
        state: &State,
        group: &SymmetryGroup,
    ) -> Result<(State, Permutation), ExpansionStop>;

    fn permute(&self, state: &State, permutation: &Permutation) -> Result<State, ExpansionStop>;

    fn permute_transition(
        &self,
        transition: &Transition,
        permutation: &Permutation,
    ) -> Result<Transition, ExpansionStop>;

    /// The image of the FR-120 identity an undefined guard names.
    fn permute_model_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop>;
}

impl Permuter<ModelSimState, ModelTransition> for ModelSystem<'_> {
    fn canonicalise(
        &self,
        state: &ModelSimState,
        group: &SymmetryGroup,
    ) -> Result<(ModelSimState, Permutation), ExpansionStop> {
        ModelSystem::canonicalise(self, state, group)
    }

    fn permute(
        &self,
        state: &ModelSimState,
        permutation: &Permutation,
    ) -> Result<ModelSimState, ExpansionStop> {
        ModelSystem::permute(self, state, permutation)
    }

    fn permute_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop> {
        ModelSystem::permute_transition(self, transition, permutation)
    }

    fn permute_model_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop> {
        ModelSystem::permute_transition(self, transition, permutation)
    }
}

/// A subject's symmetry, as [`Lift`] reads it.
pub(super) type PermuterOf<'s, S> =
    &'s dyn Permuter<<S as TransitionSystem>::State, <S as TransitionSystem>::TransitionId>;

/// A path being concretised: `σ`, the concrete state it has reached and the
/// concrete steps so far.
pub(super) struct Lift<'s, S: SubjectSystem> {
    system: &'s S,
    /// The subject's symmetry; `Some` on a reduced path.
    permuter: Option<PermuterOf<'s, S>>,
    sigma: Permutation,
    concrete: S::State,
    digest: DigestRecord,
    steps: Vec<ModelStep<S::TransitionId>>,
    /// The concrete steps still allowed (reduced paths only).
    budget: usize,
}

impl<'s, S: SubjectSystem> Lift<'s, S> {
    /// FR-161 "Start": the path from `root`, a state of the graph reduced by
    /// `group`, with the index of the subject's initial state it starts
    /// from. At most `budget` concrete steps may follow. `permuter` is the
    /// subject's symmetry, which a non-trivial `group` needs.
    ///
    /// # Errors
    ///
    /// [`LiftStop`] when the canonicaliser stops, `group` is not trivial and
    /// the subject has no symmetry, or no initial state has `root`'s key.
    pub(super) fn start(
        system: &'s S,
        permuter: Option<PermuterOf<'s, S>>,
        group: &SymmetryGroup,
        root: &S::State,
        budget: usize,
    ) -> Result<(usize, Self), LiftStop> {
        let permuter = if group.is_trivial() {
            None
        } else {
            Some(permuter.ok_or_else(|| fault("a-reduced-path-has-a-symmetry"))?)
        };
        let (wanted, _) = system.state_key(root)?;
        for (index, initial) in system.initial().into_iter().enumerate() {
            let (canonical, permutation) = match permuter {
                Some(permuter) => permuter
                    .canonicalise(&initial, group)
                    .map_err(LiftStop::Expansion)?,
                None => (initial.clone(), Permutation::identity()),
            };
            if system.state_key(&canonical)?.0 != wanted {
                continue;
            }
            let digest = system.state_key(&initial)?.1;
            return Ok((
                index,
                Self {
                    system,
                    permuter,
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
        transition: &S::TransitionId,
        target: &S::State,
        permutation: &Permutation,
    ) -> Result<(), LiftStop> {
        let Some(permuter) = self.permuter else {
            self.digest = self.system.state_key(target)?.1;
            self.concrete = target.clone();
            self.steps.push(ModelStep {
                transition: transition.clone(),
                post_state: self.digest,
            });
            return Ok(());
        };
        if self.budget == 0 {
            return Err(LiftStop::Transitions);
        }
        self.budget -= 1;
        let step = permuter
            .permute_transition(transition, &self.sigma)
            .map_err(|_| fault("a-concretised-step-encodes"))?;
        let sigma = self.sigma.compose(&permutation.inverse());
        let next = permuter
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
        if self.permuter.is_some() {
            self.steps.len()
        } else {
            0
        }
    }

    /// The concrete steps.
    pub(super) fn into_steps(self) -> Vec<ModelStep<S::TransitionId>> {
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
        let Some(permuter) = self.permuter else {
            return Ok(());
        };
        evaluation.site.state_key = self.digest;
        if let Some(guard) = &evaluation.site.transition {
            let guard = permuter
                .permute_model_transition(guard, &self.sigma)
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
