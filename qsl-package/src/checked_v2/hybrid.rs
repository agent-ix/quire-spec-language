// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447-AC-13, AC-16 (QSpec FR-481): the I2 read-back of the hybrid models
//! and claims of an admitted `quire.checked-package/v2`.
//!
//! IR's reader has already refused every FR-481 defect, so this reads the
//! admitted nodes alone, with no source: the `hybrid` nodes give a
//! [`CheckedHybridModel`] and each `temporal_clause` node whose `over`
//! parameter is typed by a model node gives a [`CheckedHybridClaim`], with
//! the horizon derived from the formula nodes by the one rule S3 uses
//! ([`hybrid_horizon`]). The read is the inverse of S4's emission, so a
//! model or claim read back equals the checked one, with these exceptions
//! that the package, not this read, decides:
//!
//! - a model's position in [`HybridRead::models`] is its position in the
//!   graph, and a claim's `model` indexes that list; the package keeps no
//!   declaration order;
//! - a function a term calls is a [`CheckedFunctionRef`] indexing the
//!   package's `function` nodes in graph order ([`HybridRead::functions`]);
//! - a capture's declared type is carried only by the node of a read of the
//!   capture, so a capture no term reads has no type in the package.

use std::collections::BTreeMap;

use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::{
    hybrid_horizon, ArithmeticOp, CaptureIndex, CheckedFunctionRef, CheckedHybridClaim,
    CheckedHybridModel, CheckedMode, CheckedTransition, CompareOp, ContinuousVariable,
    HybridActivation, HybridCapture, HybridClock, HybridFairness, HybridFormula, HybridInterval,
    HybridModelIndex, HybridTerm, InitialEntry, LogicOp, ModeIndex, ResetConstraint,
    TemporalProfile, TransitionIndex, VariableClass, VariableIndex,
};
use quire_contract_model::{
    BoundedDomainForm, CheckedNodeId, CheckedNodeKind, CheckedPackageV2, CheckedSemanticNodeV2,
    CheckedTerm, HybridForm, LiteralKind, ScalarTypeForm, TemporalForm, ValueForm,
};
use quire_exact::{
    DecimalType, FloatType, Identifier, IeeeWidth, Integer, IntegerInterval, Rational,
    RationalDomain, RoundingMode, ValueType,
};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

/// The hybrid models and claims of an admitted package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridRead {
    /// Every `hybrid_model` node, in graph order.
    pub models: Vec<ReadHybridModel>,
    /// Every hybrid claim, in graph order; each `model` indexes
    /// [`Self::models`].
    pub claims: Vec<ReadHybridClaim>,
    /// Every `function` node, in graph order: the [`CheckedFunctionRef`] of a
    /// call indexes it.
    pub functions: Vec<WireNodeId>,
}

/// A model read back, with its node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadHybridModel {
    /// The `hybrid`/`hybrid_model` node.
    pub node: WireNodeId,
    /// The model.
    pub model: CheckedHybridModel,
}

/// A claim read back, with its node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadHybridClaim {
    /// The `temporal`/`temporal_clause` node.
    pub node: WireNodeId,
    /// The claim.
    pub claim: CheckedHybridClaim,
}

/// Why a hybrid read-back stopped.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridReadError {
    /// A node of an admitted package has a shape IR's reader admits and this
    /// read does not expect: a disagreement between the two readers.
    Fault(InternalFault),
    /// A capture no term of its claim reads has no declared type in the
    /// package.
    CaptureTypeNotCarried {
        /// The claim's clause node.
        claim: WireNodeId,
        /// The capture's name.
        capture: String,
    },
    /// The read visited more term and formula nodes than its bound.
    Limit {
        /// The bound.
        limit: u64,
    },
}

/// A broken invariant between IR's admission and this read.
const SHAPE: InternalFault = InternalFault::new("I2", "admitted-hybrid-shape");

fn shape<T>(found: Option<T>) -> Result<T, HybridReadError> {
    found.ok_or(HybridReadError::Fault(SHAPE))
}

