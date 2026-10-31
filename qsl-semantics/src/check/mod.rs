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
//! refinement consumes retained source facts inside this module's own
//! private `field_refinement` submodule. `check` (layer-3-later)
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
mod abstraction;
mod assemble;
mod call_target;
mod capability;
#[allow(clippy::module_inception)]
mod check;
mod checked_dispatch;
mod claims;
mod dispatch_link;
pub mod footprint;
pub use abstraction::{
    AbstractionKey, AbstractionRefusal, AuthoredBinding, CheckedAbstractionRelation,
    CheckedBinding, CheckedFrameBinding, FrameBinding, FrameNodes, MalformedEntry, ObjectBinding,
    OperationKey, PopulationBinding, RustField, RustParameter, RustPath, RustReceiver,
};
pub use abstraction::{AbstractionNodeRefusal, AbstractionNodes};
pub use dispatch_link::PendingLinks;
mod facts;
mod family;
// The scalar limits of a caller that bounds nothing itself, for the model
// layer's kernel admissions (a Text snapshot value's retention).
pub(crate) use family::SCALAR_LIMITS_UNLIMITED as UNBOUNDED_SCALAR_LIMITS;
mod field_refinement;
mod identity;
mod model_conformance;
mod model_gate;
mod navigation;
// The nested expression forms of TC-415, shared by this crate's tests and the
// package emitter's tests, so only `test-support` exposes them.
#[cfg(any(test, feature = "test-support"))]
pub mod depth_forms;
// The sealed unions the evaluator's and replay's union tests run over, while
// no checked package can hold a union (FR-319).
#[cfg(any(test, feature = "test-support"))]
pub mod union_fixture;
// `imports` has no production caller yet (E3 imported-name resolution,
// FR-087-AC-13). Only the layer-4 reader's tests use it, so it is public
// only under `test-support` (no test-only `pub`).
#[cfg(any(test, feature = "test-support"))]
pub mod imports;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod imports;
mod ir;
mod key_order;
mod layer;
mod lowering;
pub(crate) use assemble::variant_type_declarations;
pub(crate) use lowering::record_node_key;
mod hybrid;
mod hyper_clause;
mod hyper_typing;
mod node_key;
mod observation;
mod operation_body;
mod operation_clause;
mod probabilistic;
mod profile;
mod protocol_clause;
mod protocol_controls;
mod protocol_memory;
mod protocol_obligation;
mod protocol_reference;
mod protocol_relationship;
mod protocol_requirement;
mod protocol_slots;
mod references;
mod refinement;
mod refusal;
mod region;
mod settings;
mod state_clause;
mod state_graph_claim;
mod state_model;
mod step_simulation;
mod sum_case;
mod task_set;
mod temporal_clause;
mod terminal;
mod time_units;
mod timed_model;
pub use probabilistic::{
    Bound, BoundedMeasure, CheckedPredicate, CheckedProbabilisticClaim, CheckedProbabilisticModel,
    CheckedTemporalFormula, ClaimNormalForm, Confidence, ConfidenceFault, Delay, DelayDistribution,
    DelayFault, Event, FractionMeasure, ModelSubjectRef, OverBinding, PreconditionSite, ProbForm,
    ProbScheduler, ProbabilisticCause, ProbabilisticDeclaration, ProbabilisticFeature,
    ProbabilisticOperation, RandomParameter, ReachTarget, Reward, SupportValue, WeightOwner,
    Workload,
};
pub use state_model::StateModelRefusal;
pub use sum_case::{mint_union_member_key, ExhaustivenessObligation, SumCaseCause};
pub use task_set::{
    Arrival, CheckedScheduleClaim, CheckedTaskSets, Criticality, Release, ScheduleClaim,
    SchedulingPolicy, Task, TaskSet, TaskSetDeclaration, TaskSetUnit, Wcet,
};
#[cfg(any(test, feature = "test-support"))]
pub mod stratum;
mod termination;
pub use refinement::{
    observation, AbstractSide, AbstractStepClass, AbstractStepKind, ChannelStepKind,
    CheckedRefinement, CheckedRow, FieldRow, HistoryRow, HistoryUpdate, KeyedCatalogClass,
    KeyedClassCatalog, KeyedMemoryClass, LabelStep, ListDefect, ListDefectKind, MemoryStepKind,
    ObjectRow, PopulationRow, PopulationUse, PopulationValue, ProtocolStepKind,
    RefinementCatalogClass, RefinementCatalogNode, RefinementCause, RefinementClassCatalog,
    RefinementDeclaration, RefinementFairness, RefinementFairnessOrigin, RefinementRefusal,
    RefinementStepSource, RefinementSupport, RefinementThreadTarget, RefinementV2Emission,
    RefinementV2EncodingError, RoleStepKind, RowClass, RowObservation, StepClass, StepLabel,
    StepRow, StepTarget, UnsupportedFeature,
};
pub use terminal::{
    CheckedTerminal, CheckedTerminalPredicate, TerminalClauseDeclaration, TerminalClauseKind,
    TerminalDeclaration, TerminalMode, TerminalNode, TerminalRefusal,
};
mod type_form;
mod type_invariant;
pub use type_invariant::{
    CheckedTypeInvariant, TypeInvariantDeclaration, TypeInvariantRefusal, TypeSlot,
};

use crate::model::domain_package::DomainPackageRef;
use std::collections::{BTreeMap, BTreeSet};

use check::{bind_parameters, NamedScope, Typer};
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
pub use claims::{
    ClaimSite, PathGuard, RequirementMap, RequirementRecord, ResultBound, SiteGuard, ValueClaim,
};
pub use family::{CheckedDeclaration, ValueDeclarations, ValueFunctionFamily};
pub use model_gate::{ModelFamily, ModelGateRefusal, OwnerKind};
pub use profile::{resolve_profiles, ProfileCause, ProfileRefusal};
// Named only by `fixtures::measure_resolved`'s return type.
#[cfg(any(test, feature = "test-support"))]
pub use family::DeclarationMetrics;
pub use lowering::{
    model_root_key, AdmittedModel, AuthoredBound, ForeignView, ModelClause, SemanticGraph,
    SemanticNode,
};
pub use node_key::{AbstractionTerm, FrameEntry, ObjectEntry, PopulationEntry};
pub use node_key::{
    AggregateTerm, ApplicationTerm, Binding, BindingValue, BodyTerm, FrameDeletesScoped,
    FrameField, FrameModifies, FrameScope, FrameTerm, GroupMember, GroupTerm, InvalidModelOwner,
    InvalidSourceOwner, LawRole, LeafSegment, LeafTerm, LiteralValue, MemberTerm, ModelOwner,
    NodeKeyRefusal, NodeRef, NodeTag, Operation, OperationLaw, OperationLeaf, OperationMode,
    Operator, Owner, SourceOwner, TupleMember, TupleTerm,
};

/// Carry a declared node identity from an admitted package name and its
/// explicit, ordered path segments. An opaque IR declaration string is one
/// segment; callers must not split it into inferred scopes.
pub(crate) fn declared_node_key(
    package: String,
    declared_path: Vec<String>,
    ordinal: u64,
) -> Result<quire_exact::NodeKey, quire_exact::InvalidNodeKey> {
    let requested = declared_path
        .len()
        .saturating_mul(std::mem::size_of::<quire_exact::PathSegment>());
    let mut path = Vec::new();
    path.try_reserve_exact(declared_path.len())
        .map_err(|_| quire_exact::InvalidNodeKey::Allocation { requested })?;
    path.extend(
        declared_path
            .into_iter()
            .map(quire_exact::PathSegment::Text),
    );
    quire_exact::NodeKey::new(package, path, ordinal)
}

/// Mint a producer-selected structural identity under QSpec FR-322's
/// `{package, path, ordinal}` rule. This includes the special clock-binding
/// path and recursion-group members, whose paths are producer-selected.
pub fn structural_node_key(
    package_name: &str,
    path: Vec<quire_exact::PathSegment>,
    ordinal: u64,
) -> Result<quire_exact::NodeKey, quire_exact::InvalidNodeKey> {
    quire_exact::NodeKey::new(package_name.to_owned(), path, ordinal)
}

/// Mint a derived node under QSpec FR-322's non-group path rule:
/// `[node_tag, semantic_form, ...operands]` in the containing package.
/// Recursion-group members use their producer-chosen path instead.
pub fn derived_node_key(
    package_name: &str,
    node_tag: NodeTag,
    semantic_form: &str,
    operands: Vec<quire_exact::PathSegment>,
    ordinal: u64,
) -> Result<quire_exact::NodeKey, quire_exact::InvalidNodeKey> {
    let requested = operands
        .len()
        .checked_add(2)
        .and_then(|len| len.checked_mul(std::mem::size_of::<quire_exact::PathSegment>()))
        .unwrap_or(usize::MAX);
    let mut path = Vec::new();
    path.try_reserve_exact(operands.len().saturating_add(2))
        .map_err(|_| quire_exact::InvalidNodeKey::Allocation { requested })?;
    path.push(quire_exact::PathSegment::Text(node_tag.as_str().to_owned()));
    path.push(quire_exact::PathSegment::Text(semantic_form.to_owned()));
    path.extend(operands);
    structural_node_key(package_name, path, ordinal)
}

/// The structural key of QSpec's built-in Rational scalar type (FR-092).
/// Timed tick sources use this primitive literal type even when the checked
/// unit contains no other rational expression from which S4 could discover it.
pub fn builtin_rational_type_key(
    package_name: &str,
) -> Result<quire_exact::NodeKey, quire_exact::InvalidNodeKey> {
    declared_node_key(
        package_name.to_owned(),
        vec!["scalar_type".to_owned(), "rational".to_owned()],
        0,
    )
}

/// The structural key of QSpec's built-in Integer scalar type (FR-092): the
/// symbolic element selector of a `clock_place` slot step (QSpec FR-322
/// "Timed-model nodes") references it.
pub fn builtin_integer_type_key(
    package_name: &str,
) -> Result<quire_exact::NodeKey, quire_exact::InvalidNodeKey> {
    declared_node_key(
        package_name.to_owned(),
        vec!["scalar_type".to_owned(), "integer".to_owned()],
        0,
    )
}

/// The structural key of QSpec's built-in Text scalar type (FR-092).
/// Timed source metadata uses this primitive even when no source expression
/// already caused its scalar node to enter the checked graph.
pub fn builtin_text_type_key(
    package_name: &str,
) -> Result<quire_exact::NodeKey, quire_exact::InvalidNodeKey> {
    declared_node_key(
        package_name.to_owned(),
        vec!["scalar_type".to_owned(), "text".to_owned()],
        0,
    )
}
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
    empty_scope, fixture_source, json_depth, limits, measure_resolved, received_kernel_causes,
    root_location, scope_with, SCALAR_LIMITS_UNLIMITED,
};
pub use ir::{
    Arithmetic, CaseArm, CheckedBody, CheckedLiteral, CheckedNode, Connective,
    IntegerDivisionOperator, Node, NodeId, NodeKind, OrderedKind, RecordField, RecordSlot, Slot,
    Visit,
};

pub use assemble::{
    AdmittedImport, AssemblyCause, AssemblyError, AssemblyLimits, AssemblyRefusal, AssemblyRun,
    ClockReadingDeclaration, ClockSite, ClockSlot, ClockTypePosition, ClockUse,
    DimensionEquivalence, EquivalenceClasses, EquivalenceFault, EquivalenceRetained,
    TimedDeclaration, TopologyFault, DEFAULT_DECIMAL_SCALE,
};
pub use capability::{Capability, UnknownCapabilityLabel};
pub use check::{
    DispatchOperation, EnumBinding, ModelRecords, PackageDeclarations, ResolvedSignatures,
    VariantCaseDeclaration, VariantTypeDeclaration,
};
#[cfg(any(test, feature = "test-support"))]
pub use checked_dispatch::checked_dispatch_selected_operation;
pub use checked_dispatch::{
    checked_dispatch_operation, object_type_supertypes, DispatchBridgeRefusal, DispatchRoot,
    MissingClauseField, OperationClauses,
};
pub use operation_body::OperationBodyDeclaration;
pub use operation_clause::OperationClauseDeclaration;
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::location::{Location, Origin};
pub use region::DeclarationRegions;
pub use settings::{
    checking_bounds, environment_bounds, set_checking_bound, set_environment_bound,
};
pub use state_clause::{
    AttemptDeclaration, CheckedOperationFrame, CheckedStateClause, ClauseOperation,
    OperationSelection, PopulationDomain, ProtocolClauseFamily, StateClauseDeclaration,
};
pub use step_simulation::StepSimulationEntry;
pub use type_form::{I128Limit, IntegerOutsideI128, IntegerSite, TypeFormFault};
// PR #300 review finding 4: `mint_type_declaration_identity` was `pub(super)`
// in `identity` for the same reason: no consumer outside `check` minted an
// identity directly. `mint_variant_id` itself has since moved to
// `crate::value::enumeration` (OQ-F ruling): it is `pub` there because
// `check.rs`'s own literal resolution (`Typer::name`) mints a `VariantId` for
// an enum-member literal the same way `identity::to_kernel_value_type`'s C-26
// conversion does, and both are `check`-core call sites of one canonical
// function rather than two.
pub use field_refinement::RefinementObligation;
pub use hybrid::{
    hybrid_horizon, ArithmeticOp, CaptureIndex, CheckedFunctionRef, CheckedHybridClaim,
    CheckedHybridModel, CheckedMode, CheckedTransition, CompareOp, ContinuousVariable,
    HorizonOutOfRange, HybridActivation, HybridCapture, HybridCause, HybridClaimDeclaration,
    HybridClock, HybridDeclaration, HybridFairness, HybridFormula, HybridHorizon, HybridInterval,
    HybridModelIndex, HybridTerm, InitialEntry, LogicOp, MalformedReason, ModeIndex,
    ResetConstraint, TransitionIndex, VariableClass, VariableIndex,
};
#[cfg(any(test, feature = "test-support"))]
pub use hyper_clause::check_hyper_clauses;
pub use hyper_clause::{
    decodable_argument, operation_text, ArgumentRead, AtomSlot, CheckedHyper, CheckedHyperClause,
    CheckedModelRelation, CheckedQuantifier, CheckedStepPredicate, CopyGenerator, CopyGroup,
    HyperCause, HyperDeclaration, HyperDeclarations, HyperForm, HyperIdentity, HyperTraceFamily,
    HyperUnsupported, MatchPart, ModelAlias, ObjectParameter, OperationContractClaim, StepRead,
    STUTTER_TEXT,
};
pub use identity::{
    to_kernel_value_type, CheckedClauseKind, CheckedTypeNode, CompositeForm,
    CorrespondenceConflict, InvalidSumVariants, ModelCorrespondence, ScalarShape, SumVariant,
    SumVariants,
};
pub use ir::{CollectionLoss, CollectionProperty, DispatchCandidate, DispatchTable};
pub use model_conformance::ModelConformanceFailure;
pub use navigation::{EndDeclaration, EndSide, RelationshipDeclaration, Relationships};
pub use profile::TemporalProfile;
pub use protocol_clause::{
    CausalEdge, CheckedAttempt, CheckedAwait, CheckedControl, CheckedControlTree,
    CheckedJoinPolicy, CheckedOperationNodes, CheckedOutstanding, CheckedOwner, CheckedParallel,
    CheckedProtocol, CheckedProtocolBinderKey, CheckedProtocolNodeKeys, CheckedRole, CheckedVia,
    ControlStep, EdgeKind, Port, PortRef, ProtocolNodeId,
};
pub use protocol_controls::{CheckedChannel, CheckedOrdering};
pub use protocol_memory::{
    AccessClass, CheckedAccess, CheckedFence, CheckedMemory, CheckedParallelMemory,
};
pub use protocol_obligation::{
    CaptureOwner, CaptureStage, CheckedActivation, CheckedBinder, CheckedCapture, CheckedCommit,
    CheckedCompensation, CheckedDeadline, CheckedObligations, CheckedRecovery, CheckedRetry,
    ObligationCause, ObligationOwner, ObligationStage, ReadClass,
};
pub use protocol_relationship::CheckedRelationship;
pub use protocol_requirement::CheckedRequirement;
pub use protocol_slots::{
    CheckedCase, CheckedChoice, CheckedLifetime, CheckedLoopProof, CheckedNodeBody, CheckedReaders,
    CheckedRelated, CheckedRepeat, CheckedReplicatedRole, CheckedScheduling, CheckedSlotBinder,
    CheckedSlots, CheckedTerminalMember,
};
/// The premises a [`CheckedChannel`] records exactly as written (FR-218
/// "Channels"), which its readers match on.
pub use qsl_forms::{
    AccessOrdering, ChannelCapacity, ChannelDelivery, ChannelOverflow, LossSelector,
};
pub use refusal::{
    AliasKind, AmbiguousMember, AmbiguousQualified, ArmFault, ChannelEnd, CheckCause, CheckRefusal,
    CheckingLimitKind, CheckingStage, DispatchFunctionRole, DispatchLinkCause,
    DispatchSubtypeFailure, ExcludedClauseForm, ExecutionPointRefusal, IdentityForm, IdentityNode,
    IdentityObserving, InvalidDispatchDeclaration, JoinFault, KeyFault, MeasureObligation,
    MemberCandidate, NavigationBound, Obligation, ObservationReason, PredicateRole,
    ProtocolAnchorCause, ProtocolConstruct, ProtocolReferenceCause, ProvedInterval,
    QualifiedCandidate, RelatedEnd, ShadowedDeclaration, SlotKind, StageLimitCause,
    StateGraphCause, TaskSetDefect, TaskTimeMember, TemporalCause, TimeUnitCause, TimedConstruct,
    TimedOmission, UnwrittenTimedMember,
};
pub use state_graph_claim::{
    CheckedPredicateBody, CheckedStateGraphClaim, CheckedStatePredicate, OverParameter,
    StateGraphClaim, StateGraphDeclaration, StateGraphFamily, StateGraphForm, StatePredicate,
};
#[cfg(any(test, feature = "test-support"))]
pub use temporal_clause::check_clauses;

pub use lowering::{collection_kind, CollectionShape, PlaceRow, PlaceStep};
pub use temporal_clause::{
    Activation, BinaryTemporal, CaptureSite, CheckedFormula, CheckedInterval, CheckedOperator,
    CheckedProtocolFairness, CheckedProtocolTarget, CheckedTemporalClause, FormulaNode,
    FormulaNodeId, ModelTimeBinding, ModelTimeSource, TemporalClaim, TemporalDeclaration,
    TemporalDeclarations, TemporalSelection, TemporalTraceFamily, TimeSourceKind,
    TimedPropertyForm, UnaryTemporal,
};
pub use timed_model::{
    CheckedTimedPredicate, ClockConstraint, ClockField, ClockGuard, ClockId, ClockOp,
    ClockParameterRange, ClockReset, ClockResets, CollectionWrite, ConstraintNodes,
    ConstraintTargetNodes, ConvexPart, ElementBinding, GuardNodes, InvariantNodes, PartNodes,
    QuantifiedFamily, ResetNodes, ResetTargetNodes, SlotRef, SlotSelection, StateAlternative,
    TimeInvariant, TimedClauseNodes, TimedModel, Urgency, Written,
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
            by_identity
                .entry(function.identity.clone())
                .or_insert(position);
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
    /// The package manifest's top-level name, the `package` of every
    /// structural node id the graph's nodes carry.
    package_name: String,
    scope: Scope,
    functions: CheckedFunctions,
    dispatch_tables: Vec<DispatchTable>,
    /// FR-062-AC-2/FR-065-AC-3: the occurrence-keyed source map (identity,
    /// role, ordinal) -> source [`Location`], for every function
    /// declaration and function-application occurrence this package
    /// checked.
    occurrences: family::OccurrenceMap<Location>,
    parameter_spans: BTreeMap<(quire_exact::NodeKey, quire_exact::Origin), qsl_foundation::Span>,
    /// ADR-012 §13.5, ADR-011 E7: this package's requirement records, one
    /// per claim site, keyed by the site's occurrence key. Read through
    /// [`Self::requirements`], the candidate step's own input
    /// (`qsl-route`), not dropped after `check`.
    requirements: claims::RequirementMap,
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
    /// FR-096, FR-337: the expression spans of each declaration site an
    /// `Origin::TypeDeclaration` site name locates, by that name.
    site_spans: BTreeMap<String, qsl_forms::ExpressionSpans>,
    /// FR-096, FR-302: the declaration each function synthesized for
    /// dispatch without form spans was derived from, by function index.
    synthesized_spans: BTreeMap<usize, qsl_foundation::Span>,
    /// FR-096-AC-1: the embedding map the spans resolve through, if any.
    embedding: Option<std::sync::Arc<qsl_foundation::source_map::SourceMap>>,
    /// ADR-011 §2.4 `model_selections`: each admitted domain package's
    /// selection, ascending by identity.
    model_selections: Vec<DomainPackageRef>,
    /// QSpec FR-322: the `model_root` key of each selection, in the same order.
    model_roots: Vec<quire_exact::NodeKey>,
    /// FR-110: the header-selectable layers the unit's header profiles
    /// resolve to, ascending.
    header_layers: Vec<crate::value::layer::HeaderLayer>,
    /// FR-230: checked time sources, in source declaration order.
    time_sources: Vec<ModelTimeSource>,
    /// FR-230/FR-251: exact checked time-unit scales used for timed clauses.
    time_units: time_units::TimeUnits,
    /// FR-104: every checked state clause, in declaration order.
    state_clauses: Vec<CheckedStateClause>,
    /// FR-366: every checked type-level invariant, in declaration order of
    /// the selected models and their clauses.
    type_invariants: Vec<CheckedTypeInvariant>,
    /// FR-124: the intended terminal states of each selected model, by
    /// alias.
    terminals: Vec<CheckedTerminal>,
    /// FR-230: the checked timed model of each time source, in time-source
    /// order.
    timed_models: Vec<TimedModel>,
    /// FR-135 to FR-137: every checked refinement, in declaration order.
    refinements: Vec<CheckedRefinement>,
    /// FR-304: the unit's `abstraction` declarations, in declaration order,
    /// checked into [`Self::abstraction`].
    abstractions: Vec<qsl_forms::AbstractionDeclarationForm>,
    /// FR-304: what each `model` alias exports, for the names an
    /// `abstraction` declaration writes.
    model_names: BTreeMap<String, abstraction::ModelNames>,
    /// FR-304: the relation the `abstraction` declarations check to, when
    /// the unit holds any.
    abstraction: Option<CheckedAbstractionRelation>,
    /// FR-113: every checked protocol declaration, in declaration order.
    protocols: Vec<CheckedProtocol>,
    /// FR-390: emitted protocol node keys mapped back to their one S3
    /// checked protocol declaration and slot rows.
    protocol_node_keys: BTreeMap<String, CheckedProtocolNodeKeys>,
    /// FR-337: every emitted temporal clause, in declaration order.
    temporal_clauses: Vec<TemporalClauseNode>,
    /// FR-488: every emitted hyper clause over `behaviours`, in declaration
    /// order, each carrying its `claim` occurrence.
    hyper_nodes: Vec<hyper_clause::CheckedHyperClause>,
    /// FR-180: every checked model relation emitted as a step-relation claim.
    step_relations: Vec<hyper_clause::CheckedModelRelation>,
    /// FR-165: every checked state-graph claim, in declaration order.
    state_graph_claims: Vec<CheckedStateGraphClaim>,
    /// FR-447: every checked hybrid model, in declaration order.
    hybrid_models: Vec<CheckedHybridModel>,
    /// FR-447: every checked hybrid claim, in declaration order.
    hybrid_claims: Vec<CheckedHybridClaim>,
    /// FR-185: every checked random parameter, reward and workload.
    probabilistic: CheckedProbabilisticModel,
    /// QSL FR-246: every checked task set and the claims over them.
    task_sets: CheckedTaskSets,
    /// FR-115: every operation a state clause or attempt names, with its
    /// frame node, one per (declaring type, operation name).
    operation_frames: Vec<CheckedOperationFrame>,
    /// Each `model` alias's object types, by artifact id: the table
    /// [`Self::field_domain`] resolves a field selection's alias and type in.
    model_objects: BTreeMap<String, BTreeMap<Identifier, quire_checked::key::DeclarationKey>>,
    /// FR-115, FR-121: each `model` alias's operation owners (object types,
    /// processes and state machines), by artifact id: the table
    /// [`Self::resolve_operation`] resolves an operation selection's alias
    /// and owner in.
    model_owners: BTreeMap<String, BTreeMap<Identifier, quire_checked::key::DeclarationKey>>,
    /// FR-116, FR-121: each `model` alias's top-level declarations that are
    /// not operation owners, by artifact id: the nodes that cannot declare
    /// operations a frame names.
    model_non_owners: BTreeMap<String, std::collections::BTreeSet<Identifier>>,
    /// FR-115: every admitted operation owner's effective identity, by its
    /// declaration key.
    owner_types: BTreeMap<quire_checked::key::DeclarationKey, quire_exact::EffectiveId>,
    /// FR-115: every admitted object type's effective identity, by its
    /// declaration key.
    object_types: BTreeMap<quire_checked::key::DeclarationKey, quire_exact::EffectiveId>,
    /// ADR-012 §15.4: every admitted object type's `model`/`object_type`
    /// node key, by its declaration key -- keyed from the declaration
    /// whether or not this package's lowering minted the node, so
    /// [`Self::field_domain`] names a field's declaring type even when no
    /// clause reads it.
    object_nodes: BTreeMap<quire_checked::key::DeclarationKey, quire_exact::NodeKey>,
    /// ADR-012 §15.7: each `model` alias's population declarations, by
    /// artifact id, each with its canonical member type and ordinal, as
    /// [`AdmittedModel::population_domains`] orders them: the table
    /// [`Self::population_domain`] resolves a population in.
    model_populations: BTreeMap<String, BTreeMap<Identifier, state_clause::PopulationDomain>>,
    /// FR-380: every `bound` declaration, ascending by label.
    population_bounds: Vec<CheckedPopulationBound>,
    /// FR-304: every admitted population's declaration key, with its
    /// population domain.
    population_keys: BTreeMap<quire_checked::key::DeclarationKey, state_clause::PopulationDomain>,
    /// FR-304: the key of every declaration of every admitted domain
    /// package, of any kind, so a binding keyed by a declaration of the wrong
    /// kind refuses as malformed rather than missing.
    declaration_keys: std::collections::BTreeSet<quire_checked::key::DeclarationKey>,
    /// FR-096: each protocol's own attempts' declared name spans,
    /// index-aligned with `protocols`, so an `Origin::ProtocolAttempt`
    /// resolves.
    attempt_spans: Vec<Vec<Option<qsl_foundation::Span>>>,
}

