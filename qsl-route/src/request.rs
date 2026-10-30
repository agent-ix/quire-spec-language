// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-20, ADR-014 §4 and §6 step 3: QSL's typed request
//! representation and its writer.
//!
//! The writer turns one checked item's [`Requirements`] into one requested
//! item: its capability kind, its FR-331 extent classification and, for a
//! bounded request, the proof bounds substituted for its unbounded domains.
//! QSL computes the classification; CG's `negotiate_*` reads it
//! (`ExtentClassification.finite_bound_available`) and computes none of it.
//!
//! **Available finite bound** (ADR-014 §4). For an unbounded item, a finite
//! bound is available exactly when every unbounded domain is boundable. No
//! stage limit, accounting limit, profile ceiling, default or registered
//! backend makes one available. With it, a bounded-only candidate settles
//! `requires-bound`; without it, `unsupported`/`unbounded-extent`.
//!
//! **Bounded request.** The caller answers `requires-bound` with a new item
//! that carries one [`FiniteBound`] per unbounded domain
//! ([`RequestWriter::bounded_item`]). The writer substitutes each one, so the
//! new item's extent is bounded. Its proof bounds are part of its identity,
//! and it has its own [`RequestIndex`]. The unbounded item keeps its own
//! index and classification, so a bounded result never settles it and
//! there is no `requires-bound` loop.
//!
//! **Requested items from a package's requirement records** (FR-075).
//! [`items_from_requirements`] writes one [`RequirementItem`] per record of
//! a checked package, in the records' occurrence-key order, each with its
//! candidate outcome. The driver writes a `requires-bound` item's bounded
//! follow-up as a new request of one item, carrying the original item's
//! occurrence key: [`RequirementItem::bounded`] substitutes the caller's
//! bounds by the same rule as [`RequestWriter::bounded_item`], and the
//! follow-up's [`RequirementItem::proof_bounds`] carries them with QSL's
//! domain kinds. A temporal item's subject proof bounds are a separate,
//! caller-supplied substitution ([`RequirementItem::with_subject_proof_bounds`],
//! read by [`RequirementItem::temporal_proof_bounds`]).
//!
//! **Refusals.** Before any item is written, the writer refuses a bound for
//! a key that names no unbounded domain of the item, a bound on a domain no
//! finite bound can stand for, a bound of the wrong kind, and a set that
//! misses a domain. An empty or inverted range cannot be built at all
//! ([`FiniteBound`]'s constructors refuse it). Two bounds for one domain
//! cannot be passed: the bounds are a map keyed by [`DomainKey`]. Every
//! refusal is `invalid_runtime_input`/`invalid-value`.

use std::collections::BTreeMap;

use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, FiniteBoundKind, ProofBound};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{ReductionKind, RequestIndex};
use qsl_semantics::check::{Capability, PathGuard, RequirementRecord};
use qsl_semantics::family::{ClaimExtent, Requirements, UnsupportedItem};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::observation::DocumentRef;
use quire_semantic_value::catalog::{CatalogCode, CatalogCoded};

use crate::reductions::Reductions;
use crate::{CandidateOutcome, CandidateSet, EvidenceKind, ItemEvidence, Registry};
use qsl_foundation::BackendId;

/// An item's QSpec FR-331 extent classification (QSpec FR-290 `bounded`/
/// `unbounded`, plus ADR-014 §4's available finite bound).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExtentClassification {
    /// No unbounded domain, or every one substituted by a proof bound.
    Bounded,
    /// At least one unbounded domain.
    Unbounded {
        /// Whether every unbounded domain is boundable (ADR-014 §4).
        finite_bound_available: bool,
    },
}

impl ExtentClassification {
    /// The FR-290 extent spelling: `bounded` or `unbounded`.
    pub const fn to_wire(self) -> &'static str {
        match self {
            Self::Bounded => "bounded",
            Self::Unbounded { .. } => "unbounded",
        }
    }

    /// The classification of an item with `extent` and no proof bounds.
    fn of(extent: &ClaimExtent) -> Self {
        match extent {
            ClaimExtent::Bounded => Self::Bounded,
            ClaimExtent::Unbounded(domains) => Self::Unbounded {
                finite_bound_available: domains.all_boundable(),
            },
        }
    }
}

/// QSpec FR-368 "Item kinds": the `item_kind` of a `temporal-satisfaction`
/// item over a model subject. These five are the only kinds: a transfer
/// item (QSpec FR-380) is evaluated by the verification planner and is no
/// request item, so it has none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ItemKind {
    /// An authored temporal clause.
    Formula,
    /// The derived deadlock-freedom item (FR-124).
    DeadlockFreedom,
    /// The derived time-lock-freedom item (FR-232).
    TimeLockFreedom,
    /// The derived race-freedom item (FR-224).
    RaceFreedom,
    /// A refinement item, one per checked refinement declaration (FR-144).
    Refinement,
}

impl ItemKind {
    /// Every kind, in FR-368's table order.
    pub const ALL: [Self; 5] = [
        Self::Formula,
        Self::DeadlockFreedom,
        Self::TimeLockFreedom,
        Self::RaceFreedom,
        Self::Refinement,
    ];

    /// The FR-368 `item_kind` spelling.
    pub const fn to_wire(self) -> &'static str {
        match self {
            Self::Formula => "formula",
            Self::DeadlockFreedom => "deadlock-freedom",
            Self::TimeLockFreedom => "time-lock-freedom",
            Self::RaceFreedom => "race-freedom",
            Self::Refinement => "refinement",
        }
    }
}

/// QSpec FR-368 `abstract_subject`, present exactly on a `refinement` item
/// (QSpec FR-377 "Inputs"): the abstract subject's package, a package of the
/// request's `dependencies`, and its ordered initial-state snapshots. Its
/// universes are fixed by FR-377 and it has no proof bounds, so it carries
/// neither.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractSubject {
    /// The abstract subject's package id.
    pub package_id: PackageId,
    /// The abstract subject's initial-state snapshots, in order.
    pub initial_states: Vec<DocumentRef>,
}

/// A limit a derived item may carry a caller's setting of (QSpec FR-368
/// "Negotiation of a derived or refinement item"): a limit that bounds an
/// exhaustive check. A derived item has no formula, no witness sampling and
/// no hyperproperty, so `max_automaton_states`, `max_walk_steps`,
/// `max_witness_set` and `max_relation_tuples` are not among them.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DerivedLimit {
    /// `max_states`, the explored graph's states.
    MaxStates,
    /// `max_transitions`, the explored graph's transitions.
    MaxTransitions,
    /// `max_clocks`, the live clocks of a timed state (FR-415).
    MaxClocks,
    /// `max_store_buffer`, a weak `parallel`'s store buffer (FR-439).
    MaxStoreBuffer,
    /// `max_messages`, a weak `parallel`'s messages (FR-439).
    MaxMessages,
    /// `max_live_instances`, an `on each` subject's instances (FR-428).
    MaxLiveInstances,
    /// `max_solver_steps`, an SMT query's step budget.
    MaxSolverSteps,
    /// `max_solver_time`, an SMT query's time budget in seconds.
    MaxSolverTime,
}

impl DerivedLimit {
    /// Every limit a derived item may carry.
    pub const ALL: [Self; 8] = [
        Self::MaxStates,
        Self::MaxTransitions,
        Self::MaxClocks,
        Self::MaxStoreBuffer,
        Self::MaxMessages,
        Self::MaxLiveInstances,
        Self::MaxSolverSteps,
        Self::MaxSolverTime,
    ];

    /// The limit's member name in a derived item's `limits`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::MaxStates => "max_states",
            Self::MaxTransitions => "max_transitions",
            Self::MaxClocks => "max_clocks",
            Self::MaxStoreBuffer => "max_store_buffer",
            Self::MaxMessages => "max_messages",
            Self::MaxLiveInstances => "max_live_instances",
            Self::MaxSolverSteps => "max_solver_steps",
            Self::MaxSolverTime => "max_solver_time",
        }
    }
}

