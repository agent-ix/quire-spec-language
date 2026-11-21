// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850, FR-940: the items of a `package` request, resolved against the
//! admitted package's name tables and settled over its package forms.
//!
//! A temporal clause is resolved by its `temporal_clause_names` row and run
//! over its package form ([`super::super::PackageClauseSubject`]); a
//! derived deadlock-freedom item is resolved by the identity of the model
//! selection it is over and run over the package route's model-check input
//! ([`super::super::PackageDeadlockSubject`]). Both replay a counterexample
//! from the package bytes and the handoff supply. They are negotiated,
//! echoed and written by the source route's own negotiation and writer, so
//! an item settles with the source route's record, counterexample, replay
//! and summary. An item whose family has no package form yet refuses the
//! request whole, typed and naming the family, never settling `unsupported`
//! for a fact the source route reads.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::model_subject::{CapacityBinding, RoleBinding};
use fsl_eval::simulation::model::{PackageModelInput, PendingPackageForm};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::{AnalysisView, BarePackage, CheckedNodeId, UnreadModelForm};
use fsl_replay::TerminalRecord;
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::{CandidateOutcome, Registry};
use fsl_semantics::check::{Capability, DeclarationFamily};
use serde_json::{json, Map};

use super::super::request::ProtocolBinding;
use super::super::{
    AnalyzeItem, AnalyzeLimits, ClaimItem, ClauseItemRefusal, ClauseLimits, Engines, ItemForm,
    ItemOptions, NegotiatedItem, PackageClauseSubject, PackageDeadlockSubject,
    PackageInvariantSubject, PackageItemInputs, PackageStateGraphSubject, SubjectBinding,
};
use super::{
    candidate_list, invalid_kind, negotiate_items, negotiated_routes, temporal_settings,
    unknown_claim, unknown_option, write_run, AnalyzeDocumentError, ItemReads, ItemSelection,
    Negotiate, Negotiation, ResolvedClaim, ResolvedItem, SettledParts, SettledRun,
    SubjectSelection,
};

/// A `package` request (FR-850) whose item names a family the package route
/// does not settle yet (FR-940 "Package forms of the source-checked
/// evaluators"): the request is refused whole rather than settle that item
/// `unsupported`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingPackageFamily {
    /// The request index of the first such item, `None` for the default
    /// items of a request with no `items` member and for a form the
    /// package's model-check input does not read.
    pub request_index: Option<RequestIndex>,
    /// What the item names.
    pub family: PendingFamily,
    /// The package node that carries the pending fact, when one does.
    pub node: Option<CheckedNodeId>,
    /// The words that name the claim a probabilistic refusal is at and, for
    /// an unresolved claim, why it does not resolve; `None` for every other
    /// family.
    pub note: Option<String>,
}

impl PendingPackageFamily {
    /// The node a probabilistic refusal is located at, in words: `None` for
    /// every other family.
    pub fn locus(&self) -> Option<String> {
        matches!(
            self.family,
            PendingFamily::ProbabilisticSourceModel | PendingFamily::ProbabilisticClaimUnresolved
        )
        .then(|| self.node.as_ref().map(node_words))
        .flatten()
    }

    /// The refusal's detail: the item, the family and the node.
    pub fn detail(&self) -> String {
        let at = self
            .node
            .as_ref()
            .map_or_else(String::new, |node| format!(" at {node:?}"));
        match (self.request_index, self.family) {
            (Some(index), PendingFamily::ProbabilisticSourceModel) => format!(
                ": items[{}] names the probabilistic claim {}, which cannot run on the package route: the model has source objects and the package rows carry no effects, fields or populations to step it{}",
                index.get(),
                self.note.as_deref().unwrap_or_default(),
                self.locus()
                    .map_or_else(String::new, |locus| format!(" at node `{locus}`"))
            ),
            (Some(index), PendingFamily::ProbabilisticClaimUnresolved) => format!(
                ": items[{}] names the probabilistic claim {}, which does not resolve against the package's source objects{}",
                index.get(),
                self.note.as_deref().unwrap_or_default(),
                self.locus()
                    .map_or_else(String::new, |locus| format!(" at node `{locus}`"))
            ),
            (Some(index), PendingFamily::ClauseFact(fact)) => format!(
                ": items[{}] names a temporal clause whose {fact} has no package form yet{at}",
                index.get()
            ),
            (Some(index), family) => format!(
                ": items[{}] names the {} family, which has no package form yet{at}",
                index.get(),
                family.name()
            ),
            (None, PendingFamily::ModelForm(form)) => format!(
                ": the package's {} form has no package form in the model-check input yet{at}",
                form.name()
            ),
            (None, family) => format!(": the {} rule has no package form yet{at}", family.name()),
        }
    }
}

/// The words a claim node is located by: `package/claim/probabilistic_claim/name`.
fn node_words(node: &CheckedNodeId) -> String {
    use filament_contract_model::CheckedNodePathSegment;
    let path: Vec<&str> = node
        .path
        .iter()
        .take(3)
        .filter_map(|segment| match segment {
            CheckedNodePathSegment::Name(name) => Some(&**name),
            _ => None,
        })
        .collect();
    format!("{}/{}", node.package, path.join("/"))
}

/// What a pending package-route item names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingFamily {
    /// A claimless `simulation` item (FR-634).
    Simulation,
    /// A derived item other than a model subject's deadlock-freedom item
    /// (FSpec FR-368).
    Derived,
    /// FR-730's invariant item, pending the package wire contract.
    LoopInvariant,
    /// FR-730's termination item, pending the package wire contract.
    LoopTermination,
    /// A named declaration.
    Declaration,
    /// A name the package carries no row for while it holds a claim whose
    /// name it does not carry ([`fsl_package::AnalysisView::unnamed_claims`]):
    /// the name may be that claim's, so it is not settled `unknown-claim`.
    UnnamedClaim,
    /// A temporal clause fact its package form does not read yet.
    ClauseFact(&'static str),
    /// A form of the package its model-check input does not read yet.
    ModelForm(UnreadModelForm),
    /// A probabilistic claim over a source-object model (FSL FR-740 "Source
    /// objects", FR-940-AC-6): its probabilistic model rebuilds from the
    /// package's rows, but those rows carry no effect frame, field or
    /// population, so no engine can step the model and the claim cannot run.
    ProbabilisticSourceModel,
    /// A probabilistic claim over a source-object model whose probabilistic
    /// model does not rebuild from the package's rows.
    ProbabilisticClaimUnresolved,
}

impl PendingFamily {
    /// The family's name in a refusal message.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Simulation => "simulation",
            Self::Derived => "derived",
            Self::LoopInvariant => "loop-invariant",
            Self::LoopTermination => "loop-termination",
            Self::Declaration => "declaration",
            Self::UnnamedClaim => "unnamed-claim",
            Self::ClauseFact(fact) => fact,
            Self::ModelForm(_) => "model-form",
            Self::ProbabilisticSourceModel => "probabilistic-source-model",
            Self::ProbabilisticClaimUnresolved => "probabilistic-claim-unresolved",
        }
    }
}

