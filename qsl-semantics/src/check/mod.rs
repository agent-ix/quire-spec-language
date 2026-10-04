// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §7.3 M-5 (FR-068): the layer-3 checking half of what was
//! `value::expression`. This module owns name resolution, typing, static
//! definedness and termination checking, and the checked-output types
//! ([`CheckedGraph`], [`CheckedExpression`], `CheckedFunction`) whose
//! constructors are private here (ADR-011 §4's private-constructor/
//! public-accessor mechanism). The evaluation half -- `evaluate.rs`,
//! `CheckedPackage::call`, `CheckedPackage::evaluate` and the
//! argument-admission logic -- stays at layer 5 in
//! `qsl_eval::value::expression` (a private module, not resolvable as an
//! intra-doc link) (S6a), reaching this module's state only
//! through the accessor methods below, never through a private field: the
//! two modules no longer share private state (US-009).
//!
//! # `CheckedGraph` (S3) versus `CheckedPackage` (S4) (ADR-013 T-1, FR-087,
//! S-3a)
//!
//! This module's own checked-output type is [`CheckedGraph`]: the S3
//! checker's stage output, produced by [`PackageDeclarations::check`].
//! `CheckedPackage` (the S4 in-process link step's output, `CheckedGraph`
//! plus the checked dependency closure) is a *different*, canonical type,
//! defined in the layer-4 crate `qsl-package`, not here -- `check` defines
//! no `CheckedPackage` type, method or field, and this crate cannot import
//! `qsl-package`, which depends on it (FR-087-AC-9/TC-256): the
//! `CheckedPackage::call`/`CheckedPackage::evaluate` references in this
//! module's own doc comments name the layer-5 `CheckedPackageEvaluation`
//! methods on `qsl_package::CheckedPackage`, which reach this module's state
//! only through its `CheckedGraph`-typed field and its `graph()` accessor.
//!
//! # The interim `model` -> `check` edge is closed (ADR-011 §7.3 M-2)
//!
//! FR-068 (M-5) left an interim `model` -> `check` edge: `model::checked_dispatch.rs`
//! and `model::conformance.rs` imported thirteen names this module defined
//! (`DispatchCandidate`/`DispatchOperation`/`DispatchTable`/
//! `PackageDeclarations` and
//! `established_field_fact`/`Connective`/`Established`/`Location`/`Node`/
//! `NodeKind`/`OrderedKind`/`Origin`/`ProvedInterval`) directly from
//! `crate::check`, not through `crate::value`'s aggregate re-export -- a
//! legible `model` (layer-3-earlier) -> `check` (layer-3-later) reverse
//! edge, forbidden by §6.1's intra-layer-3 order, declared rather than
//! hidden (FR-068-AC-9/FR-068-CON-5; see FR-068's Behavior section, "The
//! interim `model` -> `check` edge," now superseded).
//!
//! **M-2 closes it.** `model::checked_dispatch` moved to this
//! module's own `checked_dispatch` submodule (a private module, not
//! resolvable as an intra-doc link, matching this crate's own convention;
//! see [`checked_dispatch_operation`] for its re-exported entry point), and
//! `model::conformance::check_field_refinement_obligation` moved to this
//! module's own `field_refinement` submodule (see
//! [`check_field_refinement_obligation`]) -- `check` (layer-3-later)
//! importing from `model` (layer-3-earlier) is the *forward* direction
//! §6.1's order permits, not a reverse edge, so `model_check_edges`
//! (`xtask`'s TC-176 scan) is empty from here on. `grep -rn "use crate::check"
//! src/model/` finds nothing.
//!
//! # `family.rs`
//!
//! **Amended, PR #282 review F4:** FR-068's move surface now names
//! `family.rs` explicitly, alongside `check`, `evaluate`, `facts`, `ir`,
//! `mod` itself, `refusal` and `termination` -- an omission in the
//! requirement's original text, not a deliberate exclusion: this module
//! carries a `family` submodule with the function family's checking code
//! (see `family`'s own module doc). Node identity is `lowering`'s and
//! `node_key`'s (FR-092/FR-093).
//! Without it, `check`'s real import graph would reach back into
//! `value::expression::family`, which FR-068-AC-3 forbids.

// FR-068 itself names both the destination module (`check`) and the moved
// file (`check.rs`, holding `Typer`/`Scope`/`bind_parameters` -- name
// resolution and typing) -- the inception is the spec's own naming, not an
// accidental collision this file introduced.
mod assemble;
mod capability;
#[allow(clippy::module_inception)]
mod check;
mod checked_dispatch;
mod claims;
mod facts;
mod family;
mod field_refinement;
mod identity;
// `imports` has no production caller yet (E3 imported-name resolution,
// FR-087-AC-13). Only the layer-4 reader's tests use it, so it is public
// only under `test-support` (no test-only `pub`).
#[cfg(any(test, feature = "test-support"))]
pub mod imports;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod imports;
mod ir;
mod lowering;
mod node_key;
mod observation;
mod profile;
mod protocol_clause;
mod refusal;
mod region;
mod state_clause;
#[cfg(any(test, feature = "test-support"))]
pub mod stratum;
mod termination;
mod type_form;

use crate::model::domain_package::DomainPackageRef;
use std::collections::BTreeMap;

use check::{bind_parameters, Typer};
// `Signature` is public only under `test-support`: the layer-5 evaluator's
// tests build the resolved `Signature` `declarations_for` takes as
// `own_signature`; no shipped caller outside `check` names it.
#[cfg(any(test, feature = "test-support"))]
pub use check::{Signature, Signatures};
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) use check::{Signature, Signatures};
use facts::Definedness;
use quire_exact::Identifier;

use crate::family::FamilyContract;
use qsl_forms::{ClauseKind, Expression, FunctionDeclaration};
use qsl_foundation::diagnostic::{Locus, StageFailure};
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Cancel, ValueType};

pub use check::Scope;
pub use claims::{ClaimSite, PathGuard, RequirementRecord, ResultBound, SiteGuard, ValueClaim};
pub use family::{CheckedDeclaration, ValueDeclarations, ValueFunctionFamily};
pub use profile::{resolve_profiles, ProfileCause, ProfileRefusal};
// Named only by `fixtures::measure_resolved`'s return type.
#[cfg(any(test, feature = "test-support"))]
pub use family::DeclarationMetrics;
pub use lowering::{
    AdmittedModel, ForeignView, LockEvidence, ModelClause, NominalNode, SemanticGraph, SemanticNode,
};
pub use node_key::{
    AggregateTerm, ApplicationTerm, Binding, BindingValue, BodyTerm, FrameField, FrameTerm,
    GroupMember, GroupTerm, IntegerSite, InvalidModelOwner, InvalidSourceOwner, LawRole,
    LeafSegment, LeafTerm, LiteralValue, MemberTerm, ModelOwner, NodeKeyRefusal, NodeRef, NodeTag,
    Operation, OperationLaw, OperationLeaf, OperationMode, Operator, Owner, PackageRef,
    SourceOwner, TupleMember, TupleTerm, WireNodeRef,
};
// PR #303 review, finding N7b: `empty_scope`/`root_location` used to be
// defined twice -- once here (`check::family`'s own `checking_tests`
// module) and once more, byte-for-byte, in `value::expression::family`'s
// `family_contract_tests` module. Both build the same `Scope`/`Location`
// this crate already defines once; re-exporting the one real definition
// removes the second copy instead of letting it drift.
//
// `declarations_for` joined this list in PR #303 review round 3 (finding
// F6): the same duplication, for a `ValueDeclarations` test fixture, with
// two different parameter shapes.
//
// `check_context` and the constant are test-support views of
// `pub(crate)` items (`CheckContext::new`, `SCALAR_LIMITS_UNLIMITED`), so
// the layer-5 evaluator's tests reach them without widening the items.
#[cfg(any(test, feature = "test-support"))]
pub use family::fixtures::{
    admitted_source, check_context, declaration, declaration_signature, declarations_for,
    empty_scope, fixture_source, json_depth, limits, measure_resolved, root_location, scope_with,
    SCALAR_LIMITS_UNLIMITED,
};
pub use ir::{
    Arithmetic, CheckedBody, CheckedLiteral, CheckedNode, Connective, Node, NodeId, NodeKind,
    Observation, OrderedKind, RecordSlot, Slot, Visit,
};

pub use assemble::{
    AdmittedImport, AssemblyCause, AssemblyError, AssemblyLimits, AssemblyRefusal, TopologyFault,
    DEFAULT_DECIMAL_SCALE,
};
pub use capability::{Capability, UnknownCapabilityLabel};
pub use check::{DispatchOperation, EnumBinding, PackageDeclarations, ResolvedSignatures};
pub use checked_dispatch::{
    checked_dispatch_operation, object_type_supertypes, DispatchBridgeRefusal, DispatchRoot,
    MissingClauseField, OperationClauses,
};
pub use field_refinement::check_field_refinement_obligation;
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::location::{Location, Origin};
pub use region::DeclarationRegions;
pub use state_clause::{
    AttemptDeclaration, CheckedOperationFrame, CheckedStateClause, ClauseOperation,
    OperationSelection, ProtocolClauseFamily, StateClauseDeclaration,
};
pub use type_form::TypeFormFault;
// PR #300 review finding 4: `mint_type_declaration_identity` was `pub(super)`
// in `identity` for the same reason: no consumer outside `check` minted an
// identity directly. `mint_variant_id` itself has since moved to
// `crate::value::enumeration` (OQ-F ruling): it is `pub` there because
// `check.rs`'s own literal resolution (`Typer::name`) mints a `VariantId` for
// an enum-member literal the same way `identity::to_kernel_value_type`'s C-26
// conversion does, and both are `check`-core call sites of one canonical
// function rather than two.
pub use identity::{
    to_kernel_value_type, CheckedClauseKind, CheckedTypeNode, CorrespondenceConflict,
    InvalidSumVariants, ModelCorrespondence, ScalarShape, SumVariant, SumVariants,
};
pub use ir::{CollectionLoss, CollectionProperty, DispatchCandidate, DispatchTable};
pub use protocol_clause::{CheckedAttempt, CheckedProtocol, ProtocolNodeId};
pub use refusal::{
    AliasKind, CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, DispatchFunctionRole,
    InvalidDispatchDeclaration, KeyFault, MeasureObligation, Obligation, ProtocolAnchorCause,
    ProvedInterval, ShadowedDeclaration, StageLimitCause, WrongSnapshotCause,
};

/// A checked function.
#[derive(Debug)]
struct CheckedFunction {
    /// FR-062/FR-065: the function node's FR-092 key, minted once at check
    /// (`lowering`), independent of this function's position in the
    /// package's function list.
    identity: quire_exact::NodeKey,
    body: CheckedBody,
    slots: usize,
}

/// Every checked function with its signature, index-aligned by construction:
/// [`Self::new`] is the only way in, and it splits one list of
/// pairs, so a position names the same function in both halves.
#[derive(Debug)]
struct CheckedFunctions {
    /// Each function's signature, indexed by name once.
    signatures: Signatures,
    bodies: Vec<CheckedFunction>,
    /// Each identity's first position, so a by-identity lookup is a map
    /// lookup, not a scan.
    by_identity: BTreeMap<quire_exact::NodeKey, usize>,
}