/// A name that is no member of a derived item's `limits` (QSpec
/// FR-368-AC-20): `invalid_runtime_input`/`unknown-member`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a derived item's limits have no member `{name}`")]
pub struct UnknownDerivedLimit {
    /// The name as the caller wrote it.
    pub name: String,
}

impl std::str::FromStr for DerivedLimit {
    type Err = UnknownDerivedLimit;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|limit| limit.name() == name)
            .ok_or_else(|| UnknownDerivedLimit {
                name: name.to_owned(),
            })
    }
}

impl CatalogCoded for UnknownDerivedLimit {
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("invalid_runtime_input", "unknown-member")
    }

    /// FR-096's key table has no row for `unknown-member`.
    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        None
    }
}

/// The caller's settings a derived item carries in its `limits` (QSpec
/// FR-368 "Negotiation of a derived or refinement item"): only a limit the
/// caller set, and only a [`DerivedLimit`]. A limit the caller did not set
/// is absent, and the routed provider's published default applies. The
/// request has no request-level `limits`, so nothing is inherited from
/// another item.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedLimits {
    set: BTreeMap<DerivedLimit, u64>,
}

impl DerivedLimits {
    /// The caller's settings `settings`, by member name.
    ///
    /// # Errors
    ///
    /// [`UnknownDerivedLimit`] for the first name that is no
    /// [`DerivedLimit`].
    pub fn from_settings<'n>(
        settings: impl IntoIterator<Item = (&'n str, u64)>,
    ) -> Result<Self, UnknownDerivedLimit> {
        let mut set = BTreeMap::new();
        for (name, value) in settings {
            set.insert(name.parse()?, value);
        }
        Ok(Self { set })
    }

    /// The caller's setting of `limit`; `None` when the provider's default
    /// applies.
    pub fn get(&self, limit: DerivedLimit) -> Option<u64> {
        self.set.get(&limit).copied()
    }

    /// Each setting, in [`DerivedLimit`] order.
    pub fn iter(&self) -> impl Iterator<Item = (DerivedLimit, u64)> + '_ {
        self.set.iter().map(|(limit, value)| (*limit, *value))
    }
}

/// What a derived item carries that an item written from a record does
/// not: its candidates, computed by the writer since it has no record
/// (FR-243), and the caller's [`DerivedLimits`].
#[derive(Clone, Debug, Eq, PartialEq)]
struct DerivedPart {
    candidates: CandidateOutcome,
    limits: DerivedLimits,
}

/// One requested item (ADR-013 O-20): the occurrence it requests a claim
/// at, the checked node, its capability kind, its extent classification
/// and the proof bounds it was requested over. Two items are equal only
/// when every member is, so an item with proof bounds is never equal to
/// the unbounded item it answers.
///
/// A derived item (QSpec FR-368 "Claim and anchor") has no requirement
/// record and no authored claim: its occurrence is its subject's anchor,
/// and the writer gives it its candidates and its caller's limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestItem {
    index: RequestIndex,
    occurrence: OccurrenceKey,
    kind: Capability,
    hybrid_dynamics: bool,
    extent: ExtentClassification,
    domains: Vec<ProofBound>,
    unsupported: Option<UnsupportedItem>,
    reductions: Vec<ReductionKind>,
    property_form: Option<quire_semantic_value::fairness::PropertyForm>,
    item_kind: Option<ItemKind>,
    abstract_subject: Option<AbstractSubject>,
    derived: Option<DerivedPart>,
}

impl RequestItem {
    /// The FR-368 `item_kind` of a temporal item over a model subject;
    /// `None` for an item written with no model subject.
    pub fn item_kind(&self) -> Option<ItemKind> {
        self.item_kind
    }

    /// The abstract subject of a `refinement` item; `None` for every other
    /// item (QSpec FR-368-AC-19).
    pub fn abstract_subject(&self) -> Option<&AbstractSubject> {
        self.abstract_subject.as_ref()
    }

    /// A derived item's candidates, computed by the writer (FR-243: a timed
    /// subject's to the backends deciding timed items); `None` for an
    /// item whose candidates come from its record.
    pub fn candidates(&self) -> Option<&CandidateOutcome> {
        self.derived.as_ref().map(|derived| &derived.candidates)
    }

    /// A derived item's caller-set limits; `None` for an item that is not
    /// derived.
    pub fn derived_limits(&self) -> Option<&DerivedLimits> {
        self.derived.as_ref().map(|derived| &derived.limits)
    }

    /// The property form its record carries (FR-144: a refinement's TP-5
    /// form); `None` for an item whose form is its checked node's.
    pub fn property_form(&self) -> Option<quire_semantic_value::fairness::PropertyForm> {
        self.property_form
    }

    /// The reductions its request selects, in the order symmetry,
    /// partial-order, constraint (FR-159 Inputs), which negotiation routes
    /// by ([`crate::Registry::restrict_to_reductions`]); empty for an
    /// unreduced request.
    pub fn reductions(&self) -> &[ReductionKind] {
        &self.reductions
    }

    /// This item's own request index.
    pub fn index(&self) -> RequestIndex {
        self.index
    }

    /// The occurrence of the claim site the item requests a claim at.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The checked node the item requests a claim over: its occurrence's
    /// node.
    pub fn node(&self) -> WireNodeId {
        self.occurrence.node()
    }

    /// The requested capability kind.
    pub fn kind(&self) -> Capability {
        self.kind
    }

    /// The FR-331 `requirements` member. Empty for an ordinary item and
    /// exactly `hybrid-dynamics` for a checked hybrid claim.
    pub fn requirements(&self) -> &'static [&'static str] {
        if self.hybrid_dynamics {
            &["hybrid-dynamics"]
        } else {
            &[]
        }
    }

    /// The FR-331 extent classification.
    pub fn extent(&self) -> ExtentClassification {
        self.extent
    }

    /// The proof bounds substituted for the item's unbounded domains, in
    /// [`DomainKey`] order (QSpec FR-331 `extent.bounds`). Empty for an
    /// item requested without bounds.
    pub fn domains(&self) -> &[ProofBound] {
        &self.domains
    }

    /// The cause the writer disposed the item `Unsupported` for, routing it
    /// to no engine (FR-234 "Punctuality"); `None` for an item negotiation
    /// settles.
    pub fn unsupported(&self) -> Option<&UnsupportedItem> {
        self.unsupported.as_ref()
    }
}

/// Why [`RequestWriter::bounded_item`] refused its bounds (ADR-014 §4).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BoundRefusal {
    /// A bound's key names no unbounded domain of the item.
    #[error("no unbounded domain {domain}")]
    UnknownDomain {
        /// The supplied key.
        domain: DomainKey,
    },
    /// A bound was given for a domain no finite bound can stand for.
    #[error("domain {domain} ({kind:?}) takes no finite bound")]
    UnboundableDomain {
        /// The domain.
        domain: DomainKey,
        /// Its kind.
        kind: DomainKind,
    },
    /// A bound's variant does not match its domain's kind.
    #[error("domain {domain} takes a {expected:?} bound, not {supplied:?}")]
    KindMismatch {
        /// The domain.
        domain: DomainKey,
        /// The variant the domain takes.
        expected: FiniteBoundKind,
        /// The variant supplied.
        supplied: FiniteBoundKind,
    },
    /// An unbounded domain has no bound in the set.
    #[error("no bound for unbounded domain {domain}")]
    MissingDomain {
        /// The domain.
        domain: DomainKey,
    },
    /// The item is bounded, so no bounded request exists for it.
    #[error("the item has no unbounded domain")]
    NoUnboundedDomain,
    /// Subject proof bounds were supplied for an item that is not a
    /// `temporal-satisfaction` item over a model subject, so it has no
    /// subject (QSpec FR-331, FR-368).
    #[error("the item is not a temporal item over a model subject")]
    NoModelSubject,
    /// A subject proof bound's key is not a population: a subject's proof
    /// bounds substitute its own populations (QSpec FR-331).
    #[error("subject proof bound domain {domain} is not a population")]
    NotAPopulation {
        /// The supplied key.
        domain: DomainKey,
    },
}