/// Why a `package` request settled no document.
#[derive(Debug, thiserror::Error)]
pub enum PackageRunError {
    /// An item, or the package, names a family with no package form yet.
    #[error("unsupported_construct/not-yet-implemented{}", .0.detail())]
    Pending(PendingPackageFamily),
    /// The run was refused or faulted as the source route's is.
    #[error(transparent)]
    Document(#[from] AnalyzeDocumentError),
}

/// What one `package` request (FR-850) settles over.
pub struct PackageRun<'a> {
    /// The admitted package with the handoff's identity and equality rows
    /// attached (FR-531).
    pub package: &'a BarePackage<'a>,
    /// The admitted `checked-package.json` bytes, which replay reads.
    pub package_bytes: &'a [u8],
    /// The handoff's identity and equality file bytes.
    pub supply: fsl_replay::HandoffSupply<'a>,
    /// The request's default subject.
    pub subject: SubjectSelection,
    /// The request's shared subject binding.
    pub binding: SubjectBinding<'a>,
    /// The request's items; `None` for the default items.
    pub items: Option<&'a [ItemSelection]>,
    /// The engines `analyze` may run.
    pub engines: &'a Engines,
    /// The shared limits.
    pub limits: AnalyzeLimits,
}

/// One item as the package's name tables resolve it.
enum Named<'s> {
    Settled(ResolvedItem),
    Clause {
        index: RequestIndex,
        name: &'s str,
        claim: OccurrenceKey,
        selection: &'s ItemSelection,
    },
    Deadlock {
        index: RequestIndex,
        name: &'s str,
        selection: &'s ItemSelection,
    },
    ProtocolFreedom {
        index: RequestIndex,
        name: &'s str,
        protocol: String,
        selection: &'s ItemSelection,
    },
    StateClause {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
    StateGraph {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
}

/// How a resolved package-route claim item runs.
enum Route<'a> {
    Clause {
        subject: Box<PackageClauseSubject<'a>>,
        protocol: Option<ProtocolBinding>,
    },
    ProtocolFreedom(ProtocolBinding),
    Deadlock,
    StateClause,
    StateGraph(Box<PackageStateGraphSubject<'a>>),
}

/// An authored clause and the optional protocol subject it runs over.
struct ClauseRoute<'a> {
    subject: Box<PackageClauseSubject<'a>>,
    protocol: Option<ProtocolBinding>,
}

