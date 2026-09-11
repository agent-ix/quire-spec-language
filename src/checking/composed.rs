// SPDX-License-Identifier: AGPL-3.0-only
//! FR-040: composed profile/type admission before definedness and family checking.

pub mod proofs;
mod solver;
mod sources;
pub mod work;

use super::NativeType;
use crate::formal_source::FormalSource;
use crate::linking::composed::definition_source::RegisteredDefinition;
use crate::linking::composed::scopes::{Anchor, BinderId};
use crate::linking::composed::{binding, DeclarationId, UnitId};
use crate::syntax::ExprId;
use crate::Span;

pub use work::{Exhaustion, Limits as TypeLimits, Usage as TypeUsage};

/// Type admission only. None of these states asserts definedness or execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeDisposition {
    /// All profile/type constraints completed; proof obligations remain explicit.
    Typed,
    /// A known local or dependent refusal prevents complete type admission.
    Refused,
    /// Required input or charged type work did not complete.
    Unfinished,
}

/// Original owning declaration/unit, with unit-local syntax handles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Site {
    /// Owning declaration in the retained binding namespace.
    pub declaration: DeclarationId,
    /// Original unit containing every expression handle and span in this site.
    pub unit: UnitId,
    /// Original value occurrence, absent for declaration-level work.
    pub expression: Option<ExprId>,
    /// Half-open byte region in that unit's immutable native source.
    pub span: Span,
}

/// An unavailable existing interface, distinct from an invalid value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Prerequisite {
    /// No unique exact formal source corresponds to this authored native unit.
    FormalSource,
    /// The existing IR literal normalizer cannot represent the raw components.
    RationalNormalization,
}

/// A typed refusal; source regions and original selections remain in the report.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CauseKind {
    /// A required source, definition, model or lexical binding already refused.
    UpstreamBinding,
    /// The declaration's selected profile does not admit this form.
    ProfilePermission,
    /// Constraints require incompatible native types.
    TypeMismatch,
    /// No unique contextual native type can be established.
    AmbiguousType,
    /// The literal exceeds its selected scalar's exact domain.
    LiteralDomain,
    /// A rational literal has a zero denominator before normalization.
    ZeroDenominator,
    /// The explicit arguments do not match the authored predicate signature.
    CallArity {
        /// Parameter count in the exact callee declaration.
        expected: usize,
        /// Argument count in the original call occurrence.
        actual: usize,
    },
    /// The selected type or profile does not admit this operator.
    ForbiddenOperator,
    /// The operation requires an optional value.
    ExpectedOption,
    /// The operation requires an ordered finite sequence.
    ExpectedSequence,
    /// Dereference requires an explicitly admitted reference type.
    ExpectedReference,
    /// The receiver has no admissible member with this name.
    InvalidField,
    /// Graph operands or the selected edge do not share the required universe.
    InvalidGraphEdge,
    /// Aggregate capacity, unit or numeric representation is incompatible.
    InvalidAggregateDomain,
    /// A selected model contains a domain outside the native profile's limits.
    ModelDomain {
        /// Original input index in the retained model binding inventory.
        input: usize,
    },
    /// The selector lacks an eligible pre read or attempts to retag a capture.
    InvalidPreSelection,
    /// An exact native dependency has refused type admission.
    Dependency {
        /// Refused declaration in the original binding namespace.
        target: DeclarationId,
    },
    /// A required source or existing producer interface cannot supply the meaning.
    UnsupportedPrerequisite(Prerequisite),
}

/// A known cause, including the exact profile use whose interpretation applies.
#[derive(Clone, Debug)]
pub struct TypeCause {
    /// Original owner and location of the failed constraint.
    pub site: Site,
    /// Structured reason; no diagnostic message parsing determines its kind.
    pub kind: CauseKind,
    /// Index in the owning declaration's definition uses for an expression cause.
    /// Absent when the cause precedes typing or is independent of a profile use.
    pub profile: Option<usize>,
}