impl CatalogCoded for BoundRefusal {
    /// Every case is `invalid_runtime_input`/`invalid-value` (ADR-014 §4).
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::UnknownDomain { .. }
            | Self::UnboundableDomain { .. }
            | Self::KindMismatch { .. }
            | Self::MissingDomain { .. }
            | Self::NoModelSubject
            | Self::NotAPopulation { .. }
            | Self::NoUnboundedDomain => CatalogCode::new("invalid_runtime_input", "invalid-value"),
        }
    }

    /// FR-096's key table has no row for `invalid-value`.
    fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
        None
    }
}

/// Writes the items of one request, giving each its own [`RequestIndex`] in
/// write order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RequestWriter {
    items: Vec<RequestItem>,
    /// The reductions the request selects, one selection for every item
    /// (FR-159 Inputs).
    reductions: Reductions,
}

impl RequestWriter {
    /// An empty request.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty request selecting `reductions` (FR-159 Inputs): every item
    /// it writes carries the selection.
    pub fn reduced(reductions: Reductions) -> Self {
        Self {
            items: Vec::new(),
            reductions,
        }
    }

    /// The reductions the request selects.
    pub fn reductions(&self) -> &Reductions {
        &self.reductions
    }

    /// Write the item for the claim at `occurrence` with
    /// `requirements` and no proof bounds. Its classification comes from
    /// the checked extent alone.
    pub fn item(&mut self, occurrence: OccurrenceKey, requirements: &Requirements) -> RequestIndex {
        let extent = ExtentClassification::of(requirements.extent());
        self.push(Written::plain(occurrence, extent, Vec::new()), requirements)
    }

    /// Write the bounded item for the claim at `occurrence` with
    /// `requirements`, substituting `bounds` for its unbounded domains
    /// (ADR-014 §4 "Bounded request"). The item carries `occurrence`, the
    /// originating item's, so a follow-up request's settlement joins the
    /// requirement record it answers.
    /// Refuses, writing nothing, unless the item is unbounded, every one of
    /// its domains is boundable, and `bounds` names exactly those domains,
    /// each with a bound of its kind. Refusals are checked in that order:
    /// a bounded item, an unknown key, an unboundable domain, then each
    /// domain's missing or mismatched bound.
    pub fn bounded_item(
        &mut self,
        occurrence: OccurrenceKey,
        requirements: &Requirements,
        bounds: BTreeMap<DomainKey, FiniteBound>,
    ) -> Result<RequestIndex, BoundRefusal> {
        let ClaimExtent::Unbounded(unbounded) = requirements.extent() else {
            // A bounded item needs no proof bound; it is written by `item`.
            return Err(BoundRefusal::NoUnboundedDomain);
        };
        let unbounded: Vec<_> = unbounded
            .iter()
            .map(|(domain, kind)| (domain.clone(), kind))
            .collect();
        let domains = substitute(&unbounded, &bounds)?;
        Ok(self.push(
            Written::plain(occurrence, ExtentClassification::Bounded, domains),
            requirements,
        ))
    }

    /// The items written so far, in index order.
    pub fn items(&self) -> &[RequestItem] {
        &self.items
    }

    /// The finished request's items, in index order.
    pub fn finish(self) -> Vec<RequestItem> {
        self.items
    }

    /// Write the item of a temporal claim or refinement over a model
    /// subject (FR-336, FR-144): [`Self::item`]'s item, with its FR-368
    /// `item_kind` and, for a refinement, its abstract subject.
    pub(crate) fn model_item(
        &mut self,
        occurrence: OccurrenceKey,
        requirements: &Requirements,
        item_kind: ItemKind,
        abstract_subject: Option<AbstractSubject>,
    ) -> RequestIndex {
        let extent = ExtentClassification::of(requirements.extent());
        self.push(
            Written {
                occurrence,
                extent,
                domains: Vec::new(),
                item_kind: Some(item_kind),
                abstract_subject,
            },
            requirements,
        )
    }

    /// Write a derived item (QSpec FR-368 "Claim and anchor", "Extent",
    /// "Negotiation"): a `temporal-satisfaction` item at its subject's
    /// anchor `claim`, unbounded with no finite bound available, with no
    /// `hybrid` and no `requirements`, its `candidates` computed by the
    /// caller and its caller-set `limits`.
    pub(crate) fn derived_item(
        &mut self,
        claim: OccurrenceKey,
        item_kind: ItemKind,
        property_form: Option<quire_semantic_value::fairness::PropertyForm>,
        candidates: CandidateOutcome,
        limits: DerivedLimits,
    ) -> RequestIndex {
        let index = RequestIndex::new(self.items.len());
        self.items.push(RequestItem {
            index,
            occurrence: claim,
            kind: Capability::TemporalSatisfaction,
            hybrid_dynamics: false,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            domains: Vec::new(),
            unsupported: None,
            reductions: self.reductions.selected(),
            property_form,
            item_kind: Some(item_kind),
            abstract_subject: None,
            derived: Some(DerivedPart { candidates, limits }),
        });
        index
    }

    fn push(&mut self, written: Written, requirements: &Requirements) -> RequestIndex {
        let index = RequestIndex::new(self.items.len());
        self.items.push(RequestItem {
            index,
            occurrence: written.occurrence,
            kind: requirements.kind(),
            hybrid_dynamics: requirements.hybrid_dynamics(),
            extent: written.extent,
            domains: written.domains,
            unsupported: requirements.unsupported().cloned(),
            reductions: self.reductions.selected(),
            property_form: requirements.property_form(),
            item_kind: written.item_kind,
            abstract_subject: written.abstract_subject,
            derived: None,
        });
        index
    }
}

/// The members of an item written from a record beside the record's own.
struct Written {
    occurrence: OccurrenceKey,
    extent: ExtentClassification,
    domains: Vec<ProofBound>,
    item_kind: Option<ItemKind>,
    abstract_subject: Option<AbstractSubject>,
}

impl Written {
    /// An item with no model subject.
    fn plain(
        occurrence: OccurrenceKey,
        extent: ExtentClassification,
        domains: Vec<ProofBound>,
    ) -> Self {
        Self {
            occurrence,
            extent,
            domains,
            item_kind: None,
            abstract_subject: None,
        }
    }
}

/// ADR-014 §4 "Bounded request": the proof bounds that substitute `bounds`
/// for an item's `unbounded` domains (in key order), each with its domain's
/// QSL-classified kind, in [`DomainKey`] order. Refuses unless `bounds`
/// names exactly those domains, every one boundable, each with a bound of
/// its kind; refusals are checked in the order [`RequestWriter::bounded_item`]
/// states.
fn substitute(
    unbounded: &[(DomainKey, DomainKind)],
    bounds: &BTreeMap<DomainKey, FiniteBound>,
) -> Result<Vec<ProofBound>, BoundRefusal> {
    if let Some(domain) = bounds
        .keys()
        .find(|supplied| !unbounded.iter().any(|(domain, _)| domain == *supplied))
    {
        return Err(BoundRefusal::UnknownDomain {
            domain: domain.clone(),
        });
    }
    // An item with a domain no finite bound can stand for can never be
    // bounded, whatever else is supplied. Otherwise each domain's expected
    // bound kind is known from here on.
    let mut expected_kinds = Vec::with_capacity(unbounded.len());
    for (domain, kind) in unbounded {
        let Some(expected) = kind.finite_kind() else {
            return Err(BoundRefusal::UnboundableDomain {
                domain: domain.clone(),
                kind: *kind,
            });
        };
        expected_kinds.push((domain, *kind, expected));
    }
    let mut domains = Vec::with_capacity(expected_kinds.len());
    for (domain, kind, expected) in expected_kinds {
        let Some(bound) = bounds.get(domain) else {
            return Err(BoundRefusal::MissingDomain {
                domain: domain.clone(),
            });
        };
        let mismatch = || BoundRefusal::KindMismatch {
            domain: domain.clone(),
            expected,
            supplied: bound.kind(),
        };
        if bound.kind() != expected {
            return Err(mismatch());
        }
        domains.push(
            ProofBound::new(domain.clone(), Some(kind), bound.clone()).map_err(|_| mismatch())?,
        );
    }
    Ok(domains)
}