/// FR-850, FR-940: resolve `run`'s items against the package's name tables,
/// negotiate them with `negotiate`, run them over their package forms under
/// `cancel` and settle FR-621's document with the source route's writer.
///
/// # Errors
///
/// [`PackageRunError::Pending`] for the first item whose family has no
/// package form yet, and for a package whose model-check input meets a form
/// it does not read yet, before any engine runs;
/// [`PackageRunError::Document`] as [`super::settle_run`] refuses or faults.
pub fn settle_package_run(
    run: PackageRun<'_>,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<SettledRun, PackageRunError> {
    let PackageRun {
        package,
        package_bytes,
        supply,
        subject,
        binding,
        items,
        engines,
        limits,
    } = run;
    let defaults = items.is_none().then(|| default_selections(package));
    let items: &[ItemSelection] = items.or(defaults.as_deref()).unwrap_or_default();
    if let Some(refusal) =
        probabilistic_source_refusal(package, binding.packages, limits.normalization, items)?
    {
        return Err(PackageRunError::Pending(refusal));
    }
    let named = resolve_names(package, items)?;
    let runnable = named
        .iter()
        .any(|named| !matches!(named, Named::Settled(_)));
    // The input is built only for an item that runs, so a request whose
    // items all settle by name settles as the source route's does.
    let input = runnable
        .then(|| PackageModelInput::of(package, binding.packages, limits.normalization))
        .transpose()
        .map_err(|refusal| match refusal {
            PendingPackageForm::Form { form, node } => {
                PackageRunError::Pending(PendingPackageFamily {
                    request_index: None,
                    note: None,
                    family: PendingFamily::ModelForm(form),
                    node: Some(node),
                })
            }
            PendingPackageForm::World(_)
            | PendingPackageForm::Model(_)
            | PendingPackageForm::Operations(_)
            | PendingPackageForm::ProbabilisticNodes(_)
            | PendingPackageForm::Probabilistic(_)
            | PendingPackageForm::Reward(_)
            // Only `source_object_probabilistic_model` reads the source-object
            // rows, so `of` never gives this one.
            | PendingPackageForm::SourceModel(_) => {
                PackageRunError::Document(AnalyzeDocumentError::Fault)
            }
        })?;
    let mut registry = Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .map_err(|_| AnalyzeDocumentError::Fault)?;
    }
    let reads = ClaimReads {
        input: input.as_ref(),
        package_bytes,
        supply,
        binding: &binding,
        limits,
        subject: &subject,
        registry: &registry,
    };
    let mut resolved = Vec::with_capacity(named.len());
    let mut routes: BTreeMap<RequestIndex, Route<'_>> = BTreeMap::new();
    let mut owners: BTreeMap<OccurrenceKey, Option<String>> = BTreeMap::new();
    let mut clause_claims: Vec<OccurrenceKey> = Vec::new();
    let mut invariant_claims: Vec<OccurrenceKey> = Vec::new();
    let mut graph_claims: Vec<OccurrenceKey> = Vec::new();
    for item in named {
        match item {
            Named::Settled(settled) => resolved.push(settled),
            Named::Clause {
                index,
                name,
                claim,
                selection,
            } => match reads.clause(index, name, &claim, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ClauseRoute { subject, protocol }) => {
                    let occurrence = subject.occurrence().clone();
                    owners.insert(
                        occurrence.clone(),
                        owner_of_clause(package, subject.clause()),
                    );
                    clause_claims.push(occurrence);
                    routes.insert(index, Route::Clause { subject, protocol });
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Deadlock {
                index,
                name,
                selection,
            } => match reads.deadlock(index, name, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ()) => {
                    routes.insert(index, Route::Deadlock);
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::ProtocolFreedom {
                index,
                name,
                protocol,
                selection,
            } => match reads.protocol_freedom(index, name, &protocol, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, protocol) => {
                    routes.insert(index, Route::ProtocolFreedom(protocol));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::StateClause {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.state_clause(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ()) => {
                    invariant_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::StateClause);
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::StateGraph {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.state_graph(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, subject) => {
                    let occurrence = claim.item.claim.clone();
                    owners.insert(occurrence.clone(), owner_of_graph(package, subject.claim()));
                    graph_claims.push(occurrence);
                    routes.insert(index, Route::StateGraph(subject));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
        }
    }
    // A default derived item is named by no claim (FR-620), as the source
    // route writes it; an authored one is named by its model's identity.
    if defaults.is_some() {
        for (selection, item) in items.iter().zip(resolved.iter_mut()) {
            let derived = selection
                .temporal
                .as_ref()
                .is_some_and(|temporal| temporal.item_kind.is_some());
            if let (true, ResolvedItem::Claim(claim)) = (derived, item) {
                claim.claim_name.clear();
            }
        }
    }
    let model_input = input.as_ref().map(PackageModelInput::input);
    let admit = |item: &mut ClaimItem| match &model_input {
        Some(model) => super::admit_reductions(model, &binding, &limits, item),
        None => Ok(()),
    };
    // As the source route's owning model: a clause's `over` object type's
    // package, else the package's one model selection.
    let owner = |claim: &OccurrenceKey| {
        owners
            .get(claim)
            .cloned()
            .unwrap_or_else(|| sole_model(package))
    };
    let family = |kind: ItemKind, claim: &OccurrenceKey| match kind {
        ItemKind::Formula if invariant_claims.contains(claim) => {
            Some(DeclarationFamily::StateModel)
        }
        ItemKind::Formula => (clause_claims.contains(claim) || graph_claims.contains(claim))
            .then_some(DeclarationFamily::TemporalTrace),
        other => crate::claim_family::kind_family(other),
    };
    let Negotiation {
        negotiated,
        unrouted_items,
        settled,
        echoes,
        simulations: _,
    } = negotiate_items(
        resolved,
        negotiate,
        &ItemReads {
            admit: &admit,
            owner: &owner,
            family: &family,
        },
    )?;
    let mut analyzed = Vec::with_capacity(negotiated.len());
    if let (Some(input), Some(model)) = (input.as_ref(), model_input.as_ref()) {
        // FR-624: the shared subject binding is admitted before any item
        // runs, as on the source route.
        fsl_eval::simulation::model::ModelSystem::new(super::super::request::simulation(
            model, &binding, &limits,
        ))
        .map_err(|failure| {
            AnalyzeDocumentError::Refused(super::super::AnalyzeRefusal::Binding(failure))
        })?;
        let work = WorkInputs {
            input,
            package_bytes,
            supply,
            binding: &binding,
            limits,
            family: &family,
        };
        for item in &negotiated {
            let route = routes.remove(&item.item.request_index);
            analyzed.push(work.item(route, item));
        }
    }
    let records = super::super::analyze_items(engines, analyzed, cancel);
    let details = records.iter().map(super::super::ItemDetail::of).collect();
    let protocol_conformance = records
        .iter()
        .filter_map(|record| {
            record
                .protocol_conformance
                .clone()
                .map(|output| (record.request_index, output))
        })
        .collect();
    let (records, counterexamples) = super::super::split_records_over(records, None);
    let (targets, item_routes) = negotiated_routes(&negotiated, &unrouted_items, engines);
    Ok(write_run(
        SettledParts {
            records,
            counterexamples,
            details,
            protocol_conformance,
        },
        &settled,
        &echoes,
        &targets,
        &item_routes,
        &[],
        engines,
    )?)
}

/// The model package owning a temporal clause: its `over` object type's
/// package, else the package's one model selection.
fn owner_of_clause(
    package: &BarePackage<'_>,
    clause: &fsl_semantics::check::CheckedTemporalClause,
) -> Option<String> {
    match &clause.over().1 {
        filament_exact::ValueType::Reference(object_type) => {
            Some(object_type.original().package().to_owned())
        }
        _ => sole_model(package),
    }
}

/// The model package owning a state-graph claim: its `over` object type's
/// package, else the package's one model selection.
fn owner_of_graph(
    package: &BarePackage<'_>,
    claim: &fsl_semantics::check::CheckedStateGraphClaim,
) -> Option<String> {
    match claim.over() {
        Some(over) => Some(over.object.original().package().to_owned()),
        None => sole_model(package),
    }
}

/// Whether the package node `node` is a `claim`/`state_graph` node (FR-410).
fn is_state_graph(view: &dyn AnalysisView, node: &CheckedNodeId) -> bool {
    view.package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .is_some_and(|candidate| {
            &*candidate.node_tag == "claim" && candidate.semantic_form.as_ref() == "state_graph"
        })
}

/// The package's one model selection, when it has exactly one.
fn sole_model(package: &BarePackage<'_>) -> Option<String> {
    match AnalysisView::model_selections(package).as_slice() {
        [selection] => Some(selection.identity.clone()),
        _ => None,
    }
}

/// FSL FR-740, FR-940-AC-6: the located refusal of the first item that names
/// a probabilistic claim of a package with source objects.
///
/// The package's source-object rows rebuild the probabilistic model
/// ([`PackageModelInput::source_object_probabilistic_model`]) but carry no
/// effect frame, field or population, so no engine steps the model and no
/// probabilistic item runs: a claim whose model rebuilds is refused
/// [`PendingFamily::ProbabilisticSourceModel`], and one whose model does not
/// [`PendingFamily::ProbabilisticClaimUnresolved`], each at the claim's node.
/// `None` for a package with no source objects and for a request that names
/// no probabilistic claim, whose items settle as every other item does. A
/// package fault the model-check input would give is the same fault here.
fn probabilistic_source_refusal(
    package: &BarePackage<'_>,
    packages: &fsl_semantics::model::intake::PackageInput,
    normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
    items: &[ItemSelection],
) -> Result<Option<PendingPackageFamily>, PackageRunError> {
    if !matches!(
        package.first_unread_model_form(),
        Some((UnreadModelForm::SourceObject, _))
    ) {
        return Ok(None);
    }
    let view: &dyn AnalysisView = package;
    let declared = view.declared_names();
    let Some((index, name, node)) = items.iter().enumerate().find_map(|(position, item)| {
        let name = item.claim.as_deref()?;
        let node = declared
            .iter()
            .find(|(declared, _)| declared == name)
            .map(|(_, node)| (*node).clone())
            .or_else(|| view.claim_name(name).map(|found| found.declaration))?;
        view.package()
            .probabilistic_claims()
            .iter()
            .any(|claim| same_claim_node(&claim.node, &node))
            .then(|| (RequestIndex::new(position), name, node))
    }) else {
        return Ok(None);
    };
    let (family, note) = match PackageModelInput::source_object_probabilistic_model(
        package,
        packages,
        normalization,
    ) {
        Ok(_) => (PendingFamily::ProbabilisticSourceModel, format!("`{name}`")),
        Err(refusal) => match unresolved_reason(&refusal) {
            Some(reason) => (
                PendingFamily::ProbabilisticClaimUnresolved,
                format!("`{name}` ({reason})"),
            ),
            // The same faults `PackageModelInput::of` gives settle as a
            // fault, not as a gap of the route.
            None => return Err(PackageRunError::Document(AnalyzeDocumentError::Fault)),
        },
    };
    Ok(Some(PendingPackageFamily {
        request_index: Some(index),
        note: Some(note),
        family,
        node: Some(node),
    }))
}

/// Whether two ids name one claim node: the read of a node's path spells its
/// operands by index in one place and by node in another, so ids compare by
/// the package, ordinal and the leading `claim/probabilistic_claim/<name>`
/// segments, which no operand spelling reaches.
fn same_claim_node(left: &CheckedNodeId, right: &CheckedNodeId) -> bool {
    left.package == right.package
        && left.ordinal == right.ordinal
        && left.path.iter().take(3).eq(right.path.iter().take(3))
}

/// Why a package's probabilistic model does not rebuild, in words, for a
/// refusal that is a gap of the route; `None` for a fault of the package
/// (an unreadable environment, domain packages not admitted as a model,
/// untyped operations or unreadable probabilistic nodes), which
/// [`settle_package_run`] settles as a fault as it does for `of`.
fn unresolved_reason(refusal: &PendingPackageForm) -> Option<String> {
    match refusal {
        PendingPackageForm::Form { .. } => {
            Some("the package holds a form its model-check input does not read yet".to_owned())
        }
        PendingPackageForm::SourceModel(fault) => Some(fault.to_string()),
        PendingPackageForm::Probabilistic(_) => Some(
            "the package's random parameters and workloads do not rebuild against its model"
                .to_owned(),
        ),
        PendingPackageForm::Reward(_) => {
            Some("a reward's clause function does not project from the package".to_owned())
        }
        PendingPackageForm::World(_)
        | PendingPackageForm::Model(_)
        | PendingPackageForm::Operations(_)
        | PendingPackageForm::ProbabilisticNodes(_) => None,
    }
}

/// FR-850 "Building the analysis input": each item resolved by name against
/// the package, read only through the view (FR-940). A temporal clause is
/// named by its `temporal_clause_names` row; a derived deadlock-freedom item
/// by the identity of the model selection the package records. A name
/// neither carries settles `invalid-request` `invalid_capability`/
/// `unknown-claim` as the source route settles it, but only when the
/// package holds no claim whose name it does not carry (an `invariant`
/// state clause, a refinement, a state-graph claim): the name could be that
/// claim's.
fn resolve_names<'s>(
    view: &dyn AnalysisView,
    selections: &'s [ItemSelection],
) -> Result<Vec<Named<'s>>, PackageRunError> {
    let pending = |request_index, family| {
        PackageRunError::Pending(PendingPackageFamily {
            request_index,
            note: None,
            family,
            node: None,
        })
    };
    let clauses = view.clause_names();
    let declared = view.declared_names();
    let unnamed = !view.unnamed_claims().is_empty();
    let models: Vec<String> = view
        .model_selections()
        .into_iter()
        .map(|selection| selection.identity)
        .collect();
    selections
        .iter()
        .enumerate()
        .map(|(position, selection)| {
            let index = RequestIndex::new(position);
            let pending = |family| pending(Some(index), family);
            let item_kind = selection
                .temporal
                .as_ref()
                .and_then(|temporal| temporal.item_kind)
                .map(super::ItemKindSelection::kind);
            // FR-730 runs from checked source. Its loop item kinds have no
            // admitted package wire contract yet, regardless of the claim.
            match item_kind {
                Some(ItemKind::LoopInvariant) => return Err(pending(PendingFamily::LoopInvariant)),
                Some(ItemKind::LoopTermination) => {
                    return Err(pending(PendingFamily::LoopTermination))
                }
                Some(
                    ItemKind::Formula
                    | ItemKind::DeadlockFreedom
                    | ItemKind::TimeLockFreedom
                    | ItemKind::RaceFreedom
                    | ItemKind::Refinement,
                )
                | None => {}
            }
            let Some(name) = selection.claim.as_deref() else {
                return Err(pending(PendingFamily::Simulation));
            };
            let derived = item_kind.is_some_and(|kind| kind != ItemKind::Formula);
            if item_kind == Some(ItemKind::DeadlockFreedom)
                && selection.kind.as_deref() == Some(Capability::TemporalSatisfaction.to_wire())
                && view.protocol_execution_named(name).ok().flatten().is_some()
            {
                return Ok(Named::ProtocolFreedom {
                    index,
                    name,
                    protocol: name.to_owned(),
                    selection,
                });
            }
            if item_kind == Some(ItemKind::RaceFreedom) {
                if let Some(protocol) = protocol_parallel_name(view, name) {
                    return Ok(Named::ProtocolFreedom {
                        index,
                        name,
                        protocol,
                        selection,
                    });
                }
            }
            if let Some(row) = clauses.iter().find(|row| row.name.as_ref() == name) {
                if derived {
                    return Err(pending(PendingFamily::Derived));
                }
                let key =
                    fsl_package::checked_views::temporal_subject(view.package(), &row.declaration)
                        .ok()
                        .and_then(|subject| subject.declaration_key())
                        .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
                // W58: the declaration is its own `claim` occurrence of a
                // node structurally equal clauses may share.
                let claim = OccurrenceKey::new(
                    key,
                    filament_exact::Origin::new(filament_exact::Role::new("claim"), row.occurrence),
                );
                return Ok(Named::Clause {
                    index,
                    name,
                    claim,
                    selection,
                });
            }
            // FR-850-AC-7, FR-940-AC-7: any other claim by its `claim_names`
            // row. An `invariant` state clause runs over the package route's
            // input; a claim of another family has no package form yet.
            if let Some(found) = view.claim_name(name) {
                if derived {
                    return Err(pending(PendingFamily::Derived));
                }
                if is_state_graph(view, &found.declaration) {
                    return Ok(Named::StateGraph {
                        index,
                        name,
                        declaration: found.declaration,
                        ordinal: found.occurrence.ordinal,
                        selection,
                    });
                }
                if !is_state_clause(view, &found.declaration) {
                    return Err(pending(PendingFamily::Declaration));
                }
                return Ok(Named::StateClause {
                    index,
                    name,
                    declaration: found.declaration,
                    ordinal: found.occurrence.ordinal,
                    selection,
                });
            }
            if declared.iter().any(|(declared, _)| declared == name) {
                return Err(pending(PendingFamily::Declaration));
            }
            if derived {
                let selected = models.iter().any(|identity| identity == name);
                return match (item_kind, selected, models.len()) {
                    (Some(ItemKind::DeadlockFreedom), true, 1) => Ok(Named::Deadlock {
                        index,
                        name,
                        selection,
                    }),
                    (_, true, _) => Err(pending(PendingFamily::Derived)),
                    (_, false, _) => Ok(Named::Settled(model_alias(index, name, &models))),
                };
            }
            if unnamed {
                return Err(pending(PendingFamily::UnnamedClaim));
            }
            Ok(Named::Settled(unknown_claim(index, name)))
        })
        .collect()
}

/// The protocol name of `name` when it names one of the package protocol's
/// memory-governed parallel controls (`Protocol::path`).
fn protocol_parallel_name(view: &dyn AnalysisView, name: &str) -> Option<String> {
    let (protocol, path) = name.split_once("::")?;
    let form = view.protocol_execution_named(protocol).ok().flatten()?;
    form.controls
        .iter()
        .any(|control| {
            control.operation == fsl_package::protocol_view::ControlOperation::Parallel
                && control.path.join("::") == path
        })
        .then(|| protocol.to_owned())
}

/// The checked protocol binding carried by an item subject, if it is a
/// protocol subject. `None` distinguishes a model subject; malformed
/// capacities or role bindings name the member for the request's ordinary
/// invalid-value disposition.
fn protocol_binding(
    subject: &SubjectSelection,
    input: &PackageModelInput<'_>,
) -> Result<Option<ProtocolBinding>, &'static str> {
    let Some(selection) = subject.protocol.as_ref() else {
        return Ok(None);
    };
    let capacities = selection
        .capacities
        .iter()
        .map(|(name, value)| {
            value.parse().ok().map(|value| CapacityBinding {
                name: name.clone(),
                value,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or("capacities")?;
    let memory: Vec<_> = subject
        .memory
        .iter()
        .filter(|entry| entry.parallel.split("::").next() == Some(selection.protocol.as_str()))
        .cloned()
        .collect();
    let effective = input
        .package()
        .protocol_execution_named(&selection.protocol)
        .ok()
        .flatten()
        .and_then(|form| {
            fsl_eval::simulation::protocol::ResolvedMemory::resolve_package(&form, &memory).ok()
        })
        .map(|resolved| {
            resolved
                .parallels()
                .iter()
                .map(|parallel| (parallel.parallel.clone(), parallel.model))
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(ProtocolBinding {
        protocol: selection.protocol.clone(),
        over: selection.over.as_ref().map(|over| {
            (
                over.reference.population.clone(),
                over.reference.key.clone(),
            )
        }),
        roles: selection
            .roles
            .iter()
            .map(|(role, binding)| RoleBinding {
                role: role.clone(),
                model: binding.selection.clone(),
            })
            .collect(),
        capacities,
        memory,
        effective,
    }))
}

/// Whether the package node `node` names a state clause. The kind determines
/// whether FR-489 routes it to `contract-checker` or EN-1.
fn is_state_clause(view: &dyn AnalysisView, node: &CheckedNodeId) -> bool {
    view.package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .is_some_and(|candidate| candidate.semantic_form.as_ref() == "state_clause")
}

/// FR-850: a derived item whose `claim` names no claim row and no model
/// identity the package records settles `invalid-request`
/// `invalid_capability`/`model-alias`, naming the claim and every recorded
/// identity in model-selection order.
fn model_alias(index: RequestIndex, name: &str, identities: &[String]) -> ResolvedItem {
    let mut detail = Map::new();
    detail.insert("name".to_owned(), json!(name));
    detail.insert("identities".to_owned(), json!(identities));
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_capability", "model-alias"),
            detail,
        )),
    }
}

/// What a resolved claim reads beside its name.
enum Resolution<R> {
    Settled(ResolvedItem),
    Claim(Box<ResolvedClaim>, R),
}

/// What the package route's claim resolution reads.
struct ClaimReads<'r, 'a> {
    input: Option<&'r PackageModelInput<'a>>,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
    binding: &'r SubjectBinding<'a>,
    limits: AnalyzeLimits,
    subject: &'r SubjectSelection,
    registry: &'r Registry,
}

impl<'r, 'a: 'r> ClaimReads<'r, 'a> {
    /// The item's own subject over the request's, including a protocol
    /// subject selected by this item.
    fn own_subject(
        &self,
        _index: RequestIndex,
        selection: &ItemSelection,
    ) -> Result<SubjectSelection, PackageRunError> {
        let own = selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.subject.clone())
            .map_or_else(
                || self.subject.clone(),
                |own| own.completed_by(self.subject),
            );
        Ok(own)
    }

    /// FR-620's item over the temporal clause `name` at `key`, as the
    /// source route's `resolve_one` resolves an authored clause: its claim
    /// occurrence from the clause's name row, the record's kind, extent and
    /// candidates from the package form's claim.
    fn clause(
        &self,
        index: RequestIndex,
        name: &str,
        claim: &OccurrenceKey,
        selection: &ItemSelection,
    ) -> Result<Resolution<ClauseRoute<'r>>, PackageRunError> {
        let input = self
            .input
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let own_subject = self.own_subject(index, selection)?;
        let temporal = selection.temporal.clone().unwrap_or_default();
        let protocol = match protocol_binding(&own_subject, input) {
            Ok(protocol) => protocol,
            Err(member) => {
                return Ok(Resolution::Settled(super::invalid_value(
                    index, name, member,
                )));
            }
        };
        if temporal
            .item_kind
            .is_some_and(|kind| kind.kind() != ItemKind::Formula)
            && protocol.is_none()
        {
            return Err(PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: None,
                family: PendingFamily::Derived,
                node: None,
            }));
        }
        let mut form = temporal_settings(&temporal);
        let clause_subject = PackageClauseSubject::new(
            input,
            self.package_bytes,
            self.supply,
            self.binding.clone(),
            self.limits,
            claim,
            ClauseLimits {
                limits: form.limits.limits,
                max_depth: form.max_depth,
                max_candidates: form.max_candidates,
            },
        )
        .map_err(|refusal| match refusal {
            ClauseItemRefusal::Pending { fact, node } => {
                PackageRunError::Pending(PendingPackageFamily {
                    request_index: Some(index),
                    note: None,
                    family: PendingFamily::ClauseFact(fact),
                    node: Some(node),
                })
            }
            ClauseItemRefusal::UnknownClause | ClauseItemRefusal::UnnamedClause => {
                PackageRunError::Document(AnalyzeDocumentError::Fault)
            }
        })?;
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )));
                }
            },
            // A temporal clause's one record is `temporal-satisfaction`.
            None => Capability::TemporalSatisfaction,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(DeclarationFamily::TemporalTrace) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let clause = clause_subject.clause();
        let record = kind == Capability::TemporalSatisfaction;
        let (extent, candidates) = if record {
            (
                clause_extent(clause),
                self.registry.restrict_to_timing(
                    fsl_route::request::requirement_candidates(
                        &clause
                            .claim()
                            .requirements_over(fsl_semantics::family::ClaimExtent::Bounded),
                        self.registry,
                        named.as_ref(),
                    ),
                    false,
                ),
            )
        } else {
            (
                ExtentClassification::Bounded,
                tie(
                    self.registry.candidates(kind, named.as_ref()),
                    kind,
                    named.as_ref(),
                ),
            )
        };
        form.fairness = clause.fairness().to_vec();
        form.pending_reductions.clone_from(&selection.reductions);
        form.protocol.clone_from(&protocol);
        let item_form = (kind == Capability::TemporalSatisfaction
            || kind == Capability::GlobalConformance)
            .then(|| ItemForm::Temporal(form));
        let item = ClaimItem {
            request_index: index,
            claim: clause_subject.occurrence().clone(),
            kind,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: item_form,
        };
        let unrouted = selection.unrouted()
            || (selection.reductions.is_some() && kind != Capability::TemporalSatisfaction)
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            ClauseRoute {
                subject: Box::new(clause_subject),
                protocol,
            },
        ))
    }

    /// A protocol's deadlock-freedom item or a parallel's race-freedom item,
    /// anchored at that protocol's declaration occurrence.
    fn protocol_freedom(
        &self,
        index: RequestIndex,
        name: &str,
        protocol_name: &str,
        selection: &ItemSelection,
    ) -> Result<Resolution<ProtocolBinding>, PackageRunError> {
        let input = self
            .input
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let own_subject = self.own_subject(index, selection)?;
        if selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.item_kind)
            == Some(super::ItemKindSelection::DeadlockFreedom)
            && !own_subject
                .protocol
                .as_ref()
                .is_some_and(|selected| selected.protocol == protocol_name)
        {
            return Ok(Resolution::Settled(super::invalid_value(
                index,
                name,
                "temporal.subject.protocol",
            )));
        }
        let Some(selected) = own_subject.protocol.as_ref() else {
            let mut detail = Map::new();
            detail.insert("member".to_owned(), json!("temporal.subject.protocol"));
            return Ok(Resolution::Settled(ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(
                    TerminalRecord::disposition_declined(
                        index,
                        fsl_replay::ProofRefusalCause::InvalidInput,
                        fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                        CatalogCode::new("invalid_runtime_input", "missing-member"),
                    )
                    .with_diagnostic_detail(detail),
                ),
            }));
        };
        if selected.protocol != protocol_name {
            return Ok(Resolution::Settled(unknown_claim(index, name)));
        }
        let protocol = match protocol_binding(&own_subject, input) {
            Ok(Some(protocol)) => protocol,
            Ok(None) => return Ok(Resolution::Settled(unknown_claim(index, name))),
            Err(member) => {
                return Ok(Resolution::Settled(super::invalid_value(
                    index, name, member,
                )));
            }
        };
        let Some(form) = input
            .package()
            .protocol_execution_named(protocol_name)
            .ok()
            .flatten()
        else {
            return Ok(Resolution::Settled(unknown_claim(index, name)));
        };
        let key = input
            .package()
            .node_key(form.protocol.clause())
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let claim = OccurrenceKey::declaration(key);
        let temporal = selection.temporal.clone().unwrap_or_default();
        let form = temporal_settings(&temporal);
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let candidates = self.registry.restrict_to_timing(
            self.registry
                .candidates(Capability::TemporalSatisfaction, named.as_ref()),
            false,
        );
        let mut temporal_form = form;
        temporal_form.protocol = Some(protocol.clone());
        let extent = if temporal_form.item_kind == ItemKind::DeadlockFreedom {
            ExtentClassification::Bounded
        } else {
            ExtentClassification::Unbounded {
                finite_bound_available: false,
            }
        };
        let item = ClaimItem {
            request_index: index,
            claim: claim.clone(),
            kind: Capability::TemporalSatisfaction,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: Some(ItemForm::Temporal(temporal_form)),
        };
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject.clone(),
                unrouted: selection.unrouted() || !own_subject.same_model(self.subject),
            }),
            protocol,
        ))
    }

    /// FR-620's item over the state-graph claim `name`, whose `claim_names`
    /// row is on `declaration` at `claim` occurrence `ordinal`, as the source
    /// route's `resolve_one` resolves it: the record's extent and candidates
    /// from the package form's requirement record, narrowed to the claim's
    /// form (FR-628).
    fn state_graph(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<Box<PackageStateGraphSubject<'r>>>, PackageRunError> {
        let fault = || PackageRunError::Document(AnalyzeDocumentError::Fault);
        let input = self.input.ok_or_else(fault)?;
        let key = input.package().node_key(declaration).ok_or_else(fault)?;
        let claim = OccurrenceKey::new(
            key.clone(),
            filament_exact::Origin::new(filament_exact::Role::new("claim"), ordinal),
        );
        let own_subject = self.own_subject(index, selection)?;
        let temporal = selection.temporal.clone().unwrap_or_default();
        let mut form = temporal_settings(&temporal);
        let subject = PackageStateGraphSubject::new(
            input,
            self.package_bytes,
            self.supply,
            self.binding.clone(),
            self.limits,
            &claim,
        )
        .map_err(|refusal| {
            PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: None,
                family: refusal
                    .pending_fact()
                    .map_or(PendingFamily::Declaration, |(fact, _)| {
                        PendingFamily::ClauseFact(fact)
                    }),
                node: Some(declaration.clone()),
            })
        })?;
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )));
                }
            },
            None => Capability::TemporalSatisfaction,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(DeclarationFamily::TemporalTrace) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let record = kind == Capability::TemporalSatisfaction;
        let (extent, candidates) = if record {
            (
                ExtentClassification::Unbounded {
                    finite_bound_available: false,
                },
                self.registry.restrict_to_state_graph_form(
                    self.registry.restrict_to_timing(
                        fsl_route::request::requirement_candidates(
                            &subject.claim().claim().requirements(key),
                            self.registry,
                            named.as_ref(),
                        ),
                        false,
                    ),
                    fsl_route::StateGraphFormKind::of(subject.claim().form()),
                ),
            )
        } else {
            (
                ExtentClassification::Bounded,
                // FR-962: `resolve_tie` is the identity on every kind but
                // `operation-contract`, which excludes EN-1 for a claim that
                // is no `invariant` state clause.
                tie(
                    self.registry.candidates(kind, named.as_ref()),
                    kind,
                    named.as_ref(),
                ),
            )
        };
        if let Some(refused) = super::sampling_refusal(index, name, subject.claim(), &temporal) {
            return Ok(Resolution::Settled(refused));
        }
        form.pending_reductions.clone_from(&selection.reductions);
        let item_form = (kind == Capability::TemporalSatisfaction
            || kind == Capability::GlobalConformance)
            .then(|| ItemForm::Temporal(form));
        let item = ClaimItem {
            request_index: index,
            claim,
            kind,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: item_form,
        };
        let unrouted = selection.unrouted_beyond_sampling()
            || (selection.reductions.is_some() && kind != Capability::TemporalSatisfaction)
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            Box::new(subject),
        ))
    }

    /// FR-626's derived deadlock-freedom item over the one model selection,
    /// whose identity `name` is, as the source route's `resolve_derived`
    /// resolves it: at the selection's `model_root` anchor with the
    /// `generated` occurrence 0.
    fn deadlock(
        &self,
        index: RequestIndex,
        name: &str,
        selection: &ItemSelection,
    ) -> Result<Resolution<()>, PackageRunError> {
        let temporal = selection.temporal.clone().unwrap_or_default();
        if temporal.max_depth.is_some() {
            // FSpec FR-368-AC-20: a derived item carries no `max_depth`.
            let mut detail = Map::new();
            detail.insert("member".to_owned(), json!("temporal.max_depth"));
            return Ok(Resolution::Settled(ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(TerminalRecord::disposition_invalid_request(
                    index,
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    detail,
                )),
            }));
        }
        let root = fsl_semantics::check::model_root_key(
            &fsl_semantics::model::domain_package::DomainPackageRef {
                identity: name.to_owned(),
            },
        )
        .map_err(|_| AnalyzeDocumentError::Fault)?;
        let claim = OccurrenceKey::new(
            root,
            filament_exact::Origin::new(filament_exact::Role::new("generated"), 0),
        );
        let own_subject = self.own_subject(index, selection)?;
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let candidates = self.registry.restrict_to_timing(
            self.registry
                .candidates(Capability::TemporalSatisfaction, named.as_ref()),
            false,
        );
        let mut form = temporal_settings(&temporal);
        form.pending_reductions.clone_from(&selection.reductions);
        let item = ClaimItem {
            request_index: index,
            claim,
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: named,
            candidates: candidate_list(&candidates),
            options: ItemOptions {
                required: selection.required.unwrap_or(true),
                max_cases: None,
                max_task_product_work: None,
                max_check_work: None,
                trace: None,
                traces: selection
                    .traces
                    .as_ref()
                    .map(super::DocumentSelection::reference),
                recording: None,
                population: None,
                backend_options,
                protocol_analysis: None,
            },
            form: Some(ItemForm::Temporal(form)),
        };
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                unrouted: selection.unrouted()
                    || reads_no_clocks(selection)
                    || !own_subject.same_model(self.subject),
                subject: own_subject,
            }),
            (),
        ))
    }

    /// FR-625's item over the `invariant` state clause `name`, whose
    /// `claim_names` row is on `declaration`, as the source route's
    /// `resolve_one` resolves it: at the clause's node and `claim`
    /// occurrence, kind `operation-contract` unless the item names another,
    /// and, over a model subject, FR-489's tie narrowed to EN-1 (FR-962). An
    /// item with no model subject is decided by EN-2's contract check, which
    /// has no package form yet.
    fn state_clause(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<()>, PackageRunError> {
        let pending = |family| {
            PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: None,
                family,
                node: Some(declaration.clone()),
            })
        };
        let input = self
            .input
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let key = input
            .package()
            .node_key(declaration)
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let origin = filament_exact::Origin::new(filament_exact::Role::new("claim"), ordinal);
        let clause = input
            .state_clause(&key, &origin)
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let claim = OccurrenceKey::new(clause.identity.clone(), clause.claim.clone());
        // FR-290: the operation-contract record's extent from its domain
        // kinds, as S3 classifies the selected state clause.
        let kinds = input
            .invariant_domain_kinds(
                clause,
                filament_semantic_value::checking::CheckingLimits::default().nodes(),
            )
            .ok_or_else(|| pending(PendingFamily::Declaration))?;
        let extent = if kinds.is_empty() {
            ExtentClassification::Bounded
        } else {
            ExtentClassification::Unbounded {
                finite_bound_available: kinds.iter().all(|kind| kind.finite_kind().is_some()),
            }
        };
        let own_subject = self.own_subject(index, selection)?;
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )));
                }
            },
            None => Capability::OperationContract,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(DeclarationFamily::StateModel) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let model_subject = !own_subject.initial_states.is_empty();
        if kind == Capability::OperationContract && !model_subject {
            return Err(pending(PendingFamily::Declaration));
        }
        let candidates = state_clause_tie(
            self.registry.candidates(kind, named.as_ref()),
            kind,
            named.as_ref(),
            model_subject,
            clause.invariant,
        );
        let temporal = selection.temporal.clone().unwrap_or_default();
        let item = ClaimItem {
            request_index: index,
            claim,
            kind,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: None,
        };
        let unrouted = selection.unrouted()
            || selection.reductions.is_some()
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            (),
        ))
    }
}

