// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-107 (QSL-278) and FR-115 (QSL-300): `ProtocolClause`'s S6a/evaluation half
//! ([`super::ReferenceEvaluation`]), mirroring `family.rs`'s own
//! `ValueFunctionFamily` half (ADR-012 §2: one marker type implements both
//! halves, split across crates by the orphan rule -- see that module's own
//! doc for why the impl lives here).

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_semantics::check::{CheckedGraph, Location, Observation, ProtocolClauseFamily};
use qsl_semantics::family::{EvalOutcome, FamilyOutcome, FamilyResult};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{AdmittedInvocation, FrameVerdict, FrameWitness};
use quire_exact::{Meter, NodeKey, Outcome, Value};

use super::super::causes::ProtocolClauseFrameRefusal;

/// What one `ProtocolClause` S6a call evaluates: a state clause over its
/// admitted observations (FR-107), or an operation frame over one admitted
/// invocation (FR-115).
pub(crate) enum ProtocolClauseInput<'a> {
    /// A state clause (FR-107).
    Clause {
        /// The clause's own observation: `current` for an invariant, `post`
        /// for a postcondition, and also the environment a precondition's
        /// non-`pre(..)` reads observe (FR-104: a precondition's own
        /// observation is `pre`, so for a precondition this is the same
        /// environment as `pre`; admission gives both, and this module
        /// always reads `pre` there, never re-deriving one from the other).
        current: &'a ObjectEnvironment,
        /// The pre observation's object environment, when admission gave
        /// one (every precondition and postcondition; `None` for an
        /// invariant).
        pre: Option<&'a ObjectEnvironment>,
        /// The `self`/`result`/parameter bindings in the clause's slot
        /// order; taken by the one call.
        bindings: Option<Vec<Value>>,
    },
    /// An operation frame (FR-115): the invocation FR-106's checks 1 and 3
    /// to 10 admitted, which check 11 runs over.
    Frame(&'a AdmittedInvocation<'a>),
}

/// `ProtocolClauseFamily`'s real evaluation environment (FR-107, FR-115):
/// the checked graph the key resolves against and what the call evaluates.
pub(crate) struct ProtocolClauseEnv<'a> {
    pub(crate) graph: &'a CheckedGraph,
    pub(crate) input: ProtocolClauseInput<'a>,
    /// The last hook call's `Evaluation.location` (FR-090-OQ-3 ruling, as
    /// `family::EvaluationEnv` records it).
    pub(crate) location: Option<Location>,
    pub(crate) losses: Vec<super::super::evaluate::LocatedLoss>,
    /// FR-115: the evaluated frame witness of the last frame call that
    /// found a change outside the frame. Set exactly when that call's
    /// outcome is `Completed(false)`.
    pub(crate) witness: Option<Box<FrameWitness>>,
}

impl<'a> ProtocolClauseEnv<'a> {
    /// A state clause's environment (FR-107).
    pub(crate) fn new(
        graph: &'a CheckedGraph,
        current: &'a ObjectEnvironment,
        pre: Option<&'a ObjectEnvironment>,
        bindings: Vec<Value>,
    ) -> Self {
        Self::with_input(
            graph,
            ProtocolClauseInput::Clause {
                current,
                pre,
                bindings: Some(bindings),
            },
        )
    }

    /// An operation frame's environment (FR-115).
    pub(crate) fn frame(graph: &'a CheckedGraph, invocation: &'a AdmittedInvocation<'a>) -> Self {
        Self::with_input(graph, ProtocolClauseInput::Frame(invocation))
    }

    fn with_input(graph: &'a CheckedGraph, input: ProtocolClauseInput<'a>) -> Self {
        Self {
            graph,
            input,
            location: None,
            losses: Vec::new(),
            witness: None,
        }
    }
}

impl super::ReferenceEvaluation for ProtocolClauseFamily {
    type Observed = Value;
    type Env<'a> = ProtocolClauseEnv<'a>;
    type Key = NodeKey;