/// One requested item written from one requirement record (FR-075
/// "Requested items from a package's requirement records"): everything the
/// orchestrating driver reads about the item from QSL, including the
/// record's path condition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementItem {
    index: RequestIndex,
    occurrence: OccurrenceKey,
    kind: Capability,
    hybrid_dynamics: bool,
    extent: ExtentClassification,
    unbounded: Vec<(DomainKey, DomainKind)>,
    proof_bounds: Vec<ProofBound>,
    subject_proof_bounds: Vec<ProofBound>,
    model_subject: bool,
    result_bound: WireNodeId,
    conversion_context: Option<OccurrenceKey>,
    path_condition: Vec<PathGuard>,
    candidates: CandidateOutcome,
    unsupported: Option<UnsupportedItem>,
    evidence: Option<EvidenceKind>,
}

impl RequirementItem {
    /// The reduction this item settles V-8 `unsupported-requested-capability`
    /// on (FR-159 "Capability"): the first selected reduction that emptied
    /// its candidate set; `None` when a candidate advertises every
    /// selected reduction, or the set was empty before they were read.
    pub fn unsupported_reduction(&self) -> Option<ReductionKind> {
        match &self.candidates {
            CandidateOutcome::Candidates(set) => set.missing_reduction(),
            CandidateOutcome::UnknownBackend(_) => None,
        }
    }

    /// The evidence kind a `probabilistic-satisfaction` item names (FR-075
    /// "Evidence kind of a probabilistic item"): always one of the two kinds
    /// for such an item the builder writes, and `None` for an item of any
    /// other capability kind.
    pub fn evidence(&self) -> Option<EvidenceKind> {
        self.evidence
    }

    /// The item's request index: its position in record key order, or 0
    /// for a follow-up [`RequirementItem::bounded`] writes, the single item
    /// of its own request (ADR-014 §4, FR-075-AC-8).
    pub fn index(&self) -> RequestIndex {
        self.index
    }

    /// The record's occurrence key.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The occurrence key's node: the application the claim is over.
    pub fn node(&self) -> WireNodeId {
        self.occurrence.node()
    }

    /// The record's capability kind.
    pub fn kind(&self) -> Capability {
        self.kind
    }

    /// The provider item requirement labels in wire order.
    pub fn requirements(&self) -> &'static [&'static str] {
        if self.hybrid_dynamics {
            &["hybrid-dynamics"]
        } else {
            &[]
        }
    }

    /// The extent classification [`RequestWriter::item`] computes from the
    /// record's extent; `bounded` for a follow-up
    /// [`RequirementItem::bounded`] writes.
    pub fn extent(&self) -> ExtentClassification {
        self.extent
    }

    /// Each unbounded domain of the record's extent with its kind, in key
    /// order: the keys a bounded follow-up supplies a bound for. Empty for
    /// a bounded extent.
    pub fn unbounded(&self) -> &[(DomainKey, DomainKind)] {
        &self.unbounded
    }

    /// The proof bounds substituted for this occurrence's unbounded domains,
    /// in [`DomainKey`] order (QSpec FR-331 `extent.bounds`), each with the
    /// domain kind QSL classified for the record. Empty for an item written
    /// from a record: a checked record carries no substitution, since a
    /// proof bound is supplied by the caller (ADR-014 B-4). Non-empty only
    /// for the follow-up [`RequirementItem::bounded`] writes.
    pub fn proof_bounds(&self) -> &[ProofBound] {
        &self.proof_bounds
    }

    /// The temporal subject's proof bounds for this occurrence (QSpec FR-331
    /// `temporal.subject.proof_bounds`): `None` unless the item is
    /// `temporal-satisfaction` over a model subject, which alone has a
    /// `subject` (FR-331, FR-368); otherwise the substitutions
    /// [`RequirementItem::with_subject_proof_bounds`] supplied, in domain-key
    /// order. They are independent of [`RequirementItem::proof_bounds`] (the
    /// extent's): neither moves into the other. `Some(&[])` is such an item
    /// with no subject substitution.
    pub fn temporal_proof_bounds(&self) -> Option<&[ProofBound]> {
        (matches!(self.kind, Capability::TemporalSatisfaction) && self.model_subject)
            .then_some(self.subject_proof_bounds.as_slice())
    }

    /// This item with `bounds` as its temporal subject's proof bounds
    /// (QSpec FR-331: the subject-only substitution, supplied by the caller
    /// and independent of the extent; ADR-014 B-4). A subject's proof bounds
    /// substitute its own populations, so each key is a
    /// [`DomainKey::Population`] with a cardinality bound, and QSL writes
    /// the `population` kind (FR-097-AC-9). The index, occurrence, extent
    /// and extent bounds are unchanged. Refuses an item with no model
    /// subject, a key that is not a population and a bound that is not a
    /// cardinality, returning nothing.
    pub fn with_subject_proof_bounds(
        &self,
        bounds: &BTreeMap<DomainKey, FiniteBound>,
    ) -> Result<Self, BoundRefusal> {
        if self.temporal_proof_bounds().is_none() {
            return Err(BoundRefusal::NoModelSubject);
        }
        let subject_proof_bounds = bounds
            .iter()
            .map(|(domain, bound)| {
                if !matches!(domain, DomainKey::Population { .. }) {
                    return Err(BoundRefusal::NotAPopulation {
                        domain: domain.clone(),
                    });
                }
                let supplied = bound.kind();
                if supplied != FiniteBoundKind::Cardinality {
                    return Err(BoundRefusal::KindMismatch {
                        domain: domain.clone(),
                        expected: FiniteBoundKind::Cardinality,
                        supplied,
                    });
                }
                ProofBound::new(domain.clone(), Some(DomainKind::Population), bound.clone())
                    .map_err(|_| BoundRefusal::KindMismatch {
                        domain: domain.clone(),
                        expected: FiniteBoundKind::Cardinality,
                        supplied,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            subject_proof_bounds,
            ..self.clone()
        })
    }

    /// The bounded follow-up of this item (ADR-014 §4 "Bounded request"):
    /// the single item of its own request, so its index is 0 (FR-075-AC-8),
    /// with the same occurrence, record members and candidates, and
    /// `bounds` substituted for every unbounded domain, so its extent is
    /// `bounded` and [`RequirementItem::proof_bounds`] holds one
    /// [`ProofBound`] per domain with the kind QSL classified for it.
    /// Refuses like [`RequestWriter::bounded_item`], returning nothing.
    pub fn bounded(&self, bounds: &BTreeMap<DomainKey, FiniteBound>) -> Result<Self, BoundRefusal> {
        if self.extent == ExtentClassification::Bounded {
            return Err(BoundRefusal::NoUnboundedDomain);
        }
        let proof_bounds = substitute(&self.unbounded, bounds)?;
        Ok(Self {
            index: RequestIndex::new(0),
            extent: ExtentClassification::Bounded,
            unbounded: Vec::new(),
            proof_bounds,
            ..self.clone()
        })
    }

    /// The record's result bound, as its type node.
    pub fn result_bound(&self) -> WireNodeId {
        self.result_bound
    }

    /// The occurrence of the direct narrow that supplies this item's result
    /// bound, or `None` when the scalar application's own result type applies.
    /// The context is carried per requirement occurrence even when multiple
    /// items share one application node.
    pub fn conversion_context(&self) -> Option<&OccurrenceKey> {
        self.conversion_context.as_ref()
    }

    /// The record's path condition, outermost guard first.
    pub fn path_condition(&self) -> &[PathGuard] {
        &self.path_condition
    }

    /// The item's candidate outcome under the registry and the named
    /// backend it was built with.
    pub fn candidates(&self) -> &CandidateOutcome {
        &self.candidates
    }

    /// The cause the writer disposed the item `Unsupported` for (FR-234
    /// "Punctuality"); such an item has no candidate, so it is routed to no
    /// engine.
    pub fn unsupported(&self) -> Option<&UnsupportedItem> {
        self.unsupported.as_ref()
    }
}

/// The evidence kinds a request selects for its probabilistic items (FR-075
/// "Evidence kind of a probabilistic item").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceSelection {
    /// The request names none: each probabilistic item carries its record's
    /// evidence kind.
    Record,
    /// The request names one kind: each probabilistic item carries it,
    /// whatever the record's kind.
    One(EvidenceKind),
    /// The request names both: each probabilistic record's item carries the
    /// record's kind, and a second-kind item per such record follows the
    /// last record's item.
    Both,
}