/// FR-962's operation-contract tie rule for a package state clause. EN-1
/// decides an invariant over a model subject; pre/post clauses go to
/// `contract-checker` regardless of whether the subject has a model.
fn state_clause_tie(
    candidates: CandidateOutcome,
    kind: Capability,
    named: Option<&BackendId>,
    model_subject: bool,
    invariant: bool,
) -> CandidateOutcome {
    if named.is_some() || kind != Capability::OperationContract {
        return candidates;
    }
    if !model_subject || !invariant {
        return fsl_route::tie::exclude_en1(candidates, kind, named, false);
    }
    let CandidateOutcome::Candidates(set) = &candidates else {
        return candidates;
    };
    let mut members: Vec<&str> = set
        .candidates()
        .iter()
        .map(|candidate| candidate.id().as_str())
        .collect();
    members.sort_unstable();
    let mut tied = [
        crate::contract_check::manifest::BACKEND_ID,
        crate::model_check::manifest::BACKEND_ID,
    ];
    tied.sort_unstable();
    if members != tied {
        return candidates;
    }
    match set.narrowed_to(&BackendId::new(crate::model_check::manifest::BACKEND_ID)) {
        Some(narrowed) => CandidateOutcome::Candidates(narrowed),
        None => candidates,
    }
}

/// FR-620's item options of `selection`.
fn options(selection: &ItemSelection, backend_options: BTreeMap<String, String>) -> ItemOptions {
    ItemOptions {
        required: selection.required.unwrap_or(true),
        max_cases: selection.check.map(|check| check.limits.max_cases),
        max_task_product_work: selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.limits)
            .and_then(|limits| limits.max_task_product_work),
        max_check_work: selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.limits)
            .and_then(|limits| limits.max_check_work),
        trace: selection
            .trace
            .as_ref()
            .map(super::DocumentSelection::reference),
        traces: selection
            .traces
            .as_ref()
            .map(super::DocumentSelection::reference),
        recording: selection
            .recording
            .as_ref()
            .map(super::DocumentSelection::reference),
        population: selection.population.clone(),
        backend_options,
        protocol_analysis: None,
    }
}