/// Read every hybrid model and claim of `package`, visiting at most `limit`
/// term and formula nodes.
///
/// # Errors
///
/// [`HybridReadError`].
pub fn read_hybrids(package: &CheckedPackageV2, limit: u64) -> Result<HybridRead, HybridReadError> {
    let nodes = &package.graph().nodes;
    let kinds = package.node_kinds();
    let positions: BTreeMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (&*node.node_id.digest, position))
        .collect();
    let reader = Reader {
        package,
        positions,
        functions: nodes
            .iter()
            .zip(kinds)
            .filter(|(_, kind)| matches!(kind, CheckedNodeKind::Function(_)))
            .map(|(node, _)| &*node.node_id.digest)
            .collect(),
        limit,
        used: std::cell::Cell::new(0),
    };
    let mut models = Vec::new();
    let mut facts = Vec::new();
    for (node, kind) in nodes.iter().zip(kinds) {
        if *kind == CheckedNodeKind::Hybrid(HybridForm::Model) {
            let (model, fact) = reader.model(node)?;
            models.push(ReadHybridModel {
                node: wire_id(&node.node_id)?,
                model,
            });
            facts.push(fact);
        }
    }
    let mut claims = Vec::new();
    for (node, kind) in nodes.iter().zip(kinds) {
        if *kind != CheckedNodeKind::Temporal(TemporalForm::TemporalClause) {
            continue;
        }
        let over = reader.over(node)?;
        let Some(model_position) = facts
            .iter()
            .position(|fact| is_text(&fact.digest, &over.semantic_type.digest))
        else {
            continue;
        };
        let fact = &facts[model_position];
        let model = &models[model_position].model;
        claims.push(ReadHybridClaim {
            node: wire_id(&node.node_id)?,
            claim: reader.claim(
                node,
                over,
                model,
                fact,
                HybridModelIndex::new(model_position),
            )?,
        });
    }
    Ok(HybridRead {
        models,
        claims,
        functions: reader
            .functions
            .iter()
            .map(|digest| shape(WireNodeId::from_hex(digest)))
            .collect::<Result<_, _>>()?,
    })
}

fn wire_id(id: &CheckedNodeId) -> Result<WireNodeId, HybridReadError> {
    shape(WireNodeId::from_hex(&id.digest))
}

/// What a claim needs of the model it is over.
struct ModelFacts {
    /// The model node's digest.
    digest: Box<str>,
    /// Each variable node's digest, in variable order.
    variables: Vec<Box<str>>,
    /// Each mode node's digest, in mode order.
    modes: Vec<Box<str>>,
}

/// What one construct's terms may read.
struct Scope<'s> {
    /// Each variable node's digest and position.
    variables: BTreeMap<&'s str, usize>,
    /// The model's variable count; `0` outside a claim.
    count: usize,
    /// The declared type of each capture a term read, by capture position.
    captured: BTreeMap<usize, ValueType>,
}

struct Reader<'p> {
    package: &'p CheckedPackageV2,
    positions: BTreeMap<&'p str, usize>,
    functions: Vec<&'p str>,
    limit: u64,
    used: std::cell::Cell<u64>,
}

/// A term operation by its identity.
enum TermOperation {
    Negate,
    Not,
    Arithmetic(ArithmeticOp),
    Compare(CompareOp),
    Logic(LogicOp),
    If,
    Call,
}

#[ix_trace_rs::string_edge]
fn term_operation(identity: &str) -> Option<TermOperation> {
    Some(match identity {
        "quire.op.rational.negate" => TermOperation::Negate,
        "quire.op.boolean.not" => TermOperation::Not,
        "quire.op.rational.add" => TermOperation::Arithmetic(ArithmeticOp::Add),
        "quire.op.rational.sub" => TermOperation::Arithmetic(ArithmeticOp::Sub),
        "quire.op.rational.mul" => TermOperation::Arithmetic(ArithmeticOp::Mul),
        "quire.op.rational.div" => TermOperation::Arithmetic(ArithmeticOp::Div),
        "quire.op.rational.eq" => TermOperation::Compare(CompareOp::Eq),
        "quire.op.rational.ne" => TermOperation::Compare(CompareOp::Ne),
        "quire.op.rational.lt" => TermOperation::Compare(CompareOp::Lt),
        "quire.op.rational.le" => TermOperation::Compare(CompareOp::Le),
        "quire.op.rational.ge" => TermOperation::Compare(CompareOp::Ge),
        "quire.op.rational.gt" => TermOperation::Compare(CompareOp::Gt),
        "quire.op.boolean.and" => TermOperation::Logic(LogicOp::And),
        "quire.op.boolean.or" => TermOperation::Logic(LogicOp::Or),
        "quire.op.boolean.implies" => TermOperation::Logic(LogicOp::Implies),
        "quire.op.control.if" => TermOperation::If,
        "quire.op.function.call" => TermOperation::Call,
        _ => return None,
    })
}

/// A formula operation by its identity.
enum FormulaOperation {
    Constant(bool),
    Holds,
    Not,
    Logic(LogicOp),
    Always,
    Eventually,
    Historically,
    Once,
    Until,
    Release,
    Since,
    Triggered,
}