/// The requested items of a checked package's requirement `records`
/// (`CheckedPackage::graph().requirements()`) for a request naming no
/// evidence selection: [`items_with_evidence`] with
/// [`EvidenceSelection::Record`].
pub fn items_from_requirements(
    records: &BTreeMap<OccurrenceKey, RequirementRecord>,
    registry: &Registry,
    named: Option<&BackendId>,
) -> Vec<RequirementItem> {
    items_with_evidence(records, registry, named, EvidenceSelection::Record)
}

/// The requested items of a checked package's requirement `records`, one per
/// record in the records' occurrence-key order, each with its candidate
/// outcome from `registry` for the backend `named`, when the request names
/// one, filtered by the fairness kinds the record's requirements use
/// (FR-134), and the record's canonical path condition. A
/// `probabilistic-satisfaction` item names its evidence kind by `selection`,
/// and its candidates are the backends whose evidence-kind set holds exactly
/// that kind. When `selection` is [`EvidenceSelection::Both`], the second-kind
/// items follow, one per probabilistic record in key order, at request index
/// `n + j`. An item whose candidate set is empty or whose extent is
/// unbounded is written like any other; negotiation settles it.
pub fn items_with_evidence(
    records: &BTreeMap<OccurrenceKey, RequirementRecord>,
    registry: &Registry,
    named: Option<&BackendId>,
    selection: EvidenceSelection,
) -> Vec<RequirementItem> {
    items_with_reductions(records, registry, named, selection, &Reductions::default())
}

/// [`items_with_evidence`] for a request selecting `reductions` (FR-159):
/// each item's candidates, for a kind FR-159's preservation table reads,
/// are restricted to the backends advertising every
/// selected reduction ([`Registry::restrict_to_reductions`]), and an item
/// the restriction leaves with none names the reduction it settles V-8 on
/// ([`RequirementItem::unsupported_reduction`]).
pub fn items_with_reductions(
    records: &BTreeMap<OccurrenceKey, RequirementRecord>,
    registry: &Registry,
    named: Option<&BackendId>,
    selection: EvidenceSelection,
    reductions: &Reductions,
) -> Vec<RequirementItem> {
    let selected: std::collections::BTreeSet<ReductionKind> =
        reductions.selected().into_iter().collect();
    let probabilistic = |record: &RequirementRecord| {
        matches!(
            record.requirements().kind(),
            Capability::ProbabilisticSatisfaction
        )
    };
    let write = |position: usize,
                 occurrence: &OccurrenceKey,
                 record: &RequirementRecord,
                 evidence: Option<EvidenceKind>| {
        let requirements = record.requirements();
        let extent = ExtentClassification::of(requirements.extent());
        let unbounded = match requirements.extent() {
            ClaimExtent::Bounded => Vec::new(),
            ClaimExtent::Unbounded(domains) => domains
                .iter()
                .map(|(key, kind)| (key.clone(), kind))
                .collect(),
        };
        RequirementItem {
            index: RequestIndex::new(position),
            occurrence: occurrence.clone(),
            kind: requirements.kind(),
            hybrid_dynamics: requirements.hybrid_dynamics(),
            extent,
            unbounded,
            proof_bounds: Vec::new(),
            subject_proof_bounds: Vec::new(),
            model_subject: requirements.model_subject(),
            result_bound: record.result_bound().node(),
            conversion_context: record.conversion_context().cloned(),
            path_condition: record.path_condition().to_vec(),
            candidates: if evidence.is_none() && probabilistic(record) {
                // FR-195: a probabilistic record always carries its
                // evidence kind; one without has no backend to settle it.
                CandidateOutcome::Candidates(CandidateSet::none(requirements.kind(), named))
            } else {
                reduced_candidates(requirements, evidence, registry, named, &selected)
            },
            unsupported: requirements.unsupported().cloned(),
            evidence,
        }
    };
    let own = |record: &RequirementRecord| match selection {
        EvidenceSelection::One(kind) => probabilistic(record).then_some(kind),
        EvidenceSelection::Record | EvidenceSelection::Both => {
            record.requirements().evidence().map(EvidenceKind::from)
        }
    };
    let mut items: Vec<RequirementItem> = records
        .iter()
        .enumerate()
        .map(|(position, (occurrence, record))| write(position, occurrence, record, own(record)))
        .collect();
    if selection == EvidenceSelection::Both {
        let mut position = records.len();
        for (occurrence, record) in records {
            let Some(first) = own(record) else { continue };
            let other = match first {
                EvidenceKind::Statistical => EvidenceKind::Exact,
                EvidenceKind::Exact => EvidenceKind::Statistical,
            };
            items.push(write(position, occurrence, record, Some(other)));
            position += 1;
        }
    }
    items
}

/// Whether FR-159's preservation table reads items of `kind`: a
/// `temporal-satisfaction` item (the TP-1 to TP-4 forms and the
/// deadlock-freedom item) and each half of a `refinement` item. The
/// selection applies to them, and a reduction the table marks as not
/// preserving settles `ReductionNotPreserving` in the engine's pre-check;
/// an item of any other kind reads no reduction.
fn reads_reductions(kind: Capability) -> bool {
    matches!(
        kind,
        Capability::TemporalSatisfaction | Capability::Refinement
    )
}

/// [`candidates_of`] restricted to the backends advertising every
/// `selected` reduction, for an item of a kind the selection applies to
/// ([`reads_reductions`]); unrestricted for any other.
fn reduced_candidates(
    requirements: &Requirements,
    evidence: Option<EvidenceKind>,
    registry: &Registry,
    named: Option<&BackendId>,
    selected: &std::collections::BTreeSet<ReductionKind>,
) -> CandidateOutcome {
    let outcome = candidates_of(requirements, evidence, registry, named);
    if reads_reductions(requirements.kind()) {
        registry.restrict_to_reductions(outcome, selected)
    } else {
        outcome
    }
}

