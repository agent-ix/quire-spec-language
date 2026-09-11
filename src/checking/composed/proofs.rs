// SPDX-License-Identifier: AGPL-3.0-only
//! FR-040: authored guarded-definedness evidence, never an executable lowering.

mod correspondence;
mod dependencies;
mod engine;
pub mod work;

use super::{Site, TypeReport};
use crate::checking::{CheckBindings, ClauseBinding, ProofGoal, ProofValue};
use crate::linking::composed::{DeclarationId, UnitId};
use quire_contract_ir as ir;

pub use work::{Exhaustion, Limits as ProofLimits, Usage as ProofUsage};

/// Definedness only; even discharge does not admit a family or runtime input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofDisposition {
    /// Supported expressions discharged through the actual supplied IR interface.
    Discharged,
    /// A known type, correspondence, proof or dependent refusal remains.
    Refused,
    /// Required input or charged work did not finish.
    Unfinished,
}

/// Missing proof representation is distinct from an invalid authored value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Unsupported {
    /// Ordered query semantics cannot yet be represented by this proof graph.
    OrderedQuery,
    /// A required native value has no sound conversion through the existing IR.
    ValueRepresentation,
}

/// Exact source/owner admission failures, independent of expression validity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CorrespondenceError {
    /// No supplied source matches the original unit's identity, path and bytes.
    MissingSource,
    /// More than one supplied source selects the same unit or formal identity.
    DuplicateSource,
    /// Exact content/path differs, or a formal identity conflicts with a model.
    ForeignSource,
    /// The authored source lacks this declaration's mapping.
    MissingDeclaration,
    /// A mapping names no declaration in its exact source unit.
    ForeignDeclaration,
    /// A native name or authored requirement/clause identity was selected twice.
    DuplicateDeclaration,
    /// The execution point does not match the declaration's selected operation.
    ExecutionPoint,
}

/// Structured local proof failures retain the actual upstream diagnostics.
#[derive(Debug)]
#[non_exhaustive]
pub enum CauseKind {
    /// The original type stage already refused this declaration.
    UpstreamType,
    /// Authored correspondence failed before any dependent discharge.
    Correspondence(CorrespondenceError),
    /// A supported obligation was not proved; this is not a counterexample.
    Unproved { diagnostics: Vec<ir::Diagnostic> },
    /// The selected interface cannot supply this required meaning.
    Unsupported(Unsupported),
    /// An actual native dependency did not discharge.
    Dependency { target: DeclarationId },
}

/// One original native locus and its typed proof cause.
#[derive(Debug)]
pub struct Cause {
    /// Source and owning declaration; expression handles stay unit-local.
    pub site: Site,
    /// No diagnostic message parsing determines this classification.
    pub kind: CauseKind,
}

/// Borrowed positions in the supplied exact correspondence inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthoredBinding {
    /// Index in ProofReport::bindings().
    pub source: usize,
    /// Index in that source's CheckBindings::clauses.
    pub clause: usize,
}

/// Constructor-private evidence for one declaration's value-definedness stage.
#[derive(Debug)]
pub struct DeclarationProof {
    declaration: DeclarationId,
    unit: UnitId,
    binding: Option<AuthoredBinding>,
    environment: Option<ir::DeclarationEnvironment>,
    values: Vec<ProofValue>,
    goals: Vec<ProofGoal>,
    causes: Vec<Cause>,
    local_complete: bool,
    complete: bool,
}