/// One emitted temporal clause (FR-337 "Outputs"): the checked clause, its
/// `temporal`/`temporal_clause` node, the `claim` occurrence of this
/// declaration and its `over` parameter's node. Two structurally equal
/// declarations share one node and differ in their `claim` occurrence.
#[derive(Clone, Debug, PartialEq)]
pub struct TemporalClauseNode {
    clause: CheckedTemporalClause,
    identity: quire_exact::NodeKey,
    claim: quire_exact::Origin,
    over: quire_exact::NodeKey,
    /// Each `holds` atom's checked condition and its slot count, by the
    /// atom's formula node index.
    conditions: BTreeMap<usize, (CheckedBody, usize)>,
    /// An `on each ... when (g)` activation's checked guard and its slot
    /// count.
    guard: Option<(CheckedBody, usize)>,
    /// Each capture's checked initializer and its slot count, in source
    /// order.
    captures: Vec<(CheckedBody, usize)>,
}

impl TemporalClauseNode {
    /// The checked clause.
    pub fn clause(&self) -> &CheckedTemporalClause {
        &self.clause
    }

    /// The clause's `temporal`/`temporal_clause` node.
    pub fn identity(&self) -> quire_exact::NodeKey {
        self.identity.clone()
    }

    /// The `claim` occurrence of this declaration of the clause node.
    pub fn claim(&self) -> &quire_exact::Origin {
        &self.claim
    }

    /// The `value`/`parameter` node of the clause's `over` parameter.
    pub fn over(&self) -> quire_exact::NodeKey {
        self.over.clone()
    }

    /// The checked Boolean condition of the `holds` atom `atom` and its
    /// evaluation slot count (FR-337). The condition reads the `over`
    /// parameter at slot 0 and nothing else of the clause. `None` for a node
    /// that is no `holds` atom of this clause.
    pub fn condition(&self, atom: FormulaNodeId) -> Option<(CheckedNode<'_>, usize)> {
        self.conditions
            .get(&atom.index())
            .map(|(body, slots)| (body.root(), *slots))
    }

    /// The checked Boolean guard of an `on each ... when (g)` activation and
    /// its evaluation slot count (FR-337). The guard reads the `over`
    /// parameter at slot 0 and the trigger binder at slot 1, each a model
    /// read of the activation position's current state. `None` when the
    /// activation is `on origin` or writes no `when`.
    pub fn guard(&self) -> Option<(CheckedNode<'_>, usize)> {
        self.guard
            .as_ref()
            .map(|(body, slots)| (body.root(), *slots))
    }

    /// The checked initializer of the clause's `index`th capture and its
    /// evaluation slot count (FR-337 "Captures at S3"): the number of
    /// binders in scope for it, `over`, the trigger binder of an `on each`
    /// activation and the earlier captures. `None` past the last capture.
    pub fn capture(&self, index: usize) -> Option<(CheckedNode<'_>, usize)> {
        self.captures
            .get(index)
            .map(|(body, slots)| (body.root(), *slots))
    }

    /// The first capture, in source order, that a `holds` condition of the
    /// clause reads (FR-337 "Capture observation"): its index among the
    /// captures. A condition binds the `over` parameter, the trigger binder
    /// of an `on each` activation and then the captures, so a read of the
    /// local at slot `first + j` is a read of capture `j`; the locals a
    /// condition binds itself follow them.
    pub fn captured_read(&self) -> Option<usize> {
        let first = 1 + usize::from(self.clause.trigger().is_some());
        let count = self.clause.captures().len();
        self.conditions
            .values()
            .flat_map(|(body, _)| body.root().descendants())
            .filter_map(|node| match node.kind() {
                NodeKind::Local(slot) => slot.checked_sub(first).filter(|capture| *capture < count),
                _ => None,
            })
            .min()
    }

