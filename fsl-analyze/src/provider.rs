// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-280's in-process provider boundary over the same admitted analyze request.

use crate::analyze::{
    analyze_bound, AnalyzeLimits, AnalyzeRefusal, Engines, ItemCounterexample, NegotiatedItem,
    SourceProvision, SubjectBinding,
};
use filament_exact::Cancel;
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::diagnostic::{Code, StageFailure};
use fsl_package::CheckedPackage;
use fsl_replay::ToolFailure;
use fsl_replay::{DeclineCode, ProofRefusalCause, TerminalRecord};
pub use fsl_replay::{
    HybridAnswer, HybridProvider, HybridSolveRequest, HybridVerdict, OptionRefusal,
    UnsupportedConstruct,
};
use std::collections::BTreeMap;

use filament_exact::NodeKey;
use filament_semantic_value::catalog::InternalFault;
use fsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::{EmitLimits, EmittedPackage};
use fsl_replay::value_check::{Selection, ValueTarget};
use fsl_replay::{Certification, InconclusiveCause, Value};
use fsl_route::request::{ExtentClassification, ItemKind, RequirementItem};
use fsl_route::{BackendDescriptor, CandidateOutcome, ProviderOrigin, Registry};
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::Capability;
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::PackageInput;

use crate::analyze::{AnalyzeItem, Engine, EngineReport, Refusal};
use crate::claim_family::claim_family;
use crate::value_check::{
    confirm_reported, manifest, ValueCounterexample, ValueRefusal, ValueReport,
};

/// Records and reproduced counterexamples for one provider request.
#[derive(Debug)]
pub struct ProviderResults {
    /// One terminal record per requested item, in input order.
    pub records: Vec<TerminalRecord>,
    /// Exactly one reproduced counterexample for each refuted record.
    pub counterexamples: Vec<ItemCounterexample>,
}

impl ProviderResults {
    /// The FR-286 document of these results: the one [`crate::analyze::analyze_document`]
    /// writes for the same records.
    ///
    /// # Errors
    ///
    /// As [`crate::analyze::analyze_document`].
    pub fn document(&self) -> Result<fsl_replay::OutcomeDocument, fsl_replay::OutcomeItemError> {
        crate::analyze::analyze_document(&self.records, &self.counterexamples)
    }
}

