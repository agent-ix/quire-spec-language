// SPDX-License-Identifier: AGPL-3.0-or-later
//! The operation-owned request boundary shared by analyze and the provider.

use std::collections::BTreeMap;

use fsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use fsl_foundation::{BackendId, RequestIndex};
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::{Candidate, EvidenceKind};
use fsl_semantics::check::Capability;

use super::Diagnostic;

/// Source bytes keyed only by the package's declared source labels.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SourceProvision {
    entries: BTreeMap<(String, String), Vec<u8>>,
}

impl std::fmt::Debug for SourceProvision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(
                self.entries
                    .iter()
                    .map(|(source, bytes)| (source, bytes.len())),
            )
            .finish()
    }
}

impl SourceProvision {
    /// Provide the bytes under their declared source label.
    pub fn insert(&mut self, source: &RawSourceRef, bytes: Vec<u8>) {
        self.entries.insert(
            (source.authority().to_owned(), source.identity().to_owned()),
            bytes,
        );
    }

    /// A provision containing one source unit.
    pub fn single(source: &RawSourceRef, bytes: &[u8]) -> Self {
        let mut provision = Self::default();
        provision.insert(source, bytes.to_vec());
        provision
    }

    /// Read bytes by the package's source label, never by a path.
    pub fn get(&self, source: &RawSourceRef) -> Option<&[u8]> {
        self.entries
            .get(&(source.authority().to_owned(), source.identity().to_owned()))
            .map(Vec::as_slice)
    }

    /// Every entry as `(authority, identity, bytes)`, in label order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, &[u8])> + '_ {
        self.entries.iter().map(|((authority, identity), bytes)| {
            (authority.as_str(), identity.as_str(), bytes.as_slice())
        })
    }

    pub(super) fn wire_entries(&self) -> Vec<(String, String, Vec<u8>)> {
        self.entries
            .iter()
            .map(|((authority, identity), bytes)| {
                (authority.clone(), identity.clone(), bytes.clone())
            })
            .collect()
    }
}

/// A source named by the package was absent from its provision.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("missing source {authority}:{identity}")]
pub struct ProvisionRefusal {
    /// The missing source's declared authority.
    pub authority: String,
    /// The missing source's declared identity.
    pub identity: String,
}

impl ProvisionRefusal {
    /// FR-098's missing-selection refusal.
    pub const fn catalog_code(&self) -> filament_semantic_value::catalog::CatalogCode {
        filament_semantic_value::catalog::CatalogCode::new("missing_import", "missing-selection")
    }
}

/// Common options of one requested claim.
#[derive(Clone, Debug, Default)]
pub struct ItemOptions {
    /// Complete aggregate success requires this item.
    pub required: bool,
    /// Optional finite assignment/case budget of the native check arms.
    pub max_cases: Option<std::num::NonZeroU64>,
    /// EN-6's work bound for release-backed task-product search (FR-415).
    pub max_task_product_work: Option<std::num::NonZeroU64>,
    /// FR-418's checker work limit, `zone_check.max_check_work` (FR-461).
    pub max_check_work: Option<std::num::NonZeroU64>,
    /// The provisioned document a `finite-replay` item replays.
    pub trace: Option<fsl_semantics::model::observation::DocumentRef>,
    /// The provisioned FR-739 finite trace population for a finite hyper item.
    pub traces: Option<fsl_semantics::model::observation::DocumentRef>,
    /// The provisioned FR-737 execution assessed by `global-conformance`.
    pub recording: Option<fsl_semantics::model::observation::DocumentRef>,
    /// The caller's activated role population for a recording assessment.
    pub population: Option<serde_json::Value>,
    /// The named backend's options (FSpec FR-196, FR-331 `options`),
    /// each one the backend publishes (FR-967); empty when none are given.
    pub backend_options: std::collections::BTreeMap<String, String>,
    /// Protocol-analysis request settings, for `local-projection` and
    /// `realizability` (FR-636).
    pub protocol_analysis: Option<ProtocolAnalysisSelection>,
}

/// The request members of one local-projection or realizability item.
#[derive(Clone, Debug)]
pub struct ProtocolAnalysisSelection {
    /// The required profile word (FR-575).
    pub profile: String,
    /// The required path bound (ADR-036 PA-14).
    pub max_depth: u64,
    /// The required local-history bound, used by realizability (PA-14).
    pub max_history: Option<u64>,
    /// The role selected by `options.role`, for projection.
    pub role: Option<String>,
    /// The claim's explicitly supplied assessed root (FR-574).
    pub assessed_root: Option<serde_json::Value>,
    /// The validated checked-object root consumed by the engine.
    pub assessed_root_draft: fsl_replay::protocol_analysis::AssessedRootDraft,
    /// The protocol subject selected by `temporal.subject.protocol`.
    pub protocol: Option<ProtocolBinding>,
    /// The explicit realizability premise groups (FR-571, PA-8).
    pub premises: Option<serde_json::Value>,
}

/// One claim occurrence and its kind-specific, subject-free settings.
#[derive(Clone)]
pub struct ClaimItem {
    /// The original request index.
    pub request_index: RequestIndex,
    /// The checked claim occurrence; resolved against the operation's package.
    pub claim: OccurrenceKey,
    /// The requested capability.
    pub kind: Capability,
    /// The caller's extent classification, independent of proof bounds.
    pub extent: ExtentClassification,
    /// A caller-named backend, when present.
    pub backend: Option<BackendId>,
    /// The candidate set computed by the caller.
    pub candidates: Vec<Candidate>,
    /// Common request settings.
    pub options: ItemOptions,
    /// The capability's own request settings.
    pub form: Option<ItemForm>,
}

/// A requested key does not identify a checked claim of the requested kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClaimRefusal {
    /// No checked claim matches this complete occurrence key.
    #[error("unknown checked claim")]
    UnknownClaim,
}

