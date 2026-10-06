// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357: the scalar-parity arms of the replay facade -- the claim, outcome
//! and result types both arms share, and the operator-level entry
//! [`replay_operator_parity`]. The function-level entry,
//! [`crate::replay_value_parity`], is in `execute`.
//!
//! A scalar claim says the generated code agrees with QSL's exact
//! semantics: the generated outcome equals the exact one. It is a claim
//! about the lowering, never a property of a spec, so no arm settles
//! `Refuted`: that stays with a `Boolean` predicate that is false at its
//! bindings ([`crate::replay`]).

use qsl_foundation::diagnostic::{Code, InternalFault};
use quire_exact::{
    evaluate_integer_arithmetic, Incomplete, Integer, IntegerArithmetic, IntegerInterval, Meter,
    Outcome, Refusal, ScalarLimits, Value,
};

use qsl_foundation::digest::{DigestRecord, WireNodeId};

use crate::execute::ReplayRefusal;
use crate::identity::{ObligationIdentity, QualifiedName};
use crate::proof_result::{IncompleteCause, InconclusiveCause, TerminalValue};
use crate::result::{same_element, value_bytes};

/// The outcome of one scalar evaluation, QSL's exact one or the generated
/// code's: a value, a result outside the declared result range, or an
/// undefined or otherwise refused evaluation.
#[derive(Clone, Debug)]
pub enum ScalarOutcome {
    /// The evaluation completed this value. An operator claim's is a
    /// `Value::Integer`.
    Value(Value),
    /// The result is outside the declared result range (an integer, a
    /// decimal, a rational, a text length or a collection cardinality
    /// outside its declared bounds, or a division member outside the
    /// consumer's domain).
    OutOfRange,
    /// The evaluation is undefined: it has no mathematical value (division
    /// by zero, say).
    Undefined,
    /// The evaluation is refused for a reason other than range, such as an
    /// inexact decimal. A refusal because the result is outside its range
    /// is [`Self::OutOfRange`], never this.
    OtherRefusal,
}

impl ScalarOutcome {
    /// Whether `self` and `other` are the same outcome: the same variant,
    /// and for two values, equal under ADR-013 O-13 semantic equality.
    pub fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Value(left), Self::Value(right)) => same_element(left, right),
            (Self::OutOfRange, Self::OutOfRange)
            | (Self::Undefined, Self::Undefined)
            | (Self::OtherRefusal, Self::OtherRefusal) => true,
            (Self::Value(_), Self::OutOfRange | Self::Undefined | Self::OtherRefusal)
            | (Self::OutOfRange, Self::Value(_) | Self::Undefined | Self::OtherRefusal)
            | (Self::Undefined, Self::Value(_) | Self::OutOfRange | Self::OtherRefusal)
            | (Self::OtherRefusal, Self::Value(_) | Self::OutOfRange | Self::Undefined) => false,
        }
    }

    fn measured_bytes(&self) -> usize {
        match self {
            Self::Value(value) => value_bytes(value),
            Self::OutOfRange | Self::Undefined | Self::OtherRefusal => 0,
        }
    }
}

impl PartialEq for ScalarOutcome {
    fn eq(&self, other: &Self) -> bool {
        self.same_as(other)
    }
}

impl Eq for ScalarOutcome {}

/// One recorded operand and the range the harness drew it from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarOperand {
    /// The operand's recorded value, decoded from the witness.
    pub value: i64,
    /// The inclusive range the operand must lie in.
    pub range: IntegerInterval,
}

impl ScalarOperand {
    /// A literal operand: its range is the singleton `(value, value)`.
    pub fn literal(value: i64) -> Self {
        let exact = Integer::from(value);
        Self {
            value,
            range: IntegerInterval::spanning(exact.clone(), exact),
        }
    }

    fn exact(&self) -> Integer {
        Integer::from(self.value)
    }

    fn admitted(&self) -> bool {
        self.range.contains(&self.exact())
    }

    fn measured_bytes(&self) -> usize {
        std::mem::size_of::<i64>()
            + self.range.lower().to_string().len()
            + self.range.upper().to_string().len()
    }
}

