// SPDX-License-Identifier: AGPL-3.0-or-later
//! The operation-owned request boundary shared by analyze and the provider.

use std::collections::BTreeMap;

use qsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use qsl_foundation::{BackendId, RequestIndex};
use qsl_route::request::{ExtentClassification, ItemKind};
use qsl_route::{Candidate, EvidenceKind};
use qsl_semantics::check::Capability;

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
    pub const fn catalog_code(&self) -> quire_semantic_value::catalog::CatalogCode {
        quire_semantic_value::catalog::CatalogCode::new("missing_import", "missing-selection")
    }
}

/// Common options of one requested claim.
#[derive(Clone, Debug, Default)]
pub struct ItemOptions {
    /// Complete aggregate success requires this item.
    pub required: bool,
    /// Optional finite assignment/case budget of the native check arms.
    pub max_cases: Option<std::num::NonZeroU64>,
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

/// The request form of a kind that has one.
#[derive(Clone)]
pub enum ItemForm {
    /// An authored temporal clause.
    Temporal(TemporalItem),
    /// An exact or statistical probabilistic claim.
    Probabilistic(ProbabilisticItem),
    /// A hybrid-dynamics request; only an engine implementing this form runs it.
    Hybrid(HybridItem),
}

/// Hybrid item settings kept distinct from temporal and probabilistic forms.
#[derive(Clone, Debug, Default)]
pub struct HybridItem {
    /// The subject's explicit proof-bound substitution.
    pub proof_bounds: Vec<qsl_foundation::bound::ProofBound>,
}

/// Temporal request settings, with no separately supplied model subject.
#[derive(Clone)]
pub struct TemporalItem {
    /// The proof-bound substitution of the subject.
    pub proof_bounds: Vec<qsl_foundation::bound::ProofBound>,
    /// Formula or derived temporal item.
    pub item_kind: ItemKind,
    /// The abstract subject selected by a refinement item.
    pub abstract_subject: Option<qsl_route::request::AbstractSubject>,
    /// The resolved fairness constraints of the checked clause.
    pub fairness: Vec<quire_semantic_value::fairness::FairnessConstraint>,
    /// A requested stutter-closure premise, when supplied.
    pub stutter_closure: Option<bool>,
    /// The search horizon; no horizon when absent.
    pub max_depth: Option<usize>,
    /// The model search and automaton budgets.
    pub limits: qsl_replay::ModelCheckLimits,
    /// The most successor candidates one expansion enumerates.
    pub max_candidates: u64,
    /// The qualified checker's SMT encoding budgets.
    pub smt_encoding: Option<qsl_replay::SmtEncodingLimits>,
    /// The reductions the caller selected.
    pub reductions: qsl_route::reductions::Reductions,
}

impl Default for TemporalItem {
    fn default() -> Self {
        Self {
            proof_bounds: Vec::new(),
            item_kind: ItemKind::Formula,
            abstract_subject: None,
            fairness: Vec::new(),
            stutter_closure: None,
            max_depth: None,
            limits: qsl_replay::ModelCheckLimits::default(),
            max_candidates: super::clause_item::DEFAULT_MAX_CANDIDATES,
            smt_encoding: None,
            reductions: qsl_route::reductions::Reductions::default(),
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
    pub method: Option<qsl_replay::statistical::StatisticalMethod>,
    /// Statistical seed; absent for exact evidence.
    pub seed: Option<u64>,
    /// Statistical sampler; absent for exact evidence.
    pub sampler: Option<qsl_foundation::selection::DefinitionRef>,
    /// Maximum sampled witnesses retained.
    pub max_witnesses: Option<u32>,
    /// Declared statistical symmetries.
    pub symmetry: Vec<qsl_route::reductions::SymmetryDeclaration>,
    /// Timed statistical sampling settings.
    pub timed_sampling: Option<qsl_replay::statistical::TimedSampling>,
}

/// Kind-specific probabilistic budgets; the evidence and budget must agree.
#[derive(Clone, Copy)]
pub enum ProbabilisticLimits {
    /// Statistical sampling limits.
    Statistical(qsl_replay::statistical::StatisticalLimits),
    /// Exact product/solver/checker limits.
    Exact(crate::exact_probabilistic::ExactProbLimits),
}

/// A claim with the disposition and diagnostic supplied by negotiation.
#[derive(Clone)]
pub struct NegotiatedItem {
    /// The typed requested item.
    pub item: ClaimItem,
    /// The caller's negotiated disposition.
    pub disposition: qsl_route::routing::Disposition,
    /// The disposition's catalog diagnostic.
    pub diagnostic: Option<Diagnostic>,
}

/// Analyze negotiated items using only the admitted package, subject and supplied bytes.
///
/// # Errors
/// Returns the shared binding or missing-provision refusal before any item runs.
pub fn analyze(
    package: &qsl_package::CheckedPackage,
    request: &super::AnalyzeRequest<'_>,
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
    cancel: &quire_exact::Cancel,
) -> Result<
    qsl_foundation::diagnostic::Staged<super::AnalyzeOutcome>,
    qsl_foundation::diagnostic::StageFailure<super::AnalyzeRefusal>,
> {
    analyze_bound(
        package,
        &request.binding,
        &request.provision,
        &request.items,
        engines,
        limits,
        cancel,
    )
}

pub(crate) fn analyze_bound(
    package: &qsl_package::CheckedPackage,
    binding: &super::SubjectBinding<'_>,
    provision: &SourceProvision,
    requested: &[NegotiatedItem],
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
    cancel: &quire_exact::Cancel,
) -> Result<
    qsl_foundation::diagnostic::Staged<super::AnalyzeOutcome>,
    qsl_foundation::diagnostic::StageFailure<super::AnalyzeRefusal>,
> {
    use qsl_foundation::diagnostic::{StageFailure, Staged};
    let simulation = simulation(package, binding, limits);
    qsl_eval::simulation::model::ModelSystem::new(simulation)
        .map_err(|failure| StageFailure::Refused(super::AnalyzeRefusal::Binding(failure)))?;
    check_provision(package, provision)
        .map_err(|failure| StageFailure::Refused(super::AnalyzeRefusal::Provision(failure)))?;
    let items = requested
        .iter()
        .map(|item| work(package, binding, provision, item, limits))
        .collect::<Vec<_>>();
    let requirements = items
        .iter()
        .map(super::AnalyzeItem::requirement)
        .collect::<Vec<_>>();
    let mut settled = super::analyze_items(engines, items, cancel);
    let aggregate = super::aggregate(&requirements, &settled);
    let counterexamples = counterexamples(&mut settled);
    let records = super::terminal_records(settled);
    Ok(Staged::new(super::AnalyzeOutcome {
        records,
        counterexamples,
        aggregate,
    }))
}

fn check_provision(
    package: &qsl_package::CheckedPackage,
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

fn simulation<'a>(
    package: &'a qsl_package::CheckedPackage,
    binding: &super::SubjectBinding<'a>,
    limits: &super::AnalyzeLimits,
) -> qsl_eval::simulation::model::ModelSimulationRequest<'a> {
    qsl_eval::simulation::model::ModelSimulationRequest {
        package,
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: qsl_eval::simulation::model::ExpansionLimits {
            max_candidates: u64::MAX,
        },
        meter: limits.accounting,
        emit: limits.emit,
    }
}

fn work<'a>(
    package: &'a qsl_package::CheckedPackage,
    binding: &super::SubjectBinding<'a>,
    provision: &'a SourceProvision,
    negotiated: &'a NegotiatedItem,
    limits: &super::AnalyzeLimits,
) -> super::AnalyzeItem<'a> {
    use super::AnalyzeItem;
    let item = &negotiated.item;
    let temporal_kind = match &item.form {
        Some(ItemForm::Temporal(form)) => form.item_kind,
        Some(ItemForm::Probabilistic(_) | ItemForm::Hybrid(_)) | None => ItemKind::Formula,
    };
    let family = crate::claim_family::claim_family(package, temporal_kind, &item.claim);
    let wrong_occurrence = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *item.claim.node())
        .is_some_and(|clause| clause.claim() != item.claim.origin())
        || package
            .graph()
            .probabilistic_model()
            .claims()
            .iter()
            .filter_map(|claim| claim.occurrence())
            .find(|claim| claim.node() == item.claim.node())
            .is_some_and(|claim| claim != &item.claim);
    let family = if wrong_occurrence { None } else { family };
    let absent = || AnalyzeItem::unadapted(item.request_index, item.kind, family);
    let mut built =
        match &item.form {
            Some(ItemForm::Temporal(form)) if item.kind == Capability::TemporalSatisfaction => {
                match super::ClauseSubject::from_provision(
                    package,
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
                                subject: qsl_eval::model_subject::ModelSubject {
                                    simulation: simulation(package, binding, limits),
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
                    ) => match (
                        form.method,
                        form.seed,
                        &form.sampler,
                        form.max_witnesses,
                        form.timed_sampling,
                    ) {
                        (
                            Some(method),
                            Some(seed),
                            Some(sampler),
                            Some(max_witnesses),
                            Some(timed_sampling),
                        ) => super::statistical_item(
                            item.request_index,
                            crate::statistical::StatisticalRequest {
                                subject: qsl_eval::model_subject::ModelSubject {
                                    simulation: simulation(package, binding, limits),
                                    proof_bounds: Vec::new(),
                                },
                                claim,
                                method,
                                seed,
                                sampler: sampler.clone(),
                                limits: statistical,
                                max_witnesses,
                                symmetry: form.symmetry.clone(),
                                timed_sampling,
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
                        qsl_replay::value_check::ValueTarget::select(
                            package,
                            name.as_str(),
                            &item.claim,
                        )
                        .ok()
                    })
                    .ok_or(qsl_replay::value_check::Selection::NoForm)
                    .map(|target| crate::value_check::ValueRequest {
                        target,
                        max_cases: item.options.max_cases,
                        reported: None,
                    });
                super::value_item(item.request_index, package, &item.claim, selection)
            }
            None if item.kind == Capability::OperationContract => {
                let selection = qsl_replay::contract_check::select_contract(package, &item.claim)
                    .map(|claim| crate::contract_check::ContractRequest {
                        package,
                        claim,
                        subject: simulation(package, binding, limits),
                        abstract_subject: None,
                        max_cases: item.options.max_cases,
                    });
                super::contract_item(item.request_index, package, &item.claim, selection)
            }
            _ => absent(),
        };
    let expected = match &item.form {
        Some(ItemForm::Temporal(_)) => Some(crate::model_check::manifest::BACKEND_ID),
        Some(ItemForm::Probabilistic(form)) => match form.evidence {
            EvidenceKind::Exact => Some(crate::exact_probabilistic::manifest::BACKEND_ID),
            EvidenceKind::Statistical => Some(crate::statistical::manifest::BACKEND_ID),
        },
        Some(ItemForm::Hybrid(_)) => None,
        None => None,
    };
    if let (Some(expected), qsl_route::routing::Disposition::Supported(candidate)) =
        (expected, &negotiated.disposition)
    {
        if candidate.id().as_str() != expected {
            built.work = Some(Box::new(move |_, _| {
                super::EngineReport::Refused(super::Refusal::Unsupported(
                    quire_semantic_value::catalog::CatalogCode::new(
                        "unsupported_projection",
                        "unsupported-requested-capability",
                    ),
                ))
            }));
        }
    }
    built.kind = item.kind;
    built.family = family;
    built.mode = match item.extent {
        ExtentClassification::Bounded => qsl_route::Mode::Bounded,
        ExtentClassification::Unbounded { .. } => qsl_route::Mode::Unbounded,
    };
    built.required(item.options.required).disposed(
        negotiated.disposition.clone(),
        negotiated.diagnostic.clone(),
    )
}

fn counterexamples(records: &mut [super::AnalyzeRecord]) -> Vec<super::ItemCounterexample> {
    use super::{Counterexample, ItemCounterexample};
    let mut result = Vec::new();
    for record in records {
        let Some(terminal) = record.terminal() else {
            continue;
        };
        if !matches!(terminal.value(), qsl_replay::TerminalValue::Refuted) {
            continue;
        }
        let Some(backend) = terminal.backend().cloned() else {
            continue;
        };
        let counterexample = record
            .model_check
            .as_mut()
            .and_then(|model| model.counterexample.take())
            .map(Counterexample::Temporal)
            .or_else(|| {
                record
                    .zone_check
                    .as_mut()
                    .and_then(|zone| zone.counterexample.take())
                    .map(Counterexample::Temporal)
            })
            .or_else(|| {
                record
                    .protocol_check
                    .as_mut()
                    .and_then(|protocol| protocol.counterexample.take())
                    .map(Counterexample::Protocol)
            })
            .or_else(|| {
                record
                    .value_check
                    .take()
                    .map(|value| Counterexample::Assignments(*value))
            })
            .or_else(|| {
                record
                    .contract_check
                    .take()
                    .map(|contract| Counterexample::Contract(*contract))
            });
        if let Some(counterexample) = counterexample {
            result.push(ItemCounterexample {
                request_index: record.request_index,
                backend,
                counterexample,
            });
        }
    }
    result
}