    /// The declaration identities of the populations the clause's `holds`
    /// atoms, guard and captures name (FR-391), ascending and each once: each is required
    /// (FR-106 check 7) in every snapshot of the trace the clause is read
    /// over.
    pub fn named_populations(&self) -> Vec<String> {
        let mut named: Vec<String> = self
            .conditions
            .values()
            .chain(self.guard.as_ref())
            .chain(&self.captures)
            .flat_map(|(body, _)| body.root().descendants())
            .filter_map(|node| match node.kind() {
                NodeKind::NamedPopulation { population, .. } => Some(population.node.clone()),
                _ => None,
            })
            .collect();
        named.sort();
        named.dedup();
        named
    }
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

/// A family `check`'s stage limit as a checking refusal's cause (L6): `CheckingLimitKind::try_from(Setting)` is the named reverse of
/// `CheckingLimitKind::setting`, and a setting no checking limit has is a
/// fault in the family, not in the input.
fn limit_cause(limit: &qsl_foundation::diagnostic::LimitExceeded) -> CheckCause {
    match CheckingLimitKind::try_from(limit.setting()) {
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
        Err(setting) => CheckCause::InternalFault(Box::new(KeyFault::UncheckedLimit(setting))),
    }
}

fn root(origin: Origin) -> Location {
    Location::root(origin)
}

/// Internal declared-span key for a timed source. Timed declarations have a
/// source span but no distinct `Origin` variant; the spaced name cannot
/// collide with a QSL declaration name.
fn time_source_origin(index: usize) -> String {
    format!("time source {index}")
}

/// Internal declared-span key for a refinement declaration: the region of the
/// nodes its class catalog mints (FR-375). Spaced, so it names no declared
/// type.
fn refinement_origin(name: &str) -> String {
    format!("refinement {name}")
}

/// Internal declared-span key for a timed source's qualified operation name.
fn time_operation_origin(index: usize) -> String {
    format!("time source {index} operation")
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
                anchor: binding.anchor.clone(),
                frame: binding.frame.clone(),
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

/// A refused or exhausted type admission as the check refusal that reports it.
fn type_admission(failure: quire_semantic_value::declaration::EnvironmentFailure) -> CheckCause {
    let cause = assemble::environment_cause(failure);
    let limit = if let AssemblyCause::TypeLimit(limit) = &cause {
        Some(Box::new(limit.clone()))
    } else {
        None
    };
    CheckCause::TypeAdmission {
        code: cause.code(),
        cause: cause.catalog_code().cause(),
        limit,
    }
}

/// A refused staged-admission read as a refusal: a ceiling or storage failure
/// keeps its type-admission cause, anything else is a broken invariant.
fn pending_refusal(failure: quire_semantic_value::declaration::PendingFieldFailure) -> CheckCause {
    use quire_semantic_value::declaration::PendingFieldFailure;
    match failure {
        PendingFieldFailure::Limit(limit) => {
            type_admission(quire_semantic_value::declaration::EnvironmentFailure::Limit(limit))
        }
        PendingFieldFailure::Storage(storage) => type_admission(storage),
        other => CheckCause::InternalFault(Box::new(KeyFault::PendingField(format!("{other:?}")))),
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
        mut self,
        limits: CheckingLimits,
        cancel: &Cancel,
    ) -> Result<CheckedGraph, Vec<CheckRefusal>> {
        // FR-230: S2 retains every timed declaration independent of header
        // selection. S3 owns the facet gate and keeps incomplete timed
        // constructs from silently entering the checked package.
        if !self.timed_forms.is_empty() {
            let required = crate::value::layer::HeaderLayer::TemporalComplete;
            let selected = self.header_layers.clone();
            let admitted = crate::value::layer::HeaderLayer::closure(&selected).contains(&required);
            let units = time_units::TimeUnits::new(
                &self.units,
                &self.nominal_spans,
                &self.models,
                &self.dimension_equivalences,
                &self.source,
                &self.package_name,
            );
            // The model a `time invariant` or `urgent` declaration belongs
            // to (FR-230 "Timed facet gate"): the one model the unit's
            // `time` members name, else the unit's one selected model.
            let timed_model: Option<&str> = {
                let named: std::collections::BTreeSet<&str> = self
                    .timed_forms
                    .iter()
                    .filter_map(|form| match form {
                        TimedDeclaration::TimeSource(source) => Some(source.model.name.as_str()),
                        TimedDeclaration::TimeInvariant(_)
                        | TimedDeclaration::Urgent(_)
                        | TimedDeclaration::ClockType(_) => None,
                    })
                    .collect();
                match (named.len(), self.models.as_slice()) {
                    (1, _) => named.into_iter().next(),
                    (0, [model]) => model.alias(),
                    _ => None,
                }
            };
            let mut seen_time_sources = BTreeMap::new();
            let refusals = self
                .timed_forms
                .iter()
                .filter_map(|form| {
                    if admitted {
                        if let TimedDeclaration::TimeSource(source) = form {
                            if let Some(&first) = seen_time_sources.get(source.model.name.as_str())
                            {
                                return Some(CheckRefusal {
                                    location: root(Origin::Expression),
                                    cause: CheckCause::Temporal(Box::new(
                                        TemporalCause::DuplicateTimeSource {
                                            model: source.model.name.clone(),
                                            first,
                                            span: source.span,
                                        },
                                    )),
                                });
                            }
                            seen_time_sources.insert(source.model.name.as_str(), source.span);
                            if let Some(model) = self
                                .models
                                .iter()
                                .find(|model| model.alias() == Some(source.model.name.as_str()))
                            {
                                let model_nodes = model.object_type_model_node_keys();
                                let [model_node] = model_nodes.as_slice() else {
                                    let cause = if model_nodes.is_empty() {
                                        TemporalCause::MissingTimedModel {
                                            name: source.model.name.clone(),
                                            span: source.model.span,
                                        }
                                    } else {
                                        TemporalCause::AmbiguousTimedModel {
                                            name: source.model.name.clone(),
                                            span: source.model.span,
                                        }
                                    };
                                    return Some(CheckRefusal {
                                        location: root(Origin::Expression),
                                        cause: CheckCause::Temporal(Box::new(cause)),
                                    });
                                };
                                let target = match units.target(&source.unit.name) {
                                    Ok(Some(target)) => target,
                                    Ok(None) => {
                                        return Some(CheckRefusal {
                                            location: root(Origin::Expression),
                                            cause: CheckCause::Temporal(Box::new(
                                                TemporalCause::MalformedTimeUnit {
                                                    name: source.unit.name.clone(),
                                                    span: source.unit.span,
                                                },
                                            )),
                                        })
                                    }
                                    Err(cause) => {
                                        return Some(CheckRefusal {
                                            location: root(Origin::Expression),
                                            cause: CheckCause::Temporal(Box::new(
                                                TemporalCause::TimeQuantity {
                                                    cause: Box::new(cause),
                                                    span: source.unit.span,
                                                },
                                            )),
                                        })
                                    }
                                };
                                let mut tick_operation_key = None;
                                let mut tick_operation_span = None;
                                let source_kind = match &source.source {
                                    qsl_forms::TimeSourceSyntax::Dense => TimeSourceKind::Dense,
                                    qsl_forms::TimeSourceSyntax::Tick { operation, period } => {
                                        let operations = model.tick_operation_keys(operation);
                                        let [resolved] = operations.as_slice() else {
                                            let cause = if operations.is_empty() {
                                                TemporalCause::MissingTickOperation {
                                                    name: operation.name.clone(),
                                                    span: operation.span,
                                                }
                                            } else {
                                                TemporalCause::AmbiguousTickOperation {
                                                    name: operation.name.clone(),
                                                    span: operation.span,
                                                }
                                            };
                                            return Some(CheckRefusal {
                                                location: root(Origin::Expression),
                                                cause: CheckCause::Temporal(Box::new(cause)),
                                            });
                                        };
                                        tick_operation_key = Some((*resolved).clone());
                                        tick_operation_span = Some(operation.span);
                                        let magnitude = match probabilistic::exact_value(
                                            &period.magnitude,
                                            limits.rational_bits(),
                                        ) {
                                            Ok(value) => value,
                                            Err(probabilistic::ExactFault::ZeroDenominator) => {
                                                return Some(CheckRefusal {
                                                    location: root(Origin::Expression),
                                                    cause: CheckCause::Temporal(Box::new(
                                                        TemporalCause::ZeroDenominator {
                                                            span: period.magnitude.span,
                                                        },
                                                    )),
                                                })
                                            }
                                            Err(probabilistic::ExactFault::AboveCeiling {
                                                bits,
                                            }) => {
                                                return Some(CheckRefusal {
                                                    location: root(Origin::Expression),
                                                    cause: CheckCause::ResourceExhausted(Box::new(
                                                        StageLimitCause {
                                                            stage: CheckingStage::Typing,
                                                            kind: CheckingLimitKind::RationalBits,
                                                            limit: limits.rational_bits(),
                                                            actual: u128::from(bits),
                                                            region: region::region(
                                                                &self.source,
                                                                self.embedding.as_deref(),
                                                                period.magnitude.span,
                                                            ),
                                                        },
                                                    )),
                                                })
                                            }
                                        };
                                        let converted = match units.convert(
                                            &magnitude,
                                            &period.unit.name,
                                            &target,
                                        ) {
                                            Ok(value) => value,
                                            Err(cause) => {
                                                // A period of another time
                                                // dimension refuses at the
                                                // period; one that is no time
                                                // unit, at the unit.
                                                let span = if matches!(
                                                    cause,
                                                    TimeUnitCause::OtherTimeDimension { .. }
                                                ) {
                                                    period.span
                                                } else {
                                                    period.unit.span
                                                };
                                                return Some(CheckRefusal {
                                                    location: root(Origin::Expression),
                                                    cause: CheckCause::Temporal(Box::new(
                                                        TemporalCause::TimeQuantity {
                                                            cause: Box::new(cause),
                                                            span,
                                                        },
                                                    )),
                                                });
                                            }
                                        };
                                        if converted.is_zero()
                                            || converted.numerator().is_negative()
                                        {
                                            return Some(CheckRefusal {
                                                location: root(Origin::Expression),
                                                cause: CheckCause::Temporal(Box::new(
                                                    TemporalCause::InvalidTickPeriod {
                                                        span: period.span,
                                                    },
                                                )),
                                            });
                                        }
                                        TimeSourceKind::Tick {
                                            operation: resolved.node.clone(),
                                            period: converted,
                                        }
                                    }
                                };
                                self.time_sources.push(ModelTimeSource {
                                    model: source.model.name.clone(),
                                    span: source.span,
                                    model_node: model_node.clone(),
                                    tick_operation_key,
                                    tick_operation_span,
                                    tick_operation_anchor: None,
                                    source: source_kind,
                                    unit: source.unit.name.clone(),
                                    unit_node: target.unit(),
                                    time_dimension: target.dimension(),
                                    nominal_nodes: target.nominal_nodes().to_vec(),
                                });
                                return None;
                            }
                            return Some(CheckRefusal {
                                location: root(Origin::Expression),
                                cause: CheckCause::Temporal(Box::new(
                                    TemporalCause::MissingTimedModel {
                                        name: source.model.name.clone(),
                                        span: source.model.span,
                                    },
                                )),
                            });
                        }
                        // FR-230 "Clock-typed positions": a `Clock` in a
                        // declared type position is admitted (the assembler
                        // kept its declaration out of the value world); one
                        // inside an expression has no checked form.
                        if let TimedDeclaration::ClockType(position) = form {
                            return match position.site {
                                ClockSite::Declaration => None,
                                // FR-230 "Records, tuples and unions": a
                                // clock-bearing value read whole, or a
                                // literal with a clock component outside a
                                // reset.
                                ClockSite::Expression(used) => Some(CheckRefusal {
                                    location: root(Origin::Expression),
                                    cause: CheckCause::Temporal(Box::new(match used {
                                        ClockUse::Literal => TemporalCause::ClockWrite {
                                            span: position.span,
                                        },
                                        ClockUse::Conversion
                                        | ClockUse::Instances
                                        | ClockUse::Accumulation => {
                                            TemporalCause::ClockReadOutsideConstraint {
                                                span: position.span,
                                            }
                                        }
                                    })),
                                }),
                            };
                        }
                        // A time invariant and an `urgent` declaration are
                        // checked with the clauses they read (`timed_model`).
                        return None;
                    }
                    let cause = CheckCause::Temporal(Box::new(TemporalCause::TimedFacetRequired {
                        construct: form.construct(),
                        owner: form.owner(timed_model).to_owned(),
                        selected: selected.clone(),
                        required,
                        span: form.span(),
                    }));
                    Some(CheckRefusal {
                        location: root(Origin::Expression),
                        cause,
                    })
                })
                .collect::<Vec<_>>();
            if !refusals.is_empty() {
                return Err(refusals);
            }
        }
        // FR-230: the clocks of each timed model, read out of the state
        // clauses before they are typed.
        let timed_drafts = {
            let units = time_units::TimeUnits::new(
                &self.units,
                &self.nominal_spans,
                &self.models,
                &self.dimension_equivalences,
                &self.source,
                &self.package_name,
            );
            timed_model::prepare(
                &timed_model::Inputs {
                    models: &self.models,
                    types: &self.types,
                    time_sources: &self.time_sources,
                    timed_forms: &self.timed_forms,
                    clock_reading: &self.clock_reading,
                    operation_bodies: &self.operation_bodies,
                    units: &units,
                    rational_bits: limits.rational_bits(),
                    nodes: limits.nodes(),
                    model_spans: &self.declared_type_spans,
                },
                &mut self.state_clauses,
            )
            .map_err(|refusal| vec![refusal])?
        };
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
        // QSL FR-246: the task sets and the claims over them. They are timed
        // forms, so FR-230's facet gate admits them; every defect is reported.
        let mut checked_task_sets = CheckedTaskSets::default();
        let mut task_sets_admitted = false;
        if !self.task_set_forms.is_empty() {
            let required = crate::value::layer::HeaderLayer::TemporalComplete;
            let selected = self.header_layers.clone();
            let admitted = crate::value::layer::HeaderLayer::closure(&selected).contains(&required);
            task_sets_admitted = admitted;
            if admitted {
                let units = time_units::TimeUnits::new(
                    &self.units,
                    &self.nominal_spans,
                    &self.models,
                    &self.dimension_equivalences,
                    &self.source,
                    &self.package_name,
                );
                let checked = task_set::check(&self.task_set_forms, &units, limits.rational_bits());
                for finding in checked.findings {
                    refusals.push(CheckRefusal {
                        location: root(Origin::Expression),
                        cause: match finding {
                            task_set::Finding::Cause(cause) => {
                                CheckCause::Temporal(Box::new(cause))
                            }
                            task_set::Finding::RationalBits { span, bits } => {
                                CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                                    stage: CheckingStage::Typing,
                                    kind: CheckingLimitKind::RationalBits,
                                    limit: limits.rational_bits(),
                                    actual: u128::from(bits),
                                    region: region::region(
                                        &self.source,
                                        self.embedding.as_deref(),
                                        span,
                                    ),
                                }))
                            }
                        },
                    });
                }
                checked_task_sets = checked.model;
            } else {
                refusals.extend(self.task_set_forms.iter().map(|form| CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::Temporal(Box::new(TemporalCause::TimedFacetRequired {
                        construct: form.construct(),
                        owner: form.owner(),
                        selected: selected.clone(),
                        required,
                        span: form.span(),
                    })),
                }));
            }
        }
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
        if let Some(name) = self.execution_point_keys.iter().find(|name| {
            !self
                .state_clauses
                .iter()
                .any(|clause| &clause.name == *name)
        }) {
            return Err(vec![CheckRefusal {
                location: root(Origin::Expression),
                cause: CheckCause::MissingName(name.clone()),
            }]);
        }
        // Every declared signature is resolved against the package `Scope`
        // before dispatch validation, which compares resolved types.
        let complete_value = self
            .function_selections
            .values()
            .any(state_clause::is_complete_value)
            || self
                .state_clauses
                .iter()
                .any(|clause| state_clause::is_complete_value(&clause.selection));
        // Model intake currently admits only `quire.model.complete/v1`.
        // Carry that selection into S3 so the inherited-edge rule is
        // confined to a package with an admitted model.
        let complete_model = !self.models.is_empty();
        let scope = Scope::new(
            self.types,
            self.operations,
            self.enums,
            self.aliases,
            self.model_operations,
            self.ieee_profile,
            self.dispatch_operations,
        )
        .with_variant_types(self.variant_types)
        .with_model_records(self.model_records)
        .with_relationships(self.relationships)
        .with_imports(self.imports)
        .with_integer_division(self.integer_division)
        .with_complete_value(complete_value)
        .with_known_populations(refinement::population_names(&self.models))
        .with_clock_reading(
            self.clock_reading
                .iter()
                .map(|declaration| declaration.function.name.clone())
                .collect(),
        )
        .with_history_names(refinement::history_names(&self.refinements))
        .with_complete_model(complete_model)
        .with_model_selections(
            self.models
                .iter()
                .filter_map(|model| Some((model.alias()?.to_owned(), model.selection().clone())))
                .collect(),
        )
        .with_declared_units(&self.units)
        .with_protocols(
            self.protocols
                .iter()
                .map(|protocol| protocol.name.name.as_str()),
        );
        let model_selected = crate::value::layer::HeaderLayer::closure(&self.header_layers)
            .contains(&crate::value::layer::HeaderLayer::ModelComplete);
        let mut model_context_calls = Vec::new();
        if model_selected {
            for clause in &self.state_clauses {
                model_context_calls.extend(layer::called_functions(&clause.body));
            }
            for clause in &self.operation_clauses {
                model_context_calls.extend(layer::called_functions(&clause.clause.expression));
            }
            for temporal in &self.temporal_clauses {
                add_temporal_model_calls(&temporal.form, &mut model_context_calls);
            }
            for claim in &self.state_graph_claims {
                let mut include_predicate = |predicate: &qsl_forms::StateGraphPredicateForm| {
                    for operand in &predicate.operands {
                        model_context_calls.extend(layer::called_functions(&operand.expression));
                    }
                    for scope in &predicate.scopes {
                        for (_, value) in scope {
                            model_context_calls.extend(layer::called_functions(&value.expression));
                        }
                    }
                };
                match &claim.form.kind {
                    qsl_forms::StateGraphClaimKind::Possible { target } => {
                        include_predicate(target);
                    }
                    qsl_forms::StateGraphClaimKind::AlwaysPossible { from, target } => {
                        if let Some(from) = from {
                            include_predicate(from);
                        }
                        include_predicate(target);
                    }
                    qsl_forms::StateGraphClaimKind::UniquePath { from, to } => {
                        include_predicate(from);
                        include_predicate(to);
                    }
                }
            }
            for invariant in &self.type_invariants {
                model_context_calls
                    .extend(layer::called_functions(&invariant.invariant.expression));
            }
            for terminal in &self.terminals {
                if let terminal::TerminalClauseKind::When { predicate, .. } = &terminal.kind {
                    model_context_calls.extend(layer::called_functions(&predicate.expression));
                }
            }
            for claim in &self.hybrid_claims {
                add_temporal_model_calls(&claim.form, &mut model_context_calls);
            }
            for hybrid in &self.hybrids {
                if let Some(initial) = &hybrid.form.initial {
                    for entry in &initial.entries {
                        model_context_calls
                            .extend(layer::called_functions(&entry.condition.expression));
                    }
                }
                for mode in &hybrid.form.modes {
                    model_context_calls.extend(layer::called_functions(&mode.invariant.expression));
                    for equation in &mode.equations {
                        model_context_calls
                            .extend(layer::called_functions(&equation.rate.expression));
                    }
                    for transition in &mode.transitions {
                        model_context_calls
                            .extend(layer::called_functions(&transition.guard.expression));
                        model_context_calls
                            .extend(layer::called_functions(&transition.reset.expression));
                    }
                }
            }
            for declaration in &self.probabilistic {
                match declaration {
                    ProbabilisticDeclaration::Random(random) => {
                        for value in &random.support {
                            model_context_calls
                                .extend(layer::called_functions(&value.value.expression));
                        }
                    }
                    ProbabilisticDeclaration::Reward(reward) => {
                        model_context_calls
                            .extend(layer::called_functions(&reward.expression.expression));
                    }
                    ProbabilisticDeclaration::Claim(claim) => {
                        add_activation_model_calls(&claim.activation, &mut model_context_calls);
                        add_probabilistic_body_model_calls(&claim.body, &mut model_context_calls);
                    }
                    ProbabilisticDeclaration::Delay(_) | ProbabilisticDeclaration::Workload(_) => {}
                }
            }
            for hyper in &self.hyper_clauses {
                match hyper {
                    HyperDeclaration::Behaviours(form) => {
                        if let Some(matched) = &form.match_block {
                            model_context_calls
                                .extend(layer::called_functions(&matched.expression));
                        }
                        add_temporal_formula_model_calls(&form.body, &mut model_context_calls);
                    }
                    HyperDeclaration::Finite(form) => {
                        model_context_calls.extend(layer::called_functions(&form.body.expression));
                    }
                    HyperDeclaration::Relation(form) => {
                        model_context_calls.extend(layer::called_functions(&form.body.expression));
                    }
                }
            }
            for refinement in &self.refinements {
                for row in &refinement.form.rows {
                    match row {
                        qsl_forms::RefinementRowForm::Object(row) => {
                            for field in &row.fields {
                                model_context_calls
                                    .extend(layer::called_functions(&field.value.expression));
                            }
                        }
                        qsl_forms::RefinementRowForm::History(row) => {
                            for update in &row.updates {
                                model_context_calls
                                    .extend(layer::called_functions(&update.value.expression));
                            }
                        }
                        qsl_forms::RefinementRowForm::Population(_)
                        | qsl_forms::RefinementRowForm::Step(_)
                        | qsl_forms::RefinementRowForm::Fairness(_) => {}
                    }
                }
            }
        }
        let function_indices: std::collections::BTreeMap<_, _> = self
            .functions
            .iter()
            .enumerate()
            .map(|(index, function)| (function.name.as_str(), index))
            .collect();
        let mut model_function_bodies = vec![false; self.functions.len()];
        while let Some(name) = model_context_calls.pop() {
            let Some(index) = function_indices.get(name.as_str()).copied() else {
                continue;
            };
            if model_function_bodies[index] {
                continue;
            }
            model_function_bodies[index] = true;
            model_context_calls.extend(layer::called_functions(&self.functions[index].body));
        }
        let model_scope = check::NamedScope::of(&self.models, &scope);
        let symmetric_objects = self
            .models
            .iter()
            .flat_map(|model| {
                model.symmetric_object_types(|sub, sup| scope.types().conforms(sub, sup))
            })
            .collect();
        let mut scope = scope.with_symmetric_objects(symmetric_objects);
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
        let has_ieee_literal = self.functions.iter().any(|function| {
            function.body.contains_ieee_literal()
                || function
                    .measure
                    .as_ref()
                    .is_some_and(qsl_forms::Expression::contains_ieee_literal)
        }) || self
            .state_clauses
            .iter()
            .any(|clause| clause.body.contains_ieee_literal())
            || self
                .hyper_clauses
                .iter()
                .any(hyper_clause::declaration_has_ieee_literal);
        scope
            .select_declared_ieee_profile(&signatures, has_ieee_literal)
            .map_err(|refusal| vec![refusal])?;
        let signatures = Signatures::new(signatures);
        let mut seen_operations = std::collections::BTreeSet::new();
        for operation in &scope.dispatch_operations {
            if !seen_operations.insert((operation.receiver_type.clone(), operation.member.clone()))
            {
                refusals.push(invalid_dispatch(
                    root(Origin::Expression),
                    InvalidDispatchDeclaration::DuplicateOperation {
                        receiver_type: operation.receiver_type.clone(),
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
        let package_name = self.package_name;
        let mut type_spans = self.declared_type_spans;
        for (index, equivalence) in self.dimension_equivalences.iter().enumerate() {
            type_spans.insert(lowering::equivalence_origin(index), equivalence.span);
        }
        for refinement in &self.refinements {
            type_spans.insert(
                refinement_origin(&refinement.form.name.name),
                refinement.form.span,
            );
        }
        for (index, source) in self.time_sources.iter().enumerate() {
            type_spans.insert(time_source_origin(index), source.span);
            if let Some(span) = source.tick_operation_span {
                type_spans.insert(time_operation_origin(index), span);
            }
        }
        let mut site_spans = self.declared_site_spans;
        let synthesized_spans = self.synthesized_spans;
        let embedding = self.embedding;
        let form_spans = self
            .functions
            .iter()
            .map(|function| function.spans().cloned())
            .collect();
        let function_parameter_spans: Vec<_> = self
            .functions
            .iter()
            .map(|function| function.parameter_spans().map(<[_]>::to_vec))
            .collect();
        let owner = node_key::SourceOwner::from(&source);
        let protocol_profile = self.protocol_profile;
        let header_layers = self.header_layers;
        let models = self.models;
        let model_inputs = self.model_inputs;
        let mut pending_types = self
            .pending_types
            .map(|staged| std::cell::RefCell::new(staged.0));
        let (model_objects, object_types) = model_object_tables(&models);
        let (model_owners, owner_types) = model_owner_tables(&models);
        let model_non_owners = model_non_owner_tables(&models);
        let model_names = abstraction::model_names(&models);
        let model_populations = model_population_tables(&models);
        let population_bounds = checked_population_bounds(&models);
        let population_keys = population_key_table(&models);
        let declaration_keys = models
            .iter()
            .flat_map(AdmittedModel::record_keys)
            .cloned()
            .collect();
        let object_nodes = object_types
            .keys()
            .filter_map(|declaration| {
                models
                    .iter()
                    .find_map(|model| model.model_node_key(declaration))
                    .map(|node| (declaration.clone(), node))
            })
            .collect();
        let mut model_selections: Vec<_> = models
            .iter()
            .map(|model| model.selection().clone())
            .collect();
        model_selections.sort_by(|a, b| a.identity.cmp(&b.identity));
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
        // QSpec FR-322 `model_root`: one key per selection, from the check's
        // identity limits, shared by the refinement headers and the emitter.
        let mut model_roots = Vec::with_capacity(model_selections.len());
        for selection in &model_selections {
            match model_root_key(selection) {
                Ok(key) => model_roots.push(key),
                Err(fault) => {
                    return Err(vec![CheckRefusal {
                        location: root(Origin::Expression),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::ModelRootKey(
                            fault.to_string(),
                        ))),
                    }]);
                }
            }
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
        // FR-092 key, which `check` mints below, and carries the caller's
        // handle as the declaration's kernel key.
        // Collected unordered and ranked once below ([`key_order`]), so no
        // insertion compares node keys.
        let mut type_nodes = Vec::new();
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
            type_nodes.push((
                node.clone(),
                identity::CheckedTypeNode::Sum { node, variants },
            ));
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
        // FR-319: a body's equality over a union is typed against the sealed
        // declaration, so each union is sealed under the node id of its type
        // node before any body is typed. A union's type closure needs no
        // body: a throwaway lowering over the declarations keys it. The
        // lowering below keys the same content, so it reaches the same ids.
        {
            let mut sealing_meter =
                quire_exact::Meter::new(contract_meter_limits).with_cancel(cancel.clone());
            let mut sealing_occurrences = family::OccurrenceMap::default();
            let mut sealing = lowering::Lowering::new(
                &scope,
                &owner,
                &models,
                scope.types().units().clone(),
                protocol_profile.as_ref(),
                0,
                &mut sealing_occurrences,
                &mut sealing_meter,
                &package_name,
            );
            let mut sealed_refusals = Vec::new();
            for declaration in scope.types().composites() {
                if matches!(
                    declaration.shape(),
                    quire_semantic_value::declaration::CompositeShape::Union(_)
                ) {
                    if let Err(refusal) = sealing.composite_node(declaration.key()) {
                        sealed_refusals.push(refusal);
                    }
                }
            }
            let nodes = sealing.finish(&lowering::generated_location()).composites;
            if !sealed_refusals.is_empty() {
                return Err(sealed_refusals);
            }
            if let Err(refusal) = scope.seal_unions(&nodes) {
                return Err(vec![refusal]);
            }
        }
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
        // FR-218 "Requirements": each function's name and keyword, which a
        // `requires temporal` name that resolves to it names.
        let function_kinds: Vec<_> = self
            .functions
            .iter()
            .map(|function| (function.name.clone(), function.kind()))
            .collect();
        // FR-447: the bodies a hybrid term's calls are validated against,
        // kept before the function loop consumes the declarations.
        let hybrid_bodies: Vec<qsl_forms::Expression> =
            if self.hybrids.is_empty() && self.hybrid_claims.is_empty() {
                Vec::new()
            } else {
                self.functions
                    .iter()
                    .map(|function| function.body.clone())
                    .collect()
            };
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
                complete_value: self
                    .function_selections
                    .get(&index)
                    .is_some_and(state_clause::is_complete_value),
                model_scope: model_function_bodies
                    .get(index)
                    .copied()
                    .unwrap_or(false)
                    .then(|| model_scope.clone()),
                // A model operation in the body selects the model family's
                // typing for that body, whether the function's declared
                // profile is value-complete or model-complete.
                derived_model_family: model_function_bodies.get(index).copied().unwrap_or(false),
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
                    nodes_used = checked.body.nodes_used;
                    if let Some(refusal) = layer::typed_equalities(
                        &checked.body.body,
                        self.function_selections.get(&index),
                        scope.types(),
                        function.spans().map(|spans| &spans.body),
                    )
                    .or_else(|| {
                        layer::typed_equalities(
                            checked.body.measure.as_ref()?,
                            self.function_selections.get(&index),
                            scope.types(),
                            function.spans().and_then(|spans| spans.measure.as_ref()),
                        )
                    }) {
                        refusals.push(refusal);
                        continue;
                    }
                    // ADR-012 §13.5: read this declaration's claims before
                    // `checked` is unpacked below.
                    claims.extend(
                        family::ValueFunctionFamily::requirements(&checked)
                            .into_iter()
                            .map(|claim| claim.owned_by(&function.name)),
                    );
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
        // FR-185: every read of an operation parameter by a checked
        // precondition, so a read of a random parameter refuses.
        let mut precondition_reads = Vec::new();
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
                staged: pending_types.as_ref(),
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
                    if let Some(refusal) = layer::typed_equalities(
                        &checked.body,
                        Some(&clause.selection),
                        scope.types(),
                        Some(&clause.spans.body),
                    ) {
                        refusals.push(refusal);
                        continue;
                    }
                    if let (qsl_forms::StateClauseKind::Precondition, Some(operation)) =
                        (clause.kind, &clause.operation)
                    {
                        precondition_reads.extend(probabilistic::precondition_reads(
                            operation,
                            &checked.parameters,
                            checked.body.root(),
                            &probabilistic::PreconditionSite::Clause(clause.name.clone()),
                        ));
                    }
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
        // FR-103: each kept operation `requires`/`ensures`, checked as a
        // Boolean under the clause kind its position gives. A kept clause
        // adds no checked state clause, claim or requirement record: it
        // yields a refusal or nothing.
        for clause in &self.operation_clauses {
            let location = clause.location();
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
                staged: None,
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match state_clause::check_operation_clause(clause, &declarations, &mut contract_cx) {
                Ok(checked) => {
                    nodes_used = checked.nodes_used;
                    if clause.clause.kind
                        == quire_semantic_value::location::OperationClauseKind::Requires
                    {
                        precondition_reads.extend(probabilistic::precondition_reads(
                            &clause.operation,
                            &checked.parameters,
                            checked.body.root(),
                            &probabilistic::PreconditionSite::Requires(clause.clause.position),
                        ));
                    }
                }
                Err(refusal) => {
                    let exhausted = matches!(refusal.cause, CheckCause::ResourceExhausted(_));
                    refusals.push(refusal);
                    if exhausted || cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                }
            }
        }
        // FR-366: each type-level invariant, checked as a Boolean invariant
        // whose `self` is a value of its owner. A checked invariant adds no
        // declaration, claim or requirement record; its body is kept for
        // evaluation per value.
        let mut type_invariants = Vec::with_capacity(self.type_invariants.len());
        let function_bodies: Vec<&CheckedBody> =
            drafts.iter().map(|(_, body, _)| &body.body).collect();
        for invariant in &self.type_invariants {
            let location = invariant.location();
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
                staged: None,
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match state_clause::check_type_invariant(
                invariant,
                &declarations,
                &function_bodies,
                &mut contract_cx,
            ) {
                Ok((checked, used)) => {
                    nodes_used = used;
                    type_invariants.push(checked);
                }
                Err(refusal) => {
                    let exhausted = matches!(
                        refusal.type_invariant_cause(),
                        Some(CheckCause::ResourceExhausted(_))
                    );
                    refusals.push(refusal);
                    if exhausted || cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                }
            }
        }
        // FR-124: each `terminal` declaration, a `when` predicate checked as a
        // Boolean invariant (FR-104). A model with no declaration is checked
        // to `TerminalDeclaration::None`.
        let mut terminals = Vec::new();
        for (index, terminal) in self.terminals.iter().enumerate() {
            // FR-411: the declaration and its predicate are located at
            // declaration sites, so the emitted node's occurrences resolve.
            type_spans.insert(
                lowering::terminal_site(index, lowering::TerminalSite::Claim),
                terminal.span,
            );
            let location = lowering::terminal_location(index, lowering::TerminalSite::Predicate);
            if let terminal::TerminalClauseKind::When { predicate, .. } = &terminal.kind {
                site_spans.insert(
                    lowering::terminal_site(index, lowering::TerminalSite::Predicate),
                    predicate.spans.clone(),
                );
            }
            // STD-492: `terminal any` names no object type to hang its model
            // node on when the model declares none, so S4 would have no node
            // to write. Refused here until QSpec settles the node.
            if matches!(terminal.kind, terminal::TerminalClauseKind::Any)
                && !models.iter().any(|model| {
                    model.alias() == Some(terminal.model.as_str())
                        && model.terminal_object().is_some()
                })
            {
                refusals.push(CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::TerminalWithoutObjectType {
                        name: terminal.name.clone(),
                        span: terminal.span,
                    },
                });
                continue;
            }
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
                staged: None,
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match terminal::check_terminal(index, terminal, &declarations, &mut contract_cx) {
                Ok((declaration, used)) => {
                    nodes_used = used;
                    terminals.push(CheckedTerminal {
                        model: terminal.model.clone(),
                        name: Some(terminal.name.clone()),
                        declaration,
                        occurrence: None,
                    });
                }
                Err(refusal) => {
                    let exhausted =
                        matches!(refusal.cause.unwrapped(), CheckCause::ResourceExhausted(_));
                    refusals.push(refusal);
                    if exhausted || cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                }
            }
        }
        for (alias, kind) in &self.alias_names {
            if *kind == AliasKind::Model
                && !terminals.iter().any(|terminal| terminal.model == *alias)
            {
                terminals.push(CheckedTerminal {
                    model: alias.clone(),
                    name: None,
                    declaration: TerminalDeclaration::None,
                    occurrence: None,
                });
            }
        }
        terminals.sort_by(|left, right| left.model.cmp(&right.model));
        // FR-230: each timed model's guard parts, time invariants and
        // `urgent when` predicates, each typed as a Boolean over the state.
        let mut timed_models = {
            let location = Location::root(Origin::Expression);
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
                staged: None,
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            let checking = timed_model::Checking {
                declarations: &declarations,
                state_clauses: &state_clause_forms,
                complete_value: crate::value::layer::HeaderLayer::closure(&header_layers)
                    .contains(&crate::value::layer::HeaderLayer::ValueComplete),
                presence_places: &[],
            };
            match timed_model::check(timed_drafts, &checking, &mut contract_cx) {
                Ok((checked, used)) => {
                    nodes_used = used;
                    checked
                }
                Err(refusal) => {
                    let exhausted =
                        matches!(refusal.cause.unwrapped(), CheckCause::ResourceExhausted(_));
                    refusals.push(refusal);
                    if exhausted || cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                    Vec::new()
                }
            }
        };
        // FR-230: each `urgent when` predicate S3 lowers is located at its own
        // site, so its nodes' occurrences resolve.
        for model in &mut timed_models {
            for (_, _, predicate) in model.urgency_predicates() {
                for (site, spans) in &predicate.sites {
                    site_spans.insert(site.clone(), spans.clone());
                }
            }
            // FR-230, QSpec FR-322 "Clause nodes": so is each guard part's
            // and time invariant's predicate.
            for (site, spans) in model.predicate_sites() {
                site_spans.insert(site.clone(), spans.clone());
            }
        }
        // FR-135 to FR-137: each `refinement` declaration, its rows typed as
        // state-clause bodies over the concrete object. FR-147: the
        // `refinement_internals` value is parsed once. A refinement over a
        // concrete protocol is checked in full except for the rows the
        // protocol clause decides, which wait for the clause (FR-136 "Order
        // of the checks"); one whose abstract side is a protocol subject
        // waits for the checked protocol clauses as a whole; its refusals do
        // not stop the unit before the clauses are checked.
        let declared_refinements: std::collections::BTreeSet<&str> = self
            .refinements
            .iter()
            .map(|refinement| refinement.form.name.name.as_str())
            .collect();
        let internal_lists = refinement::ParsedLists::parse(
            self.refinement_internals.as_ref(),
            &declared_refinements,
        );
        // The defects of the `refinement_internals` value are reported
        // whatever the support set says and whatever else the stage refuses.
        for (owner, defect) in &internal_lists.defects {
            refusals.push(CheckRefusal {
                location: Location::root(Origin::Expression),
                cause: CheckCause::Refinement(Box::new(RefinementRefusal {
                    refinement: owner.clone().unwrap_or_default(),
                    span: qsl_foundation::Span { start: 0, end: 0 },
                    cause: RefinementCause::InternalList(defect.clone()),
                })),
            });
        }
        // QSpec FR-430: the models a `time` declaration names are timed.
        let timed_aliases: Vec<&str> = self
            .time_sources
            .iter()
            .map(|source| source.model.as_str())
            .collect();
        let mut refinement_slots: Vec<Option<CheckedRefinement>> = std::iter::repeat_with(|| None)
            .take(self.refinements.len())
            .collect();
        let mut protocol_refinements = Vec::new();
        // One refinement checked over the protocol clauses given: none
        // before the clauses are checked.
        macro_rules! check_one_refinement {
            ($index:expr, $refinement:expr, $views:expr, $defer_abstract:expr) => {{
                let location = Location::root(Origin::Expression);
                let declarations = state_clause::ClauseDeclarations {
                    scope: &scope,
                    signatures: &signatures,
                    dispatch_tables: &dispatch_tables,
                    models: &models,
                    checking_limits: limits,
                    location: &location,
                    nodes_used,
                    cancel: Some(cancel),
                    staged: None,
                };
                let mut contract_cx = crate::family::CheckContext::new(
                    &declarations,
                    contract_limits,
                    &mut contract_meter,
                    &mut contract_diagnostics,
                    &mut contract_scopes,
                );
                let env = refinement::RefinementEnv {
                    support: self.refinement_support,
                    lists: &internal_lists,
                    protocols: $views,
                    defer_abstract_protocol_resolution: $defer_abstract,
                    timed_models: &timed_aliases,
                    clock_reading: &self.clock_reading,
                };
                let (checked, used) = refinement::check_refinement(
                    $refinement,
                    &self.protocols,
                    &declarations,
                    &mut contract_cx,
                    &env,
                );
                nodes_used = used;
                // A limit or a cancellation stops the unit at a refusal, as
                // every other declaration's does.
                let (found, stop) = match checked {
                    refinement::RefinementCheck::Checked(checked) => {
                        refinement_slots[$index] = Some(*checked);
                        (Vec::new(), false)
                    }
                    refinement::RefinementCheck::Protocol(pending) => {
                        let stop = pending.exhausted() || cancel.tripped().is_some();
                        protocol_refinements.push(($index, *pending));
                        (Vec::new(), stop)
                    }
                    refinement::RefinementCheck::Refused(found) => {
                        let stop = cancel.tripped().is_some()
                            || found.iter().any(|refusal| {
                                matches!(
                                    refusal.cause.unwrapped(),
                                    CheckCause::ResourceExhausted(_)
                                )
                            });
                        (found, stop)
                    }
                };
                refusals.extend(found);
                if stop {
                    refusals.extend(
                        protocol_refinements
                            .into_iter()
                            .flat_map(|(_, pending)| pending.into_refusals()),
                    );
                    return Err(refusals);
                }
            }};
        }
        let is_protocol_subject = |refinement: &RefinementDeclaration| {
            matches!(
                refinement.form.abstract_side,
                qsl_forms::AbstractSideForm::Protocol { .. }
            )
        };
        for (index, refinement) in self.refinements.iter().enumerate() {
            if !is_protocol_subject(refinement) {
                check_one_refinement!(index, refinement, &[], false);
            }
        }
        // FR-218: a second reference to one protocol in the unit refuses at
        // the second: the function bodies in declaration order, then the
        // state clauses.
        refusals.extend(protocol_reference::repeated(
            drafts
                .iter()
                .map(|(_, body, _)| &body.body)
                .chain(typed_clauses.iter().map(|clause| &clause.body)),
        ));
        let deferred: Vec<CheckRefusal> = protocol_refinements
            .iter()
            .flat_map(|(_, pending)| pending.refusals().iter().cloned())
            .collect();
        // A protocol subject's own refusals are reported with the unit's
        // other refusals, whichever stage stops the unit before its clause is
        // checked (FR-136-AC-8).
        if !refusals.is_empty() {
            // FR-136: even though the unit cannot check protocol clauses,
            // protocol-abstract refinements still check their model-only
            // headers and rows. Their abstract protocol name is unresolved
            // only because clause checking is deferred, so suppress that
            // one refusal until the normal protocol pass can resolve it.
            for (index, refinement) in self.refinements.iter().enumerate() {
                if is_protocol_subject(refinement) {
                    check_one_refinement!(index, refinement, &[], true);
                }
            }
            let prechecked_deferred: Vec<CheckRefusal> = protocol_refinements
                .iter()
                .flat_map(|(_, pending)| pending.refusals().iter().cloned())
                .collect();
            return Err(with_deferred(refusals, &prechecked_deferred));
        }
        // FR-220: the model's views, for the symbolic footprints the access
        // classification of a protocol's `parallel` attempts reads.
        let access_views: Option<Vec<crate::model::normalize::EffectiveView>> = model_inputs
            .as_ref()
            .filter(|_| {
                self.protocols.iter().any(|protocol| {
                    !protocol.parallels.is_empty()
                        || protocol
                            .attempts
                            .iter()
                            .any(|attempt| attempt.ordering.is_some())
                })
            })
            .map(|(selected, _)| selected.iter().map(|model| model.view.clone()).collect());
        if let Some((selected, mut model_meter)) = model_inputs {
            match model_conformance::check(
                &selected,
                &mut model_meter,
                &models,
                &state_clause_forms,
                &typed_clauses,
                pending_types.as_mut().map(std::cell::RefCell::get_mut),
            ) {
                Ok(()) => {}
                Err(model_conformance::StageDenial::Pending(failure)) => {
                    return Err(with_deferred(
                        vec![CheckRefusal {
                            location: root(Origin::Expression),
                            cause: pending_refusal(failure),
                        }],
                        &deferred,
                    ))
                }
                Err(model_conformance::StageDenial::MissingModel(identity)) => {
                    return Err(with_deferred(
                        vec![CheckRefusal {
                            location: root(Origin::Expression),
                            cause: CheckCause::InternalFault(Box::new(
                                KeyFault::MissingAdmittedModel(identity),
                            )),
                        }],
                        &deferred,
                    ))
                }
                Err(model_conformance::StageDenial::Incomplete(incomplete)) => {
                    return Err(with_deferred(
                        vec![CheckRefusal {
                            location: root(Origin::Expression),
                            cause: CheckCause::ModelIncomplete(incomplete),
                        }],
                        &deferred,
                    ))
                }
                Err(model_conformance::StageDenial::Refused(failures)) => {
                    return Err(with_deferred(
                        failures
                            .into_iter()
                            .map(|failure| CheckRefusal {
                                location: root(Origin::Expression),
                                cause: CheckCause::ModelConformance(Box::new(failure)),
                            })
                            .collect(),
                        &deferred,
                    ))
                }
            }
        }
        // FR-110: the staged type admission completes only after the
        // conformance stage found every model redefinition conforming, and
        // before dispatch linking or sealing.
        if let Some(pending) = pending_types.take().map(std::cell::RefCell::into_inner) {
            if let Err(failure) = pending.complete() {
                return Err(with_deferred(
                    vec![CheckRefusal {
                        location: root(Origin::Expression),
                        cause: type_admission(failure),
                    }],
                    &deferred,
                ));
            }
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
        // Once those pass and the control tree and its causal edges compile
        // (FR-218), `protocol_clause::content` checks the rest -- the `using`
        // aliases, roles, node roles and owners, and binder types -- and
        // refuses `unsupported_construct`/`not-yet-implemented` at any
        // construct no checker reads yet (a `predicate` join).
        // `protocol_requirement::check` binds each `requires temporal` and
        // `protocol_relationship::check` each `relationship` (FR-218).
        // `protocol_slots::check` checks every body, guard, `visible` entry,
        // loop proof, `related by` end, replicated role lifetime and the
        // `terminal when` predicate over its readable binders, and
        // `protocol_controls::check` the channels with their FIFO keys,
        // messages, joins and `terminal`/`scheduling` members (FR-218). A
        // protocol compiles only when every part is checked.
        let mut checked_protocols = Vec::with_capacity(self.protocols.len());
        let mut protocol_bindings = Vec::with_capacity(self.protocols.len());
        // The selected protocol profile, which the structural refusals of a
        // protocol's control tree name (FR-218, FR-381).
        let protocol_profile_identity = protocol_profile
            .as_ref()
            .map(|definition| definition.identity.clone());
        // FR-218 "Requirements": the unit's declarations a `requires
        // temporal` name resolves among, each other kind by its keyword.
        let unit_declarations = protocol_requirement::UnitDeclarations::new(
            &self.temporal_clauses,
            &self.protocols,
            state_clause_forms
                .iter()
                .map(|clause| {
                    (
                        clause.name.as_str(),
                        protocol_requirement::UnitDeclarationKind::Other(match clause.kind {
                            qsl_forms::StateClauseKind::Invariant => "invariant",
                            qsl_forms::StateClauseKind::Constraint => "constraint",
                            qsl_forms::StateClauseKind::Precondition => "pre",
                            qsl_forms::StateClauseKind::Postcondition => "post",
                        }),
                    )
                })
                .chain(function_kinds.iter().map(|(name, kind)| {
                    (
                        name.as_str(),
                        protocol_requirement::UnitDeclarationKind::Callable(*kind),
                    )
                }))
                .chain(self.terminals.iter().map(|terminal| {
                    (
                        terminal.name.as_str(),
                        protocol_requirement::UnitDeclarationKind::Other("terminal"),
                    )
                }))
                .chain(self.state_graph_claims.iter().map(|claim| {
                    (
                        claim.form.name.name.as_str(),
                        protocol_requirement::UnitDeclarationKind::Other("claim"),
                    )
                }))
                .chain(self.native_names.iter().map(|name| {
                    (
                        name.as_str(),
                        protocol_requirement::UnitDeclarationKind::Other("dimension or unit"),
                    )
                }))
                .chain(scope.type_names().map(|name| {
                    (
                        name,
                        protocol_requirement::UnitDeclarationKind::Other("type"),
                    )
                })),
        );
        // The clauses whose own checks refused something, which a refinement
        // does not resolve against (FR-147 "Abstract side"), and each
        // protocol's place among the checked ones: a refinement over a
        // refused clause reports the clause's refusals alone (FR-136 "Order
        // of the checks").
        let mut refused_protocols: std::collections::BTreeSet<&str> =
            std::collections::BTreeSet::new();
        let mut protocol_places: Vec<Option<usize>> = Vec::with_capacity(self.protocols.len());
        let mut protocol_refused: Vec<bool> = Vec::with_capacity(self.protocols.len());
        // The header's temporal profiles with their aliases, in source order:
        // an `await` reads its clock role under one of them.
        let temporal_aliases: Vec<(&str, TemporalProfile)> = self
            .profile_selections
            .iter()
            .filter_map(|selection| {
                Some((
                    selection.alias.as_str(),
                    TemporalProfile::from_identity(&selection.identity)?,
                ))
            })
            .collect();
        for (index, protocol) in self.protocols.iter().enumerate() {
            let refusals_before = refusals.len();
            let attempts = self
                .protocol_attempts
                .get(index)
                .map_or(&[][..], Vec::as_slice);
            let anchored = protocol_clause::check(
                protocol,
                &self.alias_names,
                &self.native_names,
                &scope,
                &signatures,
                &unit_declarations,
                attempts,
                &state_clause_forms,
                protocol_clause::Selections {
                    protocol: protocol_profile_identity.as_deref(),
                    temporal: &temporal_aliases,
                },
            );
            // FR-381: the protocol's activation, captures and compensation
            // templates, through the clause checker over the names each
            // stage may read.
            let obligation_location = root(Origin::Expression);
            let obligation_declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &obligation_location,
                nodes_used,
                cancel: Some(cancel),
                staged: None,
            };
            let mut obligation_cx = crate::family::CheckContext::new(
                &obligation_declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            let obligations = protocol_obligation::check(
                protocol,
                anchored.as_ref().ok().map(|(checked, _)| checked),
                protocol_profile_identity.as_deref(),
                &obligation_declarations,
                &mut obligation_cx,
            );
            match anchored {
                Ok((mut checked, bound)) => {
                    let content = protocol_clause::content(protocol, &self.alias_names, &scope);
                    let content_refused = !content.is_empty();
                    refusals.extend(content);
                    match protocol_requirement::check(
                        index,
                        &self.protocols,
                        &self.temporal_clauses,
                        &unit_declarations,
                    ) {
                        Ok(requirements) => checked.requirements = requirements,
                        Err(requirement_refusals) => refusals.extend(requirement_refusals),
                    }
                    match protocol_memory::check_models(protocol, &checked.controls) {
                        Ok(memory) => checked.memory = memory,
                        Err(memory_refusals) => {
                            refusals.extend(memory_refusals.into_iter().map(|(_, refusal)| refusal))
                        }
                    }
                    match protocol_relationship::check(protocol, &models) {
                        Ok(relationships) => checked.relationships = relationships,
                        Err(relationship_refusals) => refusals.extend(
                            relationship_refusals
                                .into_iter()
                                .map(|(_, refusal)| refusal),
                        ),
                    }
                    match obligations {
                        protocol_obligation::Obligations::Checked(checked_obligations, used) => {
                            checked.obligations = *checked_obligations;
                            for (template, form) in checked
                                .obligations
                                .compensations
                                .iter_mut()
                                .zip(&protocol.compensations)
                            {
                                template.temporal_profile = temporal_aliases
                                    .iter()
                                    .find(|(alias, _)| *alias == form.using.alias)
                                    .map(|(_, profile)| *profile);
                            }
                            nodes_used = used;
                        }
                        protocol_obligation::Obligations::Refused(obligation_refusals) => {
                            refusals.extend(obligation_refusals);
                        }
                        // `content` refuses a declared type that does not
                        // resolve; with none refused the protocol would be
                        // compiled without its obligations.
                        protocol_obligation::Obligations::Unresolved if !content_refused => {
                            refusals.push(protocol_obligation::unresolved_fault());
                        }
                        protocol_obligation::Obligations::Unresolved => {}
                    }
                    // FR-218: every slot of the `run` tree and `finish`,
                    // over the binders readable at it, after the
                    // obligations' node total.
                    let slot_declarations = state_clause::ClauseDeclarations {
                        nodes_used,
                        ..obligation_declarations
                    };
                    let mut slot_cx = crate::family::CheckContext::new(
                        &slot_declarations,
                        contract_limits,
                        &mut contract_meter,
                        &mut contract_diagnostics,
                        &mut contract_scopes,
                    );
                    match protocol_slots::check(
                        protocol,
                        &checked,
                        &slot_declarations,
                        &mut slot_cx,
                        &regions,
                        protocol_profile_identity.as_deref(),
                    ) {
                        protocol_slots::Slots::Checked(slots, used) => {
                            checked.slots = *slots;
                            nodes_used = used;
                        }
                        protocol_slots::Slots::Refused(slot_refusals) => {
                            refusals.extend(slot_refusals);
                        }
                        // `content` refuses a binder type that does not
                        // resolve and an owner that is no role; with none
                        // refused the protocol would be compiled without
                        // its slots.
                        protocol_slots::Slots::Unresolved if !content_refused => {
                            refusals.push(protocol_obligation::unresolved_fault());
                        }
                        protocol_slots::Slots::Unresolved => {}
                    }
                    // FR-218: the protocol's channels, joins and
                    // `terminal`/`scheduling` members.
                    let control_declarations = state_clause::ClauseDeclarations {
                        nodes_used,
                        ..obligation_declarations
                    };
                    let mut control_cx = crate::family::CheckContext::new(
                        &control_declarations,
                        contract_limits,
                        &mut contract_meter,
                        &mut contract_diagnostics,
                        &mut contract_scopes,
                    );
                    match protocol_controls::check(
                        protocol,
                        &self.alias_names,
                        &control_declarations,
                        &mut control_cx,
                    ) {
                        Ok((channels, used)) => {
                            checked.channels = channels;
                            checked.owners = protocol_clause::owners(protocol, &checked);
                            nodes_used = used;
                        }
                        Err(control_refusals) => refusals.extend(control_refusals),
                    }
                    if refusals.len() > refusals_before {
                        refused_protocols.insert(protocol.name.name.as_str());
                    }
                    protocol_bindings.push(bound);
                    protocol_places.push(Some(checked_protocols.len()));
                    checked_protocols.push(checked);
                }
                Err(protocol_refusals) => {
                    protocol_places.push(None);
                    refusals.extend(protocol_refusals);
                    if let protocol_obligation::Obligations::Refused(obligation_refusals) =
                        obligations
                    {
                        refusals.extend(obligation_refusals);
                    }
                }
            }
            protocol_refused.push(refusals.len() > refusals_before);
            if cancel.tripped().is_some() {
                return Err(with_deferred(refusals, &deferred));
            }
        }
        // FR-147: the refinements whose abstract side is a protocol subject,
        // now that every protocol clause of the unit is checked (FR-136
        // "Order of the checks"). A clause refused by its own checks
        // reported its refusals; the refinement that names it reports none
        // for it and gives no `CheckedRefinement`.
        {
            let protocol_views: Vec<refinement::ProtocolView<'_>> = self
                .protocols
                .iter()
                .map(|form| {
                    let place = checked_protocols
                        .iter()
                        .position(|checked| checked.name == form.name.name.as_str());
                    refinement::ProtocolView {
                        form,
                        checked: place
                            .and_then(|place| checked_protocols.get(place))
                            .filter(|_| !refused_protocols.contains(form.name.name.as_str())),
                        attempts: place
                            .and_then(|place| protocol_bindings.get(place))
                            .map_or(&[], Vec::as_slice),
                    }
                })
                .collect();
            for (index, refinement) in self.refinements.iter().enumerate() {
                if is_protocol_subject(refinement) {
                    check_one_refinement!(index, refinement, &protocol_views, false);
                }
            }
        }
        // FR-136 "Order of the checks": a refinement over a concrete protocol
        // has its rows resolved and its step classes covered now that the
        // clause is checked. A refused clause reports its own refusals
        // alone, and the refinement none beyond those it already found.
        let mut selected_protocols: Vec<Option<refinement::SelectedProtocol>> =
            std::iter::repeat_with(|| None)
                .take(refinement_slots.len())
                .collect();
        let protocol_views: Vec<refinement::ProtocolView<'_>> = self
            .protocols
            .iter()
            .map(|form| {
                let place = checked_protocols
                    .iter()
                    .position(|checked| checked.name == form.name.name.as_str());
                refinement::ProtocolView {
                    form,
                    checked: place
                        .and_then(|place| checked_protocols.get(place))
                        .filter(|_| !refused_protocols.contains(form.name.name.as_str())),
                    attempts: place
                        .and_then(|place| protocol_bindings.get(place))
                        .map_or(&[], Vec::as_slice),
                }
            })
            .collect();
        for (slot, pending) in protocol_refinements {
            let index = pending.protocol();
            let place = protocol_places.get(index).copied().flatten();
            let clause = match (place, protocol_refused.get(index)) {
                (Some(place), Some(false)) => {
                    match (
                        self.protocols.get(index),
                        checked_protocols.get(place),
                        protocol_bindings.get(place),
                    ) {
                        (Some(form), Some(checked), Some(attempts)) => {
                            Some(refinement::ProtocolClause {
                                form,
                                checked,
                                attempts,
                            })
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            let location = Location::root(Origin::Expression);
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
                staged: None,
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            let (completed, used) = pending.complete(
                clause.as_ref(),
                &protocol_views,
                &declarations,
                &mut contract_cx,
            );
            nodes_used = used;
            match completed {
                Ok((checked, selected)) => {
                    refinement_slots[slot] = Some(checked);
                    selected_protocols[slot] = Some(selected);
                }
                Err(found) => refusals.extend(found),
            }
        }
        // FR-375: a model-abstract refinement whose rows select no protocol
        // was checked before any clause; its catalog takes the clauses over
        // its concrete model now.
        {
            let location = Location::root(Origin::Expression);
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
                staged: None,
            };
            for (checked, selected) in refinement_slots.iter_mut().zip(&selected_protocols) {
                let Some(checked) = checked.as_mut().filter(|_| selected.is_none()) else {
                    continue;
                };
                if let Err(fault) =
                    checked.catalog_concrete_protocols(&declarations, &protocol_views)
                {
                    refusals.push(CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(fault))),
                    });
                }
            }
        }
        let (mut refinements, selected_protocols): (Vec<_>, Vec<_>) = refinement_slots
            .into_iter()
            .zip(selected_protocols)
            .filter_map(|(checked, selected)| Some((checked?, selected)))
            .unzip();
        let root_of = |selection: &DomainPackageRef| {
            model_selections
                .iter()
                .position(|held| held.identity == selection.identity)
                .and_then(|index| model_roots.get(index).cloned())
        };
        for checked in &mut refinements {
            checked.package_name = Some(package_name.clone());
            let abstract_model = models
                .iter()
                .find(|model| model.alias() == Some(checked.abstract_model()));
            // QSpec FR-375: the refinement's subjects are the selected model
            // packages' `model_root` nodes.
            checked.abstract_subject_node =
                abstract_model.and_then(|model| root_of(model.selection()));
            let concrete_model = models
                .iter()
                .find(|model| model.alias() == Some(checked.concrete_model()));
            checked.concrete_subject_node =
                concrete_model.and_then(|model| root_of(model.selection()));
        }
        // FR-326: each `temporal` clause, checked through the `TemporalTrace`
        // family's contract under the unit's one temporal profile, then
        // FR-337: its `holds` conditions and `on each` guard typed over its
        // `over` parameter, now that every function signature is known. A
        // clause S4 does not emit (a protocol fairness target, the timed
        // profile) is refused rather than compiled and silently missing from
        // the package (`temporal_clause::not_emitted`).
        let time_units = time_units::TimeUnits::new(
            &self.units,
            &self.nominal_spans,
            &models,
            &self.dimension_equivalences,
            &source,
            &package_name,
        );
        let temporal_declarations = temporal_clause::TemporalDeclarations {
            scope: &scope,
            selections: &self.temporal_selections,
            source: &source,
            package_name: &package_name,
            embedding: embedding.as_deref(),
            position_limit: limits.nodes(),
            models: &models,
            protocols: &checked_protocols,
            timed: temporal_clause::TimedContext {
                units: &time_units,
                time_sources: &self.time_sources,
                rational_bits: limits.rational_bits(),
            },
        };
        let mut checked_temporal = Vec::with_capacity(self.temporal_clauses.len());
        for (index, clause) in self.temporal_clauses.iter().enumerate() {
            let mut contract_cx = crate::family::CheckContext::new(
                &temporal_declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            let outcome = temporal_clause::TemporalTraceFamily::check(clause, &mut contract_cx);
            // FR-337: the activation precedes the body in source, so a defect
            // of an `on each` guard refuses ahead of a refusal of the body.
            let body_refused = match &outcome {
                Ok(staged) => !temporal_clause::is_emitted(staged.value()),
                Err(StageFailure::Refused(refusal)) => {
                    temporal_clause::refuses_after_activation(&clause.form, refusal)
                }
                Err(_) => false,
            };
            if body_refused {
                let location = lowering::temporal_location(index, lowering::TemporalSite::Claim);
                let condition_declarations = state_clause::ClauseDeclarations {
                    scope: &scope,
                    signatures: &signatures,
                    dispatch_tables: &dispatch_tables,
                    models: &models,
                    checking_limits: limits,
                    location: &location,
                    nodes_used,
                    cancel: Some(cancel),
                    staged: None,
                };
                let mut condition_cx = crate::family::CheckContext::new(
                    &condition_declarations,
                    contract_limits,
                    &mut contract_meter,
                    &mut contract_diagnostics,
                    &mut contract_scopes,
                );
                if let Some(refusal) = temporal_clause::condition::guard_refusal(
                    index,
                    &clause.form,
                    &condition_declarations,
                    &mut condition_cx.reborrow(&scope),
                ) {
                    refusals.push(refusal);
                    if cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                    continue;
                }
            }
            match outcome {
                Ok(staged) if !temporal_clause::is_emitted(staged.value()) => {
                    let checked = staged.into_value();
                    if checked.has_aggregate {
                        let location =
                            lowering::temporal_location(index, lowering::TemporalSite::Claim);
                        let condition_declarations = state_clause::ClauseDeclarations {
                            scope: &scope,
                            signatures: &signatures,
                            dispatch_tables: &dispatch_tables,
                            models: &models,
                            checking_limits: limits,
                            location: &location,
                            nodes_used,
                            cancel: Some(cancel),
                            staged: None,
                        };
                        let mut condition_cx = crate::family::CheckContext::new(
                            &condition_declarations,
                            contract_limits,
                            &mut contract_meter,
                            &mut contract_diagnostics,
                            &mut contract_scopes,
                        );
                        match temporal_clause::condition::check_conditions(
                            index,
                            &clause.form,
                            &checked,
                            &condition_declarations,
                            &mut condition_cx.reborrow(&scope),
                        ) {
                            Ok((_, used)) => nodes_used = used,
                            Err(refusal) => {
                                refusals.push(refusal);
                                if cancel.tripped().is_some() {
                                    return Err(refusals);
                                }
                                continue;
                            }
                        }
                    }
                    refusals.push(temporal_clause::not_emitted(&checked));
                }
                Ok(staged) => {
                    let checked = staged.into_value();
                    let location =
                        lowering::temporal_location(index, lowering::TemporalSite::Claim);
                    let condition_declarations = state_clause::ClauseDeclarations {
                        scope: &scope,
                        signatures: &signatures,
                        dispatch_tables: &dispatch_tables,
                        models: &models,
                        checking_limits: limits,
                        location: &location,
                        nodes_used,
                        cancel: Some(cancel),
                        staged: None,
                    };
                    let mut condition_cx = crate::family::CheckContext::new(
                        &condition_declarations,
                        contract_limits,
                        &mut contract_meter,
                        &mut contract_diagnostics,
                        &mut contract_scopes,
                    );
                    match temporal_clause::condition::check_conditions(
                        index,
                        &clause.form,
                        &checked,
                        &condition_declarations,
                        &mut condition_cx.reborrow(&scope),
                    ) {
                        Ok((conditions, used)) => {
                            nodes_used = used;
                            checked_temporal.push((index, checked, conditions));
                        }
                        Err(refusal) => {
                            refusals.push(refusal);
                            if cancel.tripped().is_some() {
                                return Err(refusals);
                            }
                        }
                    }
                }
                Err(StageFailure::Limit(limit)) => {
                    refusals.push(CheckRefusal {
                        location: root(Origin::Expression),
                        cause: limit_cause(&limit),
                    });
                    if cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                }
                Err(StageFailure::Cancelled(_)) => return Err(refusals),
                Err(StageFailure::Fault(fault)) => {
                    refusals.push(CheckRefusal {
                        location: root(Origin::Expression),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(fault))),
                    });
                    return Err(refusals);
                }
                Err(StageFailure::Refused(refusal)) => refusals.push(refusal),
            }
        }
        // FR-185: the `random`, `reward` and `workload` declarations,
        // resolved, typed and recorded; FR-186: a `probabilistic` claim,
        // resolved, typed and recorded with its obligation identity.
        let probabilistic_input = probabilistic::ProbabilisticInput {
            scope: &scope,
            signatures: &signatures,
            dispatch_tables: &dispatch_tables,
            models: &models,
            model_objects: &model_objects,
            object_types: &object_types,
            time_sources: &self.time_sources,
            time_units: &time_units,
            preconditions: &precondition_reads,
            profiles: &self.profile_selections,
            limits,
            cancel,
        };
        let mut probabilistic_cx = crate::family::CheckContext::new(
            &scope,
            contract_limits,
            &mut contract_meter,
            &mut contract_diagnostics,
            &mut contract_scopes,
        );
        let checked_probabilistic = probabilistic::check(
            &self.probabilistic,
            &probabilistic_input,
            &mut probabilistic_cx,
            &mut nodes_used,
        );
        refusals.extend(checked_probabilistic.refusals);
        let probabilistic_model = checked_probabilistic.model;
        // QSL FR-249: the `release` declarations, resolved against the task
        // sets and the operations of the admitted models.
        if task_sets_admitted {
            refusals.extend(
                task_set::check_releases(
                    &self.task_set_forms,
                    &probabilistic_input,
                    &mut checked_task_sets,
                )
                .into_iter()
                .map(|cause| CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::Temporal(Box::new(cause)),
                }),
            );
        }
        // FR-165: each state-graph claim, checked through the `StateGraph`
        // family's contract once every function signature is known, against
        // the same package-wide node total and contract meter.
        let mut checked_state_graph_claims = Vec::with_capacity(self.state_graph_claims.len());
        for claim in &self.state_graph_claims {
            let location = root(Origin::Expression);
            let declarations = state_clause::ClauseDeclarations {
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                models: &models,
                checking_limits: limits,
                location: &location,
                nodes_used,
                cancel: Some(cancel),
                staged: pending_types.as_ref(),
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match state_graph_claim::StateGraphFamily::check(claim, &mut contract_cx) {
                Ok(staged) => {
                    let checked = staged.into_value();
                    nodes_used = checked.nodes_used;
                    checked_state_graph_claims.push(checked);
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
        // FR-447: each `hybrid` declaration and each claim over one, checked
        // against the same package-wide node total.
        let hybrid_callees = hybrid::Callees {
            signatures: &signatures,
            bodies: &hybrid_bodies,
        };
        let checked_hybrid = hybrid::check_models(
            &self.hybrids,
            &hybrid_callees,
            limits.nodes(),
            &mut nodes_used,
        );
        let checked_hybrid_claims = hybrid::check_claims(
            &self.hybrid_claims,
            &hybrid::ClaimInputs {
                models: &checked_hybrid.models,
                index: &checked_hybrid.index,
                scope: &scope,
                units: &time_units,
                rational_bits: limits.rational_bits(),
                callees: &hybrid_callees,
                node_limit: limits.nodes(),
            },
            &mut nodes_used,
        );
        let hybrid_exhausted = checked_hybrid
            .refusals
            .iter()
            .chain(&checked_hybrid_claims.refusals)
            .any(|refusal| matches!(refusal.cause, CheckCause::ResourceExhausted(_)));
        refusals.extend(checked_hybrid.refusals);
        refusals.extend(checked_hybrid_claims.refusals);
        if hybrid_exhausted {
            return Err(refusals);
        }
        let checked_hybrid_models = checked_hybrid.models;
        let checked_hybrid_claim_list = checked_hybrid_claims.claims;
        // FR-172: each `hyper` clause and `relation`, checked through the
        // `TemporalTrace` family under the unit's one temporal profile. A
        // clause that checks is refused as not emitted, as a `temporal`
        // clause is.
        let hyper_declarations = hyper_clause::HyperDeclarations {
            temporal: temporal_clause::TemporalDeclarations {
                scope: &scope,
                selections: &self.temporal_selections,
                source: &source,
                package_name: &package_name,
                embedding: embedding.as_deref(),
                position_limit: limits.nodes(),
                models: &models,
                protocols: &checked_protocols,
                timed: temporal_clause::TimedContext {
                    units: &time_units,
                    time_sources: &self.time_sources,
                    rational_bits: limits.rational_bits(),
                },
            },
            models: &models,
            signatures: &signatures,
            dispatch_tables: &dispatch_tables,
            checking_limits: limits,
            cancel: Some(cancel),
            nodes_used: std::cell::Cell::new(nodes_used),
            behaviours_checked: std::cell::Cell::new(0),
        };
        let mut checked_hyper: Vec<hyper_clause::CheckedHyperClause> = Vec::new();
        let mut checked_relations: Vec<hyper_clause::CheckedModelRelation> = Vec::new();
        for clause in &self.hyper_clauses {
            let mut contract_cx = crate::family::CheckContext::new(
                &hyper_declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match hyper_clause::HyperTraceFamily::check(clause, &mut contract_cx) {
                Ok(staged) => match staged.into_value() {
                    // FR-488: a clause over `behaviours` is emitted unless
                    // it is one of the constructs S4 refuses.
                    hyper_clause::CheckedHyper::Behaviours(checked) => {
                        match hyper_clause::emission::unemitted(&checked, &models) {
                            Some(refusal) => refusals.push(refusal),
                            None => checked_hyper.push(*checked),
                        }
                    }
                    hyper_clause::CheckedHyper::Relation(relation) => {
                        checked_relations.push(*relation);
                    }
                },
                Err(StageFailure::Limit(limit)) => {
                    refusals.push(CheckRefusal {
                        location: root(Origin::Expression),
                        cause: limit_cause(&limit),
                    });
                    if cancel.tripped().is_some() {
                        return Err(refusals);
                    }
                }
                Err(StageFailure::Cancelled(_)) => return Err(refusals),
                Err(StageFailure::Fault(fault)) => {
                    refusals.push(CheckRefusal {
                        location: root(Origin::Expression),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(fault))),
                    });
                    return Err(refusals);
                }
                Err(StageFailure::Refused(refusal)) => refusals.push(refusal),
            }
        }
        // The nodes the hyper conditions checked count against the package's
        // ceiling beside every other declaration's.
        nodes_used = hyper_declarations.nodes_used.get();
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
        // What each function's key hashes besides its own body: the
        // functions it calls and the protocols it refers to.
        let function_edges: Vec<(Vec<usize>, Vec<usize>)> = drafts
            .iter()
            .map(|(_, body, _)| {
                let mut callees = body.body.root().callees();
                let mut protocols = body.body.root().protocol_references();
                if let Some(measure) = &body.measure {
                    callees.extend(measure.root().callees());
                    protocols.extend(measure.root().protocol_references());
                }
                (callees, protocols)
            })
            .collect();
        let mut occurrences = family::OccurrenceMap::default();
        // FR-094: the package's units and every compound unit typing formed,
        // kept until lowering has keyed each one's type node.
        let mut units = scope.types().units().clone();
        for (_, body, _) in &mut drafts {
            units.append(std::mem::take(&mut body.formed_units));
        }
        for clause in &mut typed_clauses {
            units.append(std::mem::take(&mut clause.formed_units));
        }
        for (_, _, conditions) in &mut checked_temporal {
            for condition in conditions.all_mut() {
                units.append(std::mem::take(&mut condition.formed_units));
            }
        }
        for clause in &mut checked_hyper {
            for table in clause.take_formed_units() {
                units.append(table);
            }
        }
        for claim in &mut checked_state_graph_claims {
            for table in claim.take_formed_units() {
                units.append(table);
            }
        }
        let mut lowering = lowering::Lowering::new(
            &scope,
            &owner,
            &models,
            units,
            protocol_profile.as_ref(),
            drafts.len(),
            &mut occurrences,
            &mut contract_meter,
            &package_name,
        )
        .with_node_limit(limits.nodes(), nodes_used);
        // Relationship declarations are model graph roots. No existing
        // checked value type names one, so root them before lowering the
        // unit's functions; otherwise the relation family disappears from
        // the package even though the selected model declares it.
        let relationship_locations: BTreeMap<String, Location> = models
            .iter()
            .filter_map(|model| {
                let alias = model.alias()?.to_owned();
                Some((alias.clone(), root(Origin::TypeDeclaration { name: alias })))
            })
            .collect();
        if let Err(refusal) = lowering.relationship_nodes(&relationship_locations) {
            refusals.push(refusal);
        }
        // FR-092/FR-093: the functions and the protocols are lowered in
        // dependency order, each node's callees and referenced protocols
        // keyed before it.
        for (protocol, place) in self.protocols.iter().zip(&protocol_places) {
            let Some(checked) = place.and_then(|place| checked_protocols.get(place)) else {
                continue;
            };
            register_protocol_sites(protocol, checked, &mut type_spans, &mut site_spans);
        }
        let protocol_inputs =
            protocol_reference::inputs(&self.protocols, &protocol_places, &checked_protocols);
        let protocol_list: &[lowering::ProtocolInput<'_>] = match &protocol_inputs {
            Ok(inputs) => inputs,
            Err(refusal) => {
                refusals.push(refusal.clone());
                &[]
            }
        };
        lowering.begin_protocols(protocol_list.len());
        let temporal_positions: BTreeMap<usize, usize> = checked_temporal
            .iter()
            .enumerate()
            .map(|(dense, (source, _, _))| (*source, dense))
            .collect();
        let protocol_edges: Vec<(Vec<usize>, Vec<usize>, Vec<usize>)> = protocol_list
            .iter()
            .map(|protocol| {
                let (functions, protocols, required) = protocol.dependencies();
                let required = required
                    .iter()
                    .filter_map(|source| temporal_positions.get(source).copied())
                    .collect();
                (functions, protocols, required)
            })
            .collect();
        let temporal_edges: Vec<(Vec<usize>, Vec<usize>)> = checked_temporal
            .iter()
            .map(|(_, _, conditions)| {
                let mut functions = Vec::new();
                let mut protocols = Vec::new();
                for condition in conditions.all() {
                    let root = condition.body.root();
                    functions.extend(root.callees());
                    protocols.extend(root.protocol_references());
                }
                functions.sort_unstable();
                functions.dedup();
                protocols.sort_unstable();
                protocols.dedup();
                (functions, protocols)
            })
            .collect();
        let order = lowering::dependency_order(&function_edges, &protocol_edges, &temporal_edges);
        let lower_protocol = |lowering: &mut lowering::Lowering<'_>,
                              refusals: &mut Vec<CheckRefusal>,
                              index: usize,
                              cyclic: bool| {
            let Some(protocol) = protocol_list.get(index) else {
                return;
            };
            match lowering.protocol_node_step(index, protocol, cyclic) {
                // The refusal of a protocol with an unwritten construct is
                // this check's emission gate, as the refinement's is.
                Ok(unsupported) => {
                    if !self.emit_unwritten_protocols {
                        refusals.extend(unsupported);
                    }
                }
                Err(refusal) => refusals.push(refusal),
            }
        };
        let lower_functions = |lowering: &mut lowering::Lowering<'_>,
                               refusals: &mut Vec<CheckRefusal>,
                               group: &lowering::FunctionGroup| {
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
                        parameter_spans: function_parameter_spans
                            .get(index)
                            .and_then(|spans| spans.as_deref()),
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
        };
        let mut lowered_temporal: Vec<Option<lowering::LoweredTemporalClause>> =
            std::iter::repeat_with(|| None)
                .take(checked_temporal.len())
                .collect();
        let lower_temporal =
            |lowering: &mut lowering::Lowering<'_>,
             refusals: &mut Vec<CheckRefusal>,
             dense: usize,
             lowered_temporal: &mut Vec<Option<lowering::LoweredTemporalClause>>| {
                let Some((index, checked, conditions)) = checked_temporal.get(dense) else {
                    return;
                };
                match lowering.temporal_clause(&lowering::TemporalClauseInput {
                    index: *index,
                    clause: checked,
                    conditions,
                }) {
                    Ok(lowered) => {
                        lowering.record_temporal_key(*index, lowered.key.clone());
                        if let Some(slot) = lowered_temporal.get_mut(dense) {
                            *slot = Some(lowered);
                        }
                    }
                    Err(refusal) => {
                        lowering.record_temporal_refused(*index);
                        refusals.push(refusal);
                    }
                }
            };
        for step in &order {
            match step {
                lowering::LoweringStep::Functions(group) => {
                    lower_functions(&mut lowering, &mut refusals, group);
                }
                lowering::LoweringStep::Protocol(index) => {
                    lower_protocol(&mut lowering, &mut refusals, *index, false);
                }
                lowering::LoweringStep::Temporal(index) => {
                    lower_temporal(&mut lowering, &mut refusals, *index, &mut lowered_temporal);
                }
                lowering::LoweringStep::Cycle {
                    protocols,
                    functions,
                    temporals,
                } => {
                    for index in protocols {
                        lower_protocol(&mut lowering, &mut refusals, *index, true);
                    }
                    for group in functions {
                        lower_functions(&mut lowering, &mut refusals, group);
                    }
                    for index in temporals {
                        lower_temporal(&mut lowering, &mut refusals, *index, &mut lowered_temporal);
                    }
                }
            }
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
        let clause_name_locations: Vec<Location> = (0..state_clause_forms.len())
            .map(lowering::state_clause_name_location)
            .collect();
        // FR-105 "Outputs": each clause's qualified operation reference, where
        // the frame's `generated` occurrence is placed if the clause is the
        // first to name the operation.
        let reference_locations: Vec<Location> = (0..state_clause_forms.len())
            .map(lowering::state_operation_location)
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
        let attempt_reference_locations: Vec<Vec<Location>> = protocol_bindings
            .iter()
            .enumerate()
            .map(|(protocol_index, bindings)| {
                (0..bindings.len())
                    .map(|attempt_index| {
                        lowering::protocol_attempt_operation_location(protocol_index, attempt_index)
                    })
                    .collect()
            })
            .collect();
        // FR-381: each `compensate` template's own location, under which its
        // operation is lowered to the same shared anchor and frame nodes an
        // attempt naming the operation is (index-aligned with each checked
        // protocol's `obligations.compensations`).
        let template_locations: Vec<Vec<Location>> = checked_protocols
            .iter()
            .map(|checked| {
                checked
                    .obligations
                    .compensations
                    .iter()
                    .map(|template| {
                        root(Origin::TypeDeclaration {
                            name: lowering::compensation_origin(&checked.name, &template.name),
                        })
                    })
                    .collect()
            })
            .collect();
        // FR-105-AC-11: a frame occurrence uses the first operation-naming
        // clause or protocol attempt in source order. The declaration span
        // start orders candidates; the reference location itself resolves to
        // the qualified operation name in that declaration.
        // FR-488: each `align skip` operation of a hyper clause, resolved to
        // its declaring type and operation, anchored like a clause's.
        let hyper_skipped: Vec<(
            usize,
            usize,
            quire_exact::EffectiveId,
            crate::model::operation::OperationDeclaration,
            Location,
        )> = checked_hyper
            .iter()
            .enumerate()
            .flat_map(|(clause_index, clause)| {
                clause
                    .align_skip()
                    .iter()
                    .enumerate()
                    .filter_map({
                        let models = &models;
                        let scope = &scope;
                        move |(skip_index, key)| {
                            let declaring = models
                                .iter()
                                .find_map(|model| model.effective_id(&key.declaring))?;
                            let operation = scope
                                .operation_declaration(declaring.clone(), key.operation.as_str())?
                                .clone();
                            Some((
                                clause_index,
                                clause.span().start,
                                declaring,
                                operation,
                                hyper_clause::lowering::hyper_location(
                                    clause_index,
                                    hyper_clause::lowering::Site::AlignSkip(skip_index),
                                ),
                            ))
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let relation_operations: Vec<
            Vec<(
                quire_exact::EffectiveId,
                crate::model::operation::OperationDeclaration,
                Location,
            )>,
        > = checked_relations
            .iter()
            .enumerate()
            .map(|(relation_index, relation)| {
                relation
                    .executions()
                    .iter()
                    .zip(relation.execution_aliases())
                    .enumerate()
                    .filter_map(|(execution_index, ((_, operation), alias))| {
                        let model = models
                            .iter()
                            .find(|model| model.alias() == Some(alias.as_str()))?;
                        let declaring = model.effective_id(&operation.declaring)?;
                        let declaration = scope
                            .operation_declaration(declaring.clone(), operation.operation.as_str())?
                            .clone();
                        Some((
                            declaring,
                            declaration,
                            hyper_clause::lowering::hyper_location(
                                relation_index,
                                hyper_clause::lowering::Site::RelationExecution(
                                    relation_index,
                                    execution_index,
                                ),
                            ),
                        ))
                    })
                    .collect()
            })
            .collect();
        let mut source_ordered_anchors: Vec<(usize, lowering::AnchorInput<'_>)> =
            state_clause_forms
                .iter()
                .zip(clause_locations.iter().zip(&reference_locations))
                .filter_map(|(form, (location, reference))| {
                    form.operation.as_ref().map(|operation| {
                        (
                            form.spans.declaration.start,
                            lowering::AnchorInput {
                                declaring: operation.declaring.clone(),
                                operation: &operation.declaration,
                                location,
                                reference,
                            },
                        )
                    })
                })
                .collect();
        for (protocol_index, ((bindings, locations), references)) in protocol_bindings
            .iter()
            .zip(&attempt_locations)
            .zip(&attempt_reference_locations)
            .enumerate()
        {
            for ((bound, location), reference) in bindings.iter().zip(locations).zip(references) {
                let source_order = self
                    .protocols
                    .get(protocol_index)
                    .and_then(|protocol| protocol.declarations.get(bound.declaration))
                    .map_or(usize::MAX, |declaration| declaration.name.span.start);
                source_ordered_anchors.push((
                    source_order,
                    lowering::AnchorInput {
                        declaring: bound.operation.declaring.clone(),
                        operation: &bound.operation.declaration,
                        location,
                        reference,
                    },
                ));
            }
        }
        for (_, source_order, declaring, operation, location) in &hyper_skipped {
            source_ordered_anchors.push((
                *source_order,
                lowering::AnchorInput {
                    declaring: declaring.clone(),
                    operation,
                    location,
                    reference: location,
                },
            ));
        }
        for (relation_index, executions) in relation_operations.iter().enumerate() {
            for (declaring, operation, location) in executions {
                source_ordered_anchors.push((
                    checked_relations[relation_index].span().start,
                    lowering::AnchorInput {
                        declaring: declaring.clone(),
                        operation,
                        location,
                        reference: location,
                    },
                ));
            }
        }
        source_ordered_anchors.sort_by_key(|(source_order, _)| *source_order);
        let mut anchors: Vec<lowering::AnchorInput<'_>> = source_ordered_anchors
            .into_iter()
            .map(|(_, anchor)| anchor)
            .collect();
        for (checked, locations) in checked_protocols.iter().zip(&template_locations) {
            for (template, location) in checked.obligations.compensations.iter().zip(locations) {
                anchors.push(lowering::AnchorInput {
                    declaring: template.operation.declaring.clone(),
                    operation: &template.operation.declaration,
                    location,
                    reference: location,
                });
            }
        }
        // FR-230-AC-35: resolve each admitted tick operation to the same
        // declaration and operation-anchor machinery used by clauses and
        // protocol attempts. Keep the input owned here so the anchor joins
        // the canonical frame-registration order above.
        let timed_operations: Vec<(
            usize,
            quire_exact::EffectiveId,
            crate::model::operation::OperationDeclaration,
            Location,
            Location,
        )> = self
            .time_sources
            .iter()
            .enumerate()
            .filter_map(|(index, source)| {
                let TimeSourceKind::Tick { .. } = source.source else {
                    return None;
                };
                let key = source.tick_operation_key.as_ref()?;
                let model = models
                    .iter()
                    .find(|model| model.alias() == Some(source.model.as_str()))?;
                let (declaring, name) = model.operation_anchor_declaration(key)?;
                let operation = scope
                    .operation_declaration(declaring.clone(), &name)?
                    .clone();
                Some((
                    index,
                    declaring,
                    operation,
                    root(Origin::TypeDeclaration {
                        name: time_source_origin(index),
                    }),
                    root(Origin::TypeDeclaration {
                        name: time_operation_origin(index),
                    }),
                ))
            })
            .collect();
        for (_, declaring, operation, location, reference) in &timed_operations {
            anchors.push(lowering::AnchorInput {
                declaring: declaring.clone(),
                operation,
                location,
                reference,
            });
        }
        if let Err(refusal) = lowering.register_frame_occurrences(&anchors) {
            refusals.push(refusal);
        }
        for (index, declaring, operation, location, reference) in &timed_operations {
            let anchor = lowering::AnchorInput {
                declaring: declaring.clone(),
                operation,
                location,
                reference,
            };
            match lowering.time_source_anchor(&anchor) {
                Ok(key) => self.time_sources[*index].tick_operation_anchor = Some(key),
                Err(refusal) => refusals.push(refusal),
            }
        }
        // QSpec FR-322 "Timed-model nodes": the `clock_place` template of
        // each clock of each dense model, located at its time declaration,
        // which a `presence_read` and a clause name, before any clause whose
        // body reads a presence (QSpec FR-322 `CW-05`).
        for model in &mut timed_models {
            let Some((index, source)) = self
                .time_sources
                .iter()
                .enumerate()
                .find(|(_, source)| source.model == model.model())
            else {
                continue;
            };
            let declaration = root(Origin::TypeDeclaration {
                name: time_source_origin(index),
            });
            match lowering.clock_places(model.clocks(), source.model_node.clone(), &declaration) {
                Ok(places) => model.set_place_nodes(places),
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-488: the `operation_anchor` of each `align skip` operation.
        let mut hyper_align: Vec<Vec<quire_exact::NodeKey>> = vec![Vec::new(); checked_hyper.len()];
        for (clause_index, _, declaring, operation, location) in &hyper_skipped {
            let anchor = lowering::AnchorInput {
                declaring: declaring.clone(),
                operation,
                location,
                reference: location,
            };
            match lowering.align_skip_anchor(&anchor) {
                Ok(key) => hyper_align[*clause_index].push(key),
                Err(refusal) => refusals.push(refusal),
            }
        }
        // S3 resolves every listed operation, so a clause whose anchors fall
        // short of its list is a broken invariant, never a silent drop.
        for (clause, align) in checked_hyper.iter().zip(&hyper_align) {
            if align.len() != clause.align_skip().len() && refusals.is_empty() {
                refusals.push(CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                        quire_semantic_value::catalog::InternalFault::new(
                            "S3",
                            "align-skip-operation-resolves",
                        ),
                    ))),
                });
            }
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
            Template(usize, usize),
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
        for (protocol_index, checked) in checked_protocols.iter().enumerate() {
            let forms = self
                .protocols
                .get(protocol_index)
                .map(|protocol| protocol.compensations.as_slice())
                .unwrap_or_default();
            for template_index in 0..checked.obligations.compensations.len() {
                let start = forms
                    .get(template_index)
                    .map(|form| form.name.span.start)
                    .unwrap_or_default();
                lowering_order.push((start, Lowerable::Template(protocol_index, template_index)));
            }
        }
        lowering_order.sort_by_key(|(start, _)| *start);
        let mut lowered_templates: Vec<Vec<Option<lowering::LoweredAttempt>>> = checked_protocols
            .iter()
            .map(|checked| {
                std::iter::repeat_with(|| None)
                    .take(checked.obligations.compensations.len())
                    .collect()
            })
            .collect();
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
                    let (Some(form), Some(typed), Some(location), Some(name_location), Some(slot)) = (
                        state_clause_forms.get(index),
                        typed_clauses.get(index),
                        clause_locations.get(index),
                        clause_name_locations.get(index),
                        lowered_clauses.get_mut(index),
                    ) else {
                        continue;
                    };
                    let input = lowering::StateClauseInput {
                        kind: form.kind,
                        location,
                        name_location,
                        context: form.context.clone(),
                        anchor: form
                            .operation
                            .as_ref()
                            .map(|operation| lowering::AnchorInput {
                                declaring: operation.declaring.clone(),
                                operation: &operation.declaration,
                                location,
                                reference: location,
                            }),
                        authored_operation_name: form.authored_operation.as_deref(),
                        parameters: &typed.parameters,
                        body: typed.body.root(),
                        body_slots: &typed.slot_names,
                        population_types: &typed.population_types,
                        execution_point: form.execution_point.clone(),
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
                            declaring: bound.operation.declaring.clone(),
                            operation: &bound.operation.declaration,
                            location,
                            reference: location,
                        },
                        frame_population: bound.frame_population.clone(),
                    };
                    match lowering.protocol_attempt(&input) {
                        Ok(lowered) => *slot = Some(lowered),
                        Err(refusal) => refusals.push(refusal),
                    }
                }
                Lowerable::Template(protocol_index, template_index) => {
                    let (Some(template), Some(location), Some(slot)) = (
                        checked_protocols.get(protocol_index).and_then(|checked| {
                            checked.obligations.compensations.get(template_index)
                        }),
                        template_locations
                            .get(protocol_index)
                            .and_then(|locations| locations.get(template_index)),
                        lowered_templates
                            .get_mut(protocol_index)
                            .and_then(|lowered| lowered.get_mut(template_index)),
                    ) else {
                        continue;
                    };
                    let input = lowering::AttemptInput {
                        location,
                        anchor: lowering::AnchorInput {
                            declaring: template.operation.declaring.clone(),
                            operation: &template.operation.declaration,
                            location,
                            reference: location,
                        },
                        frame_population: template.frame_population.clone(),
                    };
                    match lowering.protocol_attempt(&input) {
                        Ok(lowered) => *slot = Some(lowered),
                        Err(refusal) => refusals.push(refusal),
                    }
                }
            }
        }
        // FR-180: lower every checked model relation once state-clause nodes
        // and operation anchors are available for its closed FR-477 joins.
        let mut lowered_relations = Vec::with_capacity(checked_relations.len());
        for (index, relation) in checked_relations.iter().enumerate() {
            let Some(operations) = relation_operations.get(index) else {
                continue;
            };
            let anchors: Vec<lowering::AnchorInput<'_>> = operations
                .iter()
                .map(|(declaring, operation, location)| lowering::AnchorInput {
                    declaring: declaring.clone(),
                    operation,
                    location,
                    reference: location,
                })
                .collect();
            let mut subjects = Vec::new();
            let mut seen_aliases = BTreeSet::new();
            for alias in relation.execution_aliases() {
                if !seen_aliases.insert(alias) {
                    continue;
                }
                let Some(model) = models
                    .iter()
                    .find(|model| model.alias() == Some(alias.as_str()))
                else {
                    refusals.push(CheckRefusal {
                        location: hyper_clause::lowering::hyper_location(
                            index,
                            hyper_clause::lowering::Site::Relation(index),
                        ),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                            quire_semantic_value::catalog::InternalFault::new(
                                "S4",
                                "step-relation-subject-selected",
                            ),
                        ))),
                    });
                    continue;
                };
                match model_root_key(model.selection()) {
                    Ok(key) => subjects.push((alias.clone(), key)),
                    Err(_) => refusals.push(CheckRefusal {
                        location: hyper_clause::lowering::hyper_location(
                            index,
                            hyper_clause::lowering::Site::Relation(index),
                        ),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                            quire_semantic_value::catalog::InternalFault::new(
                                "S4",
                                "step-relation-model-root-keyed",
                            ),
                        ))),
                    }),
                }
            }
            match lowering.step_relation(&lowering::StepRelationInput {
                index,
                relation,
                anchors: &anchors,
                subjects: &subjects,
                state_forms: &state_clause_forms,
                state_nodes: &lowered_clauses,
            }) {
                Ok(lowered) => lowered_relations.push(lowered),
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-105 "Boolean model-field transitions": the derived
        // `state`/`transition` node of each eligible (operation, Boolean
        // field) pair, from the lowered `pre` and `post` clauses.
        if refusals.is_empty() {
            let transition_clauses: Vec<lowering::TransitionClause<'_>> = state_clause_forms
                .iter()
                .zip(&lowered_clauses)
                .zip(&clause_locations)
                .filter_map(|((form, lowered), location)| {
                    let operation = form.operation.as_ref()?;
                    Some(lowering::TransitionClause {
                        kind: form.kind,
                        name: form.name.as_str(),
                        source_order: form.spans.declaration.start,
                        location,
                        context: form.context.clone(),
                        declaring: operation.declaring.clone(),
                        operation: &operation.declaration,
                        has_execution_point: form.execution_point.is_some(),
                        lowered: lowered.as_ref()?,
                    })
                })
                .collect();
            if let Err(refusal) = lowering.state_transitions(&transition_clauses) {
                refusals.push(refusal);
            }
        }
        // FR-092: the node of each protocol that holds an `attempt`, now that
        // the operation anchors and state clauses its attempts name are keyed.
        let attempt_nodes_written = refusals.is_empty();
        if attempt_nodes_written {
            let attempts: Vec<Vec<lowering::AttemptNodes>> = protocol_bindings
                .iter()
                .zip(&lowered_attempts)
                .map(|(bindings, lowered)| {
                    bindings
                        .iter()
                        .zip(lowered)
                        .filter_map(|(bound, lowered)| {
                            let contracts = bound
                                .contracts
                                .iter()
                                .map(|&index| {
                                    lowered_clauses
                                        .get(index)
                                        .and_then(Option::as_ref)
                                        .map(|clause| clause.key.clone())
                                })
                                .collect::<Option<Vec<_>>>()?;
                            Some(lowering::AttemptNodes {
                                declaration: bound.declaration,
                                anchor: lowered.as_ref()?.binding.anchor.clone(),
                                contracts,
                            })
                        })
                        .collect()
                })
                .collect();
            let compensations: Vec<Vec<lowering::CompensationNodes>> = checked_protocols
                .iter()
                .zip(&lowered_templates)
                .map(|(checked, lowered)| {
                    checked
                        .obligations
                        .compensations
                        .iter()
                        .enumerate()
                        .filter_map(|(declaration, _)| {
                            Some(lowering::CompensationNodes {
                                declaration,
                                anchor: lowered.get(declaration)?.as_ref()?.binding.anchor.clone(),
                            })
                        })
                        .collect()
                })
                .collect();
            match protocol_reference::inputs(&self.protocols, &protocol_places, &checked_protocols)
            {
                Ok(inputs) => {
                    if let Err(refusal) =
                        lowering.protocol_nodes_with_attempts(&inputs, &attempts, &compensations)
                    {
                        refusals.push(refusal);
                    }
                }
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-375: a protocol subject's abstract side names the emitted
        // protocol clause node. Resolve it after dependency-ordered protocol
        // lowering (and after attempt-bearing clauses receive their nodes).
        for checked in &mut refinements {
            let AbstractSide::Protocol { clause } = &checked.abstract_side else {
                continue;
            };
            checked.abstract_protocol = self
                .protocols
                .iter()
                .position(|protocol| protocol.name.name == *clause)
                .and_then(|index| lowering.protocol_key(index, &root(Origin::Expression)).ok());
        }
        // QSpec FR-368: each clause's own node, its subject's derived-item
        // anchor; a clause with no node keeps none.
        for (index, place) in protocol_places.iter().enumerate() {
            if let Some(checked) = place.and_then(|place| checked_protocols.get_mut(place)) {
                checked.node = lowering.protocol_key(index, &root(Origin::Expression)).ok();
            }
        }
        // FR-135: the clause a refinement's rows select, by its node. This
        // follows temporal lowering because a selected protocol may require
        // one of those clause nodes.
        for (checked, selected) in refinements.iter_mut().zip(&selected_protocols) {
            let Some(selected) = selected else {
                continue;
            };
            match lowering.protocol_key(selected.protocol(), &root(Origin::Expression)) {
                Ok(key) => checked.concrete_protocol = Some(key),
                Err(refusal)
                    if self.emit_unwritten_protocols
                        && matches!(
                            &refusal.cause,
                            CheckCause::ProtocolReference(cause)
                                if cause.construct().is_some()
                        ) => {}
                Err(refusal)
                    if !attempt_nodes_written
                        && matches!(
                            &refusal.cause,
                            CheckCause::ProtocolReference(cause)
                                if cause.construct()
                                    == Some(ProtocolConstruct::ReferenceToAttemptProtocol)
                        ) => {}
                Err(refusal) => refusals.push(selected.unkeyed(refusal)),
            }
        }
        // FR-488: each emitted hyper clause over `behaviours`, after every
        // function its conditions may call.
        let mut lowered_hyper = Vec::with_capacity(checked_hyper.len());
        for (index, clause) in checked_hyper.iter().enumerate() {
            let align = hyper_align.get(index).map_or(&[][..], Vec::as_slice);
            if align.len() != clause.align_skip().len() {
                continue;
            }
            match lowering.hyper_clause(&lowering::HyperClauseInput {
                index,
                clause,
                align_skip: align,
            }) {
                Ok(lowered) => lowered_hyper.push(lowered),
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-410: each checked state-graph claim, after every function its
        // predicates may call.
        let mut lowered_state_graph = Vec::with_capacity(checked_state_graph_claims.len());
        for (index, claim) in checked_state_graph_claims.iter().enumerate() {
            match lowering.state_graph_claim(&lowering::StateGraphClaimInput { index, claim }) {
                Ok(lowered) => lowered_state_graph.push(lowered),
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-447, QSpec FR-481: each checked hybrid model, then each claim
        // over one, after every function their terms may call.
        let mut lowered_hybrids = Vec::with_capacity(checked_hybrid_models.len());
        for (index, model) in checked_hybrid_models.iter().enumerate() {
            match lowering.hybrid_model(&lowering::HybridModelInput {
                index,
                model,
                signatures: signatures.as_slice(),
            }) {
                Ok(lowered) => lowered_hybrids.push(lowered),
                Err(refusal) => refusals.push(refusal),
            }
        }
        let mut lowered_hybrid_claims = Vec::with_capacity(checked_hybrid_claim_list.len());
        if lowered_hybrids.len() == checked_hybrid_models.len() {
            for (index, claim) in checked_hybrid_claim_list.iter().enumerate() {
                let (Some(model), Some(lowered)) = (
                    checked_hybrid_models.get(claim.model.get()),
                    lowered_hybrids.get(claim.model.get()),
                ) else {
                    continue;
                };
                match lowering.hybrid_claim(&lowering::HybridClaimInput {
                    index,
                    claim,
                    model,
                    lowered,
                    signatures: signatures.as_slice(),
                }) {
                    Ok(lowered) => lowered_hybrid_claims.push((index, lowered)),
                    Err(refusal) => refusals.push(refusal),
                }
            }
        }
        // FR-411, QSpec FR-476: each checked `terminal` declaration, after
        // every function its predicate may call.
        let mut lowered_terminals = Vec::with_capacity(terminals.len());
        for (index, declaration) in self.terminals.iter().enumerate() {
            let Some(checked) = terminals
                .iter()
                .find(|checked| checked.model == declaration.model)
            else {
                continue;
            };
            match lowering.terminal(&lowering::TerminalInput {
                index,
                model: &declaration.model,
                declaration: &checked.declaration,
            }) {
                Ok(key) => lowered_terminals.push((index, declaration.model.clone(), key)),
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-230, QSpec FR-322 "Timed-model nodes": each `urgent when`
        // predicate's expression node, which its `urgency` node references.
        for model in &mut timed_models {
            let mut lowered = Vec::new();
            for (index, urgency, predicate) in model.urgency_predicates() {
                let timed_model::Urgency::When {
                    context,
                    alternatives,
                    ..
                } = urgency
                else {
                    continue;
                };
                let sites: Vec<_> = predicate
                    .sites
                    .iter()
                    .map(|(site, _)| timed_model::site_location(site))
                    .collect();
                match lowering.timed_state(model.clocks(), context.clone(), alternatives, &sites) {
                    Ok(key) => lowered.push((index, key)),
                    Err(refusal) => refusals.push(refusal),
                }
            }
            model.set_urgency_predicate_nodes(lowered);
        }
        // FR-230, QSpec FR-322 "Clause nodes": the terms of the guards,
        // resets and time invariants of each model S4 writes whole.
        for model in &mut timed_models {
            if model.unwritten().is_some() {
                continue;
            }
            let Some(source) = self
                .time_sources
                .iter()
                .position(|source| source.model == model.model())
            else {
                continue;
            };
            let declaration = root(Origin::TypeDeclaration {
                name: time_source_origin(source),
            });
            let name = model.model().to_owned();
            let part_site = |guard: usize, part: usize| {
                timed_model::site_location(&timed_model::guard_part_site(&name, guard, part))
            };
            let invariant_site = |invariant: usize, alternative: usize| {
                timed_model::site_location(&timed_model::invariant_site(
                    &name,
                    invariant,
                    alternative,
                ))
            };
            let input = lowering::TimedClauseInput {
                model,
                clause_locations: &clause_locations,
                declaration: &declaration,
                part_site: &part_site,
                invariant_site: &invariant_site,
            };
            match lowering.timed_clauses(&input) {
                Ok(nodes) => model.set_clause_nodes(nodes),
                Err(refusal) => refusals.push(refusal),
            }
        }
        for composite in scope.types().composites() {
            match lowering.composite_node(composite.key()) {
                Ok(node) => {
                    type_nodes.push((
                        node.clone(),
                        identity::CheckedTypeNode::Composite {
                            node,
                            declaration: composite.key(),
                            form: composite.shape().into(),
                        },
                    ));
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
        // FR-094 "Dimension equivalence": one node per admitted equivalence,
        // whether or not a quantity type names either dimension.
        if let Err(refusal) = lowering.dimension_equivalence_nodes(&self.dimension_equivalences) {
            refusals.push(refusal);
        }
        // FR-375: each class catalog, lowered once its protocol clauses'
        // nodes exist; a node no other declaration located takes its one
        // `generated` occurrence at the refinement.
        for checked in &mut refinements {
            let Some(catalog) = checked.class_catalog.as_ref() else {
                continue;
            };
            let location = root(Origin::TypeDeclaration {
                name: refinement_origin(checked.name()),
            });
            match lowering.class_catalog_keys(catalog, &location) {
                Ok(keys) => checked.class_catalog_keys = keys,
                Err(refusal) => refusals.push(refusal),
            }
        }
        // FR-375: each refinement's rows, keyed once its catalog is, each
        // row expression located at its own site. A refinement whose
        // catalog names a node not written yet, or a row a form with no
        // node, is not emitted, so its rows are not keyed.
        let mut refinement_booleans = std::collections::BTreeMap::new();
        for checked in &mut refinements {
            if (checked.class_catalog.is_some() && checked.class_catalog_keys.is_none())
                || !checked.rows_have_nodes()
            {
                continue;
            }
            for row in checked.rows() {
                site_spans.insert(row.site.clone(), row.spans.clone());
            }
            let location = root(Origin::TypeDeclaration {
                name: refinement_origin(checked.name()),
            });
            match lowering.refinement_rows(checked, &location) {
                Ok(rows) => checked.row_keys = Some(rows),
                Err(refusal) => refusals.push(refusal),
            }
            // FR-144: the `Boolean` result bound of the refinement's record.
            match lowering.refinement_result_type(&location) {
                Ok(boolean) => {
                    refinement_booleans.insert(checked.name().to_owned(), boolean);
                }
                Err(refusal) => refusals.push(refusal),
            }
        }
        let lowered = lowering.finish(&lowering::generated_location());
        // FR-380: a population node's `Type` occurrences take their ordinals
        // in ascending source order, whichever writer recorded one first.
        occurrences.order_by(
            family::OccurrenceRole::Type,
            &lowered.population_nodes,
            |location| regions.region(location).map(|region| region.start()),
        );
        let parameter_spans = lowered.parameter_spans;
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
        let lowered_templates: Vec<Vec<lowering::LoweredAttempt>> = lowered_templates
            .into_iter()
            .map(|lowered| lowered.into_iter().collect::<Option<_>>())
            .collect::<Option<_>>()
            .ok_or_else(unlowered)?;
        // Each contract entry's own checked identity (FR-114 "Outputs"),
        // captured before `lowered_clauses` is consumed below.
        let lowered_clause_keys: Vec<quire_exact::NodeKey> = lowered_clauses
            .iter()
            .map(|lowered| lowered.key.clone())
            .collect();
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
        let (semantic_graph, model_correspondence, identities, binders, lowered_protocol_members) = (
            lowered.graph,
            lowered.correspondence,
            lowered.functions,
            lowered.binders,
            lowered.protocol_members,
        );
        // FR-218: bind each already-checked predicate join to its function
        // node key after the function declarations have been lowered.
        for checked_index in protocol_places.iter().flatten().copied() {
            let Some(checked) = checked_protocols.get_mut(checked_index) else {
                return Err(vec![CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                }]);
            };
            for pending in std::mem::take(&mut checked.pending_predicate_joins) {
                let identity = identities.get(pending.function).cloned().flatten();
                let Some(identity) = identity else {
                    return Err(vec![CheckRefusal {
                        location: root(Origin::Expression),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                    }]);
                };
                checked.parallels.push(CheckedParallel {
                    node: pending.node,
                    policy: CheckedJoinPolicy::Predicate { function: identity },
                    joined: pending.joined,
                    outstanding: pending.outstanding,
                });
            }
            checked.parallels.sort_by_key(|parallel| parallel.node.0);
        }
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
        // FR-146 "Claim extent": the populations each operation's contract
        // clauses range over, which its refinement `Step` records add.
        let mut contract_populations: BTreeMap<
            (quire_exact::EffectiveId, String),
            std::collections::BTreeSet<state_clause::PopulationDomain>,
        > = BTreeMap::new();
        // ... and the populations each clause ranges over, by its node, which
        // a hypothesis adds to every record.
        let mut clause_populations: BTreeMap<
            quire_exact::NodeKey,
            std::collections::BTreeSet<state_clause::PopulationDomain>,
        > = BTreeMap::new();
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
                    .first_origin(&lowered.key, family::OccurrenceRole::Claim, |at| {
                        *at == location
                    })
                    .ok_or_else(unkeyable)?;
                let claims = state_clause::ProtocolClauseFamily::requirements(&typed);
                let ranged: Vec<_> = claims
                    .iter()
                    .filter(|claim| claim.subject == state_clause::ClaimSubject::Clause)
                    .flat_map(|claim| claim.populations.iter().cloned())
                    .collect();
                if let Some(operation) = &form.operation {
                    contract_populations
                        .entry(state_clause::operation_reference(operation))
                        .or_default()
                        .extend(ranged.iter().cloned());
                }
                clause_populations
                    .entry(lowered.key.clone())
                    .or_default()
                    .extend(ranged);
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
                    match requirements.entry(OccurrenceKey::new(claims::wire(node.clone()), origin))
                    {
                        std::collections::hash_map::Entry::Vacant(slot) => {
                            slot.insert(record);
                        }
                        // Every clause naming one operation keys the same
                        // frame record; any other collision is a fault.
                        std::collections::hash_map::Entry::Occupied(existing)
                            if node != lowered.key && *existing.get() == record => {}
                        std::collections::hash_map::Entry::Occupied(_) => return Err(unkeyable()),
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
                    bounds: typed.bounds,
                })
            })
            .collect::<Result<Vec<_>, Vec<CheckRefusal>>>()?;
        // FR-220: each attempt inside a `parallel` classified as a load, a
        // store, a read-modify-write or local over the places its enclosing
        // `parallel` controls share, with its ordering admitted and, under a
        // weak default, its shape.
        if let Some(views) = access_views {
            let model = crate::model::state::StateModel::from_views(
                views,
                scope.types(),
                crate::model::accounting::ModelNormalizationLimits::default(),
            );
            let mut access_refusals = Vec::new();
            for (index, protocol) in self.protocols.iter().enumerate() {
                let (Some(checked), Some(bound)) =
                    (checked_protocols.get(index), protocol_bindings.get(index))
                else {
                    continue;
                };
                let classified = protocol_memory::classify(protocol, checked, bound, |binding| {
                    let declaring = binding.operation.declaring.clone();
                    let arguments: Vec<(String, quire_exact::EffectiveId)> = binding
                        .operation
                        .declaration
                        .parameters()
                        .iter()
                        .filter_map(|(name, value_type)| {
                            footprint::parameter_target(value_type)
                                .map(|target| (name.clone(), target))
                        })
                        .collect();
                    footprint::derive_from(
                        scope.types(),
                        &state_clauses,
                        &model,
                        footprint::Operation {
                            receiver_type: binding.receiver.clone(),
                            declaring,
                            operation: &binding.operation.declaration,
                            arguments: &arguments,
                        },
                        &|_| true,
                    )
                });
                match classified {
                    Ok(accesses) => {
                        if let Some(checked) = checked_protocols.get_mut(index) {
                            checked.memory.accesses = accesses;
                        }
                    }
                    Err(found) => {
                        access_refusals.extend(found.into_iter().map(|(_, refusal)| refusal));
                    }
                }
            }
            if !access_refusals.is_empty() {
                return Err(access_refusals);
            }
        }
        // FR-337: each emitted temporal clause's node, its `claim`
        // occurrence and its `over` node; FR-326 "Requirement record": its
        // `temporal-satisfaction` record, keyed by that occurrence.
        if lowered_temporal.len() != checked_temporal.len()
            || lowered_temporal.iter().any(Option::is_none)
        {
            return Err(unlowered());
        }
        if lowered_hyper.len() != checked_hyper.len() {
            return Err(unlowered());
        }
        if lowered_relations.len() != checked_relations.len() {
            return Err(unlowered());
        }
        if lowered_state_graph.len() != checked_state_graph_claims.len() {
            return Err(unlowered());
        }
        // FR-488 "Outputs": each hyper clause carries its `claim`
        // occurrence, the `clause` of FR-315's `HyperProductIdentity`.
        // The sites no declared form spans (slots, conjunct slots, skipped
        // operations) are located at their conjunct or clause.
        for (index, clause) in checked_hyper.iter().enumerate() {
            use hyper_clause::lowering::{hyper_site, Site};
            for slot in 0..clause.atom_slots().len() {
                type_spans.insert(hyper_site(index, Site::Slot(slot)), clause.span());
            }
            for skipped in 0..clause.align_skip().len() {
                type_spans.insert(hyper_site(index, Site::AlignSkip(skipped)), clause.span());
            }
            for (part, at) in clause.conjuncts() {
                let (conjunct, make): (_, fn(usize, usize) -> Site) = match part {
                    hyper_clause::MatchPart::Universal => {
                        (clause.mu_universal().get(at), Site::UniversalSlot)
                    }
                    hyper_clause::MatchPart::Existential => {
                        (clause.mu_existential().get(at), Site::ExistentialSlot)
                    }
                };
                let Some(conjunct) = conjunct else { continue };
                for slot in 0..conjunct.slots().len() {
                    type_spans.insert(hyper_site(index, make(at, slot)), conjunct.span());
                }
            }
        }
        for (index, relation) in checked_relations.iter().enumerate() {
            type_spans.insert(
                hyper_clause::lowering::hyper_site(
                    index,
                    hyper_clause::lowering::Site::Relation(index),
                ),
                relation.span(),
            );
            type_spans.insert(
                hyper_clause::lowering::hyper_site(
                    index,
                    hyper_clause::lowering::Site::RelationBody(index),
                ),
                relation.span(),
            );
            site_spans.insert(
                format!("relation {} body", relation.name()),
                relation.body_spans().clone(),
            );
            for execution in 0..relation.executions().len() {
                type_spans.insert(
                    hyper_clause::lowering::hyper_site(
                        index,
                        hyper_clause::lowering::Site::RelationExecution(index, execution),
                    ),
                    relation.span(),
                );
            }
        }
        // FR-411: each terminal node's `claim` occurrence, the key a reader of
        // the emitted package finds the declaration's node by.
        for (index, model, key) in &lowered_terminals {
            let location = lowering::terminal_location(*index, lowering::TerminalSite::Claim);
            let claim = occurrences
                .first_origin(key, family::OccurrenceRole::Claim, |at| *at == location)
                .ok_or_else(|| {
                    vec![CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                    }]
                })?;
            if let Some(checked) = terminals.iter_mut().find(|checked| checked.model == *model) {
                checked.occurrence =
                    Some(OccurrenceKey::new(claims::wire(key.clone()), claim.clone()));
            }
        }
        let hyper_nodes = checked_hyper
            .into_iter()
            .zip(&lowered_hyper)
            .enumerate()
            .map(|(index, (checked, lowered))| {
                let location = hyper_clause::lowering::hyper_location(
                    index,
                    hyper_clause::lowering::Site::Claim,
                );
                let claim = occurrences
                    .first_origin(&lowered.key, family::OccurrenceRole::Claim, |at| {
                        *at == location
                    })
                    .ok_or_else(|| {
                        vec![CheckRefusal {
                            location: location.clone(),
                            cause: CheckCause::InternalFault(Box::new(
                                KeyFault::UnkeyableRequirements,
                            )),
                        }]
                    })?;
                Ok(checked
                    .assembled_at(OccurrenceKey::new(claims::wire(lowered.key.clone()), claim)))
            })
            .collect::<Result<Vec<_>, Vec<CheckRefusal>>>()?;
        // FR-410 "Outputs": each state-graph claim carries its `claim`
        // occurrence; FR-165 "Requirement record": its
        // `temporal-satisfaction` record, keyed by that occurrence.
        let mut state_graph_nodes = Vec::with_capacity(checked_state_graph_claims.len());
        for (index, (checked, lowered)) in checked_state_graph_claims
            .into_iter()
            .zip(&lowered_state_graph)
            .enumerate()
        {
            let location =
                state_graph_claim::state_graph_location(index, state_graph_claim::Site::Claim);
            let unkeyable = || {
                vec![CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                }]
            };
            let claim = occurrences
                .first_origin(&lowered.key, family::OccurrenceRole::Claim, |at| {
                    *at == location
                })
                .ok_or_else(unkeyable)?;
            let (from, target) = checked.node_predicates();
            let objects: std::collections::BTreeSet<_> = from
                .into_iter()
                .chain([target])
                .filter_map(state_graph_claim::CheckedStatePredicate::body)
                .flat_map(references::body_objects)
                .collect();
            let record = claims::RequirementRecord::unguarded(
                checked
                    .claim()
                    .requirements(claims::wire(lowered.key.clone())),
                ValueType::Boolean,
                claims::wire(lowered.boolean.clone()),
                references::ModelReferences::new(objects, [], None),
            );
            match requirements.entry(OccurrenceKey::new(
                claims::wire(lowered.key.clone()),
                claim.clone(),
            )) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(record);
                }
                std::collections::hash_map::Entry::Occupied(_) => return Err(unkeyable()),
            }
            state_graph_nodes.push(
                checked.assembled_at(OccurrenceKey::new(claims::wire(lowered.key.clone()), claim)),
            );
        }
        let named_scope = NamedScope::of(&models, &scope);
        let lowered_temporal = lowered_temporal
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(unlowered)?;
        let temporal_clauses = checked_temporal
            .into_iter()
            .zip(lowered_temporal)
            .map(|((index, checked, conditions), lowered)| {
                let location = lowering::temporal_location(index, lowering::TemporalSite::Claim);
                let unkeyable = || {
                    vec![CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                    }]
                };
                let claim = occurrences
                    .first_origin(&lowered.key, family::OccurrenceRole::Claim, |at| {
                        *at == location
                    })
                    .ok_or_else(unkeyable)?;
                let objects: std::collections::BTreeSet<_> = conditions
                    .all()
                    .flat_map(|condition| references::body_objects(condition.body.root()))
                    .collect();
                // FR-391 "Requirements": every unbounded population a `holds`
                // atom names is a domain of the clause's record.
                let named_keys = {
                    let mut domains = std::collections::BTreeSet::new();
                    for condition in conditions.all() {
                        domains.extend(
                            state_clause::named_domains(condition.body.root(), &named_scope)
                                .into_iter()
                                .map(|(domain, _)| domain)
                                .filter(|domain| domain.maximum.is_none()),
                        );
                    }
                    domains
                        .into_iter()
                        .map(|domain| population_domain_key(&object_types, &object_nodes, domain))
                        .collect::<Result<Vec<_>, _>>()
                };
                for claimed in temporal_clause::TemporalTraceFamily::requirements(&checked) {
                    let extent = claimed
                        .populations()
                        .iter()
                        .map(|domain| {
                            population_domain_key(&object_types, &object_nodes, domain.clone())
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .and_then(|populations| {
                            claimed.requirements(
                                claims::wire(lowered.key.clone()),
                                &lowered
                                    .parameters
                                    .iter()
                                    .cloned()
                                    .map(claims::wire)
                                    .collect::<Vec<_>>(),
                                &populations,
                                named_keys.as_ref().map_err(Clone::clone)?,
                            )
                        })
                        .map_err(|fault| {
                            vec![CheckRefusal {
                                location: location.clone(),
                                cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                                    fault,
                                ))),
                            }]
                        })?;
                    // FR-243: a clause over a timed subject, `dense` or
                    // `tick`, is EN-6's item.
                    let record = claims::RequirementRecord::unguarded(
                        if checked.timed() {
                            extent.over_timed()
                        } else {
                            extent
                        },
                        ValueType::Boolean,
                        claims::wire(lowered.boolean.clone()),
                        references::ModelReferences::new(objects.clone(), [], None),
                    );
                    match requirements.entry(OccurrenceKey::new(
                        claims::wire(lowered.key.clone()),
                        claim.clone(),
                    )) {
                        std::collections::hash_map::Entry::Vacant(slot) => {
                            slot.insert(record);
                        }
                        std::collections::hash_map::Entry::Occupied(_) => return Err(unkeyable()),
                    }
                }
                Ok(TemporalClauseNode {
                    clause: checked,
                    identity: lowered.key,
                    claim,
                    over: lowered.over,
                    guard: conditions
                        .guard
                        .map(|condition| (condition.body, condition.slots)),
                    captures: conditions
                        .captures
                        .into_iter()
                        .map(|condition| (condition.body, condition.slots))
                        .collect(),
                    conditions: conditions
                        .holds
                        .into_iter()
                        .map(|(atom, condition)| (atom, (condition.body, condition.slots)))
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, Vec<CheckRefusal>>>()?;
        // FR-447 "The provider request": one `temporal-satisfaction` record
        // per checked hybrid claim, keyed by the claim's `claim` occurrence,
        // carrying the hybrid-dynamics requirement every hybrid claim has
        // (every checked hybrid model has a `Hybrid` variable).
        for (index, lowered) in lowered_hybrid_claims {
            let location = lowering::hybrid_location(lowering::HybridSite::Claim(index));
            let unkeyable = || {
                vec![CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                }]
            };
            let claim = occurrences
                .first_origin(&lowered.key, family::OccurrenceRole::Claim, |at| {
                    *at == location
                })
                .ok_or_else(unkeyable)?;
            let checked_claim = checked_hybrid_claim_list.get(index).ok_or_else(unkeyable)?;
            let bounded = checked_claim.horizon.is_some();
            // The class S3 recorded for each variable; the request label is
            // read from it, never classified again (FR-250).
            let hybrid_dynamics = checked_hybrid_models
                .get(checked_claim.model.get())
                .ok_or_else(unkeyable)?
                .variables
                .iter()
                .any(|variable| variable.class == hybrid::VariableClass::Hybrid);
            let mut domains = std::collections::BTreeMap::new();
            if !bounded {
                domains.insert(
                    qsl_foundation::bound::DomainKey::Node {
                        node: claims::wire(lowered.key.clone()),
                        path: Vec::new(),
                    },
                    qsl_foundation::bound::DomainKind::InfiniteTrace,
                );
            }
            let requirement = crate::family::Requirements::new(
                crate::check::Capability::TemporalSatisfaction,
                crate::family::ClaimExtent::from_domains(domains),
            );
            let record = claims::RequirementRecord::unguarded(
                if hybrid_dynamics {
                    requirement.with_hybrid_dynamics()
                } else {
                    requirement
                },
                ValueType::Boolean,
                claims::wire(lowered.boolean),
                references::ModelReferences::new(Default::default(), [], None),
            );
            match requirements.entry(OccurrenceKey::new(claims::wire(lowered.key), claim.clone())) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(record);
                }
                std::collections::hash_map::Entry::Occupied(_) => return Err(unkeyable()),
            }
        }
        // FR-144 "Requirement record and request": one `temporal-satisfaction`
        // record per refinement the package emits, keyed by the `declaration`
        // occurrence of its V2 node (QSpec FR-375), with the TP-5 form. A
        // refinement the V2 slice does not emit is refused below unless a
        // test asked for its package unwritten, and then has no node to key.
        for checked in &mut refinements {
            if !checked.v2_emittable() {
                continue;
            }
            let location = root(Origin::TypeDeclaration {
                name: refinement_origin(checked.name()),
            });
            let unkeyable = || {
                vec![CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                }]
            };
            let emission = checked.v2_emission().map_err(|_| unkeyable())?;
            let node = claims::wire(emission.identity.clone());
            let boolean = refinement_booleans
                .get(checked.name())
                .cloned()
                .ok_or_else(unkeyable)?;
            let occurrence = OccurrenceKey::new(
                node.clone(),
                quire_exact::Origin::new(quire_exact::Role::new("declaration"), 0),
            );
            // FR-430: a refinement whose concrete model is `time dense` is
            // EN-6's item. EN-6's refinement product reads only a dense
            // concrete subject, so a `tick` refinement is not marked and
            // goes to the explicit-state engines.
            let dense = self.time_sources.iter().any(|source| {
                source.model == checked.concrete_model() && source.source == TimeSourceKind::Dense
            });
            let wanted = checked.requirements(node);
            let record = claims::RequirementRecord::unguarded(
                if dense { wanted.over_timed() } else { wanted },
                ValueType::Boolean,
                claims::wire(boolean.clone()),
                references::ModelReferences::new(Default::default(), [], None),
            );
            // Two refinements of one meaning share one node and so one
            // record; S4 refuses the pair naming both (FR-375).
            match requirements.entry(occurrence.clone()) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(record);
                }
                std::collections::hash_map::Entry::Occupied(slot) if *slot.get() == record => {}
                std::collections::hash_map::Entry::Occupied(_) => return Err(unkeyable()),
            }
            checked.requirement = Some(occurrence);
            // FR-146: the per-step simulation records beside it, each keyed
            // by its O-07 `generated` occurrence at the refinement node.
            let object_node = |object: quire_exact::EffectiveId| {
                object_node_of(&object_types, &object_nodes, object)
            };
            let inputs = step_simulation::ExtentInputs {
                node: emission.identity,
                boolean,
                contracts: &contract_populations,
                clauses: &clause_populations,
                object_node: &object_node,
                types: scope.types(),
                position_limit: limits.nodes(),
                location: &location,
            };
            let written = step_simulation::simulation_records(checked, &state_clauses, &inputs)
                .map_err(|refusal| vec![refusal])?;
            let mut entries = Vec::with_capacity(written.len());
            for (entry, record) in written {
                match requirements.entry(entry.occurrence().clone()) {
                    std::collections::hash_map::Entry::Vacant(slot) => {
                        slot.insert(record);
                    }
                    std::collections::hash_map::Entry::Occupied(slot) if *slot.get() == record => {}
                    std::collections::hash_map::Entry::Occupied(_) => return Err(unkeyable()),
                }
                entries.push(entry);
            }
            checked.simulations = entries;
        }
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
                        .clone()
                        .zip(lowered_attempt.population_object.clone())
                        .into_iter()
                        .collect();
                let record = state_clause::frame_record(
                    &bound.operation,
                    frame.clone(),
                    &populations,
                    lowered_attempt.boolean.clone(),
                    scope.types(),
                    limits.nodes(),
                    location,
                )
                .map_err(|refusal| vec![refusal])?;
                match requirements.entry(OccurrenceKey::new(
                    claims::wire(frame.clone()),
                    frame_origin,
                )) {
                    std::collections::hash_map::Entry::Vacant(slot) => {
                        slot.insert(record);
                    }
                    std::collections::hash_map::Entry::Occupied(existing)
                        if *existing.get() == record => {}
                    std::collections::hash_map::Entry::Occupied(_) => return Err(unkeyable()),
                }
                let contracts = bound
                    .contracts
                    .iter()
                    .map(|&index| lowered_clause_keys.get(index).cloned())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(unkeyable)?;
                attempts.push(protocol_clause::CheckedAttempt {
                    declaration: bound.declaration,
                    operation: bound.operation.clone(),
                    receiver: bound.receiver.clone(),
                    anchor,
                    frame,
                    contracts,
                });
            }
            checked.attempts = attempts;
        }
        // FR-381 "Compensation registration": each template bound to its
        // operation's one anchor and frame node, with the one frame record
        // FR-114 keys at the frame (equal for every attempt, clause and
        // template naming the operation, so the entry is kept once).
        for ((checked, lowered), locations) in checked_protocols
            .iter_mut()
            .zip(&lowered_templates)
            .zip(&template_locations)
        {
            let mut bindings = Vec::with_capacity(lowered.len());
            for ((template, lowered_template), location) in checked
                .obligations
                .compensations
                .iter()
                .zip(lowered)
                .zip(locations)
            {
                let lowering::OperationBinding {
                    anchor,
                    frame,
                    frame_origin,
                } = lowered_template.binding.clone();
                let populations: Vec<(state_clause::PopulationDomain, quire_exact::NodeKey)> =
                    template
                        .frame_population
                        .clone()
                        .zip(lowered_template.population_object.clone())
                        .into_iter()
                        .collect();
                let record = state_clause::frame_record(
                    &template.operation,
                    frame.clone(),
                    &populations,
                    lowered_template.boolean.clone(),
                    scope.types(),
                    limits.nodes(),
                    location,
                )
                .map_err(|refusal| vec![refusal])?;
                match requirements.entry(OccurrenceKey::new(
                    claims::wire(frame.clone()),
                    frame_origin,
                )) {
                    std::collections::hash_map::Entry::Vacant(slot) => {
                        slot.insert(record);
                    }
                    std::collections::hash_map::Entry::Occupied(existing)
                        if *existing.get() == record => {}
                    std::collections::hash_map::Entry::Occupied(_) => {
                        return Err(vec![CheckRefusal {
                            location: location.clone(),
                            cause: CheckCause::InternalFault(Box::new(
                                KeyFault::UnkeyableRequirements,
                            )),
                        }]);
                    }
                }
                bindings.push(protocol_clause::CheckedOperationNodes { anchor, frame });
            }
            checked.compensation_operations = bindings;
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
        let mut unwritten = Vec::new();
        if !self.emit_unwritten_refinements {
            unwritten.extend(self.refinements.iter().filter_map(|declaration| {
                // FR-136-AC-12: a subject without exactly one state machine is
                // refused by name before the generic not-emitted refusal. S3
                // checked every declaration (an unchecked one refused above
                // and returned), so a declaration with no checked refinement
                // is a broken invariant, never a silent not-emitted.
                let Some(checked) = refinements
                    .iter()
                    .find(|checked| checked.name() == declaration.form.name.name)
                else {
                    return Some(CheckRefusal {
                        location: Location::root(Origin::Expression),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                            quire_semantic_value::catalog::InternalFault::new(
                                "S3",
                                "refinement-declaration-checked",
                            ),
                        ))),
                    });
                };
                (!checked.v2_emittable()).then(|| {
                    refinement::not_emitted(
                        declaration.form.name.name.clone(),
                        declaration.form.span,
                    )
                })
                // `None` means this checked declaration is included by the
                // current V2 emission slice.
            }));
        }
        if !self.emit_unwritten_timed_members {
            unwritten.extend(timed_models.iter().filter_map(timed_model::not_emitted));
        }
        if !unwritten.is_empty() {
            return Err(unwritten);
        }
        let mut graph = CheckedGraph {
            source,
            package_name,
            scope,
            functions,
            dispatch_tables,
            occurrences,
            parameter_spans,
            requirements: requirements.into(),
            model_correspondence,
            // A later entry for a key replaces an earlier one, as an insert
            // would: the ranking is stable and collecting keeps the last.
            type_nodes: key_order::sorted_by_key(type_nodes, |(node, _)| node)
                .into_iter()
                .collect(),
            semantic_graph,
            effective_limits: limits,
            form_spans,
            type_spans,
            site_spans,
            synthesized_spans,
            embedding,
            model_selections,
            model_roots,
            header_layers,
            time_sources: self.time_sources,
            time_units: time_units.clone(),
            state_clauses,
            type_invariants,
            terminals,
            timed_models,
            refinements,
            abstractions: self.abstractions,
            model_names,
            protocols: checked_protocols,
            protocol_node_keys: lowered_protocol_members
                .into_iter()
                .map(|(name, members)| {
                    (
                        name,
                        CheckedProtocolNodeKeys {
                            clause: Some(members.clause),
                            nodes: members.nodes,
                            channels: members.channels,
                            roles: members.roles,
                            compensations: members.compensations,
                            terminal: members.terminal,
                            binders: members.binders,
                            expressions: members.expressions,
                        },
                    )
                })
                .collect(),
            temporal_clauses,
            hyper_nodes,
            step_relations: Vec::new(),
            state_graph_claims: state_graph_nodes,
            hybrid_models: checked_hybrid_models,
            hybrid_claims: checked_hybrid_claim_list,
            probabilistic: probabilistic_model,
            task_sets: checked_task_sets,
            operation_frames,
            model_objects,
            model_owners,
            model_non_owners,
            owner_types,
            object_types,
            object_nodes,
            model_populations,
            population_bounds,
            population_keys,
            declaration_keys,
            attempt_spans: regions.attempt_spans,
            abstraction: None,
        };
        // FR-304: the unit's `abstraction` declarations check to the one
        // relation the package carries.
        match graph.check_declared_abstraction() {
            Ok(relation) => graph.abstraction = relation,
            Err(found) => {
                return Err(found
                    .into_iter()
                    .map(|(index, refusal)| CheckRefusal {
                        location: graph.abstraction_location(index),
                        cause: CheckCause::Abstraction(Box::new(refusal)),
                    })
                    .collect());
            }
        }
        finish_step_relations(&mut graph, checked_relations, lowered_relations)?;
        Ok(graph)
    }
}