/// One quire-exact integer operator and its operands, in position order.
/// Only the operators QSL lowers to a checked application node are here;
/// integer division, remainder and modulo are added when QSL lowers them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScalarOperation {
    /// `left + right`.
    Add {
        /// The left operand.
        left: ScalarOperand,
        /// The right operand.
        right: ScalarOperand,
    },
    /// `left - right`.
    Subtract {
        /// The left operand.
        left: ScalarOperand,
        /// The right operand.
        right: ScalarOperand,
    },
    /// `left * right`.
    Multiply {
        /// The left operand.
        left: ScalarOperand,
        /// The right operand.
        right: ScalarOperand,
    },
    /// `-operand`.
    Negate {
        /// The operand.
        operand: ScalarOperand,
    },
}

/// The catalog operator of a [`ScalarOperation`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarOperator {
    /// `quire.op.integer.add`.
    Add,
    /// `quire.op.integer.sub`.
    Subtract,
    /// `quire.op.integer.mul`.
    Multiply,
    /// `quire.op.integer.negate`.
    Negate,
}

impl ScalarOperator {
    /// The operation identity a checked application node of this operator
    /// carries.
    pub(crate) fn catalog_identity(self) -> &'static str {
        match self {
            Self::Add => "quire.op.integer.add",
            Self::Subtract => "quire.op.integer.sub",
            Self::Multiply => "quire.op.integer.mul",
            Self::Negate => "quire.op.integer.negate",
        }
    }
}

impl ScalarOperation {
    /// The catalog operator.
    pub fn operator(&self) -> ScalarOperator {
        match self {
            Self::Add { .. } => ScalarOperator::Add,
            Self::Subtract { .. } => ScalarOperator::Subtract,
            Self::Multiply { .. } => ScalarOperator::Multiply,
            Self::Negate { .. } => ScalarOperator::Negate,
        }
    }

    /// The first operand and the second, where the operator takes one.
    fn operands(&self) -> (&ScalarOperand, Option<&ScalarOperand>) {
        match self {
            Self::Negate { operand } => (operand, None),
            Self::Add { left, right }
            | Self::Subtract { left, right }
            | Self::Multiply { left, right } => (left, Some(right)),
        }
    }

    fn measured_bytes(&self) -> usize {
        let (first, second) = self.operands();
        first.measured_bytes() + second.map_or(0, ScalarOperand::measured_bytes)
    }
}

/// The native outcome of the proved generated artifact at the recorded
/// operands, as the driver observed it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeOutcome {
    /// The artifact completed this value.
    Completed(Integer),
    /// The artifact refused because its result is outside the result
    /// range. It maps to [`ScalarOutcome::OutOfRange`].
    RefusedOutOfRange,
    /// The artifact's operator is undefined at the operands.
    Undefined,
    /// The artifact's run stopped at a limit. A generated-artifact fault.
    Incomplete,
    /// The artifact's run failed. A generated-artifact fault.
    ExecutionFault,
}

/// An operator-level claim (FR-357): the generated artifact's native outcome
/// for one catalog operator at recorded operands equals the exact one,
/// compared on the proof projection only: `Completed(v)` agrees iff the exact
/// result is in range and equals `v`, `RefusedOutOfRange` iff the exact result is
/// outside the result range, `Undefined` iff the exact operator is undefined
/// there. Refusal causes and charge totals are not compared.
#[derive(Clone, Debug)]
pub struct OperatorClaim {
    /// The scalar node the operator is tied to.
    pub node: WireNodeId,
    /// The catalog operator and its recorded operands, each with its range.
    pub operation: ScalarOperation,
    /// The proving package's range the result must lie in.
    pub result_range: IntegerInterval,
    /// The accounting limits the exact evaluation runs under.
    pub limits: ScalarLimits,
    /// The native outcome of the same proved generated artifact at the
    /// operands.
    pub generated: NativeOutcome,
    /// The canonical generated-content identity binding the observation to
    /// the artifact, operands and limits. Carried into the claim identity;
    /// never recomputed or checked here.
    pub identity: DigestRecord,
}

