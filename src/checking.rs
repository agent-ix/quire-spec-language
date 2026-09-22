// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-006/016: native constraints and guarded IR proofs over exact authored input.

mod bindings;
pub mod composed;
mod constraints;
mod inputs;
mod proof;
mod types;
mod variables;

use quire_contract_ir as ir;

use crate::formal_source::FormalSource;
use crate::linking::{DeclarationLocation, LinkedPackage};
use crate::native_model::{NativeModel, ObjectRole, OperationRole};
use crate::syntax::ExprId;
use crate::{Code, Diagnostic, Phase, Source, Span};

pub use types::NativeType;
pub(crate) use types::{Catalog, FrameIndex};

type Result<T> = std::result::Result<T, Box<CheckingError>>;

/// A checking refusal, keeping an original upstream IR proof refusal separate
/// from the reusable [`Diagnostic`] shape (ADR-011 §6.1: `diagnostic` does not
/// import IR types).
#[derive(Clone, Debug)]
pub struct CheckingError {
    /// Stable code, native source locus and human-readable explanation.
    pub diagnostic: Box<Diagnostic>,
    /// Related formal declarations, sorted by identity and source location.
    pub related: Vec<DeclarationLocation>,
    /// Structured upstream IR proof refusal, when the proof engine rejected it.
    pub upstream: Option<Box<ir::Diagnostic>>,
}

impl std::fmt::Display for CheckingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.diagnostic, formatter)
    }
}

impl std::error::Error for CheckingError {}

impl From<Box<crate::linking::LinkingError>> for Box<CheckingError> {
    fn from(error: Box<crate::linking::LinkingError>) -> Self {
        Box::new(CheckingError {
            diagnostic: error.diagnostic,
            related: error.related,
            upstream: error.upstream,
        })
    }
}

impl From<Box<crate::formal_source::FormalSourceError>> for Box<CheckingError> {
    fn from(error: Box<crate::formal_source::FormalSourceError>) -> Self {
        Box::new(CheckingError {
            diagnostic: error.diagnostic,
            related: Vec::new(),
            upstream: error.upstream,
        })
    }
}

/// Authored correspondence for one native clause, supplied rather than minted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseBinding {
    /// Exact clause name in the native source.
    pub name: String,
    /// Authored requirement owner of the clause and its proof environment.
    pub requirement: ir::RequirementRef,
    /// Authored local clause identity under that requirement.
    pub clause: ir::ClauseId,
    /// Explicit initialization, handler or selected operation point.
    pub execution_point: ir::ExecutionPoint,
}

/// Exact native source and complete authored clause correspondence.
#[derive(Clone, Debug)]
pub struct CheckBindings {
    /// Original native bytes with the caller's explicit formal source identity.
    pub source: FormalSource,
    /// Complete mapping; input order need not be source order.
    pub clauses: Vec<ClauseBinding>,
}

/// Caller-lowered budgets for native constraints and actual proof expansion.
#[derive(Clone, Copy, Debug)]
pub struct CheckLimits {
    /// Original native expression nodes, at most 10,000.
    pub nodes: usize,
    /// Native and expanded proof depth, at most 64.
    pub depth: usize,
    /// Generated proof input values, at most 10,000.
    pub proof_values: usize,
    /// Shared proof graph nodes, at most 100,000.
    pub proof_graph_nodes: usize,
    /// Inspected presence-fact entries, at most 100,000.
    pub presence_work: usize,
    /// Total materialized IR nodes, at most 100,000.
    pub materialized_nodes: usize,
    /// Materialized nodes in any one IR goal, at most 10,000.
    pub goal_nodes: usize,
}

impl Default for CheckLimits {
    fn default() -> Self {
        Self {
            nodes: 10_000,
            depth: 64,
            proof_values: 10_000,
            proof_graph_nodes: 100_000,
            presence_work: 100_000,
            materialized_nodes: 100_000,
            goal_nodes: 10_000,
        }
    }
}

impl CheckLimits {
    fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            nodes: self.nodes.min(hard.nodes),
            depth: self.depth.min(hard.depth),
            proof_values: self.proof_values.min(hard.proof_values),
            proof_graph_nodes: self.proof_graph_nodes.min(hard.proof_graph_nodes),
            presence_work: self.presence_work.min(hard.presence_work),
            materialized_nodes: self.materialized_nodes.min(hard.materialized_nodes),
            goal_nodes: self.goal_nodes.min(hard.goal_nodes),
        }
    }
}

