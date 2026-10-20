// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176 (ADR-023 HC-1): the seams through which the self-composition
//! product reads a hyper clause's letters and its `match` block.
//!
//! A hyper clause's `holds` atoms and `match` conjuncts are typed at S3 but
//! not yet lowered to checked nodes the one clause evaluator (FR-107) can run,
//! so the product reads them through these two traits, as a temporal clause's
//! atoms are read through [`crate::model_subject::ModelLetters`]. Each
//! implementation is the caller's seam to that evaluator.

use quire_exact::{Incomplete, Meter};
use quire_semantic_value::model_transition::ModelTransition;

use qsl_semantics::check::FormulaNodeId;

use crate::model_subject::ModelInstance;
use crate::simulation::model::ModelSimState;
use crate::temporal::{AtomReading, UndefinedCause};

/// One object parameter of the clause bound to a key of its population's
/// universe (ADR-023 HS-3): within an instance, `parameter` denotes the object
/// with that key in every trace of its alias.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterBinding {
    /// The parameter's name.
    pub parameter: String,
    /// The population and key it is bound to.
    pub instance: ModelInstance,
}

/// The step a component took into a joint position: the label a `match`
/// conjunct reads (ADR-023 HS-5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepLabel<'a> {
    /// An operation of the component's model: `t.step.op`, `t.step.receiver`
    /// and `t.step.O.p` read its identity.
    Operation(&'a ModelTransition),
    /// The stutter step of a terminal component (ADR-018 SM-4).
    Stutter,
}

/// Reads the `holds` atoms of a hyper clause's body at a joint position
/// (ADR-023 HM-4).
pub trait HyperLetters {
    /// The `holds` atom `atom` for `instance` at the joint position whose
    /// component states are `states`, one per trace variable in quantifier
    /// order, charging `meter`. A member `e @ t` reads component `t`'s state
    /// (ADR-023 HS-4). An exhausted meter is `Err(Incomplete)`, never a
    /// reading.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete>;
}

/// What a `match` block's universal conjuncts say of one joint step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchReading {
    /// Every conjunct holds: the joint step is matched.
    Holds,
    /// Some conjunct is false: the joint step is not matched.
    Fails,
    /// A conjunct has no value at the joint step (ADR-023 HV-8).
    Undefined {
        /// The index of the conjunct among the clause's universal conjuncts
        /// for a `universal` reading, its existential conjuncts for an
        /// `existential` one.
        conjunct: usize,
        /// Why it has no value.
        cause: UndefinedCause,
    },
    /// A conjunct was refused at the joint step.
    Refused,
}

/// Reads a hyper clause's `match` block over a joint step: its universal part
/// `μ_U` (ADR-023 HC-1) and, for an HP-3 clause, its existential part `μ_E`
/// (ADR-023 HC-2).
pub trait StepMatch {
    /// `μ_U` for `instance` over `steps`, the labels of the universal
    /// components' steps into the joint position, one per universal trace
    /// variable in quantifier order. `μ` reads no state (ADR-023 HS-5). An
    /// exhausted meter is `Err(Incomplete)`, which the product reads as a
    /// conjunct that is incomplete there.
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete>;

    /// `μ_E` for `instance` over `steps`, the labels of every component's
    /// step into the joint position, one per trace variable in quantifier
    /// order: the universal components' first, then the existential ones'.
    /// An `Undefined` reading names its conjunct among the clause's
    /// existential conjuncts. An exhausted meter is `Err(Incomplete)`.
    fn existential(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete>;
}