/// QSL-552: finalize FR-180 claims after the checked abstraction relation is
/// known, because the implementation reference participates in claim
/// identity and the abstraction node is keyed from the assembled graph.
fn finish_step_relations(
    graph: &mut CheckedGraph,
    checked: Vec<hyper_clause::CheckedModelRelation>,
    lowered: Vec<lowering::LoweredStepRelation>,
) -> Result<(), Vec<CheckRefusal>> {
    if checked.len() != lowered.len() {
        return Err(vec![CheckRefusal {
            location: root(Origin::Expression),
            cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
        }]);
    }
    let mut relations = Vec::with_capacity(checked.len());
    // The relations' records, ranked into the package's once at the end;
    // one at an occurrence the package already keys is a fault there.
    let mut added = std::collections::HashMap::new();
    for (index, (relation, lowered)) in checked.into_iter().zip(lowered).enumerate() {
        let location = hyper_clause::lowering::hyper_location(
            index,
            hyper_clause::lowering::Site::Relation(index),
        );
        let abstraction = graph.abstraction.as_ref();
        let has_bound_frame = relation.executions().iter().any(|(_, operation)| {
            abstraction.is_some_and(|relation| {
                relation
                    .get(&AbstractionKey::Frame(operation.clone()))
                    .is_some()
            })
        });
        let abstraction_node = if has_bound_frame {
            let relation = abstraction.ok_or_else(|| {
                vec![CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                        quire_semantic_value::catalog::InternalFault::new(
                            "S4",
                            "bound-step-relation-has-abstraction",
                        ),
                    ))),
                }]
            })?;
            Some(
                graph
                    .abstraction_nodes(relation)
                    .map_err(|_| {
                        vec![CheckRefusal {
                            location: location.clone(),
                            cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                                quire_semantic_value::catalog::InternalFault::new(
                                    "S4",
                                    "step-relation-abstraction-node-keyed",
                                ),
                            ))),
                        }]
                    })?
                    .relation()
                    .key(),
            )
        } else {
            None
        };
        let boolean = lowered.boolean.clone();
        let (node, code_roots) = lowering::finish_step_relation(
            lowered,
            &relation,
            abstraction,
            abstraction_node,
            graph.package_name(),
        )
        .map_err(|refusal| vec![refusal])?;
        let key = node.key();
        graph.semantic_graph.insert_finalized(node).map_err(|_| {
            vec![CheckRefusal {
                location: location.clone(),
                cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
            }]
        })?;
        graph.occurrences.record(
            key.clone(),
            family::OccurrenceRole::Declaration,
            location.clone(),
        );
        let claim_origin =
            graph
                .occurrences
                .record(key.clone(), family::OccurrenceRole::Claim, location.clone());
        let occurrence = OccurrenceKey::new(claims::wire(key.clone()), claim_origin);
        let requirements = relation
            .model_claim()
            .requirements(claims::wire(key.clone()), &[], &[], &[])
            .map_err(|_| {
                vec![CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                }]
            })?;
        let record =
            claims::RequirementRecord::model_claim(requirements, claims::wire(boolean.clone()));
        if added.insert(occurrence.clone(), record).is_some() {
            return Err(vec![CheckRefusal {
                location: root(Origin::Expression),
                cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
            }]);
        }
        if let Some(code_claim) =
            abstraction.and_then(|abstraction| relation.code_claim(abstraction))
        {
            let roots: Vec<_> = code_roots.iter().cloned().map(claims::wire).collect();
            let code_requirements = code_claim.requirements(&roots).map_err(|_| {
                vec![CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                }]
            })?;
            let code_origin = graph.occurrences.record(
                key.clone(),
                family::OccurrenceRole::Claim,
                location.clone(),
            );
            let code_occurrence = OccurrenceKey::new(claims::wire(key.clone()), code_origin);
            let code_record = claims::RequirementRecord::unguarded(
                code_requirements,
                ValueType::Boolean,
                claims::wire(boolean),
                references::ModelReferences::default(),
            );
            if added.insert(code_occurrence, code_record).is_some() {
                return Err(vec![CheckRefusal {
                    location: root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
                }]);
            }
        }
        relations.push(relation.assembled_at(key, occurrence));
    }
    graph.step_relations = relations;
    graph.requirements = std::mem::take(&mut graph.requirements)
        .extended(added)
        .map_err(|_| {
            vec![CheckRefusal {
                location: root(Origin::Expression),
                cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
            }]
        })?;
    Ok(())
}