/// Future proof/runtime admission must satisfy these; typing does not discharge them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ObligationKind {
    /// A preceding guard must establish presence of this exact optional value.
    Presence,
    /// Arithmetic must remain inside the authored exact result domain.
    ArithmeticRange,
    /// The exact divisor must be proved nonzero where evaluated.
    Nonzero,
    /// Every possible sum projection must fit the selected result domain.
    SumProjection,
    /// Every accumulated prefix, including intermediate sums, must fit.
    SumPrefixes,
    /// The exact callee must be total over its declared parameter domains.
    PredicateTotal {
        /// Original callee, with its own profile and parameter order.
        target: DeclarationId,
    },
    /// Runtime references require their exact finite population and target closure.
    GraphClosure,
    /// Runtime input must bind the authored invocation and its operation frame.
    InvocationContext,
    /// Runtime input must preserve the authored capture's immutable value and anchor.
    CaptureInput,
}

/// A source-owned pending obligation with its actual scope, never an IR proof.
#[derive(Clone, Debug)]
pub struct Obligation {
    /// Owner and original location where the obligation arises.
    pub site: Site,
    /// Pending proof or runtime condition, never evidence of its discharge.
    pub kind: ObligationKind,
}

/// A located native type fact; it remains inspectable beside another refusal.
#[derive(Clone, Debug)]
pub struct NodeType<'a> {
    /// Original expression handle local to the enclosing declaration's unit.
    pub expression: ExprId,
    /// Original expression's half-open native byte span.
    pub span: Span,
    /// Established contextual type; absent if inference did not establish one.
    pub ty: Option<NativeType<'a>>,
    /// Actual declaration-local binding, when this expression directly reads one.
    pub binder: Option<BinderId>,
    /// Immutable origin of a direct read, distinct from its evaluation context.
    pub anchor: Option<Anchor>,
    /// Initializer/branch provenance, separate from the direct binder anchor.
    pub origin: Option<ObservationOrigin>,
    /// Exact normalized components produced through the existing IR constructor.
    pub normalized_rational: Option<(i64, i64)>,
    /// Exact applied profile-use index, including nested protocol checks.
    pub profile: usize,
}

/// The original scope owns names, roles and anchors; this adds only native type facts.
#[derive(Clone, Debug)]
pub struct BinderType<'a> {
    /// Index into the original declaration scope's binders().
    pub binder: usize,
    /// Established native type, retaining its model/scalar owner.
    pub ty: Option<NativeType<'a>>,
    /// Original lexical binding anchor, separate from initializer provenance.
    pub anchor: Anchor,
}

/// Immutable value provenance; selected alternatives retain their syntax owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ObservationOrigin {
    /// The value is independent of a snapshot observation.
    Independent,
    /// The value retains one declaration-owned observation anchor.
    Anchored(Anchor),
    /// Multiple origins depend on the original expression's evaluation path.
    Selected {
        /// Original unit, preventing cross-unit expression-handle confusion.
        unit: UnitId,
        /// Original expression retaining the contributing alternatives.
        expression: ExprId,
    },
}

/// Private construction prevents typed-only facts from becoming a checked package.
#[derive(Debug)]
pub struct DeclarationTypes<'a> {
    declaration: DeclarationId,
    unit: UnitId,
    nodes: Vec<NodeType<'a>>,
    binders: Vec<BinderType<'a>>,
    causes: Vec<TypeCause>,
    obligations: Vec<Obligation>,
    complete: bool,
    local_complete: bool,
}