/// An untimed subject reads no clock limit; the package route's
/// model-check input reads no timed model, so its subject is untimed.
fn reads_no_clocks(selection: &ItemSelection) -> bool {
    selection
        .temporal
        .as_ref()
        .and_then(|temporal| temporal.limits)
        .is_some_and(|limits| limits.max_clocks.is_some())
}

/// FR-290: the extent classification of a package-form clause's record,
/// from the kinds of its domains. The populations its `holds` atoms name
/// are population domains, which have a finite bound kind, and a package
/// form is under the infinite-trace profile, whose formula domain makes the
/// extent unbounded already, so they do not change it.
fn clause_extent(clause: &fsl_semantics::check::CheckedTemporalClause) -> ExtentClassification {
    let kinds = clause.claim().domain_kinds();
    if kinds.is_empty() {
        return ExtentClassification::Bounded;
    }
    ExtentClassification::Unbounded {
        finite_bound_available: kinds.iter().all(|kind| kind.finite_kind().is_some()),
    }
}

/// FR-962's tie rule for an item over a temporal clause: EN-1 is excluded
/// from an unnamed `operation-contract` item, since a temporal clause is no
/// `invariant` state clause, and the rule selects no provider for it.
fn tie(
    candidates: CandidateOutcome,
    kind: Capability,
    named: Option<&BackendId>,
) -> CandidateOutcome {
    if named.is_some() || kind != Capability::OperationContract {
        return candidates;
    }
    fsl_route::tie::exclude_en1(candidates, kind, named, false)
}