/// `found` with the refusals a protocol subject kept for its clause's check.
/// Add the function calls written in a temporal model-context claim. A helper
/// is typed through the StateModel family only when one of these calls (or a
/// transitive helper call) reaches it.
fn add_temporal_model_calls(form: &qsl_forms::TemporalClauseForm, calls: &mut Vec<String>) {
    add_activation_model_calls(&form.activation, calls);
    for capture in &form.captures {
        calls.extend(layer::called_functions(&capture.value.expression));
    }
    add_temporal_formula_model_calls(&form.formula, calls);
}

fn add_activation_model_calls(activation: &qsl_forms::ActivationForm, calls: &mut Vec<String>) {
    if let qsl_forms::ActivationForm::Each {
        when: Some(when), ..
    } = activation
    {
        calls.extend(layer::called_functions(&when.expression));
    }
}

fn add_temporal_formula_model_calls(
    formula: &qsl_forms::TemporalFormulaForm,
    calls: &mut Vec<String>,
) {
    for (_, node) in formula.iter() {
        if let qsl_forms::TemporalNodeForm::Holds { condition, .. } = node {
            calls.extend(layer::called_functions(&condition.expression));
        }
    }
}

fn add_measure_model_calls(measure: &qsl_forms::MeasureForm, calls: &mut Vec<String>) {
    match measure {
        qsl_forms::MeasureForm::Accumulate { window, .. }
        | qsl_forms::MeasureForm::Steps { window, .. }
        | qsl_forms::MeasureForm::Elapsed { window, .. } => {
            calls.extend(layer::called_functions(&window.from.expression));
            calls.extend(layer::called_functions(&window.until.expression));
        }
        qsl_forms::MeasureForm::Fraction(fraction) => {
            calls.extend(layer::called_functions(&fraction.predicate.expression));
        }
    }
}