impl<'a> DeclarationTypes<'a> {
    /// Original declaration in the binding namespace.
    pub fn declaration(&self) -> DeclarationId {
        self.declaration
    }
    /// Original source unit owning all local expression handles.
    pub fn unit(&self) -> UnitId {
        self.unit
    }
    /// Retained type facts in original expression-arena order.
    pub fn nodes(&self) -> &[NodeType<'a>] {
        &self.nodes
    }
    /// Find a fact by an expression handle from this declaration's unit.
    pub fn node(&self, expression: ExprId) -> Option<&NodeType<'a>> {
        self.nodes
            .binary_search_by_key(&expression.0, |node| node.expression.0)
            .ok()
            .map(|index| &self.nodes[index])
    }
    /// Types corresponding to the original lexical binders.
    pub fn binders(&self) -> &[BinderType<'a>] {
        &self.binders
    }
    /// Known local and dependency refusals retained in discovery order.
    pub fn causes(&self) -> &[TypeCause] {
        &self.causes
    }
    /// Required later proof/runtime conditions; none is discharged by typing.
    pub fn obligations(&self) -> &[Obligation] {
        &self.obligations
    }
    /// Whether required type work and dependency processing finished.
    pub fn complete(&self) -> bool {
        self.complete
    }
    /// Type-stage disposition only; known refusals survive later exhaustion.
    pub fn disposition(&self) -> TypeDisposition {
        if !self.causes.is_empty() {
            TypeDisposition::Refused
        } else if self.complete {
            TypeDisposition::Typed
        } else {
            TypeDisposition::Unfinished
        }
    }
}

/// Immutable type-stage evidence over the actual composed binding report.
#[derive(Debug)]
pub struct TypeReport<'r, 'a> {
    binding: &'r binding::Report<'a>,
    declarations: Vec<DeclarationTypes<'a>>,
    exhaustion: Option<Exhaustion>,
    limits: TypeLimits,
    usage: TypeUsage,
}

impl<'r, 'a> TypeReport<'r, 'a> {
    /// Borrow the original namespace, definition/model selections and lexical scopes.
    pub fn binding(&self) -> &'r binding::Report<'a> {
        self.binding
    }
    /// Retained declaration records; unvisited declarations have no record.
    pub fn declarations(&self) -> &[DeclarationTypes<'a>] {
        &self.declarations
    }
    /// Find a retained record by its original namespace declaration handle.
    pub fn declaration(&self, id: DeclarationId) -> Option<&DeclarationTypes<'a>> {
        self.declarations.get(id.index())
    }
    /// Unvisited original declarations remain unfinished; out-of-range handles return None.
    pub fn disposition(&self, id: DeclarationId) -> Option<TypeDisposition> {
        self.binding.namespace().declaration(id)?;
        Some(
            self.declaration(id)
                .map_or(TypeDisposition::Unfinished, DeclarationTypes::disposition),
        )
    }
    /// First unaffordable operation, preserving the charge and original locus.
    pub fn exhaustion(&self) -> Option<&Exhaustion> {
        self.exhaustion.as_ref()
    }
    /// Effective caller-lowered limits after hard ceilings were applied.
    pub fn limits(&self) -> TypeLimits {
        self.limits
    }
    /// Successfully charged work; failed charges do not increase usage.
    pub fn usage(&self) -> TypeUsage {
        self.usage
    }
}

/// Establish native profile/type facts without claiming proof or family admission.
/// Formal sources serve only existing source-dependent IR normalization here.
pub fn admit_types<'r, 'a>(
    binding: &'r binding::Report<'a>,
    sources: &[FormalSource],
    limits: TypeLimits,
) -> TypeReport<'r, 'a> {
    solver::admit(binding, sources, limits)
}

fn permissions(definition: RegisteredDefinition) -> (bool, bool) {
    match definition {
        RegisteredDefinition::StateCore => (false, false),
        RegisteredDefinition::StateQueries => (true, false),
        RegisteredDefinition::StateGraph
        | RegisteredDefinition::TemporalFacet
        | RegisteredDefinition::EventPosition
        | RegisteredDefinition::FixedSample
        | RegisteredDefinition::TimestampedWindow
        | RegisteredDefinition::Protocol => (true, true),
        RegisteredDefinition::Edition
        | RegisteredDefinition::ObservationBinding
        | RegisteredDefinition::Progress
        | RegisteredDefinition::Range
        | RegisteredDefinition::Package
        | RegisteredDefinition::Diagnostics => (false, false),
    }
}