/// Run the caller's negotiated items over its checked package, source bytes
/// and subject binding. This boundary chooses no engine and reads no file.
pub fn run(
    package: &CheckedPackage,
    binding: &SubjectBinding<'_>,
    provision: &SourceProvision,
    items: &[NegotiatedItem],
    engines: &Engines,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> ProviderResults {
    let run = crate::analyze::RunInput::source(package);
    match analyze_bound(
        &run,
        binding,
        provision,
        items,
        &std::collections::BTreeMap::new(),
        engines,
        limits,
        cancel,
    ) {
        Ok(staged) => {
            let outcome = staged.into_value();
            ProviderResults {
                records: outcome.records,
                counterexamples: outcome.counterexamples,
            }
        }
        Err(failure) => {
            let (cause, diagnostic) = match failure {
                StageFailure::Refused(AnalyzeRefusal::Provision(refusal)) => (
                    Some(ProofRefusalCause::IncompleteInput),
                    refusal.catalog_code(),
                ),
                StageFailure::Refused(AnalyzeRefusal::Package(refusal)) => (
                    Some(if refusal.code == Code::ResourceExhausted {
                        ProofRefusalCause::IncompleteInput
                    } else {
                        ProofRefusalCause::InvalidInput
                    }),
                    CatalogCode::new(refusal.code.as_str(), refusal.catalog_cause()),
                ),
                StageFailure::Refused(AnalyzeRefusal::Binding(failure)) => match failure {
                    fsl_semantics::model::observation::AdmissionFailure::Refused(record) => {
                        (Some(ProofRefusalCause::InvalidInput), record.catalog)
                    }
                    fsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => {
                        (Some(ProofRefusalCause::IncompleteInput), record.catalog)
                    }
                    fsl_semantics::model::observation::AdmissionFailure::Fault(fault) => {
                        (None, fault.catalog_code())
                    }
                },
                _ => (
                    None,
                    CatalogCode::new("runtime_invariant", "established-invariant-broken"),
                ),
            };
            let records = items
                .iter()
                .map(|item| match cause {
                    Some(cause) => TerminalRecord::disposition_declined(
                        item.item.request_index,
                        cause,
                        DeclineCode::Fsl(
                            Code::from_code(diagnostic.code()).unwrap_or(Code::RuntimeInvariant),
                        ),
                        diagnostic,
                    ),
                    None => TerminalRecord::disposition_failed(item.item.request_index, diagnostic),
                })
                .collect();
            ProviderResults {
                records,
                counterexamples: Vec::new(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The external provider arm (FR-281 "External provider arm").
//
// A registered decision procedure outside FSL decides a `value-validity`
// item over unbounded mathematical `Integer` operands, which no in-process
// engine decides (FR-495). `analyze` owns routing, negotiation and the routed
// items (FR-635, FSpec FR-711), so it also builds the FSpec FR-331 request a
// provider decides ([`ProviderRequest`]): after every item is routed and
// negotiated, one request per backend carries the package bytes of the
// unit's own S4 emission as the strict reader admitted them back, the items
// routed to that backend, the negotiated manifest entry and the caller's
// FSpec FR-462 limits. A process provider's caller only transports it.
//
// The provider's verdict is the provider's. FSL settles a `proved` verdict as
// `proved` (`uncertified` for a linked provider, `trusted` for a plugin) and
// an `unknown` verdict as `inconclusive`, but a `refuted` verdict is only a
// candidate violation: the counterexample is replayed through the native
// evaluator at its own integers, and the item settles `refuted` only when the
// replay reproduces the violation. Every type here is FSL's own, so FSL
// depends on no CG crate for the seam (FR-280).
// ---------------------------------------------------------------------------

/// The FSpec FR-462 limits of one provider's runs, by the names and with
/// the defaults FR-287 gives `--provider-limit`. The caller resolves them
/// once for the run as [`ProviderOptions`]; every request the run sends
/// carries them. `deadline_ms` binds every provider, as the
/// FR-331 request's `deadline_ms`; the other six are transport limits of a
/// process plugin, which a linked provider ignores.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderLimits {
    /// The maximum frame length, in either direction, in bytes.
    pub max_frame_bytes: u64,
    /// The maximum total frame bytes of one plugin run, both directions.
    pub max_total_bytes: u64,
    /// The provider deadline, in milliseconds (FR-331 `request.deadline_ms`).
    pub deadline_ms: u64,
    /// The grace period after `cancel` before the host ends the process, in
    /// milliseconds.
    pub grace_ms: u64,
    /// The standard-error bytes captured from the plugin.
    pub max_stderr_bytes: u64,
    /// The process memory limit, in bytes, where the platform provides one.
    pub max_memory_bytes: u64,
    /// The process CPU-time limit, in milliseconds, where the platform
    /// provides one.
    pub max_cpu_ms: u64,
}

impl ProviderLimits {
    /// The largest value a limit takes: the FR-331 count range.
    pub const MAX_VALUE: u64 = 9_007_199_254_740_991;

    /// The limit FR-287 names `name`, for the caller to set; `None` for a
    /// name FR-287 does not give.
    pub fn named_mut(&mut self, name: &str) -> Option<&mut u64> {
        match name {
            "max_frame_bytes" => Some(&mut self.max_frame_bytes),
            "max_total_bytes" => Some(&mut self.max_total_bytes),
            "deadline_ms" => Some(&mut self.deadline_ms),
            "grace_ms" => Some(&mut self.grace_ms),
            "max_stderr_bytes" => Some(&mut self.max_stderr_bytes),
            "max_memory_bytes" => Some(&mut self.max_memory_bytes),
            "max_cpu_ms" => Some(&mut self.max_cpu_ms),
            _ => None,
        }
    }
}

/// The provider options of one provider-routing run (FR-964): its FSpec
/// FR-462 limits, by FR-287's names and defaults. Every provider-routing
/// entry takes one and every `ProviderRequest` the run builds carries its
/// limits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProviderOptions {
    /// The run's FSpec FR-462 limits.
    pub provider_limits: ProviderLimits,
}

impl Default for ProviderLimits {
    /// FR-287's defaults: a 64 MiB frame, 1 GiB per run, a ten-minute
    /// deadline, five seconds of grace, 1 MiB of standard error, 4 GiB of
    /// memory and ten minutes of CPU time.
    fn default() -> Self {
        Self {
            max_frame_bytes: 67_108_864,
            max_total_bytes: 1_073_741_824,
            deadline_ms: 600_000,
            grace_ms: 5_000,
            max_stderr_bytes: 1_048_576,
            max_memory_bytes: 4_294_967_296,
            max_cpu_ms: 600_000,
        }
    }
}

/// The FSpec FR-331 provider request `analyze` builds for the items it
/// negotiated to one external provider (FR-635 "How a process provider runs
/// an item"). It is a typed mirror of FR-331's `request` member and
/// serializes to exactly that member: the package's v2 bytes as `package`,
/// the caller's `deadline_ms`, and the routed items, each at its zero-based
/// position in this request, with the routed backend among its candidates
/// and `backend` only when the analyze request named one. Beside the wire
/// members it holds the registry snapshot and the negotiated manifest entry
/// within it, the whole FR-462
/// limit set, and each item's claim target: what a linked provider reads to
/// build its query, and what a reported counterexample is replayed at.
pub struct ProviderRequest<'r> {
    package: &'r EmittedPackage,
    items: Vec<RequirementItem>,
    targets: Vec<ProviderTarget<'r>>,
    manifest: Vec<&'r BackendDescriptor>,
    descriptor: &'r BackendDescriptor,
    limits: ProviderLimits,
    /// The node graph of `package`, which writes each node as the
    /// document-held id those bytes give it (FSpec FR-322 "Operand index
    /// segments").
    graph: WireGraph,
    /// The package's dependency closure, each after the dependencies it
    /// imports (FSpec FR-331 `request.dependencies`).
    dependencies: Vec<fsl_replay::HybridProcessDependency>,
}

impl<'r> ProviderRequest<'r> {
    /// The package's dependency closure by identity with each package's v2
    /// bytes, empty when it imports nothing.
    pub fn dependencies(&self) -> &[fsl_replay::HybridProcessDependency] {
        &self.dependencies
    }

    /// The package's v2 bytes: those of the unit's own S4 emission that the
    /// strict reader admitted back (FR-331 `request.package`).
    pub fn package(&self) -> &'r EmittedPackage {
        self.package
    }

    /// The routed request items, in request order (FR-331 `request.items`),
    /// each at its position in this request; [`Self::targets`] keeps their
    /// order, and `analyze` joins each verdict back to its own item by it.
    pub fn items(&self) -> &[RequirementItem] {
        &self.items
    }

    /// The claim target of each item, in [`Self::items`] order.
    pub fn targets(&self) -> &[ProviderTarget<'r>] {
        &self.targets
    }

    /// The registry snapshot the items were negotiated against: every
    /// registered backend's descriptor, linked and external (FR-331
    /// `manifest`). [`Self::descriptor`] is one of them.
    pub fn manifest(&self) -> &[&'r BackendDescriptor] {
        &self.manifest
    }

    /// The negotiated manifest entry: the descriptor of the backend every
    /// item was negotiated to.
    pub fn descriptor(&self) -> &'r BackendDescriptor {
        self.descriptor
    }

    /// The backend every item was negotiated to.
    pub fn backend(&self) -> &'r BackendId {
        self.descriptor.id()
    }

    /// The caller's FSpec FR-462 limits for this provider.
    pub fn limits(&self) -> ProviderLimits {
        self.limits
    }

    /// The provider deadline: FR-331 `request.deadline_ms`.
    pub fn deadline_ms(&self) -> u64 {
        self.limits.deadline_ms
    }
}