fn add_probabilistic_body_model_calls(
    body: &qsl_forms::ProbabilisticBody,
    calls: &mut Vec<String>,
) {
    match body {
        qsl_forms::ProbabilisticBody::Probability { event, .. } => match event {
            qsl_forms::EventForm::Temporal(formula) => {
                add_temporal_formula_model_calls(formula, calls);
            }
            qsl_forms::EventForm::Compare {
                measure, threshold, ..
            } => {
                add_measure_model_calls(measure, calls);
                calls.extend(layer::called_functions(&threshold.expression));
            }
        },
        qsl_forms::ProbabilisticBody::Quantile {
            measure, threshold, ..
        } => {
            add_measure_model_calls(measure, calls);
            calls.extend(layer::called_functions(&threshold.expression));
        }
        qsl_forms::ProbabilisticBody::MeanOfFraction { fraction, .. } => {
            calls.extend(layer::called_functions(&fraction.predicate.expression));
        }
        qsl_forms::ProbabilisticBody::LongRunFraction { predicate, .. } => {
            calls.extend(layer::called_functions(&predicate.expression))
        }
        qsl_forms::ProbabilisticBody::ExpectedUntil {
            target, threshold, ..
        } => {
            calls.extend(layer::called_functions(&target.expression));
            calls.extend(layer::called_functions(&threshold.expression));
        }
    }
}