impl CheckedFunctions {
    fn new(functions: Vec<(Signature, CheckedFunction)>) -> Self {
        let mut by_identity = BTreeMap::new();
        for (position, (_, function)) in functions.iter().enumerate() {
            by_identity.entry(function.identity).or_insert(position);
        }
        let (signatures, bodies): (Vec<_>, Vec<_>) = functions.into_iter().unzip();
        Self {
            signatures: Signatures::new(signatures),
            bodies,
            by_identity,
        }
    }

    fn get(&self, index: usize) -> Option<(&Signature, &CheckedFunction)> {
        Some((
            self.signatures.as_slice().get(index)?,
            self.bodies.get(index)?,
        ))
    }

    /// The first function named `name`.
    fn named(&self, name: &str) -> Option<(&Signature, &CheckedFunction)> {
        self.get(self.signatures.position(name)?)
    }

    /// The first function with this identity.
    fn with_identity(
        &self,
        identity: quire_exact::NodeKey,
    ) -> Option<(&Signature, &CheckedFunction)> {
        self.get(*self.by_identity.get(&identity)?)
    }
}

fn function_state<'a>(
    (signature, function): (&'a Signature, &'a CheckedFunction),
) -> FunctionState<'a> {
    FunctionState {
        name: &signature.name,
        body: function.body.root(),
        slots: function.slots,
    }
}

/// S3's stage output (ADR-013 T-1): a package whose every function is
/// admitted. Its constructor and every field are private to this module
/// (ADR-011 §4): `package`'s S4 link step (which builds the *different*
/// canonical `CheckedPackage`, layer-4 `package`) reaches this state only
/// through the accessor methods below, never through a private field or a
/// conversion function (R-10).
///
/// TC-244 row 1 (FR-087-AC-2): an unchecked `ParsedSource` never becomes a
/// `CheckedGraph` by any path other than [`PackageDeclarations::check`] --
/// in particular, not by naming this struct's private fields directly from
/// outside `check`:
/// ```compile_fail,E0451
/// use qsl_semantics::check::CheckedGraph;
/// let forged = CheckedGraph {
///     scope: todo!(),
///     functions: Vec::new(),
///     dispatch_tables: Vec::new(),
///     occurrences: todo!(),
/// };
/// ```
/// Its pair must compile, so a rename or a move of `CheckedGraph` breaks
/// the build rather than making the block above fail for another reason
/// (stable rustdoc does not check the error code):
/// ```no_run
/// use qsl_semantics::check::CheckedGraph;
/// fn holds(graph: CheckedGraph) -> CheckedGraph {
///     graph
/// }
/// ```
///
/// quire:canonical
#[derive(Debug)]
pub struct CheckedGraph {
    /// FR-001: the checked unit's `RawSourceRef`, for the lock's `sources`
    /// entry.
    source: qsl_foundation::source::provenance::RawSourceRef,
    scope: Scope,
    functions: CheckedFunctions,
    dispatch_tables: Vec<DispatchTable>,
    /// FR-062-AC-2/FR-065-AC-3: the occurrence-keyed source map (identity,
    /// role, ordinal) -> source [`Location`], for every function
    /// declaration and function-application occurrence this package
    /// checked.
    occurrences: family::OccurrenceMap<Location>,
    /// ADR-012 §13.5, ADR-011 E7: this package's requirement records, one
    /// per claim site, keyed by the site's occurrence key. Read through
    /// [`Self::requirements`], the candidate step's own input
    /// (`qsl-route`), not dropped after `check`.
    requirements: BTreeMap<OccurrenceKey, RequirementRecord>,
    /// ADR-013 O-04/FR-088-AC-2: the model correspondence this package's own
    /// checking recorded, read only through [`Self::resolve_declaration`] --
    /// a node-id-keyed accessor, never a name-keyed one (R-06). FR-094:
    /// `check` is its only writer; it holds exactly one entry per model
    /// declaration node lowering keyed.
    model_correspondence: identity::ModelCorrespondence,
    /// ADR-013 O-14/C-26 (PR #300 review finding 1): every admitted
    /// composite and enum type declaration this package's `TypeEnvironment`
    /// and `enums` carried, converted once at `check` into a real
    /// [`identity::CheckedTypeNode`] and keyed by its own checked node id --
    /// read only through [`Self::checked_type_node`], node-id-keyed like
    /// every other accessor here (R-06).
    type_nodes: BTreeMap<quire_exact::NodeKey, identity::CheckedTypeNode>,
    /// FR-092/FR-093: every lowered, keyed node of this
    /// package's functions.
    semantic_graph: lowering::SemanticGraph,
    /// NFR-011: the ceilings this package was checked under.
    effective_limits: CheckingLimits,
    /// FR-096: each function's form spans, by declaration index, so a
    /// `Location` resolves to a region of the checked unit.
    form_spans: Vec<Option<qsl_forms::DeclarationSpans>>,
    /// FR-096: the span of each declared type's name read from the unit, by
    /// its declared name, so an `Origin::TypeDeclaration` resolves.
    type_spans: BTreeMap<String, qsl_foundation::Span>,
    /// FR-096-AC-1: the embedding map the spans resolve through, if any.
    embedding: Option<std::sync::Arc<qsl_foundation::source_map::SourceMap>>,
    /// ADR-011 §2.4 `model_selections`: each admitted domain package's
    /// selection, ascending by identity.
    model_selections: Vec<DomainPackageRef>,
    /// FR-104: every checked state clause, in declaration order.
    state_clauses: Vec<CheckedStateClause>,
    /// FR-113: every checked protocol declaration, in declaration order.
    protocols: Vec<CheckedProtocol>,
    /// FR-115: every operation a state clause or attempt names, with its
    /// frame node, one per (declaring type, operation name).
    operation_frames: Vec<CheckedOperationFrame>,
    /// FR-115: each `model` alias's object types, by artifact id: the table
    /// [`Self::resolve_operation`] resolves a `Frame` selection's alias and
    /// type in.
    model_objects: BTreeMap<String, BTreeMap<Identifier, crate::model::key::DeclarationKey>>,
    /// FR-115: every admitted object type's effective identity, by its
    /// declaration key.
    object_types: BTreeMap<crate::model::key::DeclarationKey, quire_exact::EffectiveId>,
    /// FR-096: each protocol's own attempts' declared name spans,
    /// index-aligned with `protocols`, so an `Origin::ProtocolAttempt`
    /// resolves.
    attempt_spans: Vec<Vec<Option<qsl_foundation::Span>>>,
}

/// A checked standalone expression over named parameters. Its constructor
/// and every field are private to this module (ADR-011 §4).
#[derive(Debug)]
pub struct CheckedExpression {
    parameters: Vec<(String, ValueType)>,
    root: CheckedBody,
    slots: usize,
    /// NFR-011: the ceilings this expression was checked under.
    effective_limits: CheckingLimits,
}

impl CheckedExpression {
    /// The result type.
    pub fn value_type(&self) -> &ValueType {
        self.root.root().value_type()
    }

    /// Every `convert` loss in pre-order.
    pub fn losses(&self) -> Vec<CollectionLoss> {
        self.root.root().losses()
    }

    /// Every `deref(r).f` location, whose target existence is a runtime input
    /// requirement.
    pub fn dereferences(&self) -> Vec<Location> {
        self.root.root().dereferences()
    }

    /// The declared parameters, in evaluation-slot order -- the accessor
    /// `value::expression::CheckedPackageEvaluation::evaluate` reads to
    /// validate its caller's arguments (ADR-011 §4: evaluation never reads
    /// this type's fields directly).
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked expression tree evaluation runs.
    pub fn root(&self) -> CheckedNode<'_> {
        self.root.root()
    }

    /// The checked expression's whole node arena.
    pub fn body(&self) -> &CheckedBody {
        &self.root
    }

    /// The evaluation slot count evaluation allocates.
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// NFR-011: the ceilings this expression was checked under. A
    /// standalone expression check applies the node ceiling only: it encodes no declaration preimage and charges no work, so the
    /// input-byte and work ceilings are recorded here as given but were not
    /// applied.
    pub fn effective_limits(&self) -> CheckingLimits {
        self.effective_limits
    }
}

/// One admitted function's evaluation-visible state: exactly what
/// `value::expression`'s evaluator needs (name, checked body, slot count --
/// FR-068's own Description names these as the accessor surface), without
/// exposing `CheckedFunction`'s private representation.
#[non_exhaustive]
pub struct FunctionState<'a> {
    /// The declared name.
    pub name: &'a str,
    /// The checked body.
    pub body: CheckedNode<'a>,
    /// The evaluation slot count.
    pub slots: usize,
}

/// One callable function's evaluation-visible identity and signature --
/// what `value::expression::CheckedPackageEvaluation::call` needs to route a
/// runtime `QualifiedName` lookup through
/// `value::expression`'s `ReferenceEvaluation::evaluate` and validate its
/// caller's arguments, without a direct field read.
#[non_exhaustive]
pub struct CallableFunction<'a> {
    /// The minted identity `value::expression`'s `ReferenceEvaluation::evaluate`
    /// resolves against.
    pub identity: quire_exact::NodeKey,
    /// The declared parameters, for argument admission.
    pub parameters: &'a [(String, ValueType)],
    /// The declared result type.
    pub result: &'a ValueType,
}

/// A family `check`'s stage limit as a checking refusal's cause (L6): `CheckingLimitKind::try_from(LimitKind)` is the named reverse of
/// `foundation_kind`, and a kind no checking limit names is a fault in the
/// family, not in the input.
fn limit_cause(limit: &qsl_foundation::diagnostic::LimitExceeded) -> CheckCause {
    match CheckingLimitKind::try_from(limit.kind()) {
        Ok(kind) => CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind,
            limit: limit.configured_bound(),
            actual: limit.actual(),
            region: match limit.locus() {
                Some(Locus::Region(region)) => Some(region.clone()),
                Some(Locus::Occurrence(_) | Locus::Artifact { .. }) | None => None,
            },
        })),
        Err(kind) => CheckCause::InternalFault(Box::new(KeyFault::UncheckedLimitKind(kind))),
    }
}

fn root(origin: Origin) -> Location {
    Location::root(origin)
}

/// FR-115: one [`CheckedOperationFrame`] per distinct (declaring type,
/// operation name) in `named`, in first-named order. Every clause and
/// attempt naming one operation is bound to the same anchor and frame node
/// and the same frame occurrence (FR-105, FR-104-AC-5), so the first
/// occurrence stands for all of them.
fn operation_frames<'a>(
    named: impl Iterator<Item = (&'a ClauseOperation, &'a lowering::OperationBinding)>,
) -> Vec<CheckedOperationFrame> {
    let mut frames: Vec<CheckedOperationFrame> = Vec::new();
    for (operation, binding) in named {
        let seen = frames.iter().any(|known| {
            known.operation.declaring == operation.declaring
                && known.operation.declaration.name() == operation.declaration.name()
        });
        if !seen {
            frames.push(CheckedOperationFrame {
                operation: operation.clone(),
                anchor: binding.anchor,
                frame: binding.frame,
                frame_origin: binding.frame_origin.clone(),
            });
        }
    }
    frames
}

fn invalid_dispatch(location: Location, detail: InvalidDispatchDeclaration) -> CheckRefusal {
    CheckRefusal {
        location,
        cause: CheckCause::InvalidDispatchDeclaration(Box::new(detail)),
    }
}