    /// FR-107: for a clause input, resolves `checked` against `env.graph`'s
    /// state clauses (never its functions), then runs the clause's checked
    /// body through the same task-stack machine `Value`'s own hook uses
    /// ([`super::super::evaluate::Machine::with_pre`]), giving it `env`'s pre and
    /// current observations and the clause's own per-read observation map
    /// (`CheckedStateClause::reads`), so a read under `pre(..)` observes
    /// `env.pre` and every other read observes `env.current`.
    ///
    /// FR-115: for a frame input, resolves `checked` against `env.graph`'s
    /// operation frames and runs FR-106's check 11 over the admitted
    /// invocation with that frame's effect, taken from the compiled package
    /// alone. Nothing outside the frame is `Completed(true)`. A change
    /// outside it is `Completed(false)`, a verdict that evaluated, with the
    /// frame witness in `env.witness`: never a refusal, which would settle
    /// the run without its counterexample. An invocation the frame cannot
    /// be evaluated over (a declared delta that disagrees) is the family's
    /// own refusal. Check 11 charges nothing (`admit_invocation`'s docs), so
    /// `meter` is untouched.
    ///
    /// FR-063 seam (QSL-278, mirroring `family.rs`'s own
    /// `ValueFunctionFamily::evaluate` seam exactly, the S6a family kind's
    /// own "one variant per family that implements `ReferenceEvaluation`"
    /// rule, FR-090 lines 44-45): this match's own probe arm below is
    /// `ProtocolClause`'s family growing that same, already-established
    /// per-family seam by one, not a new kind of seam -- registered in
    /// `xtask::seam_probe::checked_in_locations`.
    #[deny(clippy::wildcard_enum_match_arm)]
    #[deny(clippy::match_wildcard_for_single_variants)]
    fn evaluate<'a>(
        checked: &NodeKey,
        env: &mut ProtocolClauseEnv<'a>,
        meter: &mut Meter,
    ) -> Result<EvalOutcome<Value>, InternalFault> {
        env.location = None;
        env.losses.clear();
        env.witness = None;
        let (current, pre, bindings) = match &mut env.input {
            ProtocolClauseInput::Clause {
                current,
                pre,
                bindings,
            } => (*current, *pre, bindings.take()),
            ProtocolClauseInput::Frame(invocation) => {
                let invocation: &AdmittedInvocation<'_> = invocation;
                let frame = env
                    .graph
                    .operation_frame_by_identity(*checked)
                    .ok_or_else(|| {
                        InternalFault::new("S6a", "checked-identity-not-resolved-by-package")
                    })?;
                return Ok(
                    match invocation.check_frame(frame.operation().declaration.effect())? {
                        FrameVerdict::Holds => {
                            EvalOutcome::Kernel(Outcome::Completed(Value::Boolean(true)))
                        }
                        FrameVerdict::Violation(witness) => {
                            env.witness = Some(witness);
                            EvalOutcome::Kernel(Outcome::Completed(Value::Boolean(false)))
                        }
                        FrameVerdict::Refused(record) => EvalOutcome::Family(
                            FamilyResult::Refused(Box::new(ProtocolClauseFrameRefusal { record })),
                        ),
                    },
                );
            }
        };
        let clause = env
            .graph
            .state_clause_by_identity(*checked)
            .ok_or_else(|| InternalFault::new("S6a", "checked-identity-not-resolved-by-package"))?;
        let Some(bindings) = bindings else {
            return Err(InternalFault::new(
                "S6a",
                "evaluation-environment-arguments-already-consumed",
            ));
        };
        let reads: BTreeMap<Location, Observation> = clause
            .reads()
            .map(|(location, observation)| (location.clone(), observation))
            .collect();
        let evaluation = super::super::evaluate::Machine::with_pre(
            env.graph.scope(),
            env.graph,
            current,
            pre,
            Some(&reads),
            meter,
            env.graph.dispatch_tables(),
        )
        .run(clause.body(), clause.slots(), bindings)?;
        env.location = evaluation.location;
        env.losses = evaluation.losses;
        match evaluation.outcome {
            FamilyOutcome::Evaluated(outcome) => Ok(EvalOutcome::Kernel(outcome)),
            FamilyOutcome::FamilyEvaluated(result) => Ok(EvalOutcome::Family(result)),
            // FR-063: no arm for the probe variant under `--cfg seam_probe`
            // alone; see `family.rs`'s identical note.
            #[cfg(seam_probe_eval_downstream)]
            FamilyOutcome::__SeamProbe => {
                unreachable!("never constructed outside the probe build")
            }
        }
    }
}
