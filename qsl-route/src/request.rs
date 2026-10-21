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
//! follow-up's [`RequirementItem::proof_bounds`] (and, for a temporal item,
//! [`RequirementItem::temporal_proof_bounds`]) carry them with QSL's domain
//! kinds.
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
use qsl_foundation::RequestIndex;
use qsl_foundation::{CatalogCode, CatalogCoded};
use qsl_semantics::check::{Capability, PathGuard, RequirementRecord};
use qsl_semantics::family::{ClaimExtent, Requirements, UnsupportedItem};

use crate::{CandidateOutcome, CandidateSet, Registry};
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

/// One requested item (ADR-013 O-20): the occurrence it requests a claim
/// at, the checked node, its capability kind, its extent classification
/// and the proof bounds it was requested over. Two items are equal only
/// when every member is, so an item with proof bounds is never equal to
/// the unbounded item it answers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestItem {
    index: RequestIndex,
    occurrence: OccurrenceKey,
    kind: Capability,
    extent: ExtentClassification,
    domains: Vec<ProofBound>,
    unsupported: Option<UnsupportedItem>,
}

impl RequestItem {
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
}

impl CatalogCoded for BoundRefusal {
    /// Every case is `invalid_runtime_input`/`invalid-value` (ADR-014 §4).
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::UnknownDomain { .. }
            | Self::UnboundableDomain { .. }
            | Self::KindMismatch { .. }
            | Self::MissingDomain { .. }
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
}