/// The candidate outcome of one item's `requirements`: backends of its
/// capability kind that advertise every fairness kind it uses (FR-134) and,
/// for a timed item ([`Requirements::timed`]), decide timed items (FR-243);
/// a refinement record that is not marked timed goes to the engines that do
/// not decide timed items, EN-1 and SMT unrolling (FR-144, FR-146, FR-430,
/// ADR-020 RE-3). An item the writer disposes `Unsupported` has no
/// candidate (FR-234).
fn candidates_of(
    requirements: &Requirements,
    evidence: Option<EvidenceKind>,
    registry: &Registry,
    named: Option<&BackendId>,
) -> CandidateOutcome {
    let outcome = match evidence {
        Some(kind) => {
            registry.candidates_for_evidence(requirements.kind(), &ItemEvidence::Kind(kind), named)
        }
        None => registry.candidates_for_temporal_dynamics(
            requirements.kind(),
            named,
            requirements.fairness(),
            &Default::default(),
            requirements.hybrid_dynamics(),
        ),
    };
    let refinement = matches!(
        requirements.property_form(),
        Some(quire_semantic_value::fairness::PropertyForm::Refinement { .. })
    );
    let outcome = if refinement && !requirements.timed() {
        registry.restrict_to_untimed_refinement(outcome)
    } else {
        registry.restrict_to_timed(outcome, requirements.timed())
    };
    match outcome {
        // An unknown named backend stays `UnknownBackend` (FR-075-AC-3).
        CandidateOutcome::Candidates(_) if requirements.unsupported().is_some() => {
            CandidateOutcome::Candidates(CandidateSet::none(requirements.kind(), named))
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// FR-159 "Capability" over FR-159-AC-2's table: a `refinement` item, as
    /// a `temporal-satisfaction` one, is routed only to a backend
    /// advertising every selected reduction, so with the one refinement
    /// backend advertising no symmetry, the item selecting symmetry has no
    /// candidate and names symmetry for V-8; with a backend advertising it
    /// beside, the item routes there only. A `value-validity` item reads
    /// no reduction and keeps its candidate.
    #[trace("TC-583", "FR-159-AC-3")]
    #[test]
    fn a_refinement_item_is_restricted_by_its_requests_reductions() {
        use crate::{BackendDescriptor, Mode, ProviderOrigin};
        let backend = |id: &str, reductions: &[ReductionKind]| {
            BackendDescriptor::new(
                crate::Candidate::new(BackendId::new(id)),
                ProviderOrigin::Linked,
                [
                    (Capability::Refinement, Mode::Bounded),
                    (Capability::TemporalSatisfaction, Mode::Bounded),
                    (Capability::ValueValidity, Mode::Bounded),
                ],
                Some(std::collections::BTreeSet::from([DomainKind::Collection])),
                None,
            )
            .expect("a well-formed descriptor")
            .with_reductions(reductions.iter().copied())
        };
        let symmetry = std::collections::BTreeSet::from([ReductionKind::Symmetry]);
        let routed = |registry: &Registry, kind: Capability| {
            let requirements = Requirements::new(kind, ClaimExtent::Bounded);
            match reduced_candidates(&requirements, None, registry, None, &symmetry) {
                CandidateOutcome::Candidates(set) => (
                    set.candidates()
                        .iter()
                        .map(|candidate| candidate.id().as_str().to_owned())
                        .collect::<Vec<_>>(),
                    set.missing_reduction(),
                ),
                CandidateOutcome::UnknownBackend(id) => panic!("no backend is named: {id:?}"),
            }
        };
        let mut registry = Registry::new();
        registry
            .register(backend("plain", &[]))
            .expect("registered");
        for kind in [Capability::Refinement, Capability::TemporalSatisfaction] {
            assert_eq!(
                routed(&registry, kind),
                (Vec::new(), Some(ReductionKind::Symmetry)),
                "{kind:?}"
            );
        }
        assert_eq!(
            routed(&registry, Capability::ValueValidity),
            (vec!["plain".to_owned()], None)
        );
        registry
            .register(backend("reducing", &[ReductionKind::Symmetry]))
            .expect("registered");
        assert_eq!(
            routed(&registry, Capability::Refinement),
            (vec!["reducing".to_owned()], None)
        );
    }

    fn node(fill: u8) -> WireNodeId {
        WireNodeId::from_digest([fill; 32])
    }

    /// The first `expression` occurrence of node 9.
    fn occurrence() -> OccurrenceKey {
        OccurrenceKey::new(
            node(9),
            quire_exact::Origin::new(quire_exact::Role::new("expression"), 0),
        )
    }

    fn key(fill: u8, path: &[u32]) -> DomainKey {
        DomainKey::Node {
            node: node(fill),
            path: path.to_vec(),
        }
    }

    /// A `value-validity` claim over `domains`.
    fn claim(domains: &[(DomainKey, DomainKind)]) -> Requirements {
        Requirements::new(
            Capability::ValueValidity,
            ClaimExtent::from_domains(domains.iter().cloned().collect()),
        )
    }

    fn set_claim() -> Requirements {
        claim(&[(key(1, &[]), DomainKind::Collection)])
    }

    fn depth(maximum: u64) -> FiniteBound {
        FiniteBound::depth(maximum).unwrap()
    }

    /// FR-447-AC-17: the hybrid claim's item carries exactly the member
    /// `"requirements": ["hybrid-dynamics"]` and an ordinary item none, so
    /// the label is part of the item's identity; the label is written only
    /// on a `temporal-satisfaction` item, and writing the same record twice
    /// gives equal items.
    #[trace("TC-1187", "FR-447-AC-17")]
    #[test]
    fn request_writer_keeps_hybrid_dynamics_in_the_item_identity() {
        let ordinary = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
        let hybrid = ordinary.clone().with_hybrid_dynamics();
        let value = Requirements::new(Capability::ValueValidity, ClaimExtent::Bounded)
            .with_hybrid_dynamics();
        let mut writer = RequestWriter::new();
        let first = writer.item(occurrence(), &ordinary);
        let second = writer.item(occurrence(), &hybrid);
        let third = writer.item(occurrence(), &value);
        let items = writer.finish();
        assert_eq!(items[first.get()].requirements(), &[] as &[&str]);
        assert_eq!(items[second.get()].requirements(), ["hybrid-dynamics"]);
        assert_eq!(items[third.get()].requirements(), &[] as &[&str]);
        assert_ne!(items[first.get()], items[second.get()]);
    }

    /// TC-438 (ADR-014 §4; §10 scenarios 2 and 3): a bounded item is
    /// `bounded`; an unbounded item has a finite bound available exactly
    /// when every domain is boundable, and a loop or infinite-trace domain
    /// makes it unavailable.
    #[trace("TC-438", "FR-097-AC-3")]
    #[test]
    fn tc_438_finite_bound_available_exactly_when_every_domain_is_boundable() {
        let mut writer = RequestWriter::new();
        let bounded = writer.item(occurrence(), &claim(&[]));
        let set = writer.item(occurrence(), &set_claim());
        let set_and_loop = writer.item(
            occurrence(),
            &claim(&[
                (key(1, &[]), DomainKind::Collection),
                (key(2, &[]), DomainKind::Loop),
            ]),
        );
        let formula = writer.item(
            occurrence(),
            &Requirements::new(
                Capability::TemporalSatisfaction,
                ClaimExtent::from_domains(BTreeMap::from([(
                    key(3, &[]),
                    DomainKind::InfiniteTrace,
                )])),
            ),
        );
        let items = writer.finish();
        assert_eq!(items[bounded.get()].extent(), ExtentClassification::Bounded);
        assert_eq!(
            items[set.get()].extent(),
            ExtentClassification::Unbounded {
                finite_bound_available: true
            }
        );
        for index in [set_and_loop, formula] {
            assert_eq!(
                items[index.get()].extent(),
                ExtentClassification::Unbounded {
                    finite_bound_available: false
                }
            );
        }
        assert_eq!(
            items[formula.get()].kind(),
            Capability::TemporalSatisfaction
        );
        assert_eq!(ExtentClassification::Bounded.to_wire(), "bounded");
        assert_eq!(items[set.get()].extent().to_wire(), "unbounded");
        assert!(items.iter().all(|item| item.domains().is_empty()));
    }

    /// TC-438 (ADR-014 §4 "Bounded request", §6 step 5, §10 scenario 3):
    /// the bounded item substitutes its proof bound, classifies `bounded`,
    /// carries the bound in its identity and has its own index; the
    /// unbounded item keeps its own index and classification.
    #[trace("TC-438", "FR-097-AC-3")]
    #[test]
    fn tc_438_a_bounded_request_is_its_own_item() {
        let mut writer = RequestWriter::new();
        let requirements = claim(&[
            (key(1, &[]), DomainKind::Collection),
            (key(1, &[0]), DomainKind::Recursive),
        ]);
        let unbounded = writer.item(occurrence(), &requirements);
        let eight = writer
            .bounded_item(
                occurrence(),
                &requirements,
                BTreeMap::from([
                    (key(1, &[]), FiniteBound::cardinality(8)),
                    (key(1, &[0]), depth(3)),
                ]),
            )
            .expect("one bound per domain, each of its kind");
        let four = writer
            .bounded_item(
                occurrence(),
                &requirements,
                BTreeMap::from([
                    (key(1, &[]), FiniteBound::cardinality(4)),
                    (key(1, &[0]), depth(3)),
                ]),
            )
            .unwrap();
        let items = writer.finish();
        assert_eq!(
            [unbounded, eight, four].map(RequestIndex::get),
            [0, 1, 2],
            "each item has its own index"
        );
        assert_eq!(
            items[unbounded.get()].extent(),
            ExtentClassification::Unbounded {
                finite_bound_available: true
            }
        );
        let bounded = &items[eight.get()];
        assert_eq!(bounded.extent(), ExtentClassification::Bounded);
        assert_eq!(
            bounded.domains(),
            [
                ProofBound::new(
                    key(1, &[]),
                    Some(DomainKind::Collection),
                    FiniteBound::cardinality(8)
                )
                .unwrap(),
                ProofBound::new(key(1, &[0]), Some(DomainKind::Recursive), depth(3)).unwrap(),
            ]
        );
        // The proof bounds are part of the item: different bounds, different
        // items; neither is the unbounded item.
        assert_ne!(items[eight.get()].domains(), items[four.get()].domains());
        assert_ne!(items[eight.get()], items[unbounded.get()]);
    }

    /// FR-331 `extent.bounds[].kind`: every proof bound the writer emits
    /// carries its domain's kind, and the kind pairs with the bound's
    /// variant as `DomainKind::finite_kind` says.
    #[trace("TC-438", "FR-097-AC-9")]
    #[test]
    fn tc_438_each_proof_bound_carries_the_kind_its_bound_variant_pairs_with() {
        let requirements = claim(&[
            (key(1, &[]), DomainKind::Population),
            (key(2, &[]), DomainKind::Integer),
            (key(3, &[]), DomainKind::Recursive),
            (key(4, &[]), DomainKind::Collection),
        ]);
        let mut writer = RequestWriter::new();
        let index = writer
            .bounded_item(
                occurrence(),
                &requirements,
                BTreeMap::from([
                    (key(1, &[]), FiniteBound::cardinality(2)),
                    (
                        key(2, &[]),
                        FiniteBound::integer_range(
                            quire_exact::Integer::from(0_i64),
                            quire_exact::Integer::from(1_i64),
                        )
                        .unwrap(),
                    ),
                    (key(3, &[]), depth(2)),
                    (key(4, &[]), FiniteBound::cardinality(5)),
                ]),
            )
            .unwrap();
        let items = writer.finish();
        let kinds: Vec<_> = items[index.get()]
            .domains()
            .iter()
            .map(|bound| bound.kind())
            .collect();
        assert_eq!(
            kinds,
            [
                Some(DomainKind::Population),
                Some(DomainKind::Integer),
                Some(DomainKind::Recursive),
                Some(DomainKind::Collection)
            ]
        );
        for bound in items[index.get()].domains() {
            assert_eq!(
                bound.kind().and_then(DomainKind::finite_kind),
                Some(bound.bound().kind())
            );
        }
    }

    /// TC-438 (ADR-014 §4 refusals): an unknown domain, an unboundable
    /// domain, a bound of the wrong kind and a missing domain each refuse
    /// `invalid_runtime_input`/`invalid-value`, and nothing is written.
    #[trace("TC-438", "FR-097-AC-4")]
    #[test]
    fn tc_438_bad_bounds_refuse_before_any_item_is_written() {
        let with_loop = claim(&[
            (key(1, &[]), DomainKind::Collection),
            (key(2, &[]), DomainKind::Loop),
        ]);
        let cases = [
            (
                set_claim(),
                BTreeMap::from([
                    (key(1, &[]), FiniteBound::cardinality(8)),
                    (key(5, &[]), FiniteBound::cardinality(8)),
                ]),
                BoundRefusal::UnknownDomain {
                    domain: key(5, &[]),
                },
            ),
            (
                claim(&[]),
                BTreeMap::from([(key(1, &[]), FiniteBound::cardinality(8))]),
                BoundRefusal::NoUnboundedDomain,
            ),
            (claim(&[]), BTreeMap::new(), BoundRefusal::NoUnboundedDomain),
            (
                with_loop.clone(),
                BTreeMap::from([
                    (key(1, &[]), FiniteBound::cardinality(8)),
                    (key(2, &[]), depth(3)),
                ]),
                BoundRefusal::UnboundableDomain {
                    domain: key(2, &[]),
                    kind: DomainKind::Loop,
                },
            ),
            (
                set_claim(),
                BTreeMap::from([(key(1, &[]), depth(3))]),
                BoundRefusal::KindMismatch {
                    domain: key(1, &[]),
                    expected: FiniteBoundKind::Cardinality,
                    supplied: FiniteBoundKind::Depth,
                },
            ),
            (
                with_loop,
                BTreeMap::from([(key(1, &[]), FiniteBound::cardinality(8))]),
                BoundRefusal::UnboundableDomain {
                    domain: key(2, &[]),
                    kind: DomainKind::Loop,
                },
            ),
            (
                set_claim(),
                BTreeMap::new(),
                BoundRefusal::MissingDomain {
                    domain: key(1, &[]),
                },
            ),
            // Refusal order: an unknown key before an unboundable domain.
            (
                claim(&[
                    (key(1, &[]), DomainKind::Collection),
                    (key(2, &[]), DomainKind::Loop),
                ]),
                BTreeMap::from([(key(5, &[]), FiniteBound::cardinality(8))]),
                BoundRefusal::UnknownDomain {
                    domain: key(5, &[]),
                },
            ),
            // An unboundable domain before a missing one.
            (
                claim(&[
                    (key(1, &[]), DomainKind::Collection),
                    (key(2, &[]), DomainKind::Loop),
                ]),
                BTreeMap::new(),
                BoundRefusal::UnboundableDomain {
                    domain: key(2, &[]),
                    kind: DomainKind::Loop,
                },
            ),
            // Domain by domain in key order: key 1's wrong kind before key
            // 2's missing bound.
            (
                claim(&[
                    (key(1, &[]), DomainKind::Collection),
                    (key(2, &[]), DomainKind::Recursive),
                ]),
                BTreeMap::from([(key(1, &[]), depth(3))]),
                BoundRefusal::KindMismatch {
                    domain: key(1, &[]),
                    expected: FiniteBoundKind::Cardinality,
                    supplied: FiniteBoundKind::Depth,
                },
            ),
        ];
        for (requirements, bounds, expected) in cases {
            let mut writer = RequestWriter::new();
            let refusal = writer
                .bounded_item(occurrence(), &requirements, bounds)
                .expect_err("the bounds refuse");
            assert_eq!(refusal, expected);
            assert_eq!(
                refusal.catalog_code(),
                CatalogCode::new("invalid_runtime_input", "invalid-value")
            );
            assert!(writer.items().is_empty(), "nothing is written");
        }
    }

    /// FR-134: an item whose requirements carry `strong` is given only the
    /// backend that advertises it; the same item without the kind is given
    /// both.
    #[test]
    fn candidates_of_filters_by_the_requirements_fairness() {
        use crate::{BackendDescriptor, Candidate, Mode, ProviderOrigin};
        use quire_semantic_value::fairness::FairnessKind::{Strong, Weak};

        let temporal = |id: &str, kinds: &[quire_semantic_value::fairness::FairnessKind]| {
            BackendDescriptor::new(
                Candidate::new(BackendId::new(id)),
                ProviderOrigin::Linked,
                [(Capability::TemporalSatisfaction, Mode::Unbounded)],
                None,
                None,
            )
            .expect("a well-formed descriptor")
            .with_fairness_kinds(kinds.iter().copied())
        };
        let mut registry = Registry::new();
        registry
            .register(temporal("both", &[Weak, Strong]))
            .expect("admitted");
        registry
            .register(temporal("weak-only", &[Weak]))
            .expect("admitted");
        let item = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);

        let CandidateOutcome::Candidates(strong) =
            candidates_of(&item.clone().with_fairness([Strong]), None, &registry, None)
        else {
            panic!("a candidate outcome");
        };
        assert_eq!(
            strong.candidates(),
            [Candidate::new(BackendId::new("both"))]
        );
        let CandidateOutcome::Candidates(plain) = candidates_of(&item, None, &registry, None)
        else {
            panic!("a candidate outcome");
        };
        assert_eq!(plain.candidates().len(), 2);
    }

    /// FR-234 "Punctuality": the writer disposes an item whose requirements
    /// carry `PunctualInterval` `Unsupported`, naming the interval's span,
    /// and gives it no candidate though a backend advertises its kind; the
    /// same item without the cause keeps that backend.
    #[trace("TC-689", "FR-234-AC-3")]
    #[test]
    fn a_punctual_item_is_disposed_unsupported_and_routed_to_no_engine() {
        use crate::{BackendDescriptor, Candidate, Mode, ProviderOrigin};
        use qsl_semantics::family::UnsupportedItem;

        let mut registry = Registry::new();
        registry
            .register(
                BackendDescriptor::new(
                    Candidate::new(BackendId::new("zones")),
                    ProviderOrigin::Linked,
                    [(Capability::TemporalSatisfaction, Mode::Unbounded)],
                    None,
                    None,
                )
                .expect("a well-formed descriptor"),
            )
            .expect("admitted");
        let cause = UnsupportedItem::PunctualInterval {
            span: qsl_foundation::Span { start: 4, end: 16 },
        };
        let plain = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
        let punctual = plain.clone().with_unsupported(cause.clone());

        let mut writer = RequestWriter::new();
        writer.item(occurrence(), &punctual);
        writer.item(occurrence(), &plain);
        let [disposed, negotiated] = writer.items() else {
            panic!("two items");
        };
        assert_eq!(disposed.unsupported(), Some(&cause));
        assert_eq!(negotiated.unsupported(), None);

        let CandidateOutcome::Candidates(none) = candidates_of(&punctual, None, &registry, None)
        else {
            panic!("a candidate outcome");
        };
        assert!(none.candidates().is_empty());
        let CandidateOutcome::Candidates(some) = candidates_of(&plain, None, &registry, None)
        else {
            panic!("a candidate outcome");
        };
        assert_eq!(some.candidates(), [Candidate::new(BackendId::new("zones"))]);

        let missing = BackendId::new("missing");
        assert_eq!(
            candidates_of(&punctual, None, &registry, Some(&missing)),
            CandidateOutcome::UnknownBackend(missing)
        );
    }

    /// FR-396-AC-2, FR-399: the shared HP-4 classification is carried by the
    /// request item and removes every candidate, even when EN-1 advertises
    /// temporal satisfaction.
    #[trace("QSpec-TC-342", "QSpec-FR-396-AC-2")]
    #[test]
    fn an_hp4_hyper_form_is_unsupported_and_has_no_route_candidate() {
        use crate::{BackendDescriptor, Candidate, Mode, ProviderOrigin};
        use qsl_semantics::check::{HyperForm, HyperUnsupported};

        let mut registry = Registry::new();
        registry
            .register(
                BackendDescriptor::new(
                    Candidate::new(BackendId::new("en1")),
                    ProviderOrigin::Linked,
                    [(Capability::TemporalSatisfaction, Mode::Unbounded)],
                    None,
                    None,
                )
                .expect("a well-formed descriptor"),
            )
            .expect("admitted");
        let reason = HyperUnsupported::ExistentialBeforeUniversal;
        let requirements =
            Requirements::hyper_form(ClaimExtent::Bounded, &HyperForm::Other { reason });
        let mut writer = RequestWriter::new();
        writer.item(occurrence(), &requirements);
        let [item] = writer.items() else {
            panic!("one requested hyper item");
        };
        assert_eq!(
            item.unsupported(),
            Some(&UnsupportedItem::HyperForm { reason })
        );
        let CandidateOutcome::Candidates(candidates) =
            candidates_of(&requirements, None, &registry, None)
        else {
            panic!("an ordinary candidate set");
        };
        assert!(candidates.candidates().is_empty());

        let admitted = Requirements::hyper_form(ClaimExtent::Bounded, &HyperForm::Universal);
        let CandidateOutcome::Candidates(candidates) =
            candidates_of(&admitted, None, &registry, None)
        else {
            panic!("an ordinary candidate set");
        };
        assert_eq!(
            candidates.candidates(),
            [Candidate::new(BackendId::new("en1"))]
        );
    }

    /// QSpec FR-368-AC-21, and the producer half of AC-13 (the five
    /// spellings QSL writes; refusing another spelling is the reader's): a
    /// temporal item over a model subject has one of exactly five kinds, `formula`, `deadlock-freedom`,
    /// `time-lock-freedom`, `race-freedom` and `refinement`; `transfer` is
    /// none of them.
    #[trace("QSpec-TC-431", "QSpec-FR-368-AC-21")]
    #[test]
    fn a_model_items_kind_is_one_of_five_spellings() {
        let spelled: Vec<_> = ItemKind::ALL.into_iter().map(ItemKind::to_wire).collect();
        assert_eq!(
            spelled,
            [
                "formula",
                "deadlock-freedom",
                "time-lock-freedom",
                "race-freedom",
                "refinement"
            ]
        );
        assert!(!spelled.contains(&"transfer"));
    }

    /// The limit-name half of QSpec FR-368-AC-20 (refusing a derived item
    /// that carries `max_depth` is the reader's): a derived item's `limits`
    /// hold a caller's
    /// setting of `max_states`, `max_transitions`, `max_clocks`,
    /// `max_store_buffer`, `max_messages`, `max_live_instances`,
    /// `max_solver_steps` and `max_solver_time` only; `max_automaton_states`,
    /// `max_walk_steps`, `max_witness_set` and `max_relation_tuples` are each
    /// refused `invalid_runtime_input`/`unknown-member`, and a limit the
    /// caller does not set is absent, so the provider's default applies.
    #[trace("QSpec-TC-431")]
    #[test]
    fn a_derived_items_limits_hold_only_exhaustive_check_limits() {
        let names = [
            "max_states",
            "max_transitions",
            "max_clocks",
            "max_store_buffer",
            "max_messages",
            "max_live_instances",
            "max_solver_steps",
            "max_solver_time",
        ];
        let all = DerivedLimits::from_settings(names.iter().zip(1..).map(|(n, v)| (*n, v)))
            .expect("every exhaustive-check limit is a member");
        assert_eq!(
            all.iter()
                .map(|(limit, value)| (limit.name(), value))
                .collect::<Vec<_>>(),
            names.iter().copied().zip(1..).collect::<Vec<_>>()
        );
        for refused in [
            "max_automaton_states",
            "max_walk_steps",
            "max_witness_set",
            "max_relation_tuples",
        ] {
            let refusal =
                DerivedLimits::from_settings([("max_states", 5), (refused, 1)]).expect_err(refused);
            assert_eq!(refusal.name, refused);
            assert_eq!(
                refusal.catalog_code(),
                CatalogCode::new("invalid_runtime_input", "unknown-member")
            );
        }
        let one = DerivedLimits::from_settings([("max_clocks", 4)]).expect("a member");
        assert_eq!(one.get(DerivedLimit::MaxClocks), Some(4));
        assert_eq!(one.get(DerivedLimit::MaxStates), None);
        assert_eq!(DerivedLimits::default().iter().count(), 0);
    }
}