/// The identity of an operator-level claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorIdentity {
    /// The obligation the claim replays, as the request carried it.
    pub obligation: ObligationIdentity,
    /// The scalar node the operator is tied to.
    pub node: WireNodeId,
    /// The generated-content identity of the observation.
    pub identity: DigestRecord,
    /// The operator and its operands.
    pub operation: ScalarOperation,
    /// The result range.
    pub result_range: IntegerInterval,
    /// The limits the exact evaluation ran under.
    pub limits: ScalarLimits,
}

/// The identity of the claim a [`ScalarAgreement`] settles.
#[derive(Clone, Debug)]
pub enum ScalarClaim {
    /// An operator-level claim: its operator, operands with their ranges,
    /// result range and limits.
    Operator(Box<OperatorIdentity>),
    /// A function-level claim: the function and its parameter bindings in
    /// declared parameter order.
    Function {
        /// The obligation the claim replays, as the request carried it.
        obligation: ObligationIdentity,
        /// The selected function.
        function: QualifiedName,
        /// The admitted value of each declared parameter.
        bindings: Vec<Value>,
    },
}

impl PartialEq for ScalarClaim {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Operator(left), Self::Operator(right)) => left == right,
            (
                Self::Function {
                    obligation,
                    function,
                    bindings,
                },
                Self::Function {
                    obligation: other_obligation,
                    function: other_function,
                    bindings: other_bindings,
                },
            ) => {
                obligation == other_obligation
                    && function == other_function
                    && bindings.len() == other_bindings.len()
                    && bindings
                        .iter()
                        .zip(other_bindings)
                        .all(|(left, right)| same_element(left, right))
            }
            (Self::Operator(_), Self::Function { .. })
            | (Self::Function { .. }, Self::Operator(_)) => false,
        }
    }
}

impl Eq for ScalarClaim {}

/// The cause data of `InconclusiveCause::ScalarAgrees`: the claim, and the
/// outcome both the exact evaluation and the generated code reached, so the
/// falsified harness item does not reproduce. It holds no predicate
/// verdict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarAgreement {
    claim: Box<ScalarClaim>,
    outcome: ScalarOutcome,
}

impl ScalarAgreement {
    pub(crate) fn new(claim: ScalarClaim, outcome: ScalarOutcome) -> Self {
        Self {
            claim: Box::new(claim),
            outcome,
        }
    }

    /// The claim that did not reproduce.
    pub fn claim(&self) -> &ScalarClaim {
        &self.claim
    }

    /// The outcome both sides reached.
    pub fn outcome(&self) -> &ScalarOutcome {
        &self.outcome
    }

    /// Bytes this agreement adds to an encoded record beyond its fixed
    /// size: its operands and ranges, or its bindings, and its outcome.
    pub(crate) fn measured_bytes(&self) -> usize {
        let claim = match &*self.claim {
            ScalarClaim::Operator(identity) => {
                3 * 32
                    + identity.operation.measured_bytes()
                    + identity.result_range.lower().to_string().len()
                    + identity.result_range.upper().to_string().len()
            }
            ScalarClaim::Function {
                function, bindings, ..
            } => {
                32 + function
                    .segments()
                    .iter()
                    .map(|segment| segment.as_str().len())
                    .sum::<usize>()
                    + bindings.iter().map(value_bytes).sum::<usize>()
            }
        };
        claim + self.outcome.measured_bytes()
    }
}

/// An operand outside the range it was drawn from, refused before any
/// evaluation (`invalid_runtime_input`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperandRefusal {
    /// The operand's position: 0 for the first, 1 for the second.
    pub index: usize,
}

impl OperandRefusal {
    /// The catalog code of this refusal.
    pub fn code(&self) -> Code {
        Code::InvalidRuntimeInput
    }
}

/// A generated-artifact fault: the native run stopped before it produced
/// an outcome to compare.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedFault {
    /// The native run stopped at a limit.
    Incomplete,
    /// The native run failed.
    ExecutionFault,
}