/// The typed claim target paired with an FR-331 provider request item.
pub enum ProviderTarget<'r> {
    /// A composition: its derived premise slot keys travel with the item
    /// (FSpec FR-348-AC-3). There is nothing to replay for it.
    Composition(&'r CompositionTarget),
    /// A value-validity target whose counterexample is replayed by the value
    /// evaluator.
    Value(&'r ValueTarget<'r>),
    /// An operation-contract claim selected by FR-485.
    Contract(&'r fsl_replay::contract_check::ContractClaim),
}

/// The members of a `composition` provider item beside its claim (FSpec
/// FR-331 `premises`): the derived premise slots, each `{role, key}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionTarget {
    claim: OccurrenceKey,
    premises: Vec<serde_json::Value>,
}

impl CompositionTarget {
    /// The composition at `claim` over the derived `premises`.
    pub fn new(claim: OccurrenceKey, premises: Vec<serde_json::Value>) -> Self {
        Self { claim, premises }
    }

    /// The composition's claim occurrence.
    pub fn claim(&self) -> &OccurrenceKey {
        &self.claim
    }

    /// The derived premise slots, each `{role, key}`.
    pub fn premises(&self) -> &[serde_json::Value] {
        &self.premises
    }
}

impl ProviderTarget<'_> {
    /// The exact occurrence named by the paired provider request item.
    pub fn claim(&self) -> &OccurrenceKey {
        match self {
            Self::Value(target) => target.claim(),
            Self::Contract(target) => target.claim(),
            Self::Composition(target) => target.claim(),
        }
    }
}

impl serde::Serialize for ProviderRequest<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use base64::Engine as _;
        use serde::ser::{Error as _, SerializeMap as _};
        let items = self
            .items
            .iter()
            .zip(&self.targets)
            .map(|(item, target)| request_item_wire(&self.graph, item, target))
            .collect::<Result<Vec<_>, _>>()
            .map_err(S::Error::custom)?;
        let mut dependencies: Vec<&fsl_replay::HybridProcessDependency> =
            self.dependencies.iter().collect();
        dependencies.sort_by(|left, right| left.identity.as_bytes().cmp(right.identity.as_bytes()));
        let dependencies: Vec<serde_json::Value> = dependencies
            .into_iter()
            .map(|dependency| {
                serde_json::json!({
                    "identity": dependency.identity,
                    "package": base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .encode(&dependency.package),
                })
            })
            .collect();
        let mut map = serializer.serialize_map(Some(3 + usize::from(!dependencies.is_empty())))?;
        map.serialize_entry(
            "package",
            &base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.package.bytes()),
        )?;
        if !dependencies.is_empty() {
            map.serialize_entry("dependencies", &dependencies)?;
        }
        map.serialize_entry("deadline_ms", &self.limits.deadline_ms)?;
        map.serialize_entry("items", &items)?;
        map.end()
    }
}

/// `node`'s FR-331 `NodeId`: the document-held id the package's bytes,
/// whose node graph is `graph`, give it, each operand an index segment.
fn node_wire(graph: &WireGraph, node: &NodeKey) -> Result<serde_json::Value, serde_json::Error> {
    use serde::ser::Error as _;
    graph
        .wire_value(node)
        .map_err(|error| serde_json::Error::custom(error.to_string()))
}

/// `key`'s FR-331 `DomainKey`.
fn domain_key_wire(
    graph: &WireGraph,
    key: &DomainKey,
) -> Result<serde_json::Value, serde_json::Error> {
    Ok(match key {
        DomainKey::Node { node, path } => {
            serde_json::json!({"type": "node", "node": node_wire(graph, node)?, "path": path})
        }
        DomainKey::Population {
            member_type,
            ordinal,
        } => serde_json::json!({
            "type": "population",
            "member_type": node_wire(graph, member_type)?,
            "ordinal": ordinal,
        }),
    })
}

/// `bound`'s FR-331 `ProofBound`.
fn proof_bound_wire(
    graph: &WireGraph,
    bound: &ProofBound,
) -> Result<serde_json::Value, serde_json::Error> {
    use serde::ser::Error as _;
    let finite = match bound.bound() {
        FiniteBound::Cardinality { maximum } => {
            serde_json::json!({"type": "cardinality", "maximum": maximum})
        }
        FiniteBound::IntegerRange(interval) => serde_json::json!({
            "type": "integer-range",
            "lower": interval.lower().to_string(),
            "upper": interval.upper().to_string(),
        }),
        FiniteBound::Depth { maximum } => {
            serde_json::json!({"type": "depth", "maximum": maximum.get()})
        }
        FiniteBound::Variants { .. } => {
            return Err(serde_json::Error::custom(
                "a variants bound is a harness bound, never an FR-331 substitution",
            ));
        }
    };
    Ok(serde_json::json!({
        "domain": domain_key_wire(graph, bound.domain())?,
        "kind": bound.kind().map(DomainKind::to_wire),
        "bound": finite,
    }))
}