impl ClaimItem {
    /// Resolve a claim against its owning checked package before configuring it.
    ///
    /// # Errors
    /// Returns [`ClaimRefusal::UnknownClaim`] for an absent node, a different
    /// occurrence, or a claim to which `kind` does not apply.
    pub fn new(
        package: &fsl_package::CheckedPackage,
        request_index: RequestIndex,
        claim: OccurrenceKey,
        kind: Capability,
        extent: ExtentClassification,
    ) -> Result<Self, ClaimRefusal> {
        let graph = package.graph();
        let family = crate::claim_family::claim_family(package, ItemKind::Formula, &claim);
        // A task-set claim's node is minted beside the semantic graph
        // (FSpec FR-482), so its occurrence is the checked claim's own.
        let occurs = graph
            .occurrences()
            .any(|(node, origin, _)| node == *claim.node() && origin == *claim.origin())
            || graph
                .task_sets()
                .claims()
                .iter()
                .any(|checked| checked.occurrence().as_ref() == Some(&claim));
        let wrong_authored_occurrence = names_other_occurrence(graph, &claim)
            || graph
                .finite_hyper_clauses()
                .iter()
                .find(|clause| clause.identity() == Some(claim.node()))
                .and_then(|clause| clause.occurrence())
                .is_some_and(|occurrence| occurrence != &claim);
        if !occurs
            || wrong_authored_occurrence
            || family.is_none_or(|family| !kind.applies_to_declaration(family))
        {
            return Err(ClaimRefusal::UnknownClaim);
        }
        Ok(Self {
            request_index,
            claim: claim.clone(),
            kind,
            extent,
            backend: None,
            candidates: Vec::new(),
            options: ItemOptions::default(),
            form: if kind == Capability::TemporalSatisfaction {
                graph
                    .finite_hyper_clauses()
                    .iter()
                    .find(|clause| clause.identity() == Some(claim.node()))
                    .cloned()
                    .map(FiniteHyperItem::from)
                    .map(ItemForm::FiniteHyper)
            } else {
                None
            },
        })
    }
}

/// The request form of a kind that has one.
#[derive(Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "a public request form built by value at many call sites; boxing is an API change"
)]
pub enum ItemForm {
    /// An authored temporal clause.
    Temporal(TemporalItem),
    /// An exact or statistical probabilistic claim.
    Probabilistic(ProbabilisticItem),
    /// A hybrid-dynamics request; only an engine implementing this form runs it.
    Hybrid(HybridItem),
    /// A checked FR-898 clause over a request-supplied finite trace population.
    FiniteHyper(FiniteHyperItem),
}

/// The checked finite-hyper clause selected by a temporal-satisfaction item.
#[derive(Clone)]
pub struct FiniteHyperItem {
    /// The checked clause whose body is evaluated over the selected traces.
    pub clause: fsl_semantics::check::CheckedFiniteHyperClause,
}

impl From<fsl_semantics::check::CheckedFiniteHyperClause> for FiniteHyperItem {
    fn from(clause: fsl_semantics::check::CheckedFiniteHyperClause) -> Self {
        Self { clause }
    }
}

/// Hybrid item settings kept distinct from temporal and probabilistic forms.
#[derive(Clone, Debug, Default)]
pub struct HybridItem {
    /// The item's proof bounds (FR-750). A hybrid claim's request item names
    /// no subject to substitute them into (FR-447 writes `domains` for an
    /// unbounded claim only), so document resolution leaves this empty and
    /// the provider request does not carry it.
    pub proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    /// The item's `hybrid.limits` (FR-750): a member left unset takes the
    /// serving descriptor's published bound.
    pub limits: fsl_replay::SolverBudget,
    /// The item's FSpec FR-331 `options`.
    pub options: std::collections::BTreeMap<String, String>,
}

/// Temporal request settings, with no separately supplied model subject.
#[derive(Clone)]
pub struct TemporalItem {
    /// The proof-bound substitution of the subject.
    pub proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    /// Formula or derived temporal item.
    pub item_kind: ItemKind,
    /// The abstract subject selected by a refinement item.
    pub abstract_subject: Option<fsl_route::request::AbstractSubject>,
    /// A refinement item's abstract initial snapshots (FR-630); the
    /// concrete subject's when absent.
    pub abstract_initial: Option<Vec<fsl_semantics::model::observation::DocumentRef>>,
    /// The resolved fairness constraints of the checked clause.
    pub fairness: Vec<filament_semantic_value::fairness::FairnessConstraint>,
    /// A requested stutter-closure premise, when supplied.
    pub stutter_closure: Option<bool>,
    /// The search horizon; no horizon when absent.
    pub max_depth: Option<usize>,
    /// The model search and automaton budgets.
    pub limits: fsl_replay::ModelCheckLimits,
    /// The most successor candidates one expansion enumerates.
    pub max_candidates: u64,
    /// The qualified checker's SMT encoding budgets.
    pub smt_encoding: Option<fsl_replay::SmtEncodingLimits>,
    /// The reductions the caller selected.
    pub reductions: fsl_route::reductions::Reductions,
    /// The most live clocks one timed state holds (FSpec FR-415); the
    /// published default when absent.
    pub max_clocks: Option<std::num::NonZeroU64>,
    /// The request's `reductions` member as written, until it is admitted over
    /// the unit's declarations (FR-623).
    pub pending_reductions: Option<serde_json::Value>,
    /// The witness walks per initial state of a `possible` claim (FSpec
    /// FR-392); the published default when absent.
    pub witness_samples: Option<u64>,
    /// The witness-sampling seed; the published default when absent.
    pub seed: Option<u64>,
    /// FSL FR-1033: the protocol subject the item is decided over; the model
    /// subject when absent.
    pub protocol: Option<ProtocolBinding>,
    /// FR-225's `max_store_buffer`; EN-1's default when absent.
    pub max_store_buffer: Option<u64>,
    /// FR-225's `max_messages`; EN-1's default when absent.
    pub max_messages: Option<u64>,
}