fn with_deferred(mut found: Vec<CheckRefusal>, deferred: &[CheckRefusal]) -> Vec<CheckRefusal> {
    found.extend_from_slice(deferred);
    found
}

/// Each `model` alias's object types by artifact id (the `T` of
/// `ix://<package>/T`), and every object type's effective identity by
/// declaration key: the tables field selection and population keys read.
fn model_object_tables(models: &[AdmittedModel]) -> (ModelTypeTable, TypeIdentities) {
    type_tables(models, |model| Box::new(model.object_types()))
}

/// FR-115, FR-121: the same tables over every operation owner (an object
/// type, a process or a state machine): the tables
/// [`CheckedGraph::resolve_operation`] and [`CheckedGraph::operation_frame`]
/// read. The artifact id is derived once here, so a selection resolves by
/// map lookup.
fn model_owner_tables(models: &[AdmittedModel]) -> (ModelTypeTable, TypeIdentities) {
    type_tables(models, |model| Box::new(model.operation_owners()))
}

type ModelTypeTable = BTreeMap<String, BTreeMap<Identifier, quire_checked::key::DeclarationKey>>;
type TypeIdentities = BTreeMap<quire_checked::key::DeclarationKey, quire_exact::EffectiveId>;

/// The tables of the declarations `select` picks from each admitted model.
fn type_tables<'m>(
    models: &'m [AdmittedModel],
    select: impl Fn(
        &'m AdmittedModel,
    ) -> Box<
        dyn Iterator<
                Item = (
                    &'m quire_checked::key::DeclarationKey,
                    quire_exact::EffectiveId,
                ),
            > + 'm,
    >,
) -> (ModelTypeTable, TypeIdentities) {
    let mut by_alias = BTreeMap::new();
    let mut identities = BTreeMap::new();
    for model in models {
        let mut objects = BTreeMap::new();
        for (key, id) in select(model) {
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

/// FR-116, FR-121: each `model` alias's top-level declarations that are not
/// operation owners, by artifact id.
fn model_non_owner_tables(
    models: &[AdmittedModel],
) -> BTreeMap<String, std::collections::BTreeSet<Identifier>> {
    models
        .iter()
        .filter_map(|model| {
            let alias = model.alias()?;
            let names = model
                .non_owner_declarations()
                .filter_map(|key| {
                    crate::model::intake::type_identity_segment(&key.package, &key.node)
                        .and_then(|artifact| Identifier::new(artifact).ok())
                })
                .collect();
            Some((alias.to_owned(), names))
        })
        .collect()
}

/// ADR-012 §15.7: each `model` alias's population declarations by artifact
/// id (the `P` of `ix://<package>/P`), each with the canonical member type
/// and ordinal [`AdmittedModel::population_domains`] gives it -- the same
/// iterator a requirement record's population domain comes from.
fn model_population_tables(
    models: &[AdmittedModel],
) -> BTreeMap<String, BTreeMap<Identifier, state_clause::PopulationDomain>> {
    models
        .iter()
        .filter_map(|model| {
            let populations = model
                .population_domains()
                .filter_map(|(ordinal, population, object)| {
                    let artifact = crate::model::intake::type_identity_segment(
                        &population.key.package,
                        &population.key.node,
                    )
                    .and_then(|artifact| Identifier::new(artifact).ok())?;
                    let maximum = model
                        .population_bound(&population.key)
                        .map(|bound| bound.maximum);
                    Some((
                        artifact,
                        state_clause::PopulationDomain {
                            object,
                            ordinal,
                            maximum,
                        },
                    ))
                })
                .collect();
            Some((model.alias()?.to_owned(), populations))
        })
        .collect()
}

/// One `bound` declaration as checked (FR-380): the population it bounds
/// and its authored maximum. The bound is part of the population's meaning
/// (ADR-014 B-1), never a resource limit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedPopulationBound {
    /// The declared label.
    pub label: String,
    /// The alias of the `model` declaration whose population it bounds.
    pub model: String,
    /// The population's artifact id, the `P` of `M::P`.
    pub population: Identifier,
    /// The inclusive maximum object count `N`.
    pub maximum: u64,
}

/// FR-380: each `bound` declaration the admitted models hold, ascending by
/// label (a label names one bound, so the order depends on no declaration
/// order).
fn checked_population_bounds(models: &[AdmittedModel]) -> Vec<CheckedPopulationBound> {
    let mut bounds: Vec<CheckedPopulationBound> = models
        .iter()
        .flat_map(|model| {
            model.population_bounds().filter_map(|bound| {
                let population = crate::model::intake::type_identity_segment(
                    &bound.population.package,
                    &bound.population.node,
                )
                .and_then(|artifact| Identifier::new(artifact).ok())?;
                Some(CheckedPopulationBound {
                    label: bound.label.clone(),
                    model: model.alias()?.to_owned(),
                    population,
                    maximum: bound.maximum,
                })
            })
        })
        .collect();
    bounds.sort_by(|left, right| left.label.cmp(&right.label));
    bounds
}

/// FR-304: each admitted population's declaration key, with the population
/// domain [`AdmittedModel::population_domains`] gives it.
fn population_key_table(
    models: &[AdmittedModel],
) -> BTreeMap<quire_checked::key::DeclarationKey, state_clause::PopulationDomain> {
    models
        .iter()
        .flat_map(|model| {
            model
                .population_domains()
                .map(|(ordinal, population, object)| {
                    let maximum = model
                        .population_bound(&population.key)
                        .map(|bound| bound.maximum);
                    (
                        population.key.clone(),
                        state_clause::PopulationDomain {
                            object,
                            ordinal,
                            maximum,
                        },
                    )
                })
        })
        .collect()
}

/// The `model`/`object_type` node of the object type `object`.
fn object_node_of(
    object_types: &BTreeMap<quire_checked::key::DeclarationKey, quire_exact::EffectiveId>,
    object_nodes: &BTreeMap<quire_checked::key::DeclarationKey, quire_exact::NodeKey>,
    object: quire_exact::EffectiveId,
) -> Option<quire_exact::NodeKey> {
    object_types
        .iter()
        .find_map(|(declaration, id)| (id == &object).then_some(declaration))
        .and_then(|declaration| object_nodes.get(declaration))
        .cloned()
}

/// ADR-012 §15.7: the domain key of `domain` from the admitted models'
/// object types and their `model`/`object_type` nodes: the one place a
/// checked graph's `population_key` and the emitted `temporal-satisfaction`
/// record build it.
fn population_domain_key(
    object_types: &BTreeMap<quire_checked::key::DeclarationKey, quire_exact::EffectiveId>,
    object_nodes: &BTreeMap<quire_checked::key::DeclarationKey, quire_exact::NodeKey>,
    domain: state_clause::PopulationDomain,
) -> Result<qsl_foundation::bound::DomainKey, quire_semantic_value::catalog::InternalFault> {
    let node =
        object_node_of(object_types, object_nodes, domain.object.clone()).ok_or_else(|| {
            quire_semantic_value::catalog::InternalFault::new(
                "check.population-domain",
                "member-type-has-a-model-node",
            )
        })?;
    state_clause::population_key(&domain, node)
}

impl CheckedGraph {
    /// Checked time sources of this package, in declaration order.
    pub fn time_sources(&self) -> &[ModelTimeSource] {
        &self.time_sources
    }

    /// FR-251: the exact scale of a checked time unit to seconds, or the
    /// checked reason the name is not a linear time unit.
    pub fn time_unit_scale(&self, unit: &str) -> Result<&quire_exact::Rational, TimeUnitCause> {
        self.time_units.scale(unit)
    }
    /// FR-380: every `bound` declaration of the unit, ascending by label.
    pub fn population_bounds(&self) -> &[CheckedPopulationBound] {
        &self.population_bounds
    }

    /// ADR-011 §2.4: the selection of each domain package this package was
    /// checked against, ascending by identity: the lock's
    /// `model_selections`.
    pub fn model_selections(&self) -> &[DomainPackageRef] {
        &self.model_selections
    }

    /// The identity of the domain package the model `model` selects, where
    /// `model` names it as [`ModelTimeSource::model`] and
    /// [`CheckedTerminal::model`] do: the alias of the `model` declaration
    /// that selects it, or the package identity when no alias names it.
    /// `None` for a name no selection carries.
    pub fn model_package_identity(&self, model: &str) -> Option<&str> {
        self.model_names
            .get(model)
            .map(abstraction::ModelNames::package)
            .or_else(|| {
                self.model_selections
                    .iter()
                    .find(|selection| selection.identity == model)
                    .map(|selection| selection.identity.as_str())
            })
    }

    /// QSpec FR-322: each selection with its `model_root` node key, in
    /// `model_selections` order.
    pub fn model_roots(&self) -> impl Iterator<Item = (&DomainPackageRef, quire_exact::NodeKey)> {
        self.model_selections
            .iter()
            .zip(self.model_roots.iter().cloned())
    }

    /// FR-110: the header-selectable layers the unit's header profiles
    /// resolved to, ascending. The lock's `definition_selections` hold the
    /// `requires` closure of these.
    pub fn header_layers(&self) -> &[crate::value::layer::HeaderLayer] {
        &self.header_layers
    }

    /// FR-001: the `RawSourceRef` of the unit this package was checked
    /// from, the lock's `sources` entry (QSpec FR-322 `PackageLock`).
    pub fn source(&self) -> &qsl_foundation::source::provenance::RawSourceRef {
        &self.source
    }

    /// The package manifest's top-level name this package was checked under:
    /// the `package` of every structural node id of the graph.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// The lowered nodes, for a test that re-keys one after the nodes bound
    /// to its old key were minted (FR-218-AC-9).
    #[cfg(any(test, feature = "test-support"))]
    pub fn semantic_graph_mut(&mut self) -> &mut SemanticGraph {
        &mut self.semantic_graph
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
    ) -> Option<&quire_checked::key::DeclarationKey> {
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
        let typer = Typer::new(
            &self.scope,
            &self.functions.signatures,
            limits,
            &mut nodes,
            clause_kind,
        );
        let mut meter = quire_exact::Meter::new(family::SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = crate::family::DiagnosticSink::default();
        let mut scopes = crate::family::ScopeStack::default();
        let stage_limits = crate::family::StageLimits {
            input_bytes: limits.input_bytes(),
            node_count: limits.nodes(),
        };
        let mut cx = crate::family::CheckContext::new(
            &self.scope,
            stage_limits,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let mut typer = typer.with_context(cx.reborrow(&self.scope));
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

    /// FR-165: the checked state-graph claim declared `name`.
    pub fn state_graph_claim(&self, name: &str) -> Option<&CheckedStateGraphClaim> {
        self.state_graph_claims
            .iter()
            .find(|claim| claim.name() == name)
    }

    /// FR-165: every checked state-graph claim, in declaration order.
    pub fn state_graph_claims(&self) -> &[CheckedStateGraphClaim] {
        &self.state_graph_claims
    }

    /// FR-447: every checked hybrid model, in declaration order.
    pub fn hybrid_models(&self) -> &[CheckedHybridModel] {
        &self.hybrid_models
    }

    /// FR-447: every checked hybrid claim, in declaration order.
    pub fn hybrid_claims(&self) -> &[CheckedHybridClaim] {
        &self.hybrid_claims
    }

    /// FR-185: the package's checked random parameters, rewards and
    /// workloads.
    pub fn probabilistic_model(&self) -> &CheckedProbabilisticModel {
        &self.probabilistic
    }

    /// QSL FR-246: the package's checked task sets and the schedulability
    /// and response claims over them.
    pub fn task_sets(&self) -> &CheckedTaskSets {
        &self.task_sets
    }

    /// FR-366: every checked type-level invariant.
    pub fn type_invariants(&self) -> &[CheckedTypeInvariant] {
        &self.type_invariants
    }

    /// FR-124: the intended terminal states of each selected model, by alias
    /// in ascending order.
    pub fn terminals(&self) -> &[CheckedTerminal] {
        &self.terminals
    }

    /// FR-230: the checked timed model of each time source, in time-source
    /// order.
    pub fn timed_models(&self) -> &[TimedModel] {
        &self.timed_models
    }

    /// FR-230: the checked timed model of the model alias `model`.
    pub fn timed_model(&self, model: &str) -> Option<&TimedModel> {
        self.timed_models
            .iter()
            .find(|timed| timed.model() == model)
    }

    /// FR-124: the intended terminal states of the model `alias`; `None` when
    /// no selected model has that alias.
    pub fn terminal(&self, alias: &str) -> Option<&CheckedTerminal> {
        self.terminals
            .iter()
            .find(|terminal| terminal.model == alias)
    }

    /// FR-135 to FR-137: every checked refinement, in declaration order.
    pub fn refinements(&self) -> &[CheckedRefinement] {
        &self.refinements
    }

    /// FR-113: every checked protocol declaration, in declaration order.
    pub fn protocols(&self) -> &[CheckedProtocol] {
        &self.protocols
    }

    /// FR-488: every emitted hyper clause over `behaviours`, in declaration
    /// order, each with its `claim` occurrence.
    pub fn hyper_clauses(&self) -> &[hyper_clause::CheckedHyperClause] {
        &self.hyper_nodes
    }

    /// FR-180: every admitted `relation` declaration, in source order, with
    /// its keyed V2 step-relation claim identity.
    pub fn step_relations(&self) -> &[hyper_clause::CheckedModelRelation] {
        &self.step_relations
    }

    /// FR-337: every emitted temporal clause, in declaration order.
    pub fn temporal_clauses(&self) -> &[TemporalClauseNode] {
        &self.temporal_clauses
    }

    /// FR-113: the checked protocol declared `name`, or `None`.
    pub fn protocol(&self, name: &str) -> Option<&CheckedProtocol> {
        self.protocols.iter().find(|protocol| protocol.name == name)
    }

    /// FR-390's keyed protocol member identities, read by the selected
    /// protocol's declaration name. The mapping contains no expression
    /// bodies; evaluation resolves those from this graph's checked protocol.
    pub fn protocol_node_keys(&self, name: &str) -> Option<&CheckedProtocolNodeKeys> {
        self.protocol_node_keys.get(name)
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
        let object = self.model_owners.get(model.as_str())?.get(object)?;
        Some(OperationSelection {
            object: object.clone(),
            operation: operation.clone(),
        })
    }

    /// FR-116, FR-121: whether `object` names a declaration of the `model`
    /// alias's package that is no operation owner (a value type, an enum, a
    /// record, ...), as against naming nothing or an object type, a process
    /// or a state machine. QSpec FR-323 refuses such a `frame_operation`
    /// `malformed-declaration`.
    pub fn names_non_owner_declaration(&self, model: &Identifier, object: &Identifier) -> bool {
        self.model_non_owners
            .get(model.as_str())
            .is_some_and(|names| names.contains(object))
    }

    /// FR-121: the operation owner `declaring` (the declaring type of a
    /// [`ClauseOperation`], found through the effective view's AD-006
    /// original declaration: an object type, a process or a state machine)
    /// under its artifact id in the domain package the `model` alias binds.
    /// `None` when the alias binds no package, or `declaring` is no
    /// operation owner of that package.
    pub fn declaring_object(
        &self,
        model: &Identifier,
        declaring: quire_exact::EffectiveId,
    ) -> Option<&Identifier> {
        self.model_owners
            .get(model.as_str())?
            .iter()
            .find_map(|(artifact, declaration)| {
                (self.owner_types.get(declaration) == Some(&declaring)).then_some(artifact)
            })
    }

    /// QSpec FR-331: the names of the object type `owner`'s own fields in
    /// ascending UTF-8 byte order -- its declared data attributes and the
    /// clock-count fields its `time` declarations stand for, one sequence
    /// (a clock-count field is one of the declaring type's own fields).
    /// `None` when `owner` is not an admitted object type.
    pub fn own_field_names(&self, owner: &quire_exact::EffectiveId) -> Option<Vec<String>> {
        let mut names: Vec<String> = self
            .scope
            .types()
            .object_type(owner.clone())?
            .attributes()
            .iter()
            .map(|field| field.name().to_owned())
            .collect();
        for timed in &self.timed_models {
            for clock in timed.clocks() {
                if clock.owner() != *owner {
                    continue;
                }
                if let Some(PlaceStep::Field(name)) = clock.path().first() {
                    names.push(name.clone());
                }
            }
        }
        // `str`'s `Ord` is UTF-8 byte order.
        names.sort_unstable();
        names.dedup();
        Some(names)
    }

    /// QSpec FR-331: the ordinal `f` of the own field `name` of `owner` in
    /// [`Self::own_field_names`], the `f` of the member path `[0, f]`.
    pub fn own_field_ordinal(&self, owner: &quire_exact::EffectiveId, name: &str) -> Option<usize> {
        self.own_field_names(owner)?
            .binary_search_by(|own| own.as_str().cmp(name))
            .ok()
    }

    /// ADR-012 §15.4: the domain key of the state field `field` of the
    /// object type `object` of the `model` alias -- the object type node
    /// that *declares* the field, and path `[0, f]`, `f` the field's ordinal
    /// among that type's own field declarations in ascending field-name
    /// UTF-8 byte order. `field` resolves in `object`'s effective attribute
    /// set, so an inherited field is keyed under its declaring type, never
    /// under the subtype it was named through, and reordering declarations
    /// in source shifts no key. `Ok(None)` when the alias, the object type
    /// or the field does not resolve.
    pub fn field_domain(
        &self,
        model: &Identifier,
        object: &Identifier,
        field: &Identifier,
    ) -> Result<
        Option<qsl_foundation::bound::DomainKey>,
        quire_semantic_value::catalog::InternalFault,
    > {
        let Some(context) = self
            .model_objects
            .get(model.as_str())
            .and_then(|objects| objects.get(object))
            .and_then(|declaration| self.object_types.get(declaration))
        else {
            return Ok(None);
        };
        let types = self.scope.types();
        let Some(attribute) = types.attribute(context.clone(), field.as_str()) else {
            return Ok(None);
        };
        let fault = |invariant| {
            quire_semantic_value::catalog::InternalFault::new("check.field-domain", invariant)
        };
        let declaring = attribute.owner();
        let node = self
            .object_types
            .iter()
            .find_map(|(declaration, id)| (id == &declaring).then_some(declaration))
            .and_then(|declaration| self.object_nodes.get(declaration))
            .ok_or_else(|| fault("declaring-type-has-a-model-node"))?;
        let position = self
            .own_field_ordinal(&declaring, attribute.field().name())
            .ok_or_else(|| fault("field-is-declared-by-its-owner"))?;
        let ordinal = u32::try_from(position).map_err(|_| fault("field-ordinal-past-u32"))?;
        Ok(Some(qsl_foundation::bound::DomainKey::Node {
            node: claims::wire(node.clone()),
            path: vec![0, ordinal],
        }))
    }

    /// ADR-012 §15.7: the domain key of the population `population` of the
    /// `model` alias -- `DomainKey::Population` with its canonical member
    /// type's `model`/`object_type` node and its ordinal among the
    /// package's population declarations in ascending `DeclarationKey`
    /// order, built by the same `population_key` a requirement record's
    /// population domain is. `Ok(None)` when the alias or the population
    /// does not resolve.
    pub fn population_domain(
        &self,
        model: &Identifier,
        population: &Identifier,
    ) -> Result<
        Option<qsl_foundation::bound::DomainKey>,
        quire_semantic_value::catalog::InternalFault,
    > {
        let Some(domain) = self
            .model_populations
            .get(model.as_str())
            .and_then(|populations| populations.get(population))
        else {
            return Ok(None);
        };
        self.population_key(domain.clone()).map(Some)
    }

    /// The declaration of the object type `id`, when the package admits one.
    pub fn object_type_key(
        &self,
        id: quire_exact::EffectiveId,
    ) -> Option<&quire_checked::key::DeclarationKey> {
        self.object_types
            .iter()
            .find_map(|(key, known)| (known == &id).then_some(key))
    }

    /// ADR-012 §15.7: the domain key of `domain`, a population a checked
    /// clause ranges over ([`TemporalClaim::populations`]): the one key
    /// [`Self::population_domain`] and a requirement record build.
    pub fn population_key(
        &self,
        domain: state_clause::PopulationDomain,
    ) -> Result<qsl_foundation::bound::DomainKey, quire_semantic_value::catalog::InternalFault>
    {
        population_domain_key(&self.object_types, &self.object_nodes, domain)
    }

    /// FR-115: the frame of the operation `selection` names, and its owner's
    /// identity. The operation resolves in the owner's
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
        let context = self.owner_types.get(&selection.object)?.clone();
        let crate::model::operation::OperationLookup::Declared {
            declaring,
            operation,
        } = self.scope.operations().resolve(
            self.scope.types(),
            context.clone(),
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

    /// FR-116: whether the operation `selection` names resolves to no single
    /// operation because two supertypes of its owner expose that name
    /// and neither is more derived than the other (QSpec FR-342
    /// `ambiguous_declaration`).
    pub fn operation_is_ambiguous(&self, selection: &OperationSelection) -> bool {
        let Some(context) = self.owner_types.get(&selection.object) else {
            return false;
        };
        matches!(
            self.scope.operations().resolve(
                self.scope.types(),
                context.clone(),
                selection.operation.as_str(),
            ),
            crate::model::operation::OperationLookup::Ambiguous(_)
        )
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

    /// QSpec FR-485 "Forms": the operation frame whose
    /// `state`/`operation_anchor` node identity is `anchor`, the node a
    /// `frame` item's claim names.
    pub fn operation_frame_by_anchor(
        &self,
        anchor: quire_exact::NodeKey,
    ) -> Option<&CheckedOperationFrame> {
        self.operation_frames
            .iter()
            .find(|operation| operation.anchor == anchor)
    }

    /// The `state`/`operation_anchor` node of the operation `name` that
    /// `declaring` declares, when a clause or attempt of the unit names it.
    pub fn operation_anchor_of(
        &self,
        declaring: quire_exact::EffectiveId,
        name: &str,
    ) -> Option<quire_exact::NodeKey> {
        self.operation_frames
            .iter()
            .find(|frame| {
                frame.operation.declaring == declaring && frame.operation.declaration.name() == name
            })
            .map(|frame| frame.anchor.clone())
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

    /// Whether the package declares a function named `name`, including a
    /// function `check` synthesized from an operation `body` or a `pre`
    /// clause, which no plain call can name (FR-395 "Broken ownership").
    pub fn declares_function(&self, name: &str) -> bool {
        self.function(name).is_some()
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
        self.function(name)
            .map(|(_, function)| function.identity.clone())
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
    pub fn requirements(&self) -> &RequirementMap {
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

    /// ADR-015 D-5: the signature of the function callable by name with
    /// structural identity `node`, found among the identities this graph
    /// holds, with its function index in this graph, which the imported
    /// call records so evaluation reads it by [`Self::function_state`].
    /// `None` when no such function is declared.
    pub(crate) fn imported_function(
        &self,
        node: &quire_exact::NodeKey,
    ) -> Option<(usize, &Signature)> {
        let position = *self.functions.by_identity.get(node)?;
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
                identity: function.identity.clone(),
                parameters: &signature.parameters,
                result: &signature.result,
            })
    }

    /// The function callable by name whose minted identity is `identity`
    /// (`callable_by_name` only: a crate-internal FR-151 synthesized dispatch
    /// candidate is never reachable this way, TC-196 D07). The accessor the
    /// evaluation interface's reference adapter admits a callable through
    /// (FR-401).
    pub fn callable_by_identity(
        &self,
        identity: quire_exact::NodeKey,
    ) -> Option<CallableFunction<'_>> {
        self.functions
            .with_identity(identity)
            .filter(|(signature, _)| signature.callable_by_name)
            .map(|(signature, function)| CallableFunction {
                identity: function.identity.clone(),
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

/// FR-096: the sites a protocol's nodes are located at. Each capture binder
/// resolves to the span of its name and its initializer, like the activation
/// guard, to its own expression tree. Each `run` control
/// and the `finish` node resolve to the span of their declared name, and the
/// `finish` constraint's expression tree to its own spans.
fn register_protocol_sites(
    protocol: &qsl_forms::ProtocolDeclarationForm,
    checked: &protocol_clause::CheckedProtocol,
    type_spans: &mut BTreeMap<String, qsl_foundation::Span>,
    site_spans: &mut BTreeMap<String, qsl_forms::ExpressionSpans>,
) {
    use lowering::{protocol_site, ProtocolSite};
    let name = protocol.name.name.as_str();
    if let qsl_forms::ActivationForm::Each {
        when: Some(guard), ..
    } = &protocol.activation
    {
        site_spans.insert(
            protocol_site(name, ProtocolSite::ActivationGuard),
            guard.spans.clone(),
        );
    }
    for capture in &protocol.captures {
        let capture_name = capture.parameter.name.name.as_str();
        type_spans.insert(
            protocol_site(name, ProtocolSite::Capture(capture_name)),
            capture.parameter.name.span,
        );
        site_spans.insert(
            protocol_site(name, ProtocolSite::CaptureInitializer(capture_name)),
            capture.value.spans.clone(),
        );
    }
    for role in &protocol.roles {
        type_spans.insert(
            protocol_site(name, ProtocolSite::Role(&role.name.name)),
            role.name.span,
        );
    }
    for role in &protocol.roles {
        if let Some(qsl_forms::RoleLifetime::Until(until)) = role
            .replicated
            .as_ref()
            .map(|replicated| &replicated.lifetime)
        {
            site_spans.insert(
                protocol_site(name, ProtocolSite::RoleUntil(&role.name.name)),
                until.spans.clone(),
            );
        }
    }
    for channel in &protocol.channels {
        type_spans.insert(
            protocol_site(name, ProtocolSite::Channel(&channel.name.name)),
            channel.span,
        );
        if let qsl_forms::ChannelOrdering::Fifo { key, .. } = &channel.ordering.value {
            site_spans.insert(
                protocol_site(name, ProtocolSite::ChannelKey(&channel.name.name)),
                key.spans.clone(),
            );
        }
    }
    for template in &protocol.compensations {
        let template_name = template.name.name.as_str();
        type_spans.insert(
            protocol_site(name, ProtocolSite::Compensation(template_name)),
            template.name.span,
        );
        for (index, capture) in template.registration_captures.iter().enumerate() {
            site_spans.insert(
                protocol_site(
                    name,
                    ProtocolSite::CompensationBody(template_name, "registration-capture", index),
                ),
                capture.value.spans.clone(),
            );
        }
        site_spans.insert(
            protocol_site(
                name,
                ProtocolSite::CompensationBody(template_name, "trigger-guard", 0),
            ),
            template.guard.spans.clone(),
        );
        for (index, capture) in template.activation_captures.iter().enumerate() {
            site_spans.insert(
                protocol_site(
                    name,
                    ProtocolSite::CompensationBody(template_name, "activation-capture", index),
                ),
                capture.value.spans.clone(),
            );
        }
        site_spans.insert(
            protocol_site(
                name,
                ProtocolSite::CompensationBody(template_name, "retry", 0),
            ),
            template.retry.relation.spans.clone(),
        );
        site_spans.insert(
            protocol_site(
                name,
                ProtocolSite::CompensationBody(template_name, "recovery", 0),
            ),
            template.recover.predicate.spans.clone(),
        );
    }
    for control in &checked.controls.controls {
        if let Some(declaration) = protocol.declarations.get(control.node.0) {
            type_spans.insert(
                protocol_site(name, ProtocolSite::Control(&control.path)),
                declaration.name.span,
            );
        }
    }
    for member in &protocol.members {
        match member {
            qsl_forms::ProtocolMemberForm::Terminal(qsl_forms::TerminalForm::Any { span }) => {
                type_spans.insert(protocol_site(name, ProtocolSite::Terminal), *span);
            }
            qsl_forms::ProtocolMemberForm::Terminal(qsl_forms::TerminalForm::When {
                predicate,
                span,
            }) => {
                type_spans.insert(protocol_site(name, ProtocolSite::Terminal), *span);
                site_spans.insert(
                    protocol_site(name, ProtocolSite::TerminalBody),
                    predicate.spans.clone(),
                );
            }
            qsl_forms::ProtocolMemberForm::Scheduling(_) => {}
        }
    }
    for body in &protocol.bodies {
        if let Some(control) = checked
            .controls
            .control(protocol_clause::ProtocolNodeId(body.owner))
        {
            site_spans.insert(
                protocol_site(name, ProtocolSite::ControlBody(&control.path)),
                body.expression.spans.clone(),
            );
        }
    }
    for (node, entries) in protocol
        .choices
        .iter()
        .map(|choice| {
            (
                choice.declaration,
                choice
                    .visible
                    .iter()
                    .chain(choice.cases.iter().map(|case| &case.guard))
                    .collect::<Vec<_>>(),
            )
        })
        .chain(protocol.repeats.iter().map(|repeat| {
            (
                repeat.declaration,
                repeat
                    .visible
                    .iter()
                    .chain([&repeat.guard])
                    .chain(
                        repeat
                            .loop_proof
                            .iter()
                            .flat_map(|proof| [&proof.invariant, &proof.variant]),
                    )
                    .collect(),
            )
        }))
    {
        let Some(control) = checked
            .controls
            .control(protocol_clause::ProtocolNodeId(node))
        else {
            continue;
        };
        for entry in entries {
            if let Some(span) = protocol_slots::decision_span(entry) {
                site_spans.insert(
                    protocol_site(name, ProtocolSite::Decision(&control.path, span)),
                    entry.spans.clone(),
                );
            }
        }
    }
    let Some((finish, finish_name)) = &checked.finish else {
        return;
    };
    if let Some(declaration) = protocol.declarations.get(finish.0) {
        type_spans.insert(
            protocol_site(name, ProtocolSite::Finish(finish_name)),
            declaration.name.span,
        );
    }
    if let Some(body) = protocol.bodies.iter().find(|body| body.owner == finish.0) {
        site_spans.insert(
            protocol_site(name, ProtocolSite::FinishBody(finish_name)),
            body.expression.spans.clone(),
        );
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{Origin, Role};

    use super::*;

    /// QSL-821: `type_admission` and `pending_refusal` keep the reached
    /// ceiling, so the refinement classifier can name it; a failure that is
    /// no ceiling keeps none.
    #[test]
    fn a_type_admission_ceiling_keeps_its_limit() {
        use quire_semantic_value::declaration::{
            EnvironmentFailure, EnvironmentLimit, EnvironmentLimitKind, PendingFieldFailure,
            StorageRequest, StorageUnit,
        };
        let ceiling = || EnvironmentLimit::new(EnvironmentLimitKind::AncestorSteps, 5, 6);
        let expected = Some(Box::new(qsl_foundation::diagnostic::LimitExceeded::new(
            qsl_foundation::Setting::EnvironmentAncestorSteps,
            5,
            6,
        )));

        let CheckCause::TypeAdmission { code, limit, .. } =
            type_admission(EnvironmentFailure::Limit(ceiling()))
        else {
            panic!("expected a type admission cause");
        };
        assert_eq!(code, qsl_foundation::Code::StageLimitExceeded);
        assert_eq!(limit, expected);

        let CheckCause::TypeAdmission { limit, .. } =
            pending_refusal(PendingFieldFailure::Limit(ceiling()))
        else {
            panic!("expected a type admission cause");
        };
        assert_eq!(limit, expected);

        let CheckCause::TypeAdmission { limit, .. } = type_admission(
            EnvironmentFailure::Allocation(StorageRequest::new(8, StorageUnit::Bytes)),
        ) else {
            panic!("expected a type admission cause");
        };
        assert_eq!(limit, None);
    }

    fn declarations(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
        PackageDeclarations {
            functions,
            ..PackageDeclarations::new(
                family::fixtures::fixture_source(),
                "test-package",
                qsl_foundation::IdentityLimits::default(),
            )
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
        let key = NodeKey::new(
            "test-package".to_owned(),
            vec![quire_exact::PathSegment::Text("Flagged-handle".to_owned())],
            0,
        )
        .unwrap();
        let composite =
            CompositeDeclaration::new(key.clone(), "Flagged", CompositeShape::Record(vec![field]));
        let types = TypeEnvironment::new([composite], []).expect("one record admits cleanly");
        let graph = PackageDeclarations {
            types,
            ..PackageDeclarations::new(
                family::fixtures::fixture_source(),
                "test-package",
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .expect("one record declaration checks cleanly");

        let nodes: Vec<&CheckedTypeNode> = graph.checked_type_nodes().collect();
        assert_eq!(nodes.len(), 1, "exactly the one declared composite");
        let node = nodes[0].node();
        assert_ne!(node, key, "the caller's handle is not a node id");
        let declared = graph
            .semantic_graph()
            .node(node.clone())
            .expect("the checked type node's id is a graph node's key");
        assert_eq!(
            declared.declaration().map(|name| name[0].as_str()),
            Some("Flagged")
        );
        assert_eq!(graph.checked_type_node(node), Some(nodes[0]));
        assert!(graph.checked_type_node(key.clone()).is_none());
        assert_eq!(
            to_kernel_value_type(nodes[0]),
            quire_exact::ValueType::Composite(key)
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
        let key = NodeKey::new(
            "test-package".to_owned(),
            vec![quire_exact::PathSegment::Text("P::R-handle".to_owned())],
            0,
        )
        .unwrap();
        let composite =
            CompositeDeclaration::new(key.clone(), "P::R", CompositeShape::Record(vec![field]));
        let types = TypeEnvironment::new([composite], []).expect("one record admits cleanly");
        let graph = PackageDeclarations {
            types,
            ..PackageDeclarations::new(
                family::fixtures::fixture_source(),
                "test-package",
                qsl_foundation::IdentityLimits::default(),
            )
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
        use crate::value::semantic_node::{NodeOwner, OwnerSelection, OwnerSubject};
        use quire_exact::{NodeKey, PathSegment};
        use serde_json::json;

        let owners = OwnerSelection::new([NodeOwner::Definition(OwnerSubject::new(
            "agent-ix",
            "example-model",
            "example-model",
        ))]);
        let declaration_json = json!({
            "version": "quire.enum-declaration-node/v1",
            "owner": {"kind": "definition", "authority": "agent-ix", "identity": "example-model", "package_name": "example-model"},
            "qualified_declaration": ["Example", "Status"],
            "ordered": false,
            "members": ["Active", "Closed"],
        });
        let declaration_preimage = EnumDeclarationPreimage::from_json(declaration_json).unwrap();
        let declaration_key = NodeKey::new(
            "example-model".to_owned(),
            vec![
                PathSegment::Text("Example".to_owned()),
                PathSegment::Text("Status".to_owned()),
            ],
            0,
        )
        .unwrap();
        let declaration =
            AdmittedEnumDeclaration::admit(declaration_preimage, declaration_key.clone(), &owners)
                .unwrap();

        let members = ["Active", "Closed"]
            .into_iter()
            .map(|case| {
                let member_preimage = EnumMemberPreimage::new(declaration.key(), case).unwrap();
                let member_key = NodeKey::new(
                    "example-model".to_owned(),
                    vec![
                        PathSegment::Text("Example".to_owned()),
                        PathSegment::Text("Status".to_owned()),
                        PathSegment::Text(case.to_owned()),
                    ],
                    0,
                )
                .unwrap();
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
            ..PackageDeclarations::new(
                family::fixtures::fixture_source(),
                "test-package",
                qsl_foundation::IdentityLimits::default(),
            )
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
        assert_eq!(
            graph.checked_type_node(declaration_key.clone()),
            Some(nodes[0])
        );

        let quire_exact::ValueType::Enum(ref shape) = to_kernel_value_type(nodes[0]) else {
            panic!("the sum form must convert to ValueType::Enum");
        };
        for case in ["Active", "Closed"] {
            assert!(shape.contains(
                &crate::value::enumeration::mint_variant_id(declaration_key.clone(), case).unwrap()
            ));
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
            .occurrence(call_identity.clone(), &first)
            .expect("the first call site was recorded")
            .clone();
        let second_location = graph
            .occurrence(call_identity.clone(), &second)
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
        assert_eq!(graph.occurrence(call_identity.clone(), &third), None);

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
            renamed.occurrence(call_identity.clone(), &first).is_some(),
            "the same call identity and first occurrence key still resolve after renaming"
        );
        assert!(
            renamed.occurrence(call_identity.clone(), &second).is_some(),
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
            ..PackageDeclarations::new(
                family::fixtures::fixture_source(),
                "test-package",
                qsl_foundation::IdentityLimits::default(),
            )
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

/// FR-401-AC-3: `WrongSnapshotCause` lives in `quire-semantic-value`, and
/// `check` leaves no name for it.
///
/// ```compile_fail,E0432
/// use qsl_semantics::check::WrongSnapshotCause;
/// ```
///
/// A positive control: the surviving `CheckCause` still imports cleanly, so
/// the failure above is the removed name, not a broken module path.
///
/// ```
/// use qsl_semantics::check::CheckCause;
/// fn _use(_: Option<CheckCause>) {}
/// ```
#[cfg(doctest)]
use ix_trace_rs::trace;

#[cfg(doctest)]
#[trace("FR-401-AC-3")]
struct Fr401Ac3Doctests;