/// One routed item as FR-331 `RequestItem`. `request_index` is the item's
/// position in this request; `backend` is written only when the analyze
/// request named one, since the named backend is part of request identity
/// (FR-331 "Identity and validation").
fn request_item_wire(
    graph: &WireGraph,
    item: &RequirementItem,
    target: &ProviderTarget<'_>,
) -> Result<serde_json::Value, serde_json::Error> {
    use serde::ser::Error as _;
    let origin = item.occurrence().origin();
    let extent = match item.extent() {
        ExtentClassification::Bounded => serde_json::json!({
            "classification": "bounded",
            "bounds": item
                .proof_bounds()
                .iter()
                .map(|bound| proof_bound_wire(graph, bound))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        ExtentClassification::Unbounded {
            finite_bound_available,
        } => serde_json::json!({
            "classification": "unbounded",
            "domains": item
                .unbounded()
                .iter()
                .map(|(key, kind)| {
                    Ok(serde_json::json!({"domain": domain_key_wire(graph, key)?, "kind": kind.to_wire()}))
                })
                .collect::<Result<Vec<_>, serde_json::Error>>()?,
            "finite_bound_available": finite_bound_available,
        }),
    };
    let named = match item.candidates() {
        CandidateOutcome::Candidates(set) => set.requested_backend(),
        CandidateOutcome::UnknownBackend(backend) => Some(backend),
    };
    let candidates = match item.candidates() {
        CandidateOutcome::Candidates(set) => serde_json::json!({
            "type": "set",
            "backends": set
                .candidates()
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>(),
        }),
        CandidateOutcome::UnknownBackend(backend) => {
            serde_json::json!({"type": "unknown-backend", "backend": backend.as_str()})
        }
    };
    let index = u64::try_from(item.index().get())
        .map_err(|_| serde_json::Error::custom("a request index outside the FR-331 count range"))?;
    let mut wire = serde_json::json!({
        "request_index": index,
        "kind": item.kind().to_wire(),
        "claim": {
            "node": node_wire(graph, item.node())?,
            "occurrence": {"role": origin.role().as_str(), "ordinal": origin.ordinal()},
        },
        "extent": extent,
        "candidates": candidates,
    });
    if let Some(members) = wire.as_object_mut() {
        if let Some(named) = named {
            members.insert("backend".to_owned(), serde_json::json!(named.as_str()));
        }
        if !item.options().is_empty() {
            members.insert("options".to_owned(), serde_json::json!(item.options()));
        }
        if let ProviderTarget::Composition(composition) = target {
            members.insert(
                "premises".to_owned(),
                serde_json::Value::Array(composition.premises().to_vec()),
            );
        }
    }
    Ok(wire)
}

/// What a provider reports for one item. Only `Refuted` carries evidence,
/// and FSL replays it before it counts (FR-281 "External provider arm").
#[derive(Clone, Debug)]
pub enum ProviderVerdict {
    /// The provider established that the claim holds for every assignment.
    Proved,
    /// The provider reports an assignment of the claim's parameters, in
    /// parameter order, at which the claim is violated.
    Refuted {
        /// The reported assignment.
        counterexample: Vec<Value>,
    },
    /// The provider transport exceeded one of FR-287's byte limits.
    Failed {
        /// The FR-287 transport setting that was exceeded.
        limit: ProviderLimit,
        /// The typed FR-331/FR-462 failure cause.
        cause: ToolFailure,
    },
    /// The provider neither proved nor refuted the claim.
    Unknown,
}

/// The FR-287 provider transport byte limits that settle an item as failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderLimit {
    /// `max_frame_bytes`.
    MaxFrameBytes,
    /// `max_total_bytes`.
    MaxTotalBytes,
}

impl ProviderLimit {
    /// The FR-287 setting name reported by this limit.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaxFrameBytes => "max_frame_bytes",
            Self::MaxTotalBytes => "max_total_bytes",
        }
    }

    fn matches(self, cause: ToolFailure) -> bool {
        matches!(
            (self, cause),
            (Self::MaxFrameBytes, ToolFailure::FrameLength { .. })
                | (Self::MaxTotalBytes, ToolFailure::TotalBytes { .. })
        )
    }
}

/// A decision procedure outside FSL for items of its registered capability
/// kinds (FR-968).
pub trait ExternalProvider: std::fmt::Debug + Send + Sync {
    /// Decide every item of `request`, stopping early when `cancel` fires:
    /// one verdict per item, in [`ProviderRequest::items`] order. An item
    /// with no verdict, and every item of a list of another length, settles
    /// as [`ProviderVerdict::Unknown`].
    fn settle(&self, request: &ProviderRequest<'_>, cancel: &Cancel) -> Vec<ProviderVerdict>;
}

/// Why an external provider descriptor cannot be registered.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProviderRegistrationCause {
    /// The descriptor advertises no `(kind, mode)` pair.
    #[error("absent-kind")]
    AbsentKind,
    /// The descriptor advertises value validity only for bounded operands.
    #[error("value-validity-not-unbounded (modes: {modes:?})")]
    ValueValidityNotUnbounded {
        /// The FR-290 wire spellings advertised for `value-validity`.
        modes: Vec<String>,
    },
}

/// `Engine::external` could not register a provider descriptor.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{backend:?}: {cause}")]
pub struct ProviderRegistrationRefusal {
    /// The descriptor's `BackendId`.
    pub backend: BackendId,
    /// The descriptor rule that prevented registration.
    pub cause: ProviderRegistrationCause,
}