/// FSL FR-1033: a protocol subject as a request binds it (FSpec FR-433's
/// `protocol-subject` spelling), with the request's memory selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolBinding {
    /// The declared name of the protocol clause.
    pub protocol: String,
    /// The `over` object as its population identity and key; `None` when the
    /// request gives none.
    pub over: Option<(String, String)>,
    /// Each static role's binding.
    pub roles: Vec<fsl_eval::model_subject::RoleBinding>,
    /// Each symbolic capacity's binding.
    pub capacities: Vec<fsl_eval::model_subject::CapacityBinding>,
    /// The request's `memory` selection (FR-219).
    pub memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
    /// The effective model of each outermost `parallel`, by scoped path.
    pub effective: Vec<(String, fsl_foundation::MemoryModelName)>,
}

impl Default for TemporalItem {
    fn default() -> Self {
        Self {
            proof_bounds: Vec::new(),
            item_kind: ItemKind::Formula,
            abstract_subject: None,
            abstract_initial: None,
            fairness: Vec::new(),
            stutter_closure: None,
            max_depth: None,
            limits: fsl_replay::ModelCheckLimits::default(),
            max_candidates: super::clause_item::DEFAULT_MAX_CANDIDATES,
            smt_encoding: None,
            reductions: fsl_route::reductions::Reductions::default(),
            max_clocks: None,
            pending_reductions: None,
            witness_samples: None,
            seed: None,
            protocol: None,
            max_store_buffer: None,
            max_messages: None,
        }
    }
}

/// The settings of one probabilistic claim, never its model subject.
#[derive(Clone)]
pub struct ProbabilisticItem {
    /// Exact or statistical evidence requested.
    pub evidence: EvidenceKind,
    /// The matching engine budgets.
    pub limits: ProbabilisticLimits,
    /// Statistical method; absent for exact evidence.
    pub method: Option<fsl_replay::statistical::StatisticalMethod>,
    /// Statistical seed; absent for exact evidence.
    pub seed: Option<u64>,
    /// Statistical sampler; absent for exact evidence.
    pub sampler: Option<fsl_foundation::selection::DefinitionRef>,
    /// Maximum sampled witnesses retained.
    pub max_witnesses: Option<u32>,
    /// Number of workers requested for a statistical run (FR-331), when set.
    pub workers: Option<std::num::NonZeroU32>,
    /// Declared statistical symmetries.
    pub symmetry: Vec<fsl_route::reductions::SymmetryDeclaration>,
    /// Timed statistical sampling settings.
    pub timed_sampling: Option<fsl_replay::statistical::TimedSampling>,
}

/// Kind-specific probabilistic budgets; the evidence and budget must agree.
#[derive(Clone, Copy)]
pub enum ProbabilisticLimits {
    /// Statistical sampling limits.
    Statistical(fsl_replay::statistical::StatisticalLimits),
    /// Exact product/solver/checker limits.
    Exact(crate::exact_probabilistic::ExactProbLimits),
}

/// A claim with the disposition and diagnostic supplied by negotiation.
#[derive(Clone)]
pub struct NegotiatedItem {
    /// The typed requested item.
    pub item: ClaimItem,
    /// The caller's negotiated disposition.
    pub disposition: fsl_route::routing::Disposition,
    /// The disposition's catalog diagnostic.
    pub diagnostic: Option<Diagnostic>,
}

/// Analyze negotiated items using only the admitted package, subject and supplied bytes.
///
/// # Errors
/// Returns the shared binding or missing-provision refusal before any item runs.
pub fn analyze(
    package: &fsl_package::CheckedPackage,
    request: &super::AnalyzeRequest<'_>,
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
    cancel: &filament_exact::Cancel,
) -> Result<
    fsl_foundation::diagnostic::Staged<super::AnalyzeOutcome>,
    fsl_foundation::diagnostic::StageFailure<super::AnalyzeRefusal>,
> {
    analyze_bound(
        &super::RunInput::source(package),
        &request.binding,
        &request.provision,
        &request.items,
        engines,
        limits,
        cancel,
    )
}

pub(crate) fn analyze_bound<'a>(
    analysis: &super::RunInput<'a>,
    binding: &super::SubjectBinding<'_>,
    provision: &SourceProvision,
    requested: &[NegotiatedItem],
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
    cancel: &filament_exact::Cancel,
) -> Result<
    fsl_foundation::diagnostic::Staged<super::AnalyzeOutcome>,
    fsl_foundation::diagnostic::StageFailure<super::AnalyzeRefusal>,
> {
    let (package, input) = (analysis.package, &analysis.model);
    use fsl_foundation::diagnostic::{StageFailure, Staged};
    // The compiled package is read back through the strict reader once,
    // before the binding is admitted and before any item runs: a reader
    // refusal (a domain package the binding leaves out, among them) is this
    // typed refusal and never a fault of a later step, and the items share
    // the one read. A package that does not emit has no read, and its items
    // settle as they do without one.
    let read = match crate::provider::read_package(package, binding.packages, limits.emit) {
        Ok(read) => Some(read),
        Err(crate::provider::NotAdmitted::Refused(refusal)) => {
            return Err(StageFailure::Refused(super::AnalyzeRefusal::Package(
                refusal,
            )));
        }
        Err(crate::provider::NotAdmitted::Other) => None,
    };
    let simulation = simulation(input, binding, limits);
    fsl_eval::simulation::model::ModelSystem::new(simulation)
        .map_err(|failure| StageFailure::Refused(super::AnalyzeRefusal::Binding(failure)))?;
    check_provision(package, provision)
        .map_err(|failure| StageFailure::Refused(super::AnalyzeRefusal::Provision(failure)))?;
    let shared = read.as_ref().map(crate::provider::Shared::of);
    let items = requested
        .iter()
        .map(|item| work(analysis, binding, provision, item, engines, limits, shared))
        .collect::<Vec<_>>();
    let requirements = items
        .iter()
        .map(super::AnalyzeItem::requirement)
        .collect::<Vec<_>>();
    let settled = super::analyze_items(engines, items, cancel);
    let aggregate = super::aggregate(&requirements, &settled);
    let details = settled.iter().map(super::ItemDetail::of).collect();
    let protocol_conformance = settled
        .iter()
        .filter_map(|record| {
            record
                .protocol_conformance
                .clone()
                .map(|output| (record.request_index, output))
        })
        .collect();
    let closed_form = settled
        .iter()
        .filter_map(|record| {
            record
                .closed_form
                .as_deref()
                .map(|outcome| (record.request_index, outcome.clone()))
        })
        .collect();
    let (records, counterexamples) = super::split_records(settled, package);
    Ok(Staged::new(super::AnalyzeOutcome {
        records,
        counterexamples,
        aggregate,
        details,
        protocol_conformance,
        closed_form,
    }))
}

