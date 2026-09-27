// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-107 (QSL-278): `ProtocolClause`'s S6a/evaluation half
//! ([`super::ReferenceEvaluation`]), mirroring `family.rs`'s own
//! `ValueFunctionFamily` half (ADR-012 §2: one marker type implements both
//! halves, split across crates by the orphan rule -- see that module's own
//! doc for why the impl lives here).

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_semantics::check::{CheckedGraph, Location, Observation, ProtocolClauseFamily};
use qsl_semantics::family::{EvalOutcome, FamilyOutcome};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Meter, NodeKey, Value};

/// `ProtocolClauseFamily`'s real evaluation environment (FR-107): the
/// checked graph the clause's identity resolves against, its `current` (or
/// `post`) observation's object environment, its `pre` observation's when
/// admitted for a precondition or postcondition, and the `self`/`result`/
/// parameter bindings in the clause's own slot order.
pub(crate) struct ProtocolClauseEnv<'a> {
    pub(crate) graph: &'a CheckedGraph,
    /// The clause's own observation: `current` for an invariant, `post`
    /// for a postcondition, and also the environment a precondition's
    /// non-`pre(..)` reads observe (FR-104: a precondition's own
    /// observation is `pre`, so for a precondition this is the same
    /// environment as `pre`; admission gives both, and this module always
    /// reads `pre` there, never re-deriving one from the other).
    pub(crate) current: &'a ObjectEnvironment,
    /// The pre observation's object environment, when admission gave one
    /// (every precondition and postcondition; `None` for an invariant).
    pub(crate) pre: Option<&'a ObjectEnvironment>,
    pub(crate) bindings: Option<Vec<Value>>,
    /// The last hook call's `Evaluation.location` (FR-090-OQ-3 ruling, as
    /// `family::EvaluationEnv` records it).
    pub(crate) location: Option<Location>,
    pub(crate) losses: Vec<super::super::evaluate::LocatedLoss>,
}

impl<'a> ProtocolClauseEnv<'a> {
    pub(crate) fn new(
        graph: &'a CheckedGraph,
        current: &'a ObjectEnvironment,
        pre: Option<&'a ObjectEnvironment>,
        bindings: Vec<Value>,
    ) -> Self {
        Self {
            graph,
            current,
            pre,
            bindings: Some(bindings),
            location: None,
            losses: Vec::new(),
        }
    }
}

impl super::ReferenceEvaluation for ProtocolClauseFamily {
    type Observed = Value;
    type Env<'a> = ProtocolClauseEnv<'a>;
    type Key = NodeKey;

    /// FR-107: resolves `checked` against `env.graph`'s state clauses (never
    /// its functions), then runs the clause's checked body through the same
    /// task-stack machine `Value`'s own hook uses
    /// ([`super::super::evaluate::Machine::with_pre`]), giving it `env`'s pre and
    /// current observations and the clause's own per-read observation map
    /// (`CheckedStateClause::reads`), so a read under `pre(..)` observes
    /// `env.pre` and every other read observes `env.current`.
    fn evaluate<'a>(
        checked: &NodeKey,
        env: &mut ProtocolClauseEnv<'a>,
        meter: &mut Meter,
    ) -> Result<EvalOutcome<Value>, InternalFault> {
        env.location = None;
        env.losses.clear();
        let clause = env
            .graph
            .state_clause_by_identity(*checked)
            .ok_or_else(|| InternalFault::new("S6a", "checked-identity-not-resolved-by-package"))?;
        let Some(bindings) = env.bindings.take() else {
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
            env.current,
            env.pre,
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