/// Measured work retained only after an atomic successful check.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CheckUsage {
    /// Original syntax nodes admitted by preflight.
    pub native_nodes: usize,
    /// Maximum visited native depth.
    pub max_native_depth: usize,
    /// Generated proof values.
    pub proof_values: usize,
    /// Shared graph nodes created.
    pub proof_graph_nodes: usize,
    /// Presence entries inspected during fact propagation.
    pub presence_work: usize,
    /// IR nodes created across all goals.
    pub materialized_nodes: usize,
    /// Largest materialized goal.
    pub max_goal_nodes: usize,
    /// Greatest expanded IR depth; never reaches the IR worker threshold.
    pub max_proof_depth: usize,
}

/// Observation of a native value, including a stable selected conditional value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Observation {
    /// Literal or computed value independent of a population observation.
    Independent,
    /// One concrete input observation.
    Snapshot(ir::StateObservation),
    /// A conditional value whose alternative observations differ.
    Selected(ExprId),
}

/// A universe whose finite population must be validated before execution.
#[derive(Clone, Debug)]
pub struct UniverseRequirement<'a> {
    /// Exact admitted model supplying the role.
    pub model: &'a NativeModel,
    /// Exact target object/reference/universe role.
    pub object: &'a ObjectRole,
    /// Concrete observations needed by the original native expressions.
    pub observations: Vec<ir::StateObservation>,
}

/// Input obligations retained by static checking; no population is supplied yet.
#[derive(Clone, Debug)]
pub struct RuntimeRequirements<'a> {
    /// Exact linked context and its original model locus.
    pub context: DeclarationLocation,
    /// Admitted model supplying that context.
    pub model: &'a NativeModel,
    /// Required context observations, including post self for constant postconditions.
    pub context_observations: Vec<ir::StateObservation>,
    /// Universe/observation validation needed by reference and context values.
    pub universes: Vec<UniverseRequirement<'a>>,
    /// Exact operation, parameters, result and frame, when applicable.
    pub operation: Option<&'a OperationRole>,
    /// Requires invocation and pre/post frame validation before execution.
    pub validate_frame: bool,
}

/// Actual IR-discharge result corresponding to one native evaluation point.
#[derive(Clone, Debug)]
pub struct ProofGoal {
    native: ExprId,
    source: ir::SourceSpan,
    checked: ir::TypedExpression,
    premises: Vec<PresencePremise>,
}

impl ProofGoal {
    /// Original AST node whose potentially partial operation was checked.
    pub fn native_expression(&self) -> ExprId {
        self.native
    }
    /// Exact original native source mapped to its explicit formal identity.
    pub fn source(&self) -> &ir::SourceSpan {
        &self.source
    }
    /// Actual upstream check_expression result, never an executable native lowering.
    pub fn checked(&self) -> &ir::TypedExpression {
        &self.checked
    }
    /// Sound native presence consequences inserted before the IR goal.
    pub fn premises(&self) -> &[PresencePremise] {
        &self.premises
    }
}

/// One presence consequence of an actual preceding native guard outcome.
#[derive(Clone, Debug)]
pub struct PresencePremise {
    /// Native guard whose selected outcome establishes this fact.
    pub guard: ExprId,
    /// Selected truth outcome of that guard.
    pub outcome: bool,
    /// Original occurrence of the structured optional value.
    pub value: ExprId,
    /// True for presence, false for absence.
    pub present: bool,
}

/// One generated proof input and its original native/model correspondence.
#[derive(Clone, Debug)]
pub struct ProofValue {
    /// Symbol in the authored clause's proof environment.
    pub symbol: ir::SymbolName,
    /// First original native occurrence represented by this input.
    pub expression: ExprId,
    /// Original native source occurrence under the explicit formal identity.
    pub source: ir::SourceSpan,
    /// Original model declarations, where this symbolic value denotes a read.
    pub declarations: Vec<DeclarationLocation>,
}

/// Native typing and conditional definedness for one source-ordered clause.
#[derive(Debug)]
pub struct CheckedClause<'a> {
    binding: ClauseBinding,
    nodes: Vec<constraints::NodeType<'a>>,
    environment: ir::DeclarationEnvironment,
    values: Vec<ProofValue>,
    proofs: Vec<ProofGoal>,
    runtime: RuntimeRequirements<'a>,
}