fn check_provision(
    package: &fsl_package::CheckedPackage,
    provision: &SourceProvision,
) -> Result<(), ProvisionRefusal> {
    let mut pending = vec![package];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(package) = pending.pop() {
        if !visited.insert(package.graph().package_name()) {
            continue;
        }
        for source in package.graph().sources() {
            if provision.get(source).is_none() {
                return Err(ProvisionRefusal {
                    authority: source.authority().to_owned(),
                    identity: source.identity().to_owned(),
                });
            }
        }
        pending.extend(
            package
                .dependencies()
                .values()
                .map(std::convert::AsRef::as_ref),
        );
    }
    Ok(())
}

/// The package's dependency closure as a process hybrid request carries it
/// (FSpec FR-331 "Package bytes", FSL FR-635): each package by identity with
/// its v2 bytes, each after the dependencies it imports; `None` when one
/// does not emit.
pub(crate) fn dependency_packages(
    package: &fsl_package::CheckedPackage,
    emit: fsl_package::EmitLimits,
) -> Option<Vec<fsl_replay::HybridProcessDependency>> {
    let mut closure: Vec<fsl_replay::HybridProcessDependency> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    add_dependencies(package, emit, &mut seen, &mut closure)?;
    Some(closure)
}

/// `package`'s dependencies after their own, each once.
fn add_dependencies(
    package: &fsl_package::CheckedPackage,
    emit: fsl_package::EmitLimits,
    seen: &mut std::collections::BTreeSet<String>,
    closure: &mut Vec<fsl_replay::HybridProcessDependency>,
) -> Option<()> {
    for (identity, dependency) in package.dependencies() {
        if !seen.insert(identity.to_string()) {
            continue;
        }
        add_dependencies(dependency, emit, seen, closure)?;
        let emission = fsl_package::emit_checked(dependency, emit).ok()?;
        closure.push(fsl_replay::HybridProcessDependency {
            identity: identity.to_string(),
            package: emission.package().bytes().to_vec(),
        });
    }
    Some(())
}

pub(super) fn dependency_wires(
    package: &fsl_package::CheckedPackage,
) -> Vec<fsl_replay::DependencyEntryWire> {
    let mut pending = package.dependencies().iter().collect::<Vec<_>>();
    let mut dependencies = BTreeMap::new();
    while let Some((identity, package)) = pending.pop() {
        if dependencies.contains_key(identity) {
            continue;
        }
        dependencies.insert(
            identity.clone(),
            fsl_replay::DependencyEntryWire {
                identity: identity.to_string(),
                sources: package
                    .graph()
                    .sources()
                    .into_iter()
                    .map(|source| (source.authority().to_owned(), source.identity().to_owned()))
                    .collect(),
                refinement_internals: None,
            },
        );
        pending.extend(package.dependencies().iter());
    }
    dependencies.into_values().collect()
}

pub(crate) fn simulation<'a>(
    input: &fsl_eval::simulation::model::ModelCheckInput<'a>,
    binding: &'a super::SubjectBinding<'a>,
    limits: &super::AnalyzeLimits,
) -> fsl_eval::simulation::model::ModelSimulationRequest<'a> {
    fsl_eval::simulation::model::ModelSimulationRequest {
        input: input.clone(),
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: fsl_eval::simulation::model::ExpansionLimits {
            max_candidates: u64::MAX,
        },
        meter: limits.accounting,
        emit: limits.emit,
    }
}

fn checked_v2_package(
    shared: Option<crate::provider::Shared<'_>>,
) -> Option<Box<filament_contract_model::CheckedPackageV2>> {
    shared.map(|shared| Box::new(shared.admitted.clone()))
}

/// The role of `protocol` named `name`. Only a role of that protocol
/// resolves, so a channel, record or other declaration that shares the
/// role's name is never taken for it (FR-570).
fn protocol_role_named(
    package: &filament_contract_model::CheckedPackageV2,
    protocol: &filament_contract_model::CheckedNodeId,
    name: &str,
) -> Option<filament_contract_model::CheckedNodeId> {
    let view = fsl_package::protocol_view::protocol_view(package, protocol).ok()?;
    view.roles().iter().find_map(|id| {
        package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == *id)
            .and_then(|node| node.declaration.as_ref())
            .filter(|declaration| {
                declaration.qualified_name.last().map(|segment| &**segment) == Some(name)
            })
            .map(|_| (*id).clone())
    })
}