/// One dispatch-table function index against the dispatch operation it must
/// conform to: `expected_arity` is the receiver plus every declared
/// argument, `result` is `Boolean` for a precondition (clause or combinator)
/// and the operation's own declared result for a body -- validates every
/// dispatch table/candidate index and signature arity/type upfront, rather
/// than trusting a caller-supplied table at evaluation time. A function that
/// exists (valid `index`) is located at its own real
/// [`Origin::Body`]; an out-of-range `index` names no real declaration to
/// point at, so it is located at [`Origin::Expression`] instead.
/// `signatures` are the functions' resolved signatures, index-aligned with
/// `functions`, so a candidate's parameter and result types compare
/// directly with `operation.parameters`/`.result`.
fn validate_dispatch_function(
    signatures: &[Signature],
    index: usize,
    call_parameters: &[ValueType],
    result: &ValueType,
    role: DispatchFunctionRole,
) -> Result<(), CheckRefusal> {
    let Some(signature) = signatures.get(index) else {
        return Err(invalid_dispatch(
            root(Origin::Expression),
            InvalidDispatchDeclaration::FunctionOutOfRange { role, index },
        ));
    };
    let location = root(Origin::Body {
        function: signature.name.clone(),
        index,
    });
    let expected_arity = call_parameters.len() + 1;
    if signature.parameters.len() != expected_arity {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::Arity {
                role,
                index,
                declared: signature.parameters.len(),
                expected: expected_arity,
            },
        ));
    }
    let mismatched = signature
        .parameters
        .iter()
        .skip(1)
        .map(|(_, value_type)| value_type)
        .zip(call_parameters)
        .any(|(declared, expected)| declared != expected);
    if mismatched {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::ParameterType { role, index },
        ));
    }
    if &signature.result != result {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::ResultType { role, index },
        ));
    }
    Ok(())
}

/// Resolve one declared function's own `TypeForm` parameters and
/// result to the kernel `ValueType` (E3, `check::type_form::
/// resolve_type_form`), collecting every refusal across the whole signature
/// rather than stopping at the first, matching this module's own
/// ambiguous-name and dispatch-validation loops' all-refusals-before-any-
/// charge style.
fn resolve_signature(
    scope: &Scope,
    function: &FunctionDeclaration,
    location: &Location,
) -> Result<check::ResolvedSignature, Vec<CheckRefusal>> {
    let mut refusals = Vec::new();
    let mut parameters = Vec::with_capacity(function.parameters.len());
    for (name, type_form) in &function.parameters {
        match type_form::resolve_type_form(scope, type_form, location) {
            Ok(value_type) => parameters.push((name.clone(), value_type)),
            Err(refusal) => refusals.push(refusal),
        }
    }
    match type_form::resolve_type_form(scope, &function.result, location) {
        Ok(result) if refusals.is_empty() => Ok((parameters, result)),
        Ok(_) => Err(refusals),
        Err(refusal) => {
            refusals.push(refusal);
            Err(refusals)
        }
    }
}

impl PackageDeclarations {
    /// Check every function: duplicate names, declared types, typing, static
    /// definedness and termination, in that order. Every refusal is made
    /// before any charge; a reached checking limit is `stage_limit_exceeded`
    /// and yields no admission verdict.
    pub fn check(self, limits: CheckingLimits) -> Result<CheckedGraph, Vec<CheckRefusal>> {
        self.check_with_cancel(limits, &Cancel::new())
    }