impl RequestWriter {
    /// An empty request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Write the item for the claim at `occurrence` with
    /// `requirements` and no proof bounds. Its classification comes from
    /// the checked extent alone.
    pub fn item(&mut self, occurrence: OccurrenceKey, requirements: &Requirements) -> RequestIndex {
        let extent = ExtentClassification::of(requirements.extent());
        self.push(occurrence, requirements, extent, Vec::new())
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
            occurrence,
            requirements,
            ExtentClassification::Bounded,
            domains,
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

    fn push(
        &mut self,
        occurrence: OccurrenceKey,
        requirements: &Requirements,
        extent: ExtentClassification,
        domains: Vec<ProofBound>,
    ) -> RequestIndex {
        let index = RequestIndex::new(self.items.len());
        self.items.push(RequestItem {
            index,
            occurrence,
            kind: requirements.kind(),
            extent,
            domains,
            unsupported: requirements.unsupported().cloned(),
        });
        index
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
    extent: ExtentClassification,
    unbounded: Vec<(DomainKey, DomainKind)>,
    proof_bounds: Vec<ProofBound>,
    result_bound: WireNodeId,
    conversion_context: Option<OccurrenceKey>,
    path_condition: Vec<PathGuard>,
    candidates: CandidateOutcome,
    unsupported: Option<UnsupportedItem>,
}

impl RequirementItem {
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
    /// `temporal.subject.proof_bounds`, FR-361): `None` for an item that is
    /// not `temporal-satisfaction`; for a temporal item, the same
    /// substitution as [`RequirementItem::proof_bounds`], since the request
    /// substitutes one set of proof bounds per item. `Some(&[])` is a
    /// temporal item with no substitution.
    ///
    /// Interim (STD-213): QSpec FR-331 also shows a temporal subject bounded
    /// apart from its extent, an item whose extent stays `unbounded` while
    /// its subject carries a population bound, and FR-368 gives a subject
    /// only to an item over a model subject. QSL defines neither a
    /// subject-only substitution nor which temporal record is over a model
    /// subject, so this reads the extent substitution for every
    /// `temporal-satisfaction` item until STD-213 settles both.
    pub fn temporal_proof_bounds(&self) -> Option<&[ProofBound]> {
        matches!(self.kind, Capability::TemporalSatisfaction)
            .then_some(self.proof_bounds.as_slice())
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

/// The requested items of a checked package's requirement `records`
/// (`CheckedPackage::graph().requirements()`), one per record in the
/// records' occurrence-key order, each with its candidate outcome from
/// `registry` for the backend `named`, when the request names one, filtered
/// by the fairness kinds the record's requirements use (FR-134), and the
/// record's canonical path condition. An item whose candidate set is empty
/// or whose extent is unbounded is written like any other; negotiation
/// settles it.
pub fn items_from_requirements(
    records: &BTreeMap<OccurrenceKey, RequirementRecord>,
    registry: &Registry,
    named: Option<&BackendId>,
) -> Vec<RequirementItem> {
    records
        .iter()
        .enumerate()
        .map(|(position, (occurrence, record))| {
            let requirements = record.requirements();
            let index = RequestIndex::new(position);
            let extent = ExtentClassification::of(requirements.extent());
            let unbounded = match requirements.extent() {
                ClaimExtent::Bounded => Vec::new(),
                ClaimExtent::Unbounded(domains) => domains
                    .iter()
                    .map(|(key, kind)| (key.clone(), kind))
                    .collect(),
            };
            RequirementItem {
                index,
                occurrence: occurrence.clone(),
                kind: requirements.kind(),
                extent,
                unbounded,
                proof_bounds: Vec::new(),
                result_bound: record.result_bound().node(),
                conversion_context: record.conversion_context().cloned(),
                path_condition: record.path_condition().to_vec(),
                candidates: candidates_of(requirements, registry, named),
                unsupported: requirements.unsupported().cloned(),
            }
        })
        .collect()
}

/// The candidate outcome of one item's `requirements`: backends of its
/// capability kind that advertise every fairness kind it uses (FR-134). An
/// item the writer disposes `Unsupported` has no candidate (FR-234).
fn candidates_of(
    requirements: &Requirements,
    registry: &Registry,
    named: Option<&BackendId>,
) -> CandidateOutcome {
    if requirements.unsupported().is_some() {
        return CandidateOutcome::Candidates(CandidateSet::none(requirements.kind(), named));
    }
    registry.candidates_for_fairness(requirements.kind(), named, requirements.fairness())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

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
        use crate::{BackendDescriptor, Candidate, ManifestDigest, Mode, ProviderOrigin};
        use quire_semantic_value::fairness::FairnessKind::{Strong, Weak};

        let temporal =
            |id: &str, seed: u8, kinds: &[quire_semantic_value::fairness::FairnessKind]| {
                BackendDescriptor::new(
                    Candidate::new(BackendId::new(id)),
                    ProviderOrigin::Linked,
                    ManifestDigest::from_digest([seed; 32]),
                    [(Capability::TemporalSatisfaction, Mode::Unbounded)],
                    None,
                )
                .expect("a well-formed descriptor")
                .with_fairness_kinds(kinds.iter().copied())
            };
        let mut registry = Registry::new();
        registry
            .register(temporal("both", 1, &[Weak, Strong]))
            .expect("admitted");
        registry
            .register(temporal("weak-only", 2, &[Weak]))
            .expect("admitted");
        let item = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);

        let CandidateOutcome::Candidates(strong) =
            candidates_of(&item.clone().with_fairness([Strong]), &registry, None)
        else {
            panic!("a candidate outcome");
        };
        assert_eq!(
            strong.candidates(),
            [Candidate::new(BackendId::new("both"))]
        );
        let CandidateOutcome::Candidates(plain) = candidates_of(&item, &registry, None) else {
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
        use crate::{BackendDescriptor, Candidate, ManifestDigest, Mode, ProviderOrigin};
        use qsl_semantics::family::UnsupportedItem;

        let mut registry = Registry::new();
        registry
            .register(
                BackendDescriptor::new(
                    Candidate::new(BackendId::new("zones")),
                    ProviderOrigin::Linked,
                    ManifestDigest::from_digest([3; 32]),
                    [(Capability::TemporalSatisfaction, Mode::Unbounded)],
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

        let CandidateOutcome::Candidates(none) = candidates_of(&punctual, &registry, None) else {
            panic!("a candidate outcome");
        };
        assert!(none.candidates().is_empty());
        let CandidateOutcome::Candidates(some) = candidates_of(&plain, &registry, None) else {
            panic!("a candidate outcome");
        };
        assert_eq!(some.candidates(), [Candidate::new(BackendId::new("zones"))]);
    }
}