fn protocol_analysis_work<'a>(
    analysis: &'a super::RunInput<'a>,
    binding: &super::SubjectBinding<'a>,
    item: &super::ClaimItem,
    limits: &super::AnalyzeLimits,
    shared: Option<crate::provider::Shared<'a>>,
) -> super::AnalyzeItem<'a> {
    let Some(selection) = item.options.protocol_analysis.as_ref() else {
        return super::AnalyzeItem::unadapted(
            item.request_index,
            item.kind,
            Some(fsl_semantics::check::DeclarationFamily::Protocol),
        );
    };
    let Some(protocol_binding) = selection.protocol.as_ref() else {
        return super::AnalyzeItem::unadapted(
            item.request_index,
            item.kind,
            Some(fsl_semantics::check::DeclarationFamily::Protocol),
        );
    };
    let Some(over) = protocol_binding.over.as_ref() else {
        return super::AnalyzeItem::unadapted(
            item.request_index,
            item.kind,
            Some(fsl_semantics::check::DeclarationFamily::Protocol),
        );
    };
    let Some(v2) = checked_v2_package(shared) else {
        return super::AnalyzeItem::unadapted(
            item.request_index,
            item.kind,
            Some(fsl_semantics::check::DeclarationFamily::Protocol),
        );
    };
    let Some(protocol_node) = v2
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            &*node.semantic_form == "protocol_clause"
                && node.declaration.as_ref().is_some_and(|declaration| {
                    declaration.qualified_name.last().map(|segment| &**segment)
                        == Some(
                            protocol_binding
                                .protocol
                                .rsplit("::")
                                .next()
                                .unwrap_or(&protocol_binding.protocol),
                        )
                })
        })
        .map(|node| node.node_id.clone())
    else {
        return super::AnalyzeItem::unadapted(
            item.request_index,
            item.kind,
            Some(fsl_semantics::check::DeclarationFamily::Protocol),
        );
    };
    let definition_key = Some(format!(
        "ix://{}/{}",
        analysis.package.graph().package_name(),
        protocol_binding.protocol
    ));
    let root = selection.assessed_root_draft.clone();
    if item.kind == Capability::LocalProjection {
        let Some(role_name) = selection.role.as_deref() else {
            return super::AnalyzeItem::unadapted(
                item.request_index,
                item.kind,
                Some(fsl_semantics::check::DeclarationFamily::Protocol),
            );
        };
        // Route an unresolved name through the adapter's existing
        // `UnknownRole` invalid-value settlement. The protocol node is a
        // valid checked identity that cannot be one of its roles.
        let role = protocol_role_named(&v2, &protocol_node, role_name)
            .unwrap_or_else(|| protocol_node.clone());
        let request = crate::protocol_analysis::ProjectionRequest {
            protocol: protocol_node,
            definition_key,
            role,
            profile: Some(selection.profile.clone()),
            bounds: Some(fsl_replay::protocol_analysis::ProjectionBoundsDraft {
                max_depth: Some(selection.max_depth),
            }),
            assessed_root: Some(root),
            budget: fsl_replay::protocol_analysis::RunBudget::default(),
        };
        return crate::protocol_analysis::projection_item_owned(item.request_index, v2, request);
    }
    let model_input = &analysis.model;
    let over = super::protocol_item::over_instance(binding, over);
    let Ok(subject) = super::protocol_item::subject_of(
        model_input,
        binding,
        limits,
        super::clause_item::DEFAULT_MAX_CANDIDATES,
        protocol_binding,
        &over,
    ) else {
        return super::AnalyzeItem::unadapted(
            item.request_index,
            item.kind,
            Some(fsl_semantics::check::DeclarationFamily::Protocol),
        );
    };
    let premises = selection.premises.as_ref();
    let parse_facts = |key: &str| -> Option<Option<std::collections::BTreeSet<String>>> {
        let value = premises?.get(key)?;
        let facts = value
            .as_array()?
            .iter()
            .map(|fact| fact.as_str().map(str::to_owned))
            .collect::<Option<std::collections::BTreeSet<_>>>()?;
        Some(Some(facts))
    };
    let request = crate::protocol_analysis::RealizabilityRequest {
        protocol: protocol_node,
        subject,
        definition_key,
        strategy_profile: Some(selection.profile.clone()),
        role_knowledge: parse_facts("role_knowledge"),
        environment: parse_facts("environment"),
        bounds: Some(fsl_replay::protocol_analysis::BoundsDraft {
            max_depth: Some(selection.max_depth),
            max_history: selection.max_history,
        }),
        assessed_root: Some(root),
        budget: fsl_replay::protocol_analysis::RunBudget::default(),
    };
    crate::protocol_analysis::realizability_item_owned(item.request_index, v2, request)
}