/// What every package-route analysis item reads.
struct WorkInputs<'w, 'a> {
    input: &'a PackageModelInput<'a>,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
    binding: &'w SubjectBinding<'a>,
    limits: AnalyzeLimits,
    family: &'w dyn Fn(ItemKind, &OccurrenceKey) -> Option<DeclarationFamily>,
}

impl<'a> WorkInputs<'_, 'a> {
    /// The analysis item of the negotiated `negotiated` on the package
    /// route, as the source route's `work` builds it: a temporal clause
    /// over its package form, a derived deadlock-freedom item over the
    /// package route's input, and any other kind its unadapted item.
    fn item(&self, route: Option<Route<'a>>, negotiated: &NegotiatedItem) -> AnalyzeItem<'a> {
        let item = &negotiated.item;
        let temporal_kind = match &item.form {
            Some(ItemForm::Temporal(form)) => form.item_kind,
            Some(ItemForm::FiniteHyper(_) | ItemForm::Probabilistic(_) | ItemForm::Hybrid(_))
            | None => ItemKind::Formula,
        };
        let family = (self.family)(temporal_kind, &item.claim);
        let mut built = match (&item.form, route) {
            (
                Some(ItemForm::Temporal(form)),
                Some(Route::Clause {
                    subject,
                    protocol: Some(protocol),
                }),
            ) if item.kind == Capability::TemporalSatisfaction => subject
                .configured(form.clone())
                .protocol_item(item.request_index, protocol),
            (
                Some(ItemForm::Temporal(form)),
                Some(Route::Clause {
                    subject,
                    protocol: None,
                }),
            ) if item.kind == Capability::TemporalSatisfaction => {
                subject.configured(form.clone()).item(item.request_index)
            }
            (Some(ItemForm::Temporal(form)), Some(Route::ProtocolFreedom(protocol)))
                if item.kind == Capability::TemporalSatisfaction =>
            {
                super::super::clause_item::package_protocol_freedom_item(
                    self.input,
                    self.package_bytes,
                    self.supply,
                    self.binding.clone(),
                    self.limits,
                    super::super::clause_item::PackageProtocolSelection {
                        claim: &item.claim,
                        form,
                        protocol,
                        index: item.request_index,
                    },
                )
            }
            (Some(ItemForm::Temporal(form)), Some(Route::Deadlock))
                if item.kind == Capability::TemporalSatisfaction
                    && form.item_kind == ItemKind::DeadlockFreedom =>
            {
                PackageDeadlockSubject::new(
                    self.input,
                    self.package_bytes,
                    self.supply,
                    self.binding.clone(),
                    self.limits,
                    &item.claim,
                    form,
                )
                .item(item.request_index)
            }
            (Some(ItemForm::Temporal(form)), Some(Route::StateGraph(subject)))
                if item.kind == Capability::TemporalSatisfaction =>
            {
                subject.configured(form.clone()).item(item.request_index)
            }
            // FR-489's tie routed the invariant over the model subject to
            // EN-1 (FR-625, FR-951), as on the source route.
            (_, Some(Route::StateClause))
                if item.kind == Capability::OperationContract
                    && negotiated.disposition.target().is_some_and(|backend| {
                        backend.id().as_str() == crate::model_check::manifest::BACKEND_ID
                    }) =>
            {
                let form = match &item.form {
                    Some(ItemForm::Temporal(form)) => form.clone(),
                    Some(
                        ItemForm::FiniteHyper(_) | ItemForm::Probabilistic(_) | ItemForm::Hybrid(_),
                    )
                    | None => super::super::TemporalItem::default(),
                };
                PackageInvariantSubject::new(
                    PackageItemInputs {
                        input: self.input,
                        package_bytes: self.package_bytes,
                        supply: self.supply,
                        binding: self.binding.clone(),
                        limits: self.limits,
                    },
                    &item.claim,
                    &form,
                )
                .map_or_else(
                    || AnalyzeItem::unadapted(item.request_index, item.kind, family),
                    |subject| subject.item(item.request_index),
                )
            }
            _ => AnalyzeItem::unadapted(item.request_index, item.kind, family),
        };
        let expected = match &item.form {
            _ if item.kind == Capability::GlobalConformance => {
                Some(crate::protocol_analysis::manifest::BACKEND_ID)
            }
            Some(ItemForm::Temporal(_)) => Some(crate::model_check::manifest::BACKEND_ID),
            Some(ItemForm::FiniteHyper(_) | ItemForm::Probabilistic(_) | ItemForm::Hybrid(_))
            | None => None,
        };
        if let (Some(expected), fsl_route::routing::Disposition::Supported(candidate)) =
            (expected, &negotiated.disposition)
        {
            if candidate.id().as_str() != expected {
                built.work = Some(super::super::ItemWork::Engine(Box::new(move |_, _| {
                    super::super::EngineReport::Refused(super::super::Refusal::Unsupported(
                        fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                    ))
                })));
            }
        }
        built.kind = item.kind;
        built.family = family;
        built.claim = Some(item.claim.clone());
        built.mode = match item.extent {
            ExtentClassification::Bounded => fsl_route::Mode::Bounded,
            ExtentClassification::Unbounded { .. } => fsl_route::Mode::Unbounded,
        };
        built.required(item.options.required).disposed(
            negotiated.disposition.clone(),
            negotiated.diagnostic.clone(),
        )
    }
}