/// What an operator-level replay found. None of these is a `Refuted`
/// result.
#[derive(Debug)]
pub enum OperatorParityResult {
    /// The exact outcome and the generated one differ on the proof
    /// projection: a lowering defect, a generator fault
    /// (`TerminalValue::Failed`).
    Diverged {
        /// The exact outcome.
        exact: ScalarOutcome,
        /// The generated outcome, on the proof projection.
        generated: ScalarOutcome,
        /// The accounting charges the evaluation incurred.
        charges: ScalarLimits,
    },
    /// The exact outcome equals the generated one: the counterexample does
    /// not reproduce, which is `inconclusive` with
    /// `InconclusiveCause::ScalarAgrees`, a harness defect.
    Agrees {
        /// The claim and the outcome both sides reached.
        agreement: ScalarAgreement,
        /// The accounting charges the evaluation incurred.
        charges: ScalarLimits,
    },
    /// The native run was incomplete or failed: a generated-artifact fault
    /// (`TerminalValue::Failed`), settled without any comparison.
    GeneratedFault(GeneratedFault),
    /// An operand is outside its range, so no outcome is compared.
    RefusedInput(OperandRefusal),
    /// The exact evaluation reached a limit before it produced an outcome.
    Incomplete(Box<Incomplete>),
    /// The proving context did not recompile to the claim's package, or the
    /// claim's node or operator is not that package's: refused before any
    /// evaluation, as FR-098's identity checks refuse.
    Refused(Box<ReplayRefusal>),
}

/// What an operator-level replay settled, with the obligation identity of the
/// request carried through unchanged on every outcome.
#[derive(Debug)]
pub struct OperatorParityReport {
    obligation: ObligationIdentity,
    result: OperatorParityResult,
}

impl OperatorParityReport {
    pub(crate) fn new(obligation: ObligationIdentity, result: OperatorParityResult) -> Self {
        Self { obligation, result }
    }

    /// The obligation identity the request carried.
    pub fn obligation(&self) -> ObligationIdentity {
        self.obligation
    }

    /// What the replay found.
    pub fn result(&self) -> &OperatorParityResult {
        &self.result
    }

    /// The terminal value this report settles ([`OperatorParityResult::terminal_value`]).
    pub fn terminal_value(&self) -> TerminalValue {
        self.result.terminal_value()
    }
}

impl OperatorParityResult {
    /// The terminal value this result settles: `Failed` for a divergence or
    /// a generated-artifact fault, `Inconclusive(ScalarAgrees)` for an
    /// agreement, `Inconclusive(ReplayRefused)` with `invalid_runtime_input`
    /// for a refused operand and `Incomplete` for a reached limit.
    pub fn terminal_value(&self) -> TerminalValue {
        match self {
            Self::Diverged { .. } | Self::GeneratedFault(_) => TerminalValue::Failed,
            Self::Agrees { agreement, .. } => {
                TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(agreement.clone()))
            }
            Self::RefusedInput(refusal) => {
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(refusal.code()))
            }
            Self::Incomplete(_) => TerminalValue::Incomplete(IncompleteCause::ResourceExhausted),
            Self::Refused(refusal) => TerminalValue::from_replay_refusal(refusal),
        }
    }
}

/// An exact scalar evaluation: its outcome, or the limit it reached.
pub(crate) enum Evaluated {
    Outcome(ScalarOutcome),
    Incomplete(Box<Incomplete>),
}

/// The scalar outcome of a kernel outcome. A broken checked invariant is a
/// fault.
#[deny(clippy::wildcard_enum_match_arm)]
pub(crate) fn evaluated(outcome: Outcome<Value>) -> Result<Evaluated, InternalFault> {
    Ok(match outcome {
        Outcome::Completed(value) => Evaluated::Outcome(ScalarOutcome::Value(value)),
        Outcome::Undefined(_) => Evaluated::Outcome(ScalarOutcome::Undefined),
        Outcome::Refused(refusal) => match refusal {
            Refusal::DecimalOutOfDomain { .. }
            | Refusal::DivisionOutOfDomain { .. }
            | Refusal::ModuloOutOfDomain { .. }
            | Refusal::TextLengthOutOfDomain { .. }
            | Refusal::IntegerOutOfDomain { .. }
            | Refusal::RationalOutOfDomain { .. }
            | Refusal::IeeeRationalOutOfDomain { .. }
            | Refusal::CardinalityOutOfBound { .. } => {
                Evaluated::Outcome(ScalarOutcome::OutOfRange)
            }
            Refusal::InexactDecimal { .. }
            | Refusal::IeeeNotExact { .. }
            | Refusal::IeeeNanPayloadNotRepresentable { .. }
            | Refusal::ForeignReference { .. } => Evaluated::Outcome(ScalarOutcome::OtherRefusal),
            Refusal::CheckedInvariant => {
                return Err(InternalFault::new(
                    "replay",
                    "scalar-evaluation-keeps-its-checked-invariants",
                ))
            }
        },
        Outcome::Incomplete(incomplete) => Evaluated::Incomplete(Box::new(incomplete)),
    })
}