impl<'a> CheckedClause<'a> {
    /// Borrow each established type once for complete package feature discovery.
    pub(crate) fn expression_types(&self) -> impl Iterator<Item = &NativeType<'a>> {
        self.nodes.iter().map(|node| &node.ty)
    }
    /// Exact authored clause mapping retained in source order.
    pub fn binding(&self) -> &ClauseBinding {
        &self.binding
    }
    /// Type of an original expression belonging to this clause.
    pub fn expression_type(&self, expression: ExprId) -> Option<&NativeType<'a>> {
        self.node(expression).map(|node| &node.ty)
    }
    /// Captured observation; applying pre to a local does not retag it.
    pub fn observation(&self, expression: ExprId) -> Option<Observation> {
        self.node(expression).map(|node| node.observation)
    }
    fn node(&self, expression: ExprId) -> Option<&constraints::NodeType<'a>> {
        self.nodes
            .binary_search_by_key(&expression.0, |n| n.expression.0)
            .ok()
            .map(|index| &self.nodes[index])
    }
    /// Complete generated Input declarations owned by the authored requirement.
    pub fn proof_environment(&self) -> &ir::DeclarationEnvironment {
        &self.environment
    }
    /// Explicit correspondence for every generated proof input.
    pub fn proof_values(&self) -> &[ProofValue] {
        &self.values
    }
    /// Discharged goals in native evaluation traversal order.
    pub fn proofs(&self) -> &[ProofGoal] {
        &self.proofs
    }
    /// Population, context, invocation and frame obligations for the runtime.
    pub fn runtime_requirements(&self) -> &RuntimeRequirements<'a> {
        &self.runtime
    }
}

/// Atomically checked clauses retaining their original source and linked AST.
#[derive(Debug)]
pub struct CheckedPackage<'a> {
    linked: LinkedPackage<'a>,
    bindings: CheckBindings,
    clauses: Vec<CheckedClause<'a>>,
    usage: CheckUsage,
    catalogs: Vec<Catalog<'a>>,
}

impl<'a> CheckedPackage<'a> {
    pub(crate) fn catalogs(&self) -> &[Catalog<'a>] {
        &self.catalogs
    }
    /// Original native syntax, source, imports and model occurrences.
    pub fn linked(&self) -> &LinkedPackage<'a> {
        &self.linked
    }
    /// Exact caller-supplied source and authored clause bindings.
    pub fn bindings(&self) -> &CheckBindings {
        &self.bindings
    }
    /// Checked clauses in original source order.
    pub fn clauses(&self) -> &[CheckedClause<'a>] {
        &self.clauses
    }
    /// Actual consumed work and observed depths for this successful check.
    pub fn usage(&self) -> &CheckUsage {
        &self.usage
    }
}

fn failure(
    source: &Source,
    code: Code,
    span: Span,
    message: impl Into<String>,
) -> Box<CheckingError> {
    Box::new(CheckingError {
        diagnostic: crate::diagnostic::error(
            source,
            code,
            Phase::Check,
            span.start,
            span.end,
            message,
        ),
        related: Vec::new(),
        upstream: None,
    })
}

/// Establish native types and guarded definedness under explicit input obligations.
///
/// # Errors
/// Refuses foreign bindings, ill-typed or unavailable expressions, failed actual
/// IR proof goals and exhausted budgets without exposing a partial checked package.
pub fn check<'a>(
    linked: LinkedPackage<'a>,
    bindings: CheckBindings,
    limits: CheckLimits,
) -> Result<CheckedPackage<'a>> {
    let limits = limits.bounded();
    let order = bindings::validate(&linked, &bindings)?;
    if linked.unit().expressions().len() > limits.nodes {
        return Err(failure(
            linked.unit().source(),
            Code::ResourceExhausted,
            Span { start: 0, end: 0 },
            "native checking node limit exceeded",
        ));
    }
    let mut usage = CheckUsage {
        native_nodes: linked.unit().expressions().len(),
        ..CheckUsage::default()
    };
    let catalogs = linked
        .models()
        .iter()
        .map(|selected| {
            selected
                .native_model()
                .map(types::Catalog::historical)
                .ok_or_else(|| {
                    failure(
                        linked.unit().source(),
                        Code::InvalidModelBinding,
                        Span { start: 0, end: 0 },
                        "selected model lacks native correspondence",
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut clauses = Vec::new();
    for (index, binding_index) in order.into_iter().enumerate() {
        let binding = &bindings.clauses[binding_index];
        let typed = constraints::solve(&linked, &catalogs, index, limits, &mut usage)?;
        let proven = proof::discharge(proof::Request {
            linked: &linked,
            catalogs: &catalogs,
            index,
            binding,
            formal: &bindings.source,
            types: &typed.nodes,
            limits,
            usage: &mut usage,
        })?;
        clauses.push(CheckedClause {
            binding: binding.clone(),
            nodes: typed.nodes,
            environment: proven.environment,
            values: proven.values,
            proofs: proven.goals,
            runtime: typed.runtime,
        });
    }
    Ok(CheckedPackage {
        linked,
        bindings,
        clauses,
        usage,
        catalogs,
    })
}