/// FR-620 "Subject, compilation and default items" over a package, as the
/// source route's `default_items` writes them: one item per kind each claim
/// admits, in source order (a unit's position in the package's source list,
/// then the start byte of the claim's declaration span), then each task-set
/// schedulability claim, then the derived deadlock-freedom item of each model
/// selection. Each item carries the published default of every setting.
fn default_selections(view: &dyn AnalysisView) -> Vec<ItemSelection> {
    use filament_contract_model::CheckedSourceRegion;

    let units = &view.package().lock().sources;
    let span = |region: &CheckedSourceRegion| {
        (
            units
                .iter()
                .position(|unit| {
                    unit.authority == region.source.authority
                        && unit.identity == region.source.identity
                })
                .unwrap_or(usize::MAX),
            region.start,
        )
    };
    let mut claims: Vec<((usize, u64), String, Vec<Capability>)> = view
        .clause_names()
        .iter()
        .map(|row| {
            (
                span(&row.declaration_region),
                row.name.to_string(),
                vec![Capability::TemporalSatisfaction],
            )
        })
        .collect();
    let mut schedulability: Vec<((usize, u64), String)> = Vec::new();
    let mut declarations: Vec<&CheckedNodeId> = Vec::new();
    for row in view.claim_names() {
        if declarations.contains(&&row.declaration) {
            continue;
        }
        declarations.push(&row.declaration);
        let name = row.qualified_name.join("::");
        let at = span(&row.declaration_region);
        if is_schedulability(view, &row.declaration) {
            schedulability.push((at, name));
            continue;
        }
        let kinds = if is_state_graph(view, &row.declaration) {
            vec![Capability::TemporalSatisfaction]
        } else if is_state_clause(view, &row.declaration) {
            vec![Capability::OperationContract]
        } else {
            Vec::new()
        };
        claims.push((at, name, kinds));
    }
    claims.sort_by_key(|(left, ..)| *left);
    schedulability.sort_by_key(|(left, _)| *left);
    let item = |name: String, kind: Option<Capability>| ItemSelection {
        claim: Some(name),
        kind: kind.map(|kind| kind.to_wire().to_owned()),
        ..ItemSelection::default()
    };
    let mut items: Vec<ItemSelection> = Vec::new();
    for (_, name, kinds) in claims {
        if kinds.is_empty() {
            // A claim form with no package record of its kinds yet: the
            // name alone, which the name tables settle.
            items.push(item(name, None));
            continue;
        }
        for kind in Capability::ALL.iter().filter(|kind| kinds.contains(kind)) {
            items.push(item(name.clone(), Some(*kind)));
        }
    }
    items.extend(
        schedulability
            .into_iter()
            .map(|(_, name)| item(name, Some(Capability::Schedulability))),
    );
    items.extend(
        view.model_selections()
            .into_iter()
            .map(|selection| ItemSelection {
                claim: Some(selection.identity),
                temporal: Some(super::TemporalSelection {
                    item_kind: Some(super::ItemKindSelection::DeadlockFreedom),
                    ..super::TemporalSelection::default()
                }),
                ..ItemSelection::default()
            }),
    );
    items
}

/// Whether the package node `node` is a `claim`/`schedulability` node (FR-246).
fn is_schedulability(view: &dyn AnalysisView, node: &CheckedNodeId) -> bool {
    view.package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .is_some_and(|candidate| {
            &*candidate.node_tag == "claim" && candidate.semantic_form.as_ref() == "schedulability"
        })
}