/// What the arm reports for one provider item.
pub(crate) enum ProviderReport {
    /// The provider proved the claim.
    Proved,
    /// The provider could not decide the claim.
    Unknown,
    /// The provider's assignment-shaped counterexample could not be replayed
    /// as an operation-contract counterexample.
    ReplayParity,
    /// The provider's counterexample is for an item with no replay.
    ReplayRefused(fsl_foundation::Code),
    /// The provider transport failed while enforcing a typed byte limit.
    Failed(ToolFailure),
    /// The provider's counterexample, confirmed by replay or not.
    Replayed(ValueReport),
}

/// The FSpec FR-331 request item of the `value-validity` item at `claim`,
/// at the analyze request's `index`, with its candidates under the
/// registry of `registrations` for the request's `named` backend: the
/// item of `claim`'s requirement record, or, for a `Boolean` value clause,
/// which has no record, the clause's own item over its function's
/// parameter domains (FSpec FR-290, FR-484 "Forms"). `None` when `claim`
/// is neither, or the registrations do not form a registry.
pub fn routed_item<'d>(
    package: &CheckedPackage,
    registrations: impl IntoIterator<Item = &'d BackendDescriptor>,
    index: RequestIndex,
    claim: &OccurrenceKey,
    named: Option<&BackendId>,
) -> Option<RequirementItem> {
    let mut registry = Registry::new();
    for descriptor in registrations {
        registry.register(descriptor.clone()).ok()?;
    }
    let record = fsl_route::request::items_from_requirements(
        package.graph().requirements(),
        &registry,
        named,
    )
    .into_iter()
    .find(|item| item.occurrence() == claim && item.kind() == Capability::ValueValidity);
    if let Some(record) = record {
        return Some(record.at_index(index));
    }
    let extent = crate::analyze::document::boolean_value_clause_extent(package, claim)?;
    let result_bound = package
        .graph()
        .semantic_graph()
        .node(claim.node().clone())?
        .semantic_type()?;
    Some(RequirementItem::value_clause(
        index,
        claim.clone(),
        &extent,
        result_bound,
        registry.candidates(Capability::ValueValidity, named),
    ))
}

/// The FR-331 request item for an operation-contract claim, with candidates
/// computed from the current provider registrations (FR-968).
pub(crate) fn routed_contract_item<'d>(
    package: &CheckedPackage,
    registrations: impl IntoIterator<Item = &'d BackendDescriptor>,
    index: RequestIndex,
    claim: &OccurrenceKey,
    named: Option<&BackendId>,
) -> Option<RequirementItem> {
    let mut registry = Registry::new();
    for descriptor in registrations {
        registry.register(descriptor.clone()).ok()?;
    }
    let record = fsl_route::request::items_from_requirements(
        package.graph().requirements(),
        &registry,
        named,
    )
    .into_iter()
    .find(|item| item.occurrence() == claim && item.kind() == Capability::OperationContract);
    if let Some(record) = record {
        return Some(record.at_index(index));
    }
    let extent = crate::analyze::document::boolean_value_clause_extent(package, claim)?;
    let result_bound = package
        .graph()
        .semantic_graph()
        .node(claim.node().clone())?
        .semantic_type()?
        .clone();
    Some(RequirementItem::operation_contract_clause(
        index,
        claim.clone(),
        &extent,
        result_bound,
        registry.candidates(Capability::OperationContract, named),
    ))
}

/// What a provider item is settled from: the members its request is built
/// from and the target its verdict is replayed at.
pub struct ProviderItemInput<'a> {
    /// The checked package the claim belongs to.
    pub package: &'a CheckedPackage,
    /// The domain packages the package's model selections bind (FR-056),
    /// which the strict reader admits the package's bytes against.
    pub packages: &'a PackageInput,
    /// The claim occurrence the item settles.
    pub claim: &'a OccurrenceKey,
    /// The item's FSpec FR-331 request item ([`routed_item`]).
    pub routed: Option<RequirementItem>,
    /// The ceiling on the package's emission.
    pub emit: EmitLimits,
    /// The claim's target.
    pub selection: Result<ValueTarget<'a>, Selection>,
}

/// A provider item's members, held by its [`AnalyzeItem`] until
/// [`settle_items`] sends its request.
pub(crate) struct ProviderWork<'a> {
    package: &'a CheckedPackage,
    shared: Option<Shared<'a>>,
    packages: &'a PackageInput,
    routed: Option<RequirementItem>,
    emit: EmitLimits,
    selection: ProviderSelection<'a>,
}

/// The selected target retained until an external provider's verdict settles.
enum ProviderSelection<'a> {
    Composition(CompositionTarget),
    Value(Result<ValueTarget<'a>, Selection>),
    Contract(
        Result<
            fsl_replay::contract_check::ContractClaim,
            fsl_replay::contract_check::ContractSelection,
        >,
    ),
}

impl ProviderSelection<'_> {
    fn is_err(&self) -> bool {
        match self {
            Self::Composition(_) => false,
            Self::Value(selection) => selection.is_err(),
            Self::Contract(selection) => selection.is_err(),
        }
    }

    fn target(&self) -> Option<ProviderTarget<'_>> {
        match self {
            Self::Composition(target) => Some(ProviderTarget::Composition(target)),
            Self::Value(Ok(target)) => Some(ProviderTarget::Value(target)),
            Self::Contract(Ok(target)) => Some(ProviderTarget::Contract(target)),
            Self::Value(Err(_)) | Self::Contract(Err(_)) => None,
        }
    }
}

/// The `value-validity` item at `input.claim` that `backend`, an engine
/// built by `Engine::external`, decides, in the `unbounded` mode. Nothing
/// is sent while the item is built: [`crate::analyze::analyze_items`] sends
/// one request per backend after every item is routed and negotiated. A
/// package whose bytes do not emit or are not admitted back settles the
/// item `unsupported` with no request sent (FSpec FR-331-AC-33).
pub fn provider_item<'a>(
    index: RequestIndex,
    backend: &BackendId,
    input: ProviderItemInput<'a>,
) -> AnalyzeItem<'a> {
    provider_item_over(index, backend, input, None)
}