/// The comparison of an operator-level claim, once its proving context is
/// admitted. Each operand is admitted against its range first (a miss is
/// [`OperatorParityResult::RefusedInput`], with nothing evaluated); the
/// quire-exact operator then runs on the operands under the claim's limits,
/// with `result_range` as its result bound, and its outcome is compared with
/// the generated one on the proof projection.
///
/// The only `Err` is a broken quire-exact invariant.
pub(crate) fn compare(
    obligation: ObligationIdentity,
    claim: OperatorClaim,
) -> Result<OperatorParityResult, InternalFault> {
    let generated = match &claim.generated {
        NativeOutcome::Completed(value) => ScalarOutcome::Value(Value::Integer(value.clone())),
        NativeOutcome::RefusedOutOfRange => ScalarOutcome::OutOfRange,
        NativeOutcome::Undefined => ScalarOutcome::Undefined,
        NativeOutcome::Incomplete => {
            return Ok(OperatorParityResult::GeneratedFault(
                GeneratedFault::Incomplete,
            ))
        }
        NativeOutcome::ExecutionFault => {
            return Ok(OperatorParityResult::GeneratedFault(
                GeneratedFault::ExecutionFault,
            ))
        }
    };
    let (first, second) = claim.operation.operands();
    for (index, operand) in std::iter::once(first).chain(second).enumerate() {
        if !operand.admitted() {
            return Ok(OperatorParityResult::RefusedInput(OperandRefusal { index }));
        }
    }
    let mut meter = Meter::new(claim.limits);
    let range = &claim.result_range;
    let outcome = match &claim.operation {
        ScalarOperation::Add { left, right } => evaluate_integer_arithmetic(
            IntegerArithmetic::Add(&left.exact(), &right.exact()),
            Some(range),
            &mut meter,
        ),
        ScalarOperation::Subtract { left, right } => evaluate_integer_arithmetic(
            IntegerArithmetic::Subtract(&left.exact(), &right.exact()),
            Some(range),
            &mut meter,
        ),
        ScalarOperation::Multiply { left, right } => evaluate_integer_arithmetic(
            IntegerArithmetic::Multiply(&left.exact(), &right.exact()),
            Some(range),
            &mut meter,
        ),
        ScalarOperation::Negate { operand } => evaluate_integer_arithmetic(
            IntegerArithmetic::Negate(&operand.exact()),
            Some(range),
            &mut meter,
        ),
    };
    let charges = crate::execute::consumed(&meter);
    let exact = match evaluated(as_value(outcome))? {
        Evaluated::Outcome(exact) => exact,
        Evaluated::Incomplete(incomplete) => {
            return Ok(OperatorParityResult::Incomplete(incomplete))
        }
    };
    Ok(if !exact.same_as(&generated) {
        OperatorParityResult::Diverged {
            exact,
            generated,
            charges,
        }
    } else {
        OperatorParityResult::Agrees {
            agreement: ScalarAgreement::new(
                ScalarClaim::Operator(Box::new(OperatorIdentity {
                    obligation,
                    node: claim.node,
                    identity: claim.identity,
                    operation: claim.operation,
                    result_range: claim.result_range,
                    limits: claim.limits,
                })),
                exact,
            ),
            charges,
        }
    })
}

/// A kernel integer outcome as a kernel value outcome.
fn as_value(outcome: Outcome<Integer>) -> Outcome<Value> {
    match outcome {
        Outcome::Completed(integer) => Outcome::Completed(Value::Integer(integer)),
        Outcome::Undefined(undefined) => Outcome::Undefined(undefined),
        Outcome::Refused(refusal) => Outcome::Refused(refusal),
        Outcome::Incomplete(incomplete) => Outcome::Incomplete(incomplete),
    }
}