#[ix_trace_rs::string_edge]
fn formula_operation(identity: &str) -> Option<FormulaOperation> {
    Some(match identity {
        "quire.op.temporal.true" => FormulaOperation::Constant(true),
        "quire.op.temporal.false" => FormulaOperation::Constant(false),
        "quire.op.temporal.holds" => FormulaOperation::Holds,
        "quire.op.temporal.not" => FormulaOperation::Not,
        "quire.op.temporal.and" => FormulaOperation::Logic(LogicOp::And),
        "quire.op.temporal.or" => FormulaOperation::Logic(LogicOp::Or),
        "quire.op.temporal.implies" => FormulaOperation::Logic(LogicOp::Implies),
        "quire.op.temporal.always" => FormulaOperation::Always,
        "quire.op.temporal.eventually" => FormulaOperation::Eventually,
        "quire.op.temporal.historically" => FormulaOperation::Historically,
        "quire.op.temporal.once" => FormulaOperation::Once,
        "quire.op.temporal.until" => FormulaOperation::Until,
        "quire.op.temporal.release" => FormulaOperation::Release,
        "quire.op.temporal.since" => FormulaOperation::Since,
        "quire.op.temporal.triggered" => FormulaOperation::Triggered,
        _ => return None,
    })
}

/// The canonical spelling of a rational: `0`, `-3`, `1/2`.
#[ix_trace_rs::string_edge]
fn rational(text: &str) -> Option<Rational> {
    let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
    Rational::new(
        numerator.parse::<Integer>().ok()?,
        denominator.parse::<Integer>().ok()?,
    )
    .ok()
}

#[ix_trace_rs::string_edge]
fn clock(text: &str) -> Option<HybridClock> {
    match text {
        "model-time" => Some(HybridClock::ModelTime),
        "model-steps" => Some(HybridClock::ModelSteps),
        _ => None,
    }
}

#[ix_trace_rs::string_edge]
fn variable_class(text: &str) -> Option<VariableClass> {
    match text {
        "clock" => Some(VariableClass::Clock),
        "hybrid" => Some(VariableClass::Hybrid),
        _ => None,
    }
}

#[ix_trace_rs::string_edge]
fn relation(text: &str) -> Option<CompareOp> {
    match text {
        "ne" => Some(CompareOp::Ne),
        "lt" => Some(CompareOp::Lt),
        "le" => Some(CompareOp::Le),
        "ge" => Some(CompareOp::Ge),
        "gt" => Some(CompareOp::Gt),
        _ => None,
    }
}

#[ix_trace_rs::string_edge]
fn fairness_kind(text: &str) -> Option<FairnessKind> {
    match text {
        "weak" => Some(FairnessKind::Weak),
        "strong" => Some(FairnessKind::Strong),
        _ => None,
    }
}

#[ix_trace_rs::string_edge]
fn fairness_granularity(text: &str) -> Option<FairnessGranularity> {
    match text {
        "whole" => Some(FairnessGranularity::Whole),
        "each" => Some(FairnessGranularity::Each),
        _ => None,
    }
}

#[ix_trace_rs::string_edge]
fn is_text(text: &str, expected: &str) -> bool {
    text == expected
}

// ---------------------------------------------------------- body navigation

/// The members of the aggregate `term`.
fn members(term: &CheckedTerm) -> Result<&[CheckedTerm], HybridReadError> {
    match term {
        CheckedTerm::Aggregate { members } => Ok(members),
        _ => Err(HybridReadError::Fault(SHAPE)),
    }
}

/// The value bound to `name` in the aggregate `term`.
fn bound<'t>(term: &'t CheckedTerm, name: &str) -> Result<&'t CheckedTerm, HybridReadError> {
    shape(members(term)?.iter().find_map(|member| match member {
        CheckedTerm::Binding { name: bound, value } if is_text(bound, name) => Some(&**value),
        _ => None,
    }))
}

/// Whether the aggregate `term` binds `name`.
fn binds(term: &CheckedTerm, name: &str) -> bool {
    members(term).is_ok_and(|members| {
        members.iter().any(
            |member| matches!(member, CheckedTerm::Binding { name: bound, .. } if is_text(bound, name)),
        )
    })
}

/// The node a `reference` term names.
fn target(term: &CheckedTerm) -> Result<&CheckedNodeId, HybridReadError> {
    match term {
        CheckedTerm::Reference { target } => Ok(target),
        _ => Err(HybridReadError::Fault(SHAPE)),
    }
}

/// The text of a literal term.
fn text(term: &CheckedTerm) -> Result<&str, HybridReadError> {
    match term {
        CheckedTerm::Literal { value, .. } => shape(value.as_str()),
        _ => Err(HybridReadError::Fault(SHAPE)),
    }
}

/// The non-negative integer of a literal term.
fn number(term: &CheckedTerm) -> Result<usize, HybridReadError> {
    shape(text(term)?.parse().ok())
}

/// The last segment of `node`'s declared name.
fn last_segment(node: &CheckedSemanticNodeV2) -> Result<Identifier, HybridReadError> {
    let name = shape(node.declaration.as_ref())?;
    let segment = shape(name.qualified_name.last())?;
    shape(Identifier::new(&**segment).ok())
}

/// The wire record of an application operation, through its public
/// `Serialize` form (the typed carrier keeps its raw record internal).
fn record(operation: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(operation).unwrap_or_default()
}