/// [`provider_item`] over the run's one strict read of its package, when the
/// run holds it.
pub(crate) fn provider_item_over<'a>(
    index: RequestIndex,
    backend: &BackendId,
    input: ProviderItemInput<'a>,
    shared: Option<Shared<'a>>,
) -> AnalyzeItem<'a> {
    let family = claim_family(input.package, ItemKind::Formula, input.claim);
    AnalyzeItem::new_with_provider(
        index,
        family,
        Capability::ValueValidity,
        backend.as_str(),
        ProviderWork {
            package: input.package,
            shared,
            packages: input.packages,
            routed: input.routed,
            emit: input.emit,
            selection: ProviderSelection::Value(input.selection),
        },
    )
}

/// An `operation-contract` item at `claim` for an external provider. The
/// provider request is emitted only after negotiation (FR-968).
#[allow(
    clippy::too_many_arguments,
    reason = "provider negotiation binds request identity, source package, claim selection, and emission limits"
)]
pub(crate) fn contract_provider_item_over<'a>(
    index: RequestIndex,
    backend: &BackendId,
    package: &'a CheckedPackage,
    packages: &'a PackageInput,
    routed: Option<RequirementItem>,
    emit: EmitLimits,
    selection: Result<
        fsl_replay::contract_check::ContractClaim,
        fsl_replay::contract_check::ContractSelection,
    >,
    shared: Option<Shared<'a>>,
    claim: &OccurrenceKey,
) -> AnalyzeItem<'a> {
    let family = claim_family(package, ItemKind::Formula, claim);
    AnalyzeItem::new_with_provider(
        index,
        family,
        Capability::OperationContract,
        backend.as_str(),
        ProviderWork {
            package,
            shared,
            packages,
            routed,
            emit,
            selection: ProviderSelection::Contract(selection),
        },
    )
}

/// The `composition` item at `target` for an external provider (FSpec
/// FR-348-AC-3). The provider request is emitted only after negotiation, and
/// carries the derived `premises` and the item's `options`.
pub(crate) fn composition_provider_item_over<'a>(
    index: RequestIndex,
    backend: &BackendId,
    package: &'a CheckedPackage,
    packages: &'a PackageInput,
    (routed, emit): (Option<RequirementItem>, EmitLimits),
    target: CompositionTarget,
    shared: Option<Shared<'a>>,
) -> AnalyzeItem<'a> {
    let family = claim_family(package, ItemKind::Formula, target.claim());
    AnalyzeItem::new_with_provider(
        index,
        family,
        Capability::Composition,
        backend.as_str(),
        ProviderWork {
            package,
            shared,
            packages,
            routed,
            emit,
            selection: ProviderSelection::Composition(target),
        },
    )
}

/// The FR-331 request item of the `composition` claim at `claim`, with
/// candidates computed from the current provider registrations and the
/// item's `options`.
pub(crate) fn routed_composition_item<'d>(
    registrations: impl IntoIterator<Item = &'d BackendDescriptor>,
    index: RequestIndex,
    claim: &OccurrenceKey,
    named: Option<&BackendId>,
) -> Option<RequirementItem> {
    let mut registry = Registry::new();
    for descriptor in registrations {
        registry.register(descriptor.clone()).ok()?;
    }
    Some(RequirementItem::composition_clause(
        index,
        claim.clone(),
        registry.candidates(Capability::Composition, named),
    ))
}

/// Each provider item's verdict, or the refusal it settles with when no
/// request carried it, by request index.
pub(crate) type Settled = BTreeMap<RequestIndex, Result<ProviderVerdict, Refusal>>;

/// The package bytes of `package`'s own S4 emission, as the strict reader
/// admitted them back against `packages` (FR-940 "Inputs": the source
/// route's `AdmittedPackageRead`); `None` when they do not emit or are not
/// admitted.
pub(crate) fn admitted_bytes(
    package: &CheckedPackage,
    packages: &PackageInput,
    emit: EmitLimits,
) -> Result<EmittedPackage, NotAdmitted> {
    let read = read_package(package, packages, emit)?;
    Ok(read.emission().package().clone())
}

/// Why a compiled package has no admitted read.
pub(crate) enum NotAdmitted {
    /// The strict reader refused the package it was given back: the typed
    /// refusal, with the reader's code, cause and pointer.
    Refused(fsl_package::PackageAdmissionRefusal),
    /// The package did not emit, or the read failed for a reason that is no
    /// admission refusal.
    Other,
}

/// `package` emitted and read back through the strict reader against
/// `packages`.
pub(crate) fn read_package<'p>(
    package: &'p CheckedPackage,
    packages: &PackageInput,
    emit: EmitLimits,
) -> Result<fsl_package::AdmittedPackageRead<'p>, NotAdmitted> {
    let identity =
        LibraryName::new(package.graph().package_name()).map_err(|_| NotAdmitted::Other)?;
    fsl_package::read_evaluation_package(
        package,
        identity,
        packages,
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT),
        emit,
    )
    .map_err(|refusal| {
        let typed = fsl_package::PackageAdmissionRefusal::of_import_view(&refusal, None);
        // A missing import names its identity from the package bytes it
        // refused; the read does not return them, so they are emitted again
        // on this error path only.
        let typed = match typed {
            Some(typed) if typed.code == Code::MissingImport => {
                let emitted = fsl_package::emit_checked(package, emit).ok();
                fsl_package::PackageAdmissionRefusal::of_import_view(
                    &refusal,
                    emitted.as_ref().map(|emission| emission.package().bytes()),
                )
            }
            other => other,
        };
        typed.map_or(NotAdmitted::Other, NotAdmitted::Refused)
    })
}