impl DeclarationProof {
    /// Original native declaration; all retained expression IDs belong to unit().
    pub fn declaration(&self) -> DeclarationId {
        self.declaration
    }
    /// Original source unit, separating identical expression offsets elsewhere.
    pub fn unit(&self) -> UnitId {
        self.unit
    }
    /// Exact supplied owner/clause mapping, when correspondence admitted it.
    pub fn authored_binding(&self) -> Option<AuthoredBinding> {
        self.binding
    }
    /// Symbolic definedness environment under the supplied requirement owner.
    /// Numeric domains and option/sequence structure are retained; opaque native
    /// values use Boolean witnesses. Consult types() on the report for native
    /// types. This environment is proof input, never executable source IR.
    pub fn environment(&self) -> Option<&ir::DeclarationEnvironment> {
        self.environment.as_ref()
    }
    /// Symbolic definedness inputs; these are not executable source expressions.
    pub fn values(&self) -> &[ProofValue] {
        &self.values
    }
    /// Actual upstream TypedExpression results with original source spans.
    pub fn goals(&self) -> &[ProofGoal] {
        &self.goals
    }
    /// Known causes retained even when later work exhausts its budget.
    pub fn causes(&self) -> &[Cause] {
        &self.causes
    }
    /// Local discharge and all required native dependencies have completed.
    pub fn complete(&self) -> bool {
        self.complete && self.causes.is_empty()
    }
    /// Proof-stage result, never interchangeable with executable admission.
    pub fn disposition(&self) -> ProofDisposition {
        if !self.causes.is_empty() {
            ProofDisposition::Refused
        } else if self.complete {
            ProofDisposition::Discharged
        } else {
            ProofDisposition::Unfinished
        }
    }
}

/// Immutable proof evidence borrowing the exact type result and authored bindings.
#[derive(Debug)]
pub struct ProofReport<'p, 'r, 'a> {
    types: &'p TypeReport<'r, 'a>,
    bindings: &'p [CheckBindings],
    declarations: Vec<DeclarationProof>,
    exhaustion: Option<Exhaustion>,
    limits: ProofLimits,
    usage: ProofUsage,
}

impl<'p, 'r, 'a> ProofReport<'p, 'r, 'a> {
    /// Original type facts, profiles, model selections and pending runtime roles.
    pub fn types(&self) -> &'p TypeReport<'r, 'a> {
        self.types
    }
    /// Caller-supplied original sources and semantic owners, never inferred here.
    pub fn bindings(&self) -> &'p [CheckBindings] {
        self.bindings
    }
    /// Partial declaration evidence in namespace order.
    pub fn declarations(&self) -> &[DeclarationProof] {
        &self.declarations
    }
    /// One original declaration, absent if work did not reach it.
    pub fn declaration(&self, id: DeclarationId) -> Option<&DeclarationProof> {
        self.declarations.get(id.index())
    }
    /// Exact authored clause selection retained by this declaration.
    pub fn clause_binding(&self, id: DeclarationId) -> Option<&'p ClauseBinding> {
        let at = self.declaration(id)?.binding?;
        Some(&self.bindings[at.source].clauses[at.clause])
    }
    /// Native type refusal remains visible even before proof work reaches it.
    pub fn disposition(&self, id: DeclarationId) -> Option<ProofDisposition> {
        let typed = self.types.disposition(id)?;
        Some(self.declaration(id).map_or_else(
            || match typed {
                super::TypeDisposition::Refused => ProofDisposition::Refused,
                super::TypeDisposition::Typed | super::TypeDisposition::Unfinished => {
                    ProofDisposition::Unfinished
                }
            },
            DeclarationProof::disposition,
        ))
    }
    /// First unaffordable operation; retries use fresh counters and immutable inputs.
    pub fn exhaustion(&self) -> Option<&Exhaustion> {
        self.exhaustion.as_ref()
    }
    /// Effective caller-lowered capacities after independent hard clamping.
    pub fn limits(&self) -> ProofLimits {
        self.limits
    }
    /// Successful charges, including partial graph/materialization work.
    pub fn usage(&self) -> ProofUsage {
        self.usage
    }
}

/// Discharge supported native value obligations through the actual pinned IR.
/// Unsupported representations remain explicit and leave full FR-040 open.
pub fn discharge<'p, 'r, 'a>(
    types: &'p TypeReport<'r, 'a>,
    bindings: &'p [CheckBindings],
    limits: ProofLimits,
) -> ProofReport<'p, 'r, 'a> {
    engine::discharge(types, bindings, limits)
}