/// The application at `node`'s body root.
fn application(term: &CheckedTerm) -> Result<(&[CheckedTerm], Option<String>), HybridReadError> {
    match term {
        CheckedTerm::Application {
            operation,
            arguments,
            ..
        } => Ok((
            arguments,
            record(operation)
                .get("identity")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
        )),
        _ => Err(HybridReadError::Fault(SHAPE)),
    }
}

/// A timed interval's bound `value`, `{numerator, denominator}` as decimal
/// strings. A macro, so the JSON type of `value` is never named.
macro_rules! timed_bound {
    ($value:expr) => {{
        let value = $value;
        let part = |name: &str| -> Option<Integer> { value.get(name)?.as_str()?.parse().ok() };
        part("numerator")
            .zip(part("denominator"))
            .and_then(|(numerator, denominator)| Rational::new(numerator, denominator).ok())
    }};
}

impl Reader<'_> {
    fn charge(&self) -> Result<(), HybridReadError> {
        let used = self.used.get().saturating_add(1);
        self.used.set(used);
        if used > self.limit {
            return Err(HybridReadError::Limit { limit: self.limit });
        }
        Ok(())
    }

    fn node(&self, id: &CheckedNodeId) -> Result<&CheckedSemanticNodeV2, HybridReadError> {
        let position = shape(self.positions.get(&*id.digest))?;
        shape(self.package.graph().nodes.get(*position))
    }

    fn kind(&self, id: &CheckedNodeId) -> Result<CheckedNodeKind, HybridReadError> {
        let position = shape(self.positions.get(&*id.digest))?;
        shape(self.package.node_kinds().get(*position).copied())
    }

    fn body(&self, node: &CheckedSemanticNodeV2) -> Result<&CheckedTerm, HybridReadError> {
        shape(self.package.term(&node.node_id))
    }

    /// `(name, level)` of the `value`/`parameter` node `node`.
    fn parameter(&self, node: &CheckedSemanticNodeV2) -> Result<(String, usize), HybridReadError> {
        let body = self.body(node)?;
        Ok((
            text(bound(body, "name")?)?.to_owned(),
            number(bound(body, "level")?)?,
        ))
    }

    /// The `over` parameter node of the clause `clause`.
    fn over(
        &self,
        clause: &CheckedSemanticNodeV2,
    ) -> Result<&CheckedSemanticNodeV2, HybridReadError> {
        let (arguments, _) = application(self.body(clause)?)?;
        self.node(target(shape(arguments.first())?)?)
    }

    // ------------------------------------------------------------- models

    fn model(
        &self,
        node: &CheckedSemanticNodeV2,
    ) -> Result<(CheckedHybridModel, ModelFacts), HybridReadError> {
        let body = self.body(node)?;
        let name = last_segment(node)?;

        let mut variables = Vec::new();
        let mut variable_digests = Vec::new();
        let mut scope = Scope {
            variables: BTreeMap::new(),
            count: 0,
            captured: BTreeMap::new(),
        };
        for (position, entry) in members(bound(body, "variables")?)?.iter().enumerate() {
            let parameter = self.node(target(bound(entry, "parameter")?)?)?;
            let class = shape(variable_class(text(bound(entry, "class")?)?))?;
            variables.push(ContinuousVariable {
                name: shape(Identifier::new(self.parameter(parameter)?.0).ok())?,
                class,
            });
            variable_digests.push(parameter.node_id.digest.clone());
            scope.variables.insert(&*parameter.node_id.digest, position);
        }

        let mode_nodes: Vec<&CheckedSemanticNodeV2> = members(bound(body, "modes")?)?
            .iter()
            .map(|mode| self.node(target(mode)?))
            .collect::<Result<_, _>>()?;
        let mut modes = Vec::with_capacity(mode_nodes.len());
        for mode in &mode_nodes {
            modes.push(self.mode(mode, &mut scope)?);
        }

        let initial = if binds(body, "initial") {
            let mut entries = Vec::new();
            for item in members(bound(body, "initial")?)? {
                entries.push(InitialEntry {
                    mode: ModeIndex::new(number(bound(item, "mode")?)?),
                    condition: self.term(target(bound(item, "condition")?)?, &mut scope)?,
                });
            }
            entries
        } else {
            vec![InitialEntry {
                mode: ModeIndex::new(0),
                condition: HybridTerm::Boolean(true),
            }]
        };

        let facts = ModelFacts {
            digest: node.node_id.digest.clone(),
            variables: variable_digests,
            modes: mode_nodes
                .iter()
                .map(|mode| mode.node_id.digest.clone())
                .collect(),
        };
        Ok((
            CheckedHybridModel {
                name,
                variables,
                initial,
                modes,
            },
            facts,
        ))
    }

    fn mode(
        &self,
        node: &CheckedSemanticNodeV2,
        scope: &mut Scope<'_>,
    ) -> Result<CheckedMode, HybridReadError> {
        let body = self.body(node)?;
        let mut rates = Vec::new();
        for rate in members(bound(body, "rates")?)? {
            rates.push(self.term(target(rate)?, scope)?);
        }
        let mut transitions = Vec::new();
        for transition in members(bound(body, "transitions")?)? {
            transitions.push(self.transition(self.node(target(transition)?)?, scope)?);
        }
        Ok(CheckedMode {
            name: last_segment(node)?,
            invariant: self.term(target(bound(body, "invariant")?)?, scope)?,
            rates,
            transitions,
        })
    }

    fn transition(
        &self,
        node: &CheckedSemanticNodeV2,
        scope: &mut Scope<'_>,
    ) -> Result<CheckedTransition, HybridReadError> {
        let body = self.body(node)?;
        let mut resets = Vec::new();
        for reset in members(bound(body, "resets")?)? {
            let relation = if binds(reset, "relation") {
                shape(relation(text(bound(reset, "relation")?)?))?
            } else {
                CompareOp::Eq
            };
            resets.push(ResetConstraint {
                variable: VariableIndex::new(number(bound(reset, "variable")?)?),
                relation,
                value: self.term(target(bound(reset, "value")?)?, scope)?,
            });
        }
        Ok(CheckedTransition {
            target: ModeIndex::new(number(bound(body, "target")?)?),
            guard: self.term(target(bound(body, "guard")?)?, scope)?,
            resets,
        })
    }

    // -------------------------------------------------------------- terms

    /// The term the node `root` is, over `scope`.
    fn term(
        &self,
        root: &CheckedNodeId,
        scope: &mut Scope<'_>,
    ) -> Result<HybridTerm, HybridReadError> {
        enum Step<'p> {
            Visit(&'p CheckedNodeId),
            Build(&'p CheckedSemanticNodeV2, TermOperation),
        }
        let mut steps = vec![Step::Visit(root)];
        let mut done: Vec<HybridTerm> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Visit(id) => {
                    self.charge()?;
                    let node = self.node(id)?;
                    match self.kind(id)? {
                        CheckedNodeKind::Value(ValueForm::Parameter) => {
                            done.push(self.read(node, scope)?);
                        }
                        CheckedNodeKind::Value(ValueForm::Literal) => {
                            let CheckedTerm::Literal {
                                value_kind, value, ..
                            } = self.body(node)?
                            else {
                                return Err(HybridReadError::Fault(SHAPE));
                            };
                            done.push(match value_kind {
                                LiteralKind::Rational => {
                                    HybridTerm::Constant(shape(rational(shape(value.as_str())?))?)
                                }
                                LiteralKind::Boolean => {
                                    HybridTerm::Boolean(shape(value.as_bool())?)
                                }
                                _ => return Err(HybridReadError::Fault(SHAPE)),
                            });
                        }
                        CheckedNodeKind::Expression(_) => {
                            let (arguments, identity) = application(self.body(node)?)?;
                            let operation = shape(term_operation(shape(identity.as_deref())?))?;
                            // A call's first argument is the callee.
                            let operands = match operation {
                                TermOperation::Call => shape(arguments.get(1..))?,
                                _ => arguments,
                            };
                            steps.push(Step::Build(node, operation));
                            for operand in operands.iter().rev() {
                                steps.push(Step::Visit(target(operand)?));
                            }
                        }
                        _ => return Err(HybridReadError::Fault(SHAPE)),
                    }
                }
                Step::Build(node, operation) => {
                    let (arguments, _) = application(self.body(node)?)?;
                    let mut pop = || shape(done.pop());
                    let built = match operation {
                        TermOperation::Negate => HybridTerm::Negate(Box::new(pop()?)),
                        TermOperation::Not => HybridTerm::Not(Box::new(pop()?)),
                        TermOperation::Arithmetic(op) => {
                            let (right, left) = (pop()?, pop()?);
                            HybridTerm::Arithmetic(op, Box::new(left), Box::new(right))
                        }
                        TermOperation::Compare(op) => {
                            let (right, left) = (pop()?, pop()?);
                            HybridTerm::Compare(op, Box::new(left), Box::new(right))
                        }
                        TermOperation::Logic(op) => {
                            let (right, left) = (pop()?, pop()?);
                            HybridTerm::Logic(op, Box::new(left), Box::new(right))
                        }
                        TermOperation::If => {
                            let (otherwise, then, condition) = (pop()?, pop()?, pop()?);
                            HybridTerm::Conditional(
                                Box::new(condition),
                                Box::new(then),
                                Box::new(otherwise),
                            )
                        }
                        TermOperation::Call => {
                            let callee = target(shape(arguments.first())?)?;
                            let function = shape(
                                self.functions
                                    .iter()
                                    .position(|digest| is_text(digest, &callee.digest)),
                            )?;
                            let mut operands = Vec::with_capacity(arguments.len());
                            for _ in 1..arguments.len() {
                                operands.push(pop()?);
                            }
                            operands.reverse();
                            HybridTerm::Call(CheckedFunctionRef::new(function), operands)
                        }
                    };
                    done.push(built);
                }
            }
        }
        match <[HybridTerm; 1]>::try_from(done) {
            Ok([term]) => Ok(term),
            Err(_) => Err(HybridReadError::Fault(SHAPE)),
        }
    }

    /// The read the `value`/`parameter` node `node` is: a model variable, an
    /// activation read at level `n + i` or a capture read at level `2n + j`.
    fn read(
        &self,
        node: &CheckedSemanticNodeV2,
        scope: &mut Scope<'_>,
    ) -> Result<HybridTerm, HybridReadError> {
        if let Some(&position) = scope.variables.get(&*node.node_id.digest) {
            return Ok(HybridTerm::Variable(VariableIndex::new(position)));
        }
        let (_, level) = self.parameter(node)?;
        let count = scope.count;
        if count == 0 || level < count {
            return Err(HybridReadError::Fault(SHAPE));
        }
        if level < 2 * count {
            return Ok(HybridTerm::ActivationVariable(VariableIndex::new(
                level - count,
            )));
        }
        let capture = level - 2 * count;
        let ty = self.value_type(&node.semantic_type)?;
        scope.captured.entry(capture).or_insert(ty);
        Ok(HybridTerm::Capture(CaptureIndex::new(capture)))
    }

    // -------------------------------------------------------------- types

    /// The `ValueType` the type node `id` is.
    fn value_type(&self, id: &CheckedNodeId) -> Result<ValueType, HybridReadError> {
        let node = self.node(id)?;
        let kind = self.kind(id)?;
        let literal = |name: &str| -> Result<Integer, HybridReadError> {
            shape(text(bound(self.body(node)?, name)?)?.parse().ok())
        };
        let scale = |name: &str| -> Result<u64, HybridReadError> {
            Ok(number(bound(self.body(node)?, name)?)? as u64)
        };
        match kind {
            CheckedNodeKind::ScalarType(ScalarTypeForm::Boolean) => Ok(ValueType::Boolean),
            CheckedNodeKind::ScalarType(ScalarTypeForm::Integer) => Ok(ValueType::Integer),
            CheckedNodeKind::BoundedDomain(BoundedDomainForm::IntegerRange) => {
                let interval = shape(IntegerInterval::new(literal("min")?, literal("max")?).ok())?;
                Ok(ValueType::Int(interval))
            }
            CheckedNodeKind::BoundedDomain(BoundedDomainForm::RationalRange) => {
                let numerator = shape(
                    IntegerInterval::new(literal("numerator_min")?, literal("numerator_max")?).ok(),
                )?;
                let denominator = shape(
                    IntegerInterval::new(literal("denominator_min")?, literal("denominator_max")?)
                        .ok(),
                )?;
                Ok(ValueType::Rational(shape(
                    RationalDomain::new(numerator, denominator).ok(),
                )?))
            }
            CheckedNodeKind::BoundedDomain(BoundedDomainForm::DecimalRange) => {
                let rounding = shape(RoundingMode::from_code(text(bound(
                    self.body(node)?,
                    "rounding",
                )?)?))?;
                Ok(ValueType::Decimal(shape(
                    DecimalType::new(
                        literal("coefficient_min")?,
                        literal("coefficient_max")?,
                        scale("scale_min")?,
                        scale("scale_max")?,
                        rounding,
                    )
                    .ok(),
                )?))
            }
            CheckedNodeKind::BoundedDomain(BoundedDomainForm::FloatRounding) => {
                let rounding = shape(RoundingMode::from_code(text(bound(
                    self.body(node)?,
                    "rounding",
                )?)?))?;
                let width = match self.kind(&node.semantic_type)? {
                    CheckedNodeKind::ScalarType(ScalarTypeForm::Float32) => IeeeWidth::Binary32,
                    CheckedNodeKind::ScalarType(ScalarTypeForm::Float64) => IeeeWidth::Binary64,
                    _ => return Err(HybridReadError::Fault(SHAPE)),
                };
                Ok(ValueType::Float(FloatType::new(width, rounding)))
            }
            _ => Err(HybridReadError::Fault(SHAPE)),
        }
    }

    // ------------------------------------------------------------- claims

    fn claim(
        &self,
        node: &CheckedSemanticNodeV2,
        over: &CheckedSemanticNodeV2,
        model: &CheckedHybridModel,
        facts: &ModelFacts,
        index: HybridModelIndex,
    ) -> Result<CheckedHybridClaim, HybridReadError> {
        let body = self.body(node)?;
        let (arguments, _) = application(body)?;
        let [_, clock_argument, activation_argument, captures_argument, fairness_argument, formula_argument] =
            arguments
        else {
            return Err(HybridReadError::Fault(SHAPE));
        };
        let CheckedTerm::Application { operation, .. } = body else {
            return Err(HybridReadError::Fault(SHAPE));
        };
        let record = record(operation);
        let profile = shape(
            record
                .get("laws")
                .and_then(|laws| laws.get(0))
                .and_then(|law| law.get("definition"))
                .and_then(|definition| definition.get("identity"))
                .and_then(|identity| identity.as_str())
                .and_then(TemporalProfile::from_identity),
        )?;
        let clock = shape(clock(text(clock_argument)?))?;
        let mut scope = Scope {
            variables: facts
                .variables
                .iter()
                .enumerate()
                .map(|(position, digest)| (&**digest, position))
                .collect(),
            count: model.variables.len(),
            captured: BTreeMap::new(),
        };

        let activation = if is_text(text(bound(activation_argument, "kind")?)?, "each") {
            let trigger = self.node(target(bound(activation_argument, "trigger")?)?)?;
            let guard = if binds(activation_argument, "guard") {
                Some(self.term(target(bound(activation_argument, "guard")?)?, &mut scope)?)
            } else {
                None
            };
            HybridActivation::Each {
                parameter: shape(Identifier::new(self.parameter(trigger)?.0).ok())?,
                guard,
            }
        } else {
            HybridActivation::Origin
        };

        let mut written = Vec::new();
        for capture in members(captures_argument)? {
            let CheckedTerm::Binding { name, value } = capture else {
                return Err(HybridReadError::Fault(SHAPE));
            };
            written.push((
                shape(Identifier::new(&**name).ok())?,
                self.term(target(value)?, &mut scope)?,
            ));
        }

        let formula = self.formula(target(formula_argument)?, &mut scope)?;

        let mut captures = Vec::with_capacity(written.len());
        for (position, (name, value)) in written.into_iter().enumerate() {
            let Some(ty) = scope.captured.get(&position).cloned() else {
                return Err(HybridReadError::CaptureTypeNotCarried {
                    claim: wire_id(&node.node_id)?,
                    capture: name.as_str().to_owned(),
                });
            };
            captures.push(HybridCapture { name, ty, value });
        }

        let mut fairness = Vec::new();
        for member in members(fairness_argument)? {
            fairness.push(self.fairness(self.node(target(member)?)?, model, facts)?);
        }

        let horizon = shape(hybrid_horizon(&formula, clock, &activation).ok())?;
        Ok(CheckedHybridClaim {
            model: index,
            profile,
            over: shape(Identifier::new(self.parameter(over)?.0).ok())?,
            clock,
            activation,
            captures,
            fairness,
            formula,
            horizon,
        })
    }

    /// The fairness constraint the `temporal`/`fairness` node `node` is: the
    /// transitions of one mode, or those of one mode that go to one target.
    fn fairness(
        &self,
        node: &CheckedSemanticNodeV2,
        model: &CheckedHybridModel,
        facts: &ModelFacts,
    ) -> Result<HybridFairness, HybridReadError> {
        let CheckedTerm::Application { operation, .. } = self.body(node)? else {
            return Err(HybridReadError::Fault(SHAPE));
        };
        let record = record(operation);
        let member = shape(record.get("member"))?;
        let field = |name: &str| shape(member.get(name).and_then(|value| value.as_str()));
        let declaration = shape(
            member
                .get("declaration")
                .and_then(|declaration| declaration.get("digest"))
                .and_then(|digest| digest.as_str()),
        )?;
        let name = field("name")?;
        let mode_named = |name: &str| {
            model
                .modes
                .iter()
                .position(|mode| is_text(mode.name.as_str(), name))
        };
        let (transitions, named_target): (Vec<(ModeIndex, TransitionIndex)>, Option<ModeIndex>) =
            if is_text(declaration, &facts.digest) {
                // `fair S`: every transition of `S`.
                let mode = shape(mode_named(name))?;
                (
                    (0..model.modes[mode].transitions.len())
                        .map(|position| (ModeIndex::new(mode), TransitionIndex::new(position)))
                        .collect(),
                    None,
                )
            } else {
                // `fair S::T`: the transitions of `S` that go to `T`.
                let mode = shape(
                    facts
                        .modes
                        .iter()
                        .position(|digest| is_text(digest, declaration)),
                )?;
                let target = shape(mode_named(name))?;
                (
                    model.modes[mode]
                        .transitions
                        .iter()
                        .enumerate()
                        .filter(|(_, transition)| transition.target.get() == target)
                        .map(|(position, _)| (ModeIndex::new(mode), TransitionIndex::new(position)))
                        .collect(),
                    Some(ModeIndex::new(target)),
                )
            };
        Ok(HybridFairness {
            kind: shape(fairness_kind(field("fairness_kind")?))?,
            granularity: shape(fairness_granularity(field("granularity")?))?,
            named_target,
            transitions,
        })
    }

    /// The formula the `temporal`/`formula` node `root` is.
    fn formula(
        &self,
        root: &CheckedNodeId,
        scope: &mut Scope<'_>,
    ) -> Result<HybridFormula, HybridReadError> {
        enum Step<'p> {
            Visit(&'p CheckedNodeId),
            Build(&'p CheckedSemanticNodeV2, FormulaOperation),
        }
        let mut steps = vec![Step::Visit(root)];
        let mut done: Vec<HybridFormula> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Visit(id) => {
                    self.charge()?;
                    let node = self.node(id)?;
                    let (arguments, identity) = application(self.body(node)?)?;
                    let operation = shape(formula_operation(shape(identity.as_deref())?))?;
                    match operation {
                        FormulaOperation::Constant(value) => {
                            done.push(HybridFormula::Holds(HybridTerm::Boolean(value)));
                        }
                        FormulaOperation::Holds => {
                            let operand = shape(arguments.first())?;
                            done.push(HybridFormula::Holds(self.term(target(operand)?, scope)?));
                        }
                        _ => {
                            steps.push(Step::Build(node, operation));
                            for operand in arguments.iter().rev() {
                                steps.push(Step::Visit(target(operand)?));
                            }
                        }
                    }
                }
                Step::Build(node, operation) => {
                    let interval = self.interval(node)?;
                    let mut pop = || shape(done.pop());
                    let built = match operation {
                        FormulaOperation::Not => HybridFormula::Not(Box::new(pop()?)),
                        FormulaOperation::Logic(op) => {
                            let (right, left) = (pop()?, pop()?);
                            HybridFormula::Logic(op, Box::new(left), Box::new(right))
                        }
                        FormulaOperation::Always => {
                            HybridFormula::Always(interval, Box::new(pop()?))
                        }
                        FormulaOperation::Eventually => {
                            HybridFormula::Eventually(interval, Box::new(pop()?))
                        }
                        FormulaOperation::Historically => {
                            HybridFormula::Historically(interval, Box::new(pop()?))
                        }
                        FormulaOperation::Once => HybridFormula::Once(interval, Box::new(pop()?)),
                        FormulaOperation::Until => {
                            let (right, left) = (pop()?, pop()?);
                            HybridFormula::Until(interval, Box::new(left), Box::new(right))
                        }
                        FormulaOperation::Release => {
                            let (right, left) = (pop()?, pop()?);
                            HybridFormula::Release(interval, Box::new(left), Box::new(right))
                        }
                        FormulaOperation::Since => {
                            let (right, left) = (pop()?, pop()?);
                            HybridFormula::Since(interval, Box::new(left), Box::new(right))
                        }
                        FormulaOperation::Triggered => {
                            let (right, left) = (pop()?, pop()?);
                            HybridFormula::Triggered(interval, Box::new(left), Box::new(right))
                        }
                        FormulaOperation::Constant(_) | FormulaOperation::Holds => {
                            return Err(HybridReadError::Fault(SHAPE))
                        }
                    };
                    done.push(built);
                }
            }
        }
        match <[HybridFormula; 1]>::try_from(done) {
            Ok([formula]) => Ok(formula),
            Err(_) => Err(HybridReadError::Fault(SHAPE)),
        }
    }

    /// The interval of the formula node `node`'s operator, in the integer or
    /// the timed form (QSpec FR-370); `None` for an operator with none.
    fn interval(
        &self,
        node: &CheckedSemanticNodeV2,
    ) -> Result<Option<HybridInterval>, HybridReadError> {
        let CheckedTerm::Application { operation, .. } = self.body(node)? else {
            return Err(HybridReadError::Fault(SHAPE));
        };
        let record = record(operation);
        let Some(interval) = record
            .get("member")
            .and_then(|member| member.get("interval"))
            .filter(|interval| !interval.is_null())
        else {
            return Ok(None);
        };
        let lower = shape(interval.get("lower"))?;
        let upper = shape(interval.get("upper"))?;
        if lower.is_string() {
            // The integer form: both ends closed, `upper` `null` for `[a, *]`.
            let whole = |text: &str| shape(rational(text));
            return Ok(Some(HybridInterval {
                lower: whole(shape(lower.as_str())?)?,
                lower_open: false,
                upper: match upper.as_str() {
                    Some(upper) => Some(whole(upper)?),
                    None => None,
                },
                upper_open: upper.is_null(),
            }));
        }
        let end = |name: &str| -> Result<bool, HybridReadError> {
            Ok(is_text(
                shape(interval.get(name).and_then(|end| end.as_str()))?,
                "open",
            ))
        };
        Ok(Some(HybridInterval {
            lower: shape(timed_bound!(lower))?,
            lower_open: end("lower_end")?,
            upper: if upper.is_null() {
                None
            } else {
                Some(shape(timed_bound!(upper))?)
            },
            upper_open: end("upper_end")?,
        }))
    }
}