fn work<'a>(
    analysis: &'a super::RunInput<'a>,
    binding: &'a super::SubjectBinding<'a>,
    provision: &'a SourceProvision,
    negotiated: &'a NegotiatedItem,
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
    shared: Option<crate::provider::Shared<'a>>,
) -> super::AnalyzeItem<'a> {
    let (package, input) = (analysis.package, &analysis.model);
    use super::AnalyzeItem;
    let item = &negotiated.item;
    let temporal_kind = match &item.form {
        Some(ItemForm::Temporal(form)) => form.item_kind,
        Some(ItemForm::FiniteHyper(_) | ItemForm::Probabilistic(_) | ItemForm::Hybrid(_))
        | None => ItemKind::Formula,
    };
    let family = if item.kind == Capability::GlobalConformance {
        Some(fsl_semantics::check::DeclarationFamily::Protocol)
    } else {
        crate::claim_family::claim_family(package, temporal_kind, &item.claim)
    };
    let wrong_occurrence = names_other_occurrence(package.graph(), &item.claim);
    let family = if wrong_occurrence { None } else { family };
    let absent = || AnalyzeItem::unadapted(item.request_index, item.kind, family);
    // FR-243: an item naming `options.method` `digital-clocks` negotiated to
    // EN-1's digital-clock route; every other timed item is EN-6's.
    let digital = item
        .options
        .backend_options
        .get(crate::zone_check::manifest::METHOD)
        .is_some_and(|method| method == crate::model_check::manifest::METHOD_DIGITAL_CLOCKS);
    let mut built = match &item.form {
        Some(ItemForm::FiniteHyper(form)) if item.kind == Capability::TemporalSatisfaction => {
            super::finite_hyper_item::item(analysis, binding, item, form, limits)
        }
        Some(ItemForm::Temporal(_))
            if matches!(
                item.kind,
                Capability::LocalProjection | Capability::Realizability
            ) =>
        {
            protocol_analysis_work(analysis, binding, item, limits, shared)
        }
        Some(ItemForm::Temporal(_form)) if item.kind == Capability::GlobalConformance => {
            if _form.item_kind == ItemKind::DeadlockFreedom {
                super::protocol_item::protocol_clause_item(
                    analysis,
                    binding,
                    *limits,
                    provision,
                    &item.claim,
                    item.request_index,
                    _form,
                )
                .unwrap_or_else(absent)
            } else {
                let protocol = package.graph().protocols().iter().find(|protocol| {
                    package
                        .graph()
                        .protocol_node_keys(&protocol.name)
                        .and_then(|keys| keys.clause.as_ref())
                        == Some(item.claim.node())
                });
                match protocol {
                    Some(protocol) => {
                        let missing_role = if item.options.recording.is_some() {
                            None
                        } else {
                            let selected_model_packages: std::collections::BTreeSet<&str> = binding
                                .universes
                                .iter()
                                .map(|universe| universe.population.package.as_str())
                                .collect();
                            protocol.roles.iter().find(|role| {
                                role.context.as_ref().is_some_and(|context| {
                                    !selected_model_packages.contains(context.original().package())
                                })
                            })
                        };
                        if let Some(role) = missing_role {
                            crate::protocol_analysis::conformance::missing_role_item(
                                item.request_index,
                                &role.name,
                            )
                        } else {
                            let recording = item
                                .options
                                .recording
                                .as_ref()
                                .and_then(|reference| binding.snapshots.get(reference));
                            let recording_identity = item.options.recording.as_ref().cloned();
                            match recording {
                                Some(recording) => {
                                    crate::protocol_analysis::conformance::global_recording_item(
                                        item.request_index,
                                        package,
                                        None,
                                        recording_identity,
                                        Some(recording),
                                        item.options.population.clone(),
                                        limits.replay,
                                    )
                                }
                                None => {
                                    crate::protocol_analysis::conformance::global_behavior_item(
                                        item.request_index,
                                        analysis,
                                        binding.clone(),
                                        *limits,
                                        protocol,
                                        _form,
                                    )
                                }
                            }
                        }
                    }
                    None => absent(),
                }
            }
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && package
                    .graph()
                    .hyper_clauses()
                    .iter()
                    .any(|clause| clause.assembled_occurrence() == Some(&item.claim)) =>
        {
            super::hyper_route::hyper_item(
                analysis,
                binding,
                *limits,
                &item.claim,
                item.request_index,
                form,
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && super::document::is_state_graph(package, &item.claim) =>
        {
            super::state_graph_item::state_graph_item(
                analysis,
                binding,
                *limits,
                provision,
                &item.claim,
                item.request_index,
                form,
                digital.then_some(crate::model_check::manifest::METHOD_DIGITAL_CLOCKS),
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && matches!(form.item_kind, ItemKind::Formula | ItemKind::Refinement)
                && !super::document::timed(package)
                && super::refinement_item::refinement_at(package, &item.claim).is_some() =>
        {
            super::refinement_item::refinement_item(
                analysis,
                binding,
                *limits,
                &item.claim,
                item.request_index,
                form,
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction && form.protocol.is_some() =>
        {
            super::protocol_item::protocol_clause_item(
                analysis,
                binding,
                *limits,
                provision,
                &item.claim,
                item.request_index,
                form,
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && matches!(
                    form.item_kind,
                    ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom
                ) =>
        {
            if super::document::timed(package) {
                super::timed_item::zone_derived_item(
                    analysis,
                    binding,
                    *limits,
                    provision,
                    &item.claim,
                    item.request_index,
                    form,
                )
                .unwrap_or_else(absent)
            } else if form.item_kind == ItemKind::DeadlockFreedom {
                super::derived_item::deadlock_item(
                    analysis,
                    binding,
                    *limits,
                    provision,
                    &item.claim,
                    item.request_index,
                    form,
                )
                .unwrap_or_else(absent)
            } else {
                absent()
            }
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && form.item_kind == ItemKind::Formula
                && super::document::claim_timed(package, &item.claim, binding) =>
        {
            if digital {
                super::digital_item::digital_clause_item(
                    analysis,
                    binding,
                    *limits,
                    provision,
                    &item.claim,
                    item.request_index,
                    form,
                )
                .unwrap_or_else(absent)
            } else {
                super::timed_item::zone_clause_item(
                    analysis,
                    binding,
                    *limits,
                    provision,
                    &item.claim,
                    item.request_index,
                    form,
                )
                .unwrap_or_else(absent)
            }
        }
        Some(ItemForm::Temporal(form)) if item.kind == Capability::TemporalSatisfaction => {
            match super::ClauseSubject::from_provision(
                analysis,
                binding.clone(),
                *limits,
                provision,
                &item.claim,
                super::ClauseLimits {
                    limits: form.limits.limits,
                    max_depth: form.max_depth,
                    max_candidates: form.max_candidates,
                },
                form.proof_bounds.clone(),
            ) {
                Ok(subject) => subject.configured(form.clone()).item(item.request_index),
                Err(_) => absent(),
            }
        }
        Some(ItemForm::Hybrid(form)) if item.kind == Capability::TemporalSatisfaction => {
            super::hybrid_item::hybrid_item(
                package,
                item.request_index,
                &item.claim,
                form,
                limits.accounting.work_units,
                limits.deadline,
                engines.options().provider_limits,
                engines.manifest_wire(),
                item.candidates
                    .iter()
                    .map(|candidate| candidate.id().clone())
                    .collect(),
                super::hybrid_item::Spelling {
                    packages: binding.packages,
                    emit: limits.emit,
                    shared,
                },
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Probabilistic(form))
            if item.kind == Capability::ProbabilisticSatisfaction =>
        {
            let claim = package
                .graph()
                .probabilistic_model()
                .claims()
                .iter()
                .find(|claim| claim.occurrence() == Some(&item.claim));
            match (claim, form.evidence, form.limits) {
                (Some(claim), EvidenceKind::Exact, ProbabilisticLimits::Exact(exact)) => {
                    super::exact_probabilistic_item(
                        item.request_index,
                        crate::exact_probabilistic::ProductRequest {
                            subject: fsl_eval::model_subject::ModelSubject {
                                simulation: simulation(input, binding, limits),
                                proof_bounds: Vec::new(),
                            },
                            claim,
                            limits: exact,
                        },
                    )
                }
                (
                    Some(claim),
                    EvidenceKind::Statistical,
                    ProbabilisticLimits::Statistical(statistical),
                ) => match (form.method, form.seed, &form.sampler) {
                    (Some(method), Some(seed), Some(sampler)) => super::statistical_item(
                        item.request_index,
                        crate::statistical::StatisticalRequest {
                            subject: fsl_eval::model_subject::ModelSubject {
                                simulation: simulation(input, binding, limits),
                                proof_bounds: Vec::new(),
                            },
                            claim,
                            method,
                            seed,
                            sampler: sampler.clone(),
                            limits: statistical,
                            max_witnesses: form
                                .max_witnesses
                                .unwrap_or(crate::statistical::DEFAULT_MAX_WITNESSES),
                            symmetry: form.symmetry.clone(),
                            timed_sampling: form.timed_sampling.unwrap_or_default(),
                        },
                    ),
                    _ => absent(),
                },
                _ => absent(),
            }
        }
        None if item.kind == Capability::ValueValidity => {
            let selection = package
                .graph()
                .semantic_graph()
                .nodes()
                .filter_map(|node| node.declaration())
                .filter_map(|name| name.last())
                .find_map(|name| {
                    fsl_replay::value_check::ValueTarget::select(
                        package,
                        name.as_str(),
                        &item.claim,
                    )
                    .ok()
                })
                .ok_or(fsl_replay::value_check::Selection::NoForm)
                .map(|target| target.with_limits(fsl_replay::AccountingLimits(limits.accounting)));
            if let Some(backend) = negotiated.disposition.target().filter(|backend| {
                engines
                    .get(backend.id())
                    .is_some_and(|engine| engine.provider().is_some())
            }) {
                let routed = crate::provider::routed_item(
                    package,
                    engines.registrations(),
                    item.request_index,
                    &item.claim,
                    item.backend.as_ref(),
                )
                .map(|routed| routed.with_options(item.options.backend_options.clone()));
                crate::provider::provider_item_over(
                    item.request_index,
                    backend.id(),
                    crate::provider::ProviderItemInput {
                        package,
                        packages: binding.packages,
                        claim: &item.claim,
                        routed,
                        emit: limits.emit,
                        selection,
                    },
                    shared,
                )
            } else {
                let selection = selection.map(|target| crate::value_check::ValueRequest {
                    target,
                    max_cases: item.options.max_cases,
                    reported: None,
                });
                super::value_item(item.request_index, package, &item.claim, selection)
            }
        }
        None if item.kind == Capability::FiniteReplay => {
            let protocol = package.graph().protocols().iter().find(|protocol| {
                package
                    .graph()
                    .protocol_node_keys(&protocol.name)
                    .and_then(|keys| keys.clause.as_ref())
                    == Some(item.claim.node())
            });
            match protocol {
                Some(protocol) => {
                    let trace = item
                        .options
                        .trace
                        .as_ref()
                        .and_then(|reference| binding.snapshots.get(reference))
                        .map_or(
                            Err(crate::protocol_analysis::TraceDefect::unreadable()),
                            |bytes| crate::protocol_analysis::SuppliedTrace::read(bytes),
                        );
                    crate::protocol_analysis::finite_replay_item(
                        item.request_index,
                        analysis,
                        binding.clone(),
                        *limits,
                        provision,
                        protocol,
                        trace,
                    )
                }
                None => absent(),
            }
        }
        // FR-489's tie-break routed an invariant over the request's model
        // subject to EN-1 (FR-625, FR-951).
        _ if item.kind == Capability::OperationContract
            && negotiated.disposition.target().is_some_and(|backend| {
                backend.id().as_str() == crate::model_check::manifest::BACKEND_ID
            }) =>
        {
            let form = match &item.form {
                Some(ItemForm::Temporal(form)) => form.clone(),
                Some(
                    ItemForm::FiniteHyper(_) | ItemForm::Probabilistic(_) | ItemForm::Hybrid(_),
                )
                | None => TemporalItem::default(),
            };
            super::invariant_item::invariant_item(
                analysis,
                binding,
                *limits,
                provision,
                &item.claim,
                item.request_index,
                &form,
            )
            .unwrap_or_else(absent)
        }
        None if item.kind == Capability::OperationContract
            && negotiated.disposition.target().is_some_and(|backend| {
                engines
                    .get(backend.id())
                    .is_some_and(|engine| engine.provider().is_some())
            }) =>
        {
            let backend = negotiated
                .disposition
                .target()
                .expect("the provider target");
            let routed = crate::provider::routed_contract_item(
                package,
                engines.registrations(),
                item.request_index,
                &item.claim,
                item.backend.as_ref(),
            )
            .map(|routed| routed.with_options(item.options.backend_options.clone()));
            let selection = fsl_replay::contract_check::select_contract(package, &item.claim);
            crate::provider::contract_provider_item_over(
                item.request_index,
                backend.id(),
                package,
                binding.packages,
                routed,
                limits.emit,
                selection,
                shared,
                &item.claim,
            )
        }
        None if item.kind == Capability::OperationContract => {
            let selection =
                fsl_replay::contract_check::select_contract(package, &item.claim).map(|claim| {
                    crate::contract_check::ContractRequest {
                        package,
                        claim,
                        subject: simulation(input, binding, limits),
                        abstract_subject: None,
                        max_cases: item.options.max_cases,
                    }
                });
            super::contract_item(item.request_index, package, &item.claim, selection)
        }
        None if item.kind == Capability::Schedulability => {
            if negotiated.disposition.target().is_some_and(|backend| {
                backend.id().as_str() == crate::zone_check::manifest::BACKEND_ID
            }) {
                super::task_product_item::item(
                    analysis,
                    binding,
                    *limits,
                    item.options.max_task_product_work,
                    item.options.max_check_work,
                    &item.claim,
                    item.request_index,
                )
                .unwrap_or_else(absent)
            } else {
                super::schedule_item::schedule_item(package, item.request_index, &item.claim)
                    .unwrap_or_else(absent)
            }
        }
        _ => absent(),
    };
    let expected = match &item.form {
        _ if matches!(
            item.kind,
            Capability::LocalProjection | Capability::Realizability
        ) =>
        {
            Some(crate::protocol_analysis::manifest::BACKEND_ID)
        }
        _ if item.kind == Capability::GlobalConformance => {
            Some(crate::protocol_analysis::manifest::BACKEND_ID)
        }
        Some(ItemForm::Temporal(_)) if super::document::is_state_graph(package, &item.claim) => {
            Some(if super::document::timed(package) && !digital {
                crate::zone_check::manifest::BACKEND_ID
            } else {
                crate::model_check::manifest::BACKEND_ID
            })
        }
        Some(ItemForm::Temporal(form))
            if digital
                && form.item_kind == ItemKind::Formula
                && super::document::claim_timed(package, &item.claim, binding) =>
        {
            Some(crate::model_check::manifest::BACKEND_ID)
        }
        Some(ItemForm::Temporal(form))
            if if form.item_kind == ItemKind::Formula {
                super::document::claim_timed(package, &item.claim, binding)
            } else {
                super::document::timed(package)
            } =>
        {
            Some(crate::zone_check::manifest::BACKEND_ID)
        }
        Some(ItemForm::Temporal(_)) => Some(crate::model_check::manifest::BACKEND_ID),
        Some(ItemForm::FiniteHyper(_)) => Some(crate::model_check::manifest::BACKEND_ID),
        Some(ItemForm::Probabilistic(form)) => match form.evidence {
            EvidenceKind::Exact => Some(crate::exact_probabilistic::manifest::BACKEND_ID),
            EvidenceKind::Statistical => Some(crate::statistical::manifest::BACKEND_ID),
        },
        Some(ItemForm::Hybrid(_)) => None,
        None if item.kind == Capability::Schedulability => {
            negotiated.disposition.target().map(|backend| {
                if backend.id().as_str() == crate::zone_check::manifest::BACKEND_ID {
                    crate::zone_check::manifest::BACKEND_ID
                } else {
                    crate::schedulability::manifest::BACKEND_ID
                }
            })
        }
        None => None,
    };
    if let (Some(expected), fsl_route::routing::Disposition::Supported(candidate)) =
        (expected, &negotiated.disposition)
    {
        if candidate.id().as_str() != expected {
            built.work = Some(super::ItemWork::Engine(Box::new(move |_, _| {
                super::EngineReport::Refused(super::Refusal::Unsupported(
                    fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                ))
            })));
        }
    }
    built.kind = item.kind;
    built.family = family;
    built.claim = Some(item.claim.clone());
    // EN-6 decides over the subject's universes alone (FR-239): it advertises
    // the bounded mode, and an item it runs is read in it. EN-7 decides a
    // task set in closed form, which has no bound to ask for, so a
    // schedulability item is read in the `unbounded` mode it advertises
    // (FR-247).
    let zone = match &item.form {
        Some(ItemForm::Temporal(form)) if form.item_kind == ItemKind::Formula => {
            super::document::claim_timed(package, &item.claim, binding)
        }
        Some(ItemForm::Temporal(_)) => super::document::timed(package),
        _ => false,
    };
    built.mode = match item.extent {
        _ if zone => fsl_route::Mode::Bounded,
        _ if item.kind == Capability::Schedulability
            && negotiated.disposition.target().is_some_and(|backend| {
                backend.id().as_str() == crate::zone_check::manifest::BACKEND_ID
            }) =>
        {
            fsl_route::Mode::Bounded
        }
        _ if item.kind == Capability::Schedulability => fsl_route::Mode::Unbounded,
        ExtentClassification::Bounded => fsl_route::Mode::Bounded,
        ExtentClassification::Unbounded { .. } => fsl_route::Mode::Unbounded,
    };
    if matches!(&item.form, Some(ItemForm::Hybrid(_))) {
        built = built.hybrid();
    }
    built.required(item.options.required).disposed(
        negotiated.disposition.clone(),
        negotiated.diagnostic.clone(),
    )
}

/// FR-940 "Claim occurrence ordinals": whether `claim` names a node an
/// authored temporal clause or probabilistic claim declares, at an
/// occurrence none of that node's declarations owns. Structurally equal
/// declarations share one node, so the whole key (node, `claim`, ordinal)
/// is matched against every declaration of the node, never the first.
fn names_other_occurrence(
    graph: &fsl_semantics::check::CheckedGraph,
    claim: &OccurrenceKey,
) -> bool {
    let temporal = || {
        graph
            .temporal_clauses()
            .iter()
            .filter(|clause| clause.identity() == *claim.node())
    };
    let probabilistic = || {
        graph
            .probabilistic_model()
            .claims()
            .iter()
            .filter_map(|checked| checked.occurrence())
            .filter(|checked| checked.node() == claim.node())
    };
    (temporal().next().is_some() && !temporal().any(|clause| clause.claim() == claim.origin()))
        || (probabilistic().next().is_some() && !probabilistic().any(|checked| checked == claim))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::SourceProvision;
    use fsl_foundation::source::provenance::RawSourceRef;

    #[trace("TC-2804", "FR-992-AC-1")]
    #[test]
    fn source_provision_entries_are_in_label_order() {
        let mut provision = SourceProvision::default();
        let z = RawSourceRef::new("b", "z").expect("valid source label");
        let a = RawSourceRef::new("a", "a").expect("valid source label");
        provision.insert(&z, b"z".to_vec());
        provision.insert(&a, b"a".to_vec());

        let entries: Vec<_> = provision.entries().collect();
        assert_eq!(entries, vec![("a", "a", &b"a"[..]), ("b", "z", &b"z"[..])]);
    }
}