/// The one strict read of a run's package, borrowed by every item that
/// needs the admitted package or its bytes.
#[derive(Clone, Copy)]
pub(crate) struct Shared<'r> {
    /// The package as the strict reader admitted it.
    pub(crate) admitted: &'r filament_contract_model::CheckedPackageV2,
    /// The bytes that were read.
    pub(crate) emitted: &'r EmittedPackage,
}

impl<'r> Shared<'r> {
    /// The borrow of `read`.
    pub(crate) fn of(read: &'r fsl_package::AdmittedPackageRead<'_>) -> Self {
        Self {
            admitted: read.admitted(),
            emitted: read.emission().package(),
        }
    }
}

/// The provider items of one backend over one package, in request order.
type Group<'g, 'w> = (
    &'g Engine,
    &'g CheckedPackage,
    Vec<(RequestIndex, &'g ProviderWork<'w>)>,
);

/// Send one FSpec FR-331 request to each external provider that `items`
/// route to, after routing and negotiation (FR-635), and return each item's
/// verdict. Nothing is sent once `cancel` has fired: `run` settles every
/// item of a cancelled handle `incomplete`.
pub(crate) fn settle_items(
    engines: &Engines,
    items: &[AnalyzeItem<'_>],
    cancel: &Cancel,
) -> Settled {
    let mut settled = Settled::new();
    if cancel.cause().is_some() {
        return settled;
    }
    // The items of each backend and package, in request order.
    let mut groups: Vec<Group<'_, '_>> = Vec::new();
    for (engine, index, work) in items.iter().filter_map(|item| item.provider_route(engines)) {
        if work.selection.is_err() {
            continue;
        }
        let same = |group: &&mut (&Engine, &CheckedPackage, _)| {
            group.0.id() == engine.id() && std::ptr::eq(group.1, work.package)
        };
        match groups.iter_mut().find(same) {
            Some(group) => group.2.push((index, work)),
            None => groups.push((engine, work.package, vec![(index, work)])),
        }
    }
    for (engine, package, members) in groups {
        let Some(provider) = engine.provider() else {
            continue;
        };
        let mut routed = Vec::new();
        let mut targets = Vec::new();
        let mut indices = Vec::new();
        for (index, work) in &members {
            match (&work.routed, work.selection.target()) {
                (Some(item), Some(target)) => {
                    routed.push(item.at_index(RequestIndex::new(routed.len())));
                    targets.push(target);
                    indices.push(*index);
                }
                _ => {
                    settled.insert(
                        *index,
                        Err(Refusal::Failed(
                            InternalFault::new("external-provider", "routed-request-item")
                                .catalog_code(),
                        )),
                    );
                }
            }
        }
        if routed.is_empty() {
            continue;
        }
        let Some((_, first)) = members.first() else {
            continue;
        };
        let owned;
        let bytes = match first.shared {
            Some(shared) => shared.emitted,
            None => match admitted_bytes(package, first.packages, first.emit) {
                Ok(bytes) => {
                    owned = bytes;
                    &owned
                }
                Err(_) => {
                    for index in indices {
                        settled.insert(index, Err(Refusal::NotEmitted));
                    }
                    continue;
                }
            },
        };
        let Ok(graph) = WireGraph::of_package_bytes(bytes.bytes()) else {
            for index in indices {
                settled.insert(index, Err(Refusal::NotEmitted));
            }
            continue;
        };
        let Some(dependencies) = crate::analyze::dependency_packages(package, first.emit) else {
            for index in indices {
                settled.insert(index, Err(Refusal::NotEmitted));
            }
            continue;
        };
        let request = ProviderRequest {
            dependencies,
            graph,
            package: bytes,
            items: routed,
            targets,
            manifest: engines.registrations().collect(),
            descriptor: engine.descriptor(),
            limits: engines.options().provider_limits,
        };
        let verdicts = provider.settle(&request, cancel);
        let complete = verdicts.len() == indices.len();
        let mut verdicts = verdicts.into_iter();
        for index in indices {
            let verdict = verdicts
                .next()
                .filter(|_| complete)
                .unwrap_or(ProviderVerdict::Unknown);
            settled.insert(index, Ok(verdict));
        }
    }
    settled
}

/// Settle a provider item from its verdict: a refusal of its target first,
/// then the refusal its request settled, then the verdict, whose
/// counterexample is replayed before it is anything but a candidate.
pub(crate) fn finish(
    work: ProviderWork<'_>,
    settled: Option<Result<ProviderVerdict, Refusal>>,
    cancel: &Cancel,
    engine: &Engine,
) -> EngineReport {
    let composition = matches!(work.selection, ProviderSelection::Composition(_));
    let target = match work.selection {
        ProviderSelection::Value(Ok(target)) => Some(target),
        ProviderSelection::Value(Err(Selection::NoForm))
        | ProviderSelection::Contract(Err(fsl_replay::contract_check::ContractSelection::NoForm)) =>
        {
            return EngineReport::Refused(crate::value_check::invalid_input());
        }
        ProviderSelection::Value(Err(Selection::MissingNode))
        | ProviderSelection::Contract(Err(
            fsl_replay::contract_check::ContractSelection::MissingNode
            | fsl_replay::contract_check::ContractSelection::Unbound(_),
        )) => return EngineReport::Refused(crate::value_check::incomplete_input()),
        ProviderSelection::Value(Err(Selection::Fault(fault)))
        | ProviderSelection::Contract(Err(fsl_replay::contract_check::ContractSelection::Fault(
            fault,
        ))) => return EngineReport::Refused(Refusal::Failed(fault.catalog_code())),
        // A contract and a composition have no value target to replay a
        // counterexample at: a `refuted` verdict is never accepted for them.
        ProviderSelection::Contract(Ok(_)) | ProviderSelection::Composition(_) => None,
    };
    let verdict = match settled {
        Some(Ok(verdict)) => verdict,
        Some(Err(refusal)) => return EngineReport::Refused(refusal),
        None => {
            return EngineReport::Refused(Refusal::Failed(
                InternalFault::new("external-provider", "engine-holds-provider").catalog_code(),
            ));
        }
    };
    let budget = engine
        .max_cases()
        .unwrap_or(manifest::DEFAULT_MAX_CASES)
        .get();
    match target {
        Some(target) => report(&target, verdict, cancel, budget),
        None => match verdict {
            ProviderVerdict::Proved => EngineReport::Provider(Box::new(ProviderReport::Proved)),
            ProviderVerdict::Unknown => EngineReport::Provider(Box::new(ProviderReport::Unknown)),
            ProviderVerdict::Failed { limit, cause } if limit.matches(cause) => {
                EngineReport::Provider(Box::new(ProviderReport::Failed(cause)))
            }
            ProviderVerdict::Failed { .. } => EngineReport::Refused(Refusal::Failed(
                InternalFault::new("external-provider", "limit-cause-mismatch").catalog_code(),
            )),
            // A composition has no replay: its counterexample is not
            // replayable (FSpec FR-290 "Replay context").
            ProviderVerdict::Refuted { .. } if composition => EngineReport::Provider(Box::new(
                ProviderReport::ReplayRefused(fsl_foundation::Code::UnknownRequiredFeature),
            )),
            ProviderVerdict::Refuted { .. } => {
                EngineReport::Provider(Box::new(ProviderReport::ReplayParity))
            }
        },
    }
}

/// The provider's verdict as the arm's report: a counterexample is replayed
/// before it is anything but a candidate.
fn report(
    target: &ValueTarget<'_>,
    verdict: ProviderVerdict,
    cancel: &Cancel,
    budget: u64,
) -> EngineReport {
    let counterexample = match verdict {
        ProviderVerdict::Proved => return EngineReport::Provider(Box::new(ProviderReport::Proved)),
        ProviderVerdict::Unknown => {
            return EngineReport::Provider(Box::new(ProviderReport::Unknown));
        }
        ProviderVerdict::Failed { limit, cause } => {
            if !limit.matches(cause) {
                return EngineReport::Refused(Refusal::Failed(
                    InternalFault::new("external-provider", "limit-cause-mismatch").catalog_code(),
                ));
            }
            return EngineReport::Provider(Box::new(ProviderReport::Failed(cause)));
        }
        ProviderVerdict::Refuted { counterexample } => counterexample,
    };
    let parameters = target.parameters().iter().map(|p| p.wire()).collect();
    let claim = target.claim().clone();
    let replayed = |report| EngineReport::Provider(Box::new(ProviderReport::Replayed(report)));
    match confirm_reported(target, &counterexample, cancel, budget) {
        Ok((outcome, _)) => replayed(ValueReport::Found(outcome, parameters, claim)),
        Err(ValueRefusal::Unsupported(form, reason)) => {
            replayed(ValueReport::Unsupported(form, reason))
        }
        Err(ValueRefusal::Fault(fault)) => {
            EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
        }
        // `confirm_reported` replays at the reported integers and never
        // asks for a bound.
        Err(ValueRefusal::RequiresBound) => EngineReport::Refused(Refusal::Failed(
            InternalFault::new("external-provider", "replay-needs-no-bound").catalog_code(),
        )),
    }
}

/// The certification of a provider's proof (FSpec FR-331, FR-354): a linked
/// first-party provider is `uncertified`, a plugin process `trusted`.
fn certification(origin: ProviderOrigin) -> Certification {
    match origin {
        ProviderOrigin::Linked => Certification::Uncertified,
        ProviderOrigin::Process => Certification::Trusted,
    }
}

/// Settle the arm's report as the item's FR-331 terminal record, with the
/// counterexample a `refuted` item keeps.
pub(crate) fn settle_record(
    report: ProviderReport,
    index: RequestIndex,
    backend: BackendId,
    origin: ProviderOrigin,
) -> (TerminalRecord, Option<ValueCounterexample>) {
    match report {
        ProviderReport::Proved => (
            TerminalRecord::proved_by_provider(index, backend, certification(origin)),
            None,
        ),
        ProviderReport::Unknown => (
            TerminalRecord::inconclusive(
                index,
                backend,
                InconclusiveCause::SolverInconclusive {
                    trace: None,
                    replay: None,
                },
            ),
            None,
        ),
        ProviderReport::ReplayParity => (
            TerminalRecord::inconclusive(
                index,
                backend,
                InconclusiveCause::ReplayParity(fsl_replay::DisagreementCause::Verdicts {
                    proved: fsl_replay::Verdict::from_category(
                        filament_semantic_value::catalog::Category::Violation,
                    ),
                    replayed: fsl_replay::Verdict::from_category(
                        filament_semantic_value::catalog::Category::Success,
                    ),
                }),
            ),
            None,
        ),
        ProviderReport::ReplayRefused(code) => (
            TerminalRecord::inconclusive(index, backend, InconclusiveCause::ReplayRefused(code)),
            None,
        ),
        ProviderReport::Failed(cause) => (TerminalRecord::tool_failed(index, backend, cause), None),
        ProviderReport::Replayed(report) => crate::value_check::settle(report, index, backend),
    }
}