    /// [`Self::check`] under a caller-owned [`Cancel`] handle (FR-276): the
    /// contract meter, the measure pass and the typer's node charge poll it
    /// at every charge, so a cancelled handle stops the check at its next
    /// charge, inside a declaration as well as between them. The stop is a
    /// refusal list like a reached work budget; a caller that holds the
    /// handle reads [`Cancel::tripped`] to tell the two apart.
    pub fn check_with_cancel(
        self,
        limits: CheckingLimits,
        cancel: &Cancel,
    ) -> Result<CheckedGraph, Vec<CheckRefusal>> {
        // FR-096: a family limit's locus resolves through the unit's spans.
        let regions = self.regions();
        let body_location = |index: usize, name: &str| {
            root(Origin::Body {
                function: name.to_owned(),
                index,
            })
        };
        let clause_location = |index: usize, name: &str| {
            root(Origin::StateClause {
                clause: name.to_owned(),
                index,
            })
        };
        let mut refusals = Vec::new();
        // One pass groups every declaration by name, so the
        // duplicate check is a map lookup per declaration, not a pairwise
        // comparison. Each group's declarations stay in declaration order,
        // functions first. FR-104/FR-109: state clauses and functions share
        // one selection namespace, so a clause named like another clause or
        // a function refuses at every declaration of the name.
        let mut by_name: BTreeMap<&str, Vec<Location>> = BTreeMap::new();
        for (index, function) in self.functions.iter().enumerate() {
            by_name
                .entry(function.name.as_str())
                .or_default()
                .push(body_location(index, &function.name));
        }
        for (index, clause) in self.state_clauses.iter().enumerate() {
            by_name
                .entry(clause.name.as_str())
                .or_default()
                .push(clause_location(index, &clause.name));
        }
        let declarations = self
            .functions
            .iter()
            .enumerate()
            .map(|(index, function)| (function.name.as_str(), body_location(index, &function.name)))
            .chain(
                self.state_clauses
                    .iter()
                    .enumerate()
                    .map(|(index, clause)| {
                        (clause.name.as_str(), clause_location(index, &clause.name))
                    }),
            );
        for (name, location) in declarations {
            let loci = by_name.get(name).map_or(&[][..], Vec::as_slice);
            if loci.len() > 1 {
                refusals.push(CheckRefusal {
                    location,
                    cause: CheckCause::AmbiguousName {
                        name: name.to_owned(),
                        loci: loci.to_vec(),
                    },
                });
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // Every declared signature is resolved against the package `Scope`
        // before dispatch validation, which compares resolved types.
        let scope = Scope::new(
            self.types,
            self.operations,
            self.enums,
            self.aliases,
            self.model_operations,
            self.ieee_profile,
            self.dispatch_operations,
        )
        .with_imports(self.imports);
        let dispatch_tables = self.dispatch_tables;
        if let Some(index) = self
            .resolved_signatures
            .first_out_of_range(self.functions.len())
        {
            return Err(vec![invalid_dispatch(
                root(Origin::Expression),
                InvalidDispatchDeclaration::ResolvedSignatureOutOfRange { index },
            )]);
        }
        let mut signatures: Vec<Signature> = Vec::with_capacity(self.functions.len());
        for (index, function) in self.functions.iter().enumerate() {
            let location = body_location(index, &function.name);
            let resolved = match self.resolved_signatures.get(index) {
                // `check::checked_dispatch`'s synthesized clauses: already
                // resolved from a model signature, never rendered back into
                // syntax (see that module's own doc).
                Some((parameters, result)) => Ok((parameters.clone(), result.clone())),
                None => resolve_signature(&scope, function, &location),
            };
            match resolved {
                Ok((parameters, result)) => signatures.push(Signature {
                    name: function.name.clone(),
                    parameters,
                    result,
                    callable_by_name: function.callable_by_name(),
                }),
                Err(mut function_refusals) => refusals.append(&mut function_refusals),
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let signatures = Signatures::new(signatures);
        let mut seen_operations = std::collections::BTreeSet::new();
        for operation in &scope.dispatch_operations {
            if !seen_operations.insert((operation.receiver_type, operation.member.clone())) {
                refusals.push(invalid_dispatch(
                    root(Origin::Expression),
                    InvalidDispatchDeclaration::DuplicateOperation {
                        receiver_type: operation.receiver_type,
                        member: operation.member.clone(),
                    },
                ));
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        for operation in &scope.dispatch_operations {
            let Some(table) = dispatch_tables.get(operation.table) else {
                refusals.push(invalid_dispatch(
                    root(Origin::Expression),
                    InvalidDispatchDeclaration::TableOutOfRange {
                        member: operation.member.clone(),
                        table: operation.table,
                    },
                ));
                continue;
            };
            for (_, candidate) in table.entries() {
                if let Err(refusal) = validate_dispatch_function(
                    signatures.as_slice(),
                    candidate.body,
                    &operation.parameters,
                    &operation.result,
                    DispatchFunctionRole::Body,
                ) {
                    refusals.push(refusal);
                }
                if let Some(precondition) = candidate.precondition {
                    if let Err(refusal) = validate_dispatch_function(
                        signatures.as_slice(),
                        precondition,
                        &operation.parameters,
                        &ValueType::Boolean,
                        DispatchFunctionRole::Precondition,
                    ) {
                        refusals.push(refusal);
                    }
                }
                for &clause in &candidate.precondition_clauses {
                    if let Err(refusal) = validate_dispatch_function(
                        signatures.as_slice(),
                        clause,
                        &operation.parameters,
                        &ValueType::Boolean,
                        DispatchFunctionRole::PreconditionClause,
                    ) {
                        refusals.push(refusal);
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let source = self.source;
        let type_spans = self.declared_type_spans;
        let embedding = self.embedding;
        let form_spans = self
            .functions
            .iter()
            .map(|function| function.spans().cloned())
            .collect();
        let owner = node_key::SourceOwner::from(&source);
        let lock_evidence = self.lock_evidence;
        let models = self.models;
        let (model_objects, object_types) = model_object_tables(&models);
        let mut model_selections: Vec<_> = models
            .iter()
            .map(|model| model.selection().clone())
            .collect();
        model_selections.sort_by(|a, b| (&a.identity, &a.version).cmp(&(&b.identity, &b.version)));
        // FR-094 (ADR-013 O-01): a check selects one version of each domain
        // package identity, so a model owner names one declaration.
        let mut selected = std::collections::BTreeSet::new();
        if let Some(model) = models
            .iter()
            .find(|model| !selected.insert(model.selection().identity.as_str()))
        {
            return Err(vec![CheckRefusal {
                location: root(Origin::Expression),
                cause: CheckCause::InternalFault(Box::new(KeyFault::DuplicateModelSelection(
                    model.selection().identity.clone(),
                ))),
            }]);
        }
        if let Some((&index, _)) = self.model_clauses.range(self.functions.len()..).next() {
            return Err(vec![invalid_dispatch(
                root(Origin::Expression),
                InvalidDispatchDeclaration::ModelClauseOutOfRange { index },
            )]);
        }
        let model_clauses = self.model_clauses;
        // FR-094: the object type `T` of each `Population<T>[N]` parameter,
        // from its resolved type form (the checked type carries only `N`).
        let mut population_targets: Vec<Vec<Option<quire_exact::EffectiveId>>> =
            Vec::with_capacity(self.functions.len());
        for (index, function) in self.functions.iter().enumerate() {
            let location = body_location(index, &function.name);
            let mut targets = Vec::with_capacity(function.parameters.len());
            for (_, form) in &function.parameters {
                match type_form::population_target(&scope, form, &location) {
                    Ok(target) => targets.push(target),
                    Err(refusal) => refusals.push(refusal),
                }
            }
            population_targets.push(targets);
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let mut drafts: Vec<(
            Signature,
            family::CheckedDeclarationBody,
            qsl_forms::DeclarationKind,
        )> = Vec::with_capacity(self.functions.len());
        // ADR-012 §13.5, ADR-011 E7: every admitted declaration's claims,
        // read from the contract's `Checked` value before it is unpacked
        // into `drafts` below, and keyed by occurrence once lowering has
        // recorded every occurrence (`requirements`, below).
        let mut claims: Vec<claims::ValueClaim> = Vec::new();
        // Identity, the real typing/definedness verdict and one
        // success diagnostic are all produced through one call into the
        // checked-family contract's own `check` hook
        // (`family::ValueFunctionFamily::check`) -- not, as before this
        // ticket, identity alone through the contract with the real
        // typing/definedness verdict made separately by a `Typer` this loop
        // constructed and drove directly on the side. `check` itself now
        // calls `family::check_declaration_body`; this loop calls the
        // contract exactly once per declaration and reaches that verdict
        // only through it. Termination stays below, over every declaration's
        // own `calls` this loop collects: see `check_declaration_body`'s
        // own doc for why that one part cannot move the same way.
        // ADR-013 O-14/C-26: every admitted enum declaration becomes a
        // `CheckedTypeNode::Sum` identified by its `AdmittedEnumDeclaration::key()`,
        // and every admitted composite (below, once lowering has keyed it)
        // a `CheckedTypeNode::Composite` identified by its FR-092 key.
        // FR-092-AC-12: a declared composite's checked type node takes its
        // FR-092 key, which `check` mints below, not the caller's handle.
        let mut type_nodes = BTreeMap::new();
        for enum_binding in scope.enums() {
            let node = enum_binding.declaration.key();
            // Every case name here was already validated as
            // `^[A-Za-z_][A-Za-z0-9_]*$` and checked distinct from its
            // siblings when this `EnumBinding`'s own `AdmittedEnumDeclaration`/
            // `EnumMemberPreimage` were admitted (`value/enumeration.rs`'s
            // `EnumDeclarationPreimage::from_json`/`AdmittedEnumDeclaration::
            // admit_member`) -- an `EnumBinding` reaching `check` at all
            // already carries that guarantee, so re-deriving an
            // `Identifier`/`SumVariants` here cannot fail in production.
            // PR #300 review round 2 (HIGH-2): `expect` rather than a
            // silent `continue`/skip, so a genuine violation of that
            // upstream invariant is loud, not a quietly missing checked
            // type node (the review's own "or `expect`, if truly
            // unreachable" alternative for this exact site).
            let variants: Vec<identity::SumVariant> = enum_binding
                .members
                .iter()
                .map(|member| {
                    Identifier::new(member.case().to_owned())
                        .map(identity::SumVariant::new)
                        .expect(
                            "an EnumBinding's own AdmittedEnumDeclaration/EnumMemberPreimage \
                             admission already validated every case name as an \
                             identifier before this declaration ever reached `check`",
                        )
                })
                .collect();
            let variants = identity::SumVariants::new(
                variants,
                enum_binding.declaration.declaration().is_ordered(),
            )
            .expect(
                "an EnumBinding's own AdmittedEnumDeclaration admission already refuses two \
                 members sharing one declared case name, and already refuses an \
                 unordered declaration whose members are not sorted by case, before \
                 this declaration ever reaches `check`",
            );
            type_nodes.insert(node, identity::CheckedTypeNode::Sum { node, variants });
        }
        // `node_count` reads the same `CheckingLimits.nodes()` the `Typer`
        // below honors: it is the ceiling claim classification walks under,
        // not a stage-entry check. `Typer`'s counter (seeded from
        // `nodes_used` below) accumulates across every declaration in the
        // package against that bound and stops at the node whose entry
        // crosses it. `input_bytes` reads `CheckingLimits`' own dedicated
        // knob (`with_input_bytes`). `work_budget` has no `StageLimits`
        // field at all (PR #302 review finding 3): it is charged against
        // `contract_meter`'s own `work_units` bound instead, read from
        // `CheckingLimits::work_budget`. All three are NFR-011's finite
        // defaults unless a caller sets them. The mechanism is real
        // (`CheckContext::check_input_bytes` and a `cx.meter` charge for
        // `work_budget`) and is exercised directly against tight fixtures in
        // `qsl-eval/src/value/expression/family.rs`'s `family_contract_tests`.
        let contract_limits = crate::family::StageLimits {
            input_bytes: limits.input_bytes(),
            node_count: limits.nodes(),
        };
        let contract_meter_limits = quire_exact::ScalarLimits {
            work_units: limits.work_budget(),
            ..family::SCALAR_LIMITS_UNLIMITED
        };
        let mut contract_meter =
            quire_exact::Meter::new(contract_meter_limits).with_cancel(cancel.clone());
        let mut contract_diagnostics = crate::family::DiagnosticSink::default();
        let mut contract_scopes = crate::family::ScopeStack::default();
        // PR #303 review round 3, finding F1: the running total of
        // `Expression` nodes every declaration admitted so far in this
        // package has consumed -- this loop's own state, read by each
        // iteration's `ValueDeclarations::nodes_used` and advanced from each
        // admitted declaration's own `CheckedDeclarationBody::nodes_used`,
        // restoring `CheckingLimits::new`'s documented, package-wide `nodes`
        // bound (see `check::family::check_declaration_body`'s own doc for
        // why this, not a `Cell` or a second call, is the mechanism).
        let mut nodes_used = 0_u64;
        for (index, function) in self.functions.into_iter().enumerate() {
            // FR-092 "Function nodes": the kind travels with the draft to
            // lowering, which keys a predicate's node `predicate`.
            let kind = function.kind();
            let location = body_location(index, &function.name);
            let measure_location = root(Origin::Measure {
                function: function.name.clone(),
                index,
            });
            let declarations = family::ValueDeclarations {
                scope: &scope,
                signatures: &signatures,
                own_signature: &signatures.as_slice()[index],
                dispatch_tables: &dispatch_tables,
                checking_limits: limits,
                location: &location,
                measure_location: &measure_location,
                nodes_used,
                regions: Some(&regions),
                cancel: Some(cancel),
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match family::ValueFunctionFamily::check(&function, &mut contract_cx) {
                Ok(staged) => {
                    // The real typing/definedness verdict travels out
                    // through `Staged::into_value` itself (a `CheckedDeclaration`
                    // carrying both the minted identity and the real checked
                    // body -- PR #303 review, finding N3), not a side
                    // channel: no `.expect(...)` unwrap of a slot `check`
                    // might not have filled.
                    let checked = staged.into_value();
                    // ADR-012 §13.5: read this declaration's claims before
                    // `checked` is unpacked below.
                    claims.extend(family::ValueFunctionFamily::requirements(&checked));
                    // PR #303 review round 3, finding F1: advance the
                    // package's running node total from this admitted
                    // declaration's own final count, so the next
                    // declaration's `Typer` picks up where this one left
                    // off instead of restarting at zero.
                    nodes_used = checked.body.nodes_used;
                    drafts.push((signatures.as_slice()[index].clone(), checked.body, kind));
                }
                // NFR-011: a reached stage limit stops checking.
                Err(StageFailure::Limit(limit)) => {
                    refusals.push(CheckRefusal {
                        location: location.clone(),
                        cause: limit_cause(&limit),
                    });
                    return Err(refusals);
                }
                Err(StageFailure::Cancelled(_)) => return Err(refusals),
                Err(StageFailure::Fault(fault)) => {
                    refusals.push(CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(fault))),
                    });
                    return Err(refusals);
                }
                Err(StageFailure::Refused(refusal)) => {
                    let exhausted = matches!(refusal.cause, CheckCause::ResourceExhausted(_));
                    refusals.push(refusal);
                    if exhausted {
                        return Err(refusals);
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // FR-104: each state clause, checked through the `ProtocolClause`
        // family's own contract once every function signature is known (a
        // clause body may call a function), against the same package-wide
        // node total and contract meter.
        let state_clause_forms = self.state_clauses;
        let mut typed_clauses = Vec::with_capacity(state_clause_forms.len());
        for (index, clause) in state_clause_forms.iter().enumerate() {
            let location = clause_location(index, &clause.name);
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match state_clause::ProtocolClauseFamily::check(clause, &mut contract_cx) {
                Ok(staged) => {
                    let checked = staged.into_value();
                    nodes_used = checked.nodes_used;
                    typed_clauses.push(checked);
                }
                Err(StageFailure::Limit(limit)) => {
                    refusals.push(CheckRefusal {
                        location,
                        cause: limit_cause(&limit),
                    });
                    if cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                }
                Err(StageFailure::Cancelled(_)) => return Err(refusals),
                Err(StageFailure::Fault(fault)) => {
                    refusals.push(CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(fault))),
                    });
                    return Err(refusals);
                }
                Err(StageFailure::Refused(refusal)) => {
                    let exhausted = matches!(refusal.cause, CheckCause::ResourceExhausted(_));
                    refusals.push(refusal);
                    if exhausted {
                        return Err(refusals);
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // FR-113: each protocol declaration's scoped anchors, resolved
        // through its nested control scopes (`check::protocol_clause`), and
        // its binders checked against the package's own aliases and native
        // declarations (`scope`, `signatures`) for the no-shadowing rule.
        // FR-114: each `attempt`'s own `contracts` list, bound
        // against its already-resolved operation (`self.protocol_attempts`)
        // and the unit's own checked state clauses (`state_clause_forms`).
        // Independent of every other declaration kind otherwise: a
        // protocol's own content is checked from its own form alone.
        // Once those pass, `protocol_clause::
        // content` checks the rest -- the `using` alias, roles, attempt
        // roles and binder types -- and refuses `unsupported_construct`/
        // `not-yet-implemented` at any construct no checker reads yet
        // (every node kind but `sequence`/`attempt`/`finish`, channels,
        // relationships, requirements, captures, `activation on each`,
        // replicated roles, `related by`, and any body but a bare Boolean
        // literal). A protocol compiles only when every part is checked.
        let mut checked_protocols = Vec::with_capacity(self.protocols.len());
        let mut protocol_bindings = Vec::with_capacity(self.protocols.len());
        for (index, protocol) in self.protocols.iter().enumerate() {
            let attempts = self
                .protocol_attempts
                .get(index)
                .map_or(&[][..], Vec::as_slice);
            match protocol_clause::check(
                protocol,
                &self.alias_names,
                &self.native_names,
                &scope,
                &signatures,
                attempts,
                &state_clause_forms,
            ) {
                Ok((checked, bound)) => {
                    refusals.extend(protocol_clause::content(
                        protocol,
                        &self.alias_names,
                        &scope,
                    ));
                    protocol_bindings.push(bound);
                    checked_protocols.push(checked);
                }
                Err(protocol_refusals) => refusals.extend(protocol_refusals),
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // Static definedness is checked per declaration now, inside
        // `family::check_declaration_body`, immediately after that same
        // declaration's typing -- `calls` (one entry per admitted function,
        // in the same order as `functions`) is collected there, not by a
        // separate pass over `&functions` here. Termination is unchanged: it
        // is still a whole-package call-graph analysis over every
        // declaration's own `calls` at once (see `check_declaration_body`'s
        // own doc for why it cannot move alongside typing/definedness).
        let members: Vec<termination::Member<'_>> = drafts
            .iter()
            .map(|(signature, body, _)| termination::Member {
                name: &signature.name,
                parameters: &signature.parameters,
                measure: body.measure.as_ref().map(CheckedBody::root),
                calls: &body.calls,
            })
            .collect();
        let mut refusals = termination::check(&members);
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // FR-092/FR-093: lower every function to FR-322 nodes
        // and key them, each callee before its callers, and record every
        // node's source occurrences (FR-062-AC-2/FR-065-AC-3's
        // occurrence-keyed source map). A function's identity is its
        // function node's key; a call's is its `expression` node's key.
        let locations: Vec<Location> = drafts
            .iter()
            .enumerate()
            .map(|(index, (signature, _, _))| body_location(index, &signature.name))
            .collect();
        let callees: Vec<Vec<usize>> = drafts
            .iter()
            .map(|(_, body, _)| {
                let mut callees = body.body.root().callees();
                if let Some(measure) = &body.measure {
                    callees.extend(measure.root().callees());
                }
                callees
            })
            .collect();
        let order = lowering::lowering_order(&callees);
        let mut occurrences = family::OccurrenceMap::default();
        // FR-094: the package's units and every compound unit typing formed,
        // kept until lowering has keyed each one's type node.
        let mut units = scope.types().units().clone();
        for (_, body, _) in &mut drafts {
            units.extend(std::mem::take(&mut body.formed_units));
        }
        for clause in &mut typed_clauses {
            units.extend(std::mem::take(&mut clause.formed_units));
        }
        let mut lowering = lowering::Lowering::new(
            &scope,
            &owner,
            &models,
            units,
            &lock_evidence,
            drafts.len(),
            &mut occurrences,
            &mut contract_meter,
        )
        .with_node_limit(limits.nodes(), nodes_used);
        for group in &order {
            let inputs: Vec<lowering::FunctionInput<'_>> = group
                .members
                .iter()
                .filter_map(|&index| {
                    let (signature, body, kind) = drafts.get(index)?;
                    Some(lowering::FunctionInput {
                        name: &signature.name,
                        kind: *kind,
                        location: &locations[index],
                        parameters: &signature.parameters,
                        result: &signature.result,
                        body: body.body.root(),
                        body_slots: &body.slot_names,
                        measure: body.measure.as_ref().map(CheckedBody::root),
                        measure_slots: &body.measure_slot_names,
                        population_targets: population_targets
                            .get(index)
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                        clause: model_clauses.get(&index),
                    })
                })
                .collect();
            refusals.extend(lowering.function_group(group, &inputs));
        }
        // FR-104 "Requirements" (SR-736 FND-008): mint every named
        // operation's frame occurrence up front, in ascending (declaring,
        // name) order, before any clause is lowered, so its ordinal never
        // depends on which clause names it first. FR-105 (SR-751 round 2):
        // each anchor still carries a real clause location (never
        // `generated_location`), because `frame`/`operation_anchor` name no
        // position of their own -- the operation is declared in the domain
        // package, not the unit.
        let clause_locations: Vec<Location> = state_clause_forms
            .iter()
            .enumerate()
            .map(|(index, form)| clause_location(index, &form.name))
            .collect();
        // FR-114: every protocol attempt's own real location
        // (never `generated_location`), the same reason a clause's own
        // `AnchorInput::location` must be real (FR-096).
        let attempt_locations: Vec<Vec<Location>> = protocol_bindings
            .iter()
            .enumerate()
            .map(|(protocol_index, bindings)| {
                (0..bindings.len())
                    .map(|attempt_index| {
                        root(Origin::ProtocolAttempt {
                            protocol: protocol_index,
                            attempt: attempt_index,
                        })
                    })
                    .collect()
            })
            .collect();
        let mut anchors: Vec<lowering::AnchorInput<'_>> = state_clause_forms
            .iter()
            .zip(&clause_locations)
            .filter_map(|(form, location)| {
                form.operation
                    .as_ref()
                    .map(|operation| lowering::AnchorInput {
                        declaring: operation.declaring,
                        operation: &operation.declaration,
                        location,
                    })
            })
            .collect();
        for (bindings, locations) in protocol_bindings.iter().zip(&attempt_locations) {
            for (bound, location) in bindings.iter().zip(locations) {
                anchors.push(lowering::AnchorInput {
                    declaring: bound.operation.declaring,
                    operation: &bound.operation.declaration,
                    location,
                });
            }
        }
        if let Err(refusal) = lowering.register_frame_occurrences(&anchors) {
            refusals.push(refusal);
        }
        // FR-104: each state clause after every function it may call,
        // minting its node identity and recording its `claim` occurrence.
        // FR-114: each protocol attempt, bound (S3) to its
        // operation, lowered to that operation's `operation_anchor` and
        // `frame` node identity through the same machinery a `pre`/`post`
        // clause's own binding calls (`Lowering::protocol_attempt`) -- no
        // second frame node concept. Clauses and attempts are lowered in one
        // pass, in source order of their declarations (SR-770 FND-004), so
        // the `anchor` occurrences of an operation named by both get
        // ordinals in source order, as FR-114 "Behavior" requires.
        enum Lowerable {
            Clause(usize),
            Attempt(usize, usize),
        }
        let mut lowering_order: Vec<(usize, Lowerable)> = state_clause_forms
            .iter()
            .enumerate()
            .map(|(index, form)| (form.spans.declaration.start, Lowerable::Clause(index)))
            .collect();
        for (protocol_index, bindings) in protocol_bindings.iter().enumerate() {
            let declarations = self
                .protocols
                .get(protocol_index)
                .map(|protocol| protocol.declarations.as_slice())
                .unwrap_or_default();
            for (attempt_index, bound) in bindings.iter().enumerate() {
                let start = declarations
                    .get(bound.declaration)
                    .map(|declaration| declaration.name.span.start)
                    .unwrap_or_default();
                lowering_order.push((start, Lowerable::Attempt(protocol_index, attempt_index)));
            }
        }
        lowering_order.sort_by_key(|(start, _)| *start);
        let mut lowered_clauses: Vec<Option<lowering::LoweredClause>> =
            std::iter::repeat_with(|| None)
                .take(state_clause_forms.len())
                .collect();
        let mut lowered_attempts: Vec<Vec<Option<lowering::LoweredAttempt>>> = protocol_bindings
            .iter()
            .map(|bindings| {
                std::iter::repeat_with(|| None)
                    .take(bindings.len())
                    .collect()
            })
            .collect();
        for (_, item) in lowering_order {
            match item {
                Lowerable::Clause(index) => {
                    let (Some(form), Some(typed), Some(location), Some(slot)) = (
                        state_clause_forms.get(index),
                        typed_clauses.get(index),
                        clause_locations.get(index),
                        lowered_clauses.get_mut(index),
                    ) else {
                        continue;
                    };
                    let input = lowering::StateClauseInput {
                        kind: form.kind,
                        location,
                        context: form.context,
                        anchor: form
                            .operation
                            .as_ref()
                            .map(|operation| lowering::AnchorInput {
                                declaring: operation.declaring,
                                operation: &operation.declaration,
                                location,
                            }),
                        parameters: &typed.parameters,
                        body: typed.body.root(),
                        body_slots: &typed.slot_names,
                        population_types: &typed.population_types,
                    };
                    match lowering.state_clause(&input) {
                        Ok(lowered) => *slot = Some(lowered),
                        Err(refusal) => refusals.push(refusal),
                    }
                }
                Lowerable::Attempt(protocol_index, attempt_index) => {
                    let (Some(bound), Some(location), Some(slot)) = (
                        protocol_bindings
                            .get(protocol_index)
                            .and_then(|bindings| bindings.get(attempt_index)),
                        attempt_locations
                            .get(protocol_index)
                            .and_then(|locations| locations.get(attempt_index)),
                        lowered_attempts
                            .get_mut(protocol_index)
                            .and_then(|lowered| lowered.get_mut(attempt_index)),
                    ) else {
                        continue;
                    };
                    let input = lowering::AttemptInput {
                        location,
                        anchor: lowering::AnchorInput {
                            declaring: bound.operation.declaring,
                            operation: &bound.operation.declaration,
                            location,
                        },
                        frame_population: bound.frame_population,
                    };
                    match lowering.protocol_attempt(&input) {
                        Ok(lowered) => *slot = Some(lowered),
                        Err(refusal) => refusals.push(refusal),
                    }
                }
            }
        }
        for composite in scope.types().composites() {
            match lowering.composite_node(composite.key()) {
                Ok(node) => {
                    type_nodes.insert(node, identity::CheckedTypeNode::Composite { node });
                }
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-092 rule 1: every admitted enum's declaration and member nodes,
        // which enum types and enum literals name by key.
        for enum_binding in scope.enums() {
            if let Err(refusal) = lowering.enum_nodes(enum_binding) {
                refusals.push(refusal);
            }
        }
        let lowered = lowering.finish(&lowering::generated_location());
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // Every clause and attempt lowered above (a failed one refused, and
        // `refusals` is empty here); a slot left empty is an internal fault,
        // never a silent omission.
        let unlowered = || {
            vec![CheckRefusal {
                location: root(Origin::Expression),
                cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
            }]
        };
        let lowered_clauses: Vec<lowering::LoweredClause> = lowered_clauses
            .into_iter()
            .collect::<Option<_>>()
            .ok_or_else(unlowered)?;
        let lowered_attempts: Vec<Vec<lowering::LoweredAttempt>> = lowered_attempts
            .into_iter()
            .map(|lowered| lowered.into_iter().collect::<Option<_>>())
            .collect::<Option<_>>()
            .ok_or_else(unlowered)?;
        // Each contract entry's own checked identity (FR-114 "Outputs"),
        // captured before `lowered_clauses` is consumed below.
        let lowered_clause_keys: Vec<quire_exact::NodeKey> =
            lowered_clauses.iter().map(|lowered| lowered.key).collect();
        // FR-115: each operation a `pre`/`post` clause or an attempt names,
        // with the frame node lowering bound it to (FR-105, FR-114).
        let clause_frames =
            state_clause_forms
                .iter()
                .zip(&lowered_clauses)
                .filter_map(|(form, lowered)| {
                    Some((form.operation.as_ref()?, lowered.binding.as_ref()?))
                });
        let attempt_frames = protocol_bindings
            .iter()
            .zip(&lowered_attempts)
            .flat_map(|(bindings, lowered)| bindings.iter().zip(lowered))
            .map(|(bound, lowered)| (&bound.operation, &lowered.binding));
        let operation_frames = operation_frames(clause_frames.chain(attempt_frames));
        // FR-094: `check` is the model correspondence's only writer; it
        // holds exactly the model declaration nodes lowering keyed, and a
        // conflicting entry refused the lowering above (FR-303).
        let (semantic_graph, model_correspondence, identities, binders) = (
            lowered.graph,
            lowered.correspondence,
            lowered.functions,
            lowered.binders,
        );
        // ADR-012 §13.5, ADR-011 E7: exactly one record per claim site,
        // keyed by the `expression` occurrence recorded at the site's own
        // location (ADR-013 O-07). A claim that cannot be keyed is a fault,
        // never an omission.
        let mut requirements = claims::key_claims(claims, &occurrences, &semantic_graph, &binders)
            .map_err(|fault| {
                vec![CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(fault)),
                }]
            })?;
        // FR-104 "Requirements": one `operation-contract` record per clause,
        // keyed by its `claim` occurrence, and one per frame of an operation
        // a `pre` or `post` clause names, keyed by the frame node's own
        // occurrence.
        let state_clauses = state_clause_forms
            .into_iter()
            .zip(typed_clauses)
            .zip(lowered_clauses)
            .enumerate()
            .map(|(index, ((form, typed), lowered))| {
                let location = clause_location(index, &form.name);
                let unkeyable = || {
                    vec![CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                    }]
                };
                let claim = occurrences
                    .iter_role(family::OccurrenceRole::Claim)
                    .find(|(node, _, at)| *node == lowered.key && **at == location)
                    .map(|(_, origin, _)| origin)
                    .ok_or_else(unkeyable)?;
                let claims = state_clause::ProtocolClauseFamily::requirements(&typed);
                let keyed = state_clause::clause_records(
                    &typed,
                    &claims,
                    &form,
                    &lowered,
                    &claim,
                    scope.types(),
                    limits.nodes(),
                    &location,
                )
                .map_err(|refusal| vec![refusal])?;
                for state_clause::KeyedClaim {
                    node,
                    origin,
                    record,
                } in keyed
                {
                    match requirements.entry(OccurrenceKey::new(claims::wire(node), origin)) {
                        std::collections::btree_map::Entry::Vacant(slot) => {
                            slot.insert(record);
                        }
                        // Every clause naming one operation keys the same
                        // frame record; any other collision is a fault.
                        std::collections::btree_map::Entry::Occupied(existing)
                            if node != lowered.key && *existing.get() == record => {}
                        std::collections::btree_map::Entry::Occupied(_) => return Err(unkeyable()),
                    }
                }
                Ok(CheckedStateClause {
                    name: form.name,
                    kind: form.kind,
                    context: form.context,
                    operation: form.operation,
                    parameters: typed.parameters,
                    body: typed.body,
                    slots: typed.slots,
                    observations: typed.observations,
                    identity: lowered.key,
                    claim,
                    spans: form.spans,
                    has_result: typed.has_result,
                })
            })
            .collect::<Result<Vec<_>, Vec<CheckRefusal>>>()?;
        // FR-114 "Requirements": one `operation-contract` record
        // per attempt's frame, keyed by the frame node's own occurrence,
        // computed by the same `state_clause::frame_record` a `pre`/`post`
        // clause's frame claim uses (SR-770 FND-003), so an attempt and a
        // clause naming one operation always build equal records and the
        // dedup below keeps one (FR-104-AC-5's rule extended to attempts,
        // FR-114-AC-1: "exactly one operation-contract record for the
        // frame"). FR-114 "Outputs": each attempt, fully checked -- its
        // operation's anchor and frame node identity, and each `contracts`
        // entry's own checked state-clause identity.
        for (((bindings, lowered), locations), checked) in protocol_bindings
            .iter()
            .zip(&lowered_attempts)
            .zip(&attempt_locations)
            .zip(checked_protocols.iter_mut())
        {
            let mut attempts = Vec::with_capacity(bindings.len());
            for ((bound, lowered_attempt), location) in bindings.iter().zip(lowered).zip(locations)
            {
                let unkeyable = || {
                    vec![CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                    }]
                };
                let lowering::OperationBinding {
                    anchor,
                    frame,
                    frame_origin,
                } = lowered_attempt.binding.clone();
                let populations: Vec<(state_clause::PopulationDomain, quire_exact::NodeKey)> =
                    bound
                        .frame_population
                        .zip(lowered_attempt.population_object)
                        .into_iter()
                        .collect();
                let record = state_clause::frame_record(
                    &bound.operation,
                    frame,
                    &populations,
                    lowered_attempt.boolean,
                    scope.types(),
                    limits.nodes(),
                    location,
                )
                .map_err(|refusal| vec![refusal])?;
                match requirements.entry(OccurrenceKey::new(claims::wire(frame), frame_origin)) {
                    std::collections::btree_map::Entry::Vacant(slot) => {
                        slot.insert(record);
                    }
                    std::collections::btree_map::Entry::Occupied(existing)
                        if *existing.get() == record => {}
                    std::collections::btree_map::Entry::Occupied(_) => return Err(unkeyable()),
                }
                let contracts = bound
                    .contracts
                    .iter()
                    .map(|&index| lowered_clause_keys.get(index).copied())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(unkeyable)?;
                attempts.push(protocol_clause::CheckedAttempt {
                    declaration: bound.declaration,
                    anchor,
                    frame,
                    contracts,
                });
            }
            checked.attempts = attempts;
        }
        let functions = CheckedFunctions::new(
            drafts
                .into_iter()
                .zip(identities)
                .filter_map(|((signature, body, _), identity)| {
                    Some((
                        signature,
                        CheckedFunction {
                            identity: identity?,
                            body: body.body,
                            slots: body.slots,
                        },
                    ))
                })
                .collect(),
        );
        Ok(CheckedGraph {
            source,
            scope,
            functions,
            dispatch_tables,
            occurrences,
            requirements,
            model_correspondence,
            type_nodes,
            semantic_graph,
            effective_limits: limits,
            form_spans,
            type_spans,
            embedding,
            model_selections,
            state_clauses,
            protocols: checked_protocols,
            operation_frames,
            model_objects,
            object_types,
            attempt_spans: regions.attempt_spans,
        })
    }
}

/// FR-115: the tables [`CheckedGraph::resolve_operation`] and
/// [`CheckedGraph::operation_frame`] read, from the admitted models: each
/// `model` alias's object types by artifact id (the `T` of `ix://<package>/T`),
/// and every object type's effective identity by declaration key. The
/// artifact id is derived once here, so a selection resolves by map lookup.
fn model_object_tables(
    models: &[AdmittedModel],
) -> (
    BTreeMap<String, BTreeMap<Identifier, crate::model::key::DeclarationKey>>,
    BTreeMap<crate::model::key::DeclarationKey, quire_exact::EffectiveId>,
) {
    let mut by_alias = BTreeMap::new();
    let mut identities = BTreeMap::new();
    for model in models {
        let mut objects = BTreeMap::new();
        for (key, id) in model.object_types() {
            identities.insert(key.clone(), id);
            if let Some(artifact) =
                crate::model::intake::type_identity_segment(&key.package, &key.node)
                    .and_then(|artifact| Identifier::new(artifact).ok())
            {
                objects.insert(artifact, key.clone());
            }
        }
        if let Some(alias) = model.alias() {
            by_alias.insert(alias.to_owned(), objects);
        }
    }
    (by_alias, identities)
}

impl CheckedGraph {
    /// ADR-011 §2.4: the selection of each domain package this package was
    /// checked against, ascending by identity: the lock's
    /// `model_selections`.
    pub fn model_selections(&self) -> &[DomainPackageRef] {
        &self.model_selections
    }

    /// FR-001: the `RawSourceRef` of the unit this package was checked
    /// from, the lock's `sources` entry (QSpec FR-322 `PackageLock`).
    pub fn source(&self) -> &qsl_foundation::source::provenance::RawSourceRef {
        &self.source
    }

    /// FR-092/FR-093: every lowered, keyed node of this package's functions.
    pub fn semantic_graph(&self) -> &SemanticGraph {
        &self.semantic_graph
    }

    /// NFR-011: the ceilings this package was checked under.
    pub fn effective_limits(&self) -> CheckingLimits {
        self.effective_limits
    }

    /// ADR-013 O-04: `node`'s `DeclarationKey`, read only from the model
    /// correspondence this package's own checking recorded (FR-088-AC-2).
    /// Node-id-keyed, never name-keyed (R-06/FR-088-AC-5): `CheckedGraph`
    /// exposes no accessor that takes a name and returns a node id or
    /// declaration.
    pub fn resolve_declaration(
        &self,
        node: quire_exact::NodeKey,
    ) -> Option<&crate::model::key::DeclarationKey> {
        self.model_correspondence.resolve(node)
    }

    /// ADR-013 O-14/C-26 (PR #300 review finding 1): `node`'s own checked
    /// type descriptor, when this package admitted a composite or enum
    /// declaration that minted that node id. Node-id-keyed, never
    /// name-keyed (R-06), like every other accessor here.
    pub fn checked_type_node(&self, node: quire_exact::NodeKey) -> Option<&CheckedTypeNode> {
        self.type_nodes.get(&node)
    }

    /// Every checked type node this package admitted, in node-id order --
    /// the listing accessor a test or a future C-26 consumer uses to find a
    /// declaration's own minted node id without independently recomputing
    /// it (there is no name-keyed lookup, R-06).
    pub fn checked_type_nodes(&self) -> impl Iterator<Item = &CheckedTypeNode> {
        self.type_nodes.values()
    }

    /// Check a standalone expression over `parameters`, against `expected`
    /// when given, as a function or operation body (`ClauseKind::Body`).
    /// `pre(...)` refuses `wrong_snapshot`/`wrong-anchor` here: this is not
    /// an operation's postcondition, the only clause FR-153's anchor table
    /// admits it in. Use [`Self::check_postcondition_expression`] to check a
    /// real postcondition, where `pre(...)` is legal, or
    /// [`Self::check_clause_expression`] for any other [`ClauseKind`].
    pub fn check_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        self.check_clause_expression(
            parameters,
            expression,
            expected,
            ClauseKind::Body,
            mode,
            limits,
        )
    }

    /// Check a standalone expression as an operation's postcondition
    /// (`ClauseKind::Postcondition`): identical to [`Self::check_expression`],
    /// except `pre(...)` is legal (FR-153's own anchor table;
    /// shared-grammar.md's caller-side anchor operations), subject to its
    /// own eligible-operand rule (FR-042's Behavior clause, `Typer`'s
    /// `Expression::Pre` arm).
    pub fn check_postcondition_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        self.check_clause_expression(
            parameters,
            expression,
            expected,
            ClauseKind::Postcondition,
            mode,
            limits,
        )
    }

    /// Check a standalone expression over `parameters` as `clause_kind`,
    /// against `expected` when given. FR-151's dispatch-call restriction
    /// (TC-196 D06/D07) gates on `clause_kind`, not on syntax alone; `pre(...)`
    /// is legal exactly when `clause_kind` is [`ClauseKind::Postcondition`]
    /// (`Typer`'s `Expression::Pre` arm).
    pub fn check_clause_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        clause_kind: ClauseKind,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        let location = root(Origin::Expression);
        let mut nodes = 0_u64;
        let mut typer = Typer::new(
            &self.scope,
            &self.functions.signatures,
            limits,
            &mut nodes,
            clause_kind,
        );
        bind_parameters(&mut typer, &parameters, &location)?;
        let root = match expected {
            Some(expected) => {
                typer.check_declared_type(expected, &location)?;
                typer.check_as(expression, expected, &location)?
            }
            None => typer.infer(expression, None, &location)?,
        };
        let slots = typer.slots();
        if mode == CheckMode::Linked {
            Definedness::new(
                parameters.len(),
                &self.dispatch_tables,
                &self.scope.dispatch_operations,
            )
            .check(root.root())?;
        }
        Ok(CheckedExpression {
            parameters,
            root,
            slots,
            effective_limits: limits,
        })
    }

    /// FR-104: the checked state clause declared `name`. Clauses and
    /// functions share one namespace, so a name names at most one
    /// declaration of either.
    pub fn state_clause(&self, name: &str) -> Option<&CheckedStateClause> {
        self.state_clauses.iter().find(|clause| clause.name == name)
    }

    /// FR-104: every checked state clause, in declaration order.
    pub fn state_clauses(&self) -> &[CheckedStateClause] {
        &self.state_clauses
    }

    /// FR-113: every checked protocol declaration, in declaration order.
    pub fn protocols(&self) -> &[CheckedProtocol] {
        &self.protocols
    }

    /// FR-113: the checked protocol declared `name`, or `None`.
    pub fn protocol(&self, name: &str) -> Option<&CheckedProtocol> {
        self.protocols.iter().find(|protocol| protocol.name == name)
    }

    /// FR-115 (ADR-017 PF-3): the operation `model::object::operation`
    /// names, resolved once against this package: `model` to the domain
    /// package selection its `model` declaration binds, and `object` to the
    /// object type of that package whose artifact id it is. `operation`
    /// stays an identifier, looked up under the resolved declaration key by
    /// [`Self::operation_frame`]. `None` when the alias or the type does not
    /// resolve; the entry then reports `missing_declaration`/`missing-name`.
    pub fn resolve_operation(
        &self,
        model: &Identifier,
        object: &Identifier,
        operation: &Identifier,
    ) -> Option<OperationSelection> {
        let object = self.model_objects.get(model.as_str())?.get(object)?;
        Some(OperationSelection {
            object: object.clone(),
            operation: operation.clone(),
        })
    }

    /// FR-115: the frame of the operation `selection` names, and its object
    /// type's identity. The operation resolves in the object type's
    /// effective view exactly as FR-104's Resolution resolves a clause's
    /// operation (`OperationTable::resolve`), so an inherited operation
    /// yields its declaring type's frame. `None` when the operation
    /// resolves to no single operation, or no clause or attempt of this unit
    /// names it (the package then holds no frame node for it, FR-105).
    ///
    /// Only [`Self::resolve_operation`] builds a selection; a formatted name
    /// does not select:
    /// ```compile_fail,E0308
    /// # fn select(graph: &qsl_semantics::check::CheckedGraph) {
    /// graph.operation_frame(&format!("Config::ConfigVersion::attemptUpdate"));
    /// # }
    /// ```
    pub fn operation_frame(
        &self,
        selection: &OperationSelection,
    ) -> Option<(quire_exact::EffectiveId, &CheckedOperationFrame)> {
        let context = *self.object_types.get(&selection.object)?;
        let crate::model::operation::OperationLookup::Declared {
            declaring,
            operation,
        } = self.scope.operations().resolve(
            self.scope.types(),
            context,
            selection.operation.as_str(),
        )
        else {
            return None;
        };
        self.operation_frames
            .iter()
            .find(|frame| {
                frame.operation.declaring == declaring
                    && frame.operation.declaration.name() == operation.name()
            })
            .map(|frame| (context, frame))
    }

    /// FR-115: the operation frame whose frame node identity is `frame`,
    /// the lookup the S6a `ProtocolClause` frame arm resolves its key by.
    /// Two operations share a frame node only when the node's content --
    /// declaring type, `modifies`, `creates` and `deletes` -- is equal
    /// (FR-105), so either one's effect is the same frame.
    pub fn operation_frame_by_identity(
        &self,
        frame: quire_exact::NodeKey,
    ) -> Option<&CheckedOperationFrame> {
        self.operation_frames
            .iter()
            .find(|operation| operation.frame == frame)
    }

    /// FR-107: the checked state clause whose minted node identity is
    /// `identity` (`CheckedStateClause::identity`) -- the accessor the S6a
    /// `ProtocolClause` evaluate hook resolves its bare identity key
    /// against, node-id-keyed like every other S6a lookup (R-06). `None`
    /// when no clause of this package mints that identity.
    pub fn state_clause_by_identity(
        &self,
        identity: quire_exact::NodeKey,
    ) -> Option<&CheckedStateClause> {
        self.state_clauses
            .iter()
            .find(|clause| clause.identity() == identity)
    }

    /// The first function named `name`, with its signature.
    fn function(&self, name: &str) -> Option<(&Signature, &CheckedFunction)> {
        self.functions.named(name)
    }

    /// The `deref(r).f` locations of a function body, or `None` for an
    /// undeclared name.
    pub fn dereferences(&self, function: &str) -> Option<Vec<Location>> {
        self.function(function)
            .map(|(_, function)| function.body.root().dereferences())
    }

    /// The `convert` losses of a function body, or `None` for an undeclared
    /// name.
    pub fn losses(&self, function: &str) -> Option<Vec<CollectionLoss>> {
        self.function(function)
            .map(|(_, function)| function.body.root().losses())
    }

    /// FR-065-AC-2: `name`'s checked identity, minted once at `check` and
    /// unchanged by anything else in the package. `None` for an undeclared
    /// name.
    pub fn function_identity(&self, name: &str) -> Option<quire_exact::NodeKey> {
        self.function(name).map(|(_, function)| function.identity)
    }

    /// FR-062-AC-2/FR-065-AC-3: the source location recorded for one
    /// occurrence (identity, role, ordinal) of a checked function
    /// declaration or function-application call, if `check` recorded one.
    pub fn occurrence(
        &self,
        identity: quire_exact::NodeKey,
        origin: &quire_exact::Origin,
    ) -> Option<&Location> {
        self.occurrences.resolve(identity, origin)
    }

    /// ADR-013 O-07: every occurrence `check` recorded, as (node, origin,
    /// location), ascending by node and role, each role in ordinal order.
    /// FR-093: every lowered node has at least one.
    pub fn occurrences(
        &self,
    ) -> impl Iterator<Item = (quire_exact::NodeKey, quire_exact::Origin, &Location)> {
        self.occurrences.iter()
    }

    /// This package's requirement records (ADR-012 §13.5, ADR-011 E7): one
    /// per claim site, keyed by the site's occurrence key, in the request
    /// order. `qsl-route`'s request builder reads these from the
    /// in-process `CheckedPackage` through `CheckedPackage::graph().
    /// requirements()`.
    pub fn requirements(&self) -> &BTreeMap<OccurrenceKey, RequirementRecord> {
        &self.requirements
    }

    /// One admitted function's own name, checked body and evaluation slot
    /// count, by its index in the package -- the index a checked
    /// `NodeKind::Call` or dispatch candidate names. The accessor
    /// `value::expression`'s evaluator reads where a call runs, so no
    /// per-evaluation function list is built, and `check` never
    /// constructs a layer-5 type (FR-068-AC-3).
    pub fn function_state(&self, index: usize) -> Option<FunctionState<'_>> {
        self.functions.get(index).map(function_state)
    }

    /// One admitted function's evaluation-visible state, by its minted
    /// identity -- the accessor
    /// `value::expression`'s `ReferenceEvaluation::evaluate`'s `Value` family
    /// implementation (`value::expression::family`) resolves a checked call
    /// against.
    pub fn function_by_identity(
        &self,
        identity: quire_exact::NodeKey,
    ) -> Option<FunctionState<'_>> {
        self.functions.with_identity(identity).map(function_state)
    }

    /// ADR-015 D-5: the signature of the function callable by name whose
    /// checked identity has the bytes of `node`, found among the identities
    /// this graph holds and never by minting a `NodeKey` (ADR-013 O-04,
    /// R-10), with its function index in this graph, which the imported
    /// call records so evaluation reads it by [`Self::function_state`].
    /// `None` when no such function is declared.
    pub(crate) fn imported_function(
        &self,
        node: &qsl_foundation::digest::WireNodeId,
    ) -> Option<(usize, &Signature)> {
        let position = self
            .functions
            .by_identity
            .iter()
            .find(|(identity, _)| identity.as_bytes() == node.as_bytes())
            .map(|(_, position)| *position)?;
        self.functions
            .get(position)
            .map(|(signature, _)| (position, signature))
            .filter(|(_, signature)| signature.callable_by_name)
    }

    /// `name`'s identity and declared parameters, filtered to functions a
    /// plain named call may resolve to (`callable_by_name`) -- the accessor
    /// `value::expression::CheckedPackageEvaluation::call` uses to resolve a
    /// runtime `QualifiedName` lookup and validate its caller's arguments,
    /// without a direct field read (TC-196 D07's bypass: a crate-internal
    /// FR-151 synthesized dispatch candidate is never reachable this way).
    pub fn callable(&self, name: &str) -> Option<CallableFunction<'_>> {
        self.function(name)
            .filter(|(signature, _)| signature.callable_by_name)
            .map(|(signature, function)| CallableFunction {
                identity: function.identity,
                parameters: &signature.parameters,
                result: &signature.result,
            })
    }

    /// The scope every declared name resolves against -- the accessor
    /// `value::expression::CheckedPackageEvaluation::evaluate` passes through
    /// to the evaluator.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// The checked dispatch tables `scope`'s dispatch operations index into
    /// -- the accessor the evaluator reads directly, alongside `scope`.
    pub fn dispatch_tables(&self) -> &[DispatchTable] {
        &self.dispatch_tables
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{Origin, Role};

    use super::*;

    fn declarations(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
        PackageDeclarations {
            functions,
            ..PackageDeclarations::new(family::fixtures::fixture_source())
        }
    }

    /// PR #300 review finding 1: a real composite declaration in
    /// `PackageDeclarations.types` becomes a real `CheckedTypeNode`,
    /// convertible through C-26, not only in `identity`'s own
    /// hand-constructed unit tests.
    ///
    /// FR-092-AC-12: the checked type node's id is the record's FR-092 node
    /// key, the key of its semantic-graph node, not the handle the caller
    /// passed to `CompositeDeclaration::new`.
    #[trace("TC-259", "FR-088-AC-7")]
    #[test]
    fn composite_declaration_becomes_a_real_checked_type_node() {
        use quire_exact::{NodeKey, Presence};
        use quire_semantic_value::declaration::FieldDeclaration;
        use quire_semantic_value::declaration::{
            CompositeDeclaration, CompositeShape, TypeEnvironment,
        };

        let field = FieldDeclaration::new("flag", ValueType::Boolean, Presence::Required);
        let key = NodeKey::from_digest([0x11; 32]);
        let composite =
            CompositeDeclaration::new(key, "Flagged", CompositeShape::Record(vec![field]));
        let types = TypeEnvironment::new([composite], []).expect("one record admits cleanly");
        let graph = PackageDeclarations {
            types,
            ..PackageDeclarations::new(family::fixtures::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("one record declaration checks cleanly");

        let nodes: Vec<&CheckedTypeNode> = graph.checked_type_nodes().collect();
        assert_eq!(nodes.len(), 1, "exactly the one declared composite");
        let node = nodes[0].node();
        assert_ne!(node, key, "the caller's handle is not a node id");
        let declared = graph
            .semantic_graph()
            .node(node)
            .expect("the checked type node's id is a graph node's key");
        assert_eq!(
            declared.declaration().map(|name| name[0].as_str()),
            Some("Flagged")
        );
        assert_eq!(graph.checked_type_node(node), Some(nodes[0]));
        assert!(graph.checked_type_node(key).is_none());
        assert_eq!(
            to_kernel_value_type(nodes[0]),
            quire_exact::ValueType::Composite(node)
        );
    }

    /// PR #300 review round 2 (HIGH-2): a package-qualified declared name
    /// (`"P::R"`, the FR-143 R-03 fixture convention, `tests/it/
    /// composite_values.rs`) gets a real checked type node -- the prior
    /// code silently dropped any composite whose declared name was not
    /// itself a single valid `Identifier` (no refusal, no diagnostic,
    /// `check()` still returning `Ok`, review round 2's own probe: "0
    /// checked type nodes, check returns Ok, no refusal or diagnostic").
    /// The node id is the record's FR-092 key (FR-092-AC-12), whose
    /// `declaration` carries the qualified name's segments.
    #[trace("TC-252", "FR-088-AC-9")]
    #[test]
    fn composite_type_node_for_a_qualified_declared_name() {
        use quire_exact::{NodeKey, Presence};
        use quire_semantic_value::declaration::FieldDeclaration;
        use quire_semantic_value::declaration::{
            CompositeDeclaration, CompositeShape, TypeEnvironment,
        };

        let field = FieldDeclaration::new("flag", ValueType::Boolean, Presence::Required);
        let key = NodeKey::from_digest([0x22; 32]);
        let composite = CompositeDeclaration::new(key, "P::R", CompositeShape::Record(vec![field]));
        let types = TypeEnvironment::new([composite], []).expect("one record admits cleanly");
        let graph = PackageDeclarations {
            types,
            ..PackageDeclarations::new(family::fixtures::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("a package-qualified declared name checks cleanly");

        let nodes: Vec<&CheckedTypeNode> = graph.checked_type_nodes().collect();
        assert_eq!(
            nodes.len(),
            1,
            "a package-qualified declared name must still get a checked type node"
        );
        assert_ne!(nodes[0].node(), key);
        let declared = graph
            .semantic_graph()
            .node(nodes[0].node())
            .expect("the checked type node's id is a graph node's key");
        let name: Vec<&str> = declared
            .declaration()
            .unwrap_or_default()
            .iter()
            .map(|segment| segment.as_str())
            .collect();
        assert_eq!(name, ["P", "R"]);
    }

    /// PR #300 review round 2 (HIGH-1, L10): the enum/Sum companion to
    /// `composite_declaration_becomes_a_real_checked_type_node` above --
    /// round 1's own review named this as a real gap ("no real-`check()`
    /// test for the enum/Sum path"). A real `AdmittedEnumDeclaration`, admitted the
    /// same way `qsl-eval/tests/it/collection_algebra.rs`'s own fixtures build one,
    /// becomes a real `CheckedTypeNode::Sum` whose own node id is exactly
    /// `AdmittedEnumDeclaration::key()` -- the same identity `Typer::type_named`
    /// and every `EnumValue::declaration()` already resolve a reference
    /// against (FR-088-AC-7 step 4) -- and whose `VariantId`s are minted
    /// from that same key, never a second, parallel one (FR-088-AC-10).
    #[trace("TC-259", "FR-088-AC-7")]
    #[test]
    fn enum_declaration_becomes_a_real_checked_type_node() {
        use crate::value::enumeration::{
            AdmittedEnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
        };
        use crate::value::semantic_node::{
            NodeIdentityPreimage, NodeOwner, OwnerSelection, OwnerSubject,
        };
        use quire_exact::{NodeKey, NODE_KEY_DOMAIN};
        use serde_json::json;

        let owners = OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
            authority: "agent-ix".to_owned(),
            identity: "example-model".to_owned(),
        })]);
        let declaration_json = json!({
            "version": "quire.enum-declaration-node/v1",
            "owner": {"kind": "definition", "authority": "agent-ix", "identity": "example-model"},
            "qualified_declaration": ["Example", "Status"],
            "ordered": false,
            "members": ["Active", "Closed"],
        });
        let declaration_preimage = EnumDeclarationPreimage::from_json(declaration_json).unwrap();
        let declaration_key = NodeKey::from_digest(declaration_preimage.digest().unwrap());
        let declaration =
            AdmittedEnumDeclaration::admit(declaration_preimage, declaration_key, &owners).unwrap();

        let members = ["Active", "Closed"]
            .into_iter()
            .map(|case| {
                let member_json = json!({
                    "version": "quire.enum-member-node/v1",
                    "declaration_node_id": {
                        "domain": NODE_KEY_DOMAIN,
                        "digest": declaration.key().to_string(),
                    },
                    "case": case,
                });
                let member_preimage = EnumMemberPreimage::from_json(member_json).unwrap();
                let member_key = NodeKey::from_digest(member_preimage.digest().unwrap());
                declaration
                    .admit_member(&member_preimage, member_key)
                    .unwrap()
            })
            .collect();

        let enums = vec![EnumBinding {
            name: "Status".to_owned(),
            declaration,
            members,
        }];
        let graph = PackageDeclarations {
            enums,
            ..PackageDeclarations::new(family::fixtures::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("one enum declaration checks cleanly");

        let nodes: Vec<&CheckedTypeNode> = graph.checked_type_nodes().collect();
        assert_eq!(nodes.len(), 1, "exactly the one declared enum");
        assert_eq!(
            nodes[0].node(),
            declaration_key,
            "the checked type node's id must be the declaration's own existing key"
        );
        assert_eq!(graph.checked_type_node(declaration_key), Some(nodes[0]));

        let quire_exact::ValueType::Enum(ref shape) = to_kernel_value_type(nodes[0]) else {
            panic!("the sum form must convert to ValueType::Enum");
        };
        for case in ["Active", "Closed"] {
            assert!(shape.contains(crate::value::enumeration::mint_variant_id(
                declaration_key,
                case
            )));
        }
    }

    /// PR #300 review finding 7: TC-249's own identity/occurrence-key
    /// mechanism, demonstrated through a real `check()` run rather than
    /// only a hand-built `OccurrenceMap` (`identity`'s own unit test keeps
    /// that mechanism coverage; this is the real-checker, adverse
    /// companion FR-088-AC-3 also names). This crate has no `claim`/
    /// `temporal`/`protocol` clause syntax yet (FR-088-CON-1 scopes clause
    /// identity to the identity/occurrence-key mechanism only, not the
    /// syntax that would produce one), so the closest real,
    /// checker-produced case of "one identity, two distinct occurrences"
    /// is two calls to the same function with the same arguments: the
    /// FR-093 call node's key hashes the callee's key and the arguments
    /// alone, never a resolved index or the caller's own identity, so both
    /// call sites share one node -- the same
    /// "structurally identical content shares one id" premise ADR-013 O-04
    /// states for clause identity -- while `check`'s own `OccurrenceMap`
    /// still gives each call site its own (role, ordinal), exactly O-07's
    /// shape.
    #[trace("TC-249", "FR-088-AC-3")]
    #[test]
    fn call_occurrences_of_the_same_callee_share_an_identity_and_disambiguate_by_occurrence_key() {
        fn literal_function(name: &str, body: Expression) -> FunctionDeclaration {
            FunctionDeclaration::new(
                name,
                Vec::new(),
                qsl_forms::TypeForm::builtin(
                    qsl_forms::BuiltinType::Boolean,
                    qsl_foundation::Span { start: 0, end: 0 },
                ),
                None,
                body,
            )
        }
        let call_helper = || Expression::call("helper".to_owned(), Vec::new());
        let graph = declarations(vec![
            literal_function("helper", Expression::boolean(true)),
            literal_function("caller_one", call_helper()),
            literal_function("caller_two", call_helper()),
        ])
        .check(CheckingLimits::default())
        .expect("two callers of one no-argument function check cleanly");

        // Steps 2-3: both call sites mint the same identity, but distinct
        // occurrence keys.
        let call_identity = graph
            .semantic_graph()
            .nodes()
            .find(|node| node.semantic_form() == "call")
            .map(SemanticNode::key)
            .expect("the call node was lowered");
        assert_eq!(
            graph
                .semantic_graph()
                .nodes()
                .filter(|node| node.semantic_form() == "call")
                .count(),
            1,
            "both call sites lower to one node"
        );
        let first = Origin::new(Role::new("expression"), 0);
        let second = Origin::new(Role::new("expression"), 1);
        let first_location = graph
            .occurrence(call_identity, &first)
            .expect("the first call site was recorded")
            .clone();
        let second_location = graph
            .occurrence(call_identity, &second)
            .expect("the second call site was recorded")
            .clone();

        // Step 4: (identity, occurrence key) disambiguates what identity
        // alone cannot; a third, never-recorded ordinal resolves to
        // nothing rather than aliasing an existing occurrence.
        assert_ne!(
            first_location, second_location,
            "two distinct source occurrences must not collapse to one location"
        );
        let third = Origin::new(Role::new("expression"), 2);
        assert_eq!(graph.occurrence(call_identity, &third), None);

        // Step 5 (adverse, R-05): renaming both callers changes their
        // *diagnostic* location text (each `Location` names its own
        // enclosing function, exactly as a diagnostic pointer should), but
        // leaves the shared call *identity* and both *occurrence keys*
        // (role, ordinal) unchanged -- display/diagnostic text is never
        // load-bearing for identity or the occurrence key that
        // disambiguates it, only for where a human-readable message points.
        let renamed = declarations(vec![
            literal_function("helper", Expression::boolean(true)),
            literal_function("renamed_caller_one", call_helper()),
            literal_function("renamed_caller_two", call_helper()),
        ])
        .check(CheckingLimits::default())
        .expect("renaming the callers changes no checking outcome");
        assert!(
            renamed.occurrence(call_identity, &first).is_some(),
            "the same call identity and first occurrence key still resolve after renaming"
        );
        assert!(
            renamed.occurrence(call_identity, &second).is_some(),
            "the same call identity and second occurrence key still resolve after renaming"
        );
        assert_ne!(
            renamed.occurrence(call_identity, &first),
            Some(&first_location),
            "the diagnostic location's own embedded function name is expected to change"
        );
    }

    /// ADR-013 O-07 (FR-322): every node of a checked package's semantic
    /// graph has at least one source occurrence. A function with a
    /// parameter, a call and a conditional lowers type, parameter,
    /// expression and function nodes, and each is recorded.
    #[trace("TC-420", "FR-095-AC-1")]
    #[test]
    fn every_lowered_node_has_a_source_occurrence() {
        let boolean = || {
            qsl_forms::TypeForm::builtin(
                qsl_forms::BuiltinType::Boolean,
                qsl_foundation::Span { start: 0, end: 0 },
            )
        };
        let graph = declarations(vec![
            FunctionDeclaration::new(
                "helper",
                vec![("flag".to_owned(), boolean())],
                boolean(),
                None,
                Expression::name("flag".to_owned()),
            ),
            FunctionDeclaration::new(
                "caller",
                Vec::new(),
                boolean(),
                None,
                Expression::if_then_else(
                    Expression::boolean(true),
                    Expression::call("helper".to_owned(), vec![Expression::boolean(false)]),
                    Expression::boolean(false),
                ),
            ),
        ])
        .check(CheckingLimits::default())
        .expect("a caller of a one-parameter function checks");
        let mut nodes = 0;
        for node in graph.semantic_graph().nodes() {
            nodes += 1;
            assert!(
                graph.occurrences.has(node.key()),
                "node {} ({}) has no source occurrence",
                node.key(),
                node.semantic_form()
            );
        }
        assert!(nodes >= 5, "only {nodes} nodes lowered");
    }

    /// A resolved-signature entry keyed past `functions` stands in for no
    /// declaration, and is refused rather than ignored.
    #[test]
    fn an_out_of_range_resolved_signature_is_refused() {
        let mut resolved_signatures = check::ResolvedSignatures::default();
        resolved_signatures.insert(0, (Vec::new(), ValueType::Boolean));
        let refusals = PackageDeclarations {
            resolved_signatures,
            ..PackageDeclarations::new(family::fixtures::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect_err("index 0 names no function");
        assert!(matches!(
            refusals.as_slice(),
            [CheckRefusal {
                cause: CheckCause::InvalidDispatchDeclaration(box_detail),
                ..
            }] if matches!(
                **box_detail,
                InvalidDispatchDeclaration::ResolvedSignatureOutOfRange { index: 0 }
            )
        ));
    }
}
