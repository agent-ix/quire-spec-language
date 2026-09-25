// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-20, ADR-014 §4 and §6 step 3 (QSL-140): QSL's typed request
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
//! **Refusals.** Before any item is written, the writer refuses a bound for
//! a key that names no unbounded domain of the item, a bound on a domain no
//! finite bound can stand for, a bound of the wrong kind, and a set that
//! misses a domain. An empty or inverted range cannot be built at all
//! ([`FiniteBound`]'s constructors refuse it). Two bounds for one domain
//! cannot be passed: the bounds are a map keyed by [`DomainKey`]. Every
//! refusal is `invalid_runtime_input`/`invalid-value`.

use std::collections::BTreeMap;

use qsl_foundation::bound::{DomainKey, FiniteBound, FiniteBoundKind, ProofBound};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{CatalogCode, CatalogCoded};
use qsl_semantics::check::Capability;
use qsl_semantics::family::{ClaimExtent, DomainKind, Requirements};

/// An item's position in one request (QSpec FR-331 `request_index`). Each
/// item, bounded or not, has its own.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestIndex(usize);

impl RequestIndex {
    /// The zero-based position.
    pub fn get(self) -> usize {
        self.0
    }
}

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

/// One requested item (ADR-013 O-20): the checked node, its capability
/// kind, its extent classification and the proof bounds it was requested
/// over. Two items are equal only when every member is, so an item with
/// proof bounds is never equal to the unbounded item it answers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestItem {
    index: RequestIndex,
    node: WireNodeId,
    kind: Capability,
    extent: ExtentClassification,
    domains: Vec<ProofBound>,
}

impl RequestItem {
    /// This item's own request index.
    pub fn index(&self) -> RequestIndex {
        self.index
    }

    /// The checked node the item requests a claim over.
    pub fn node(&self) -> WireNodeId {
        self.node
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
    /// [`DomainKey`] order (QSpec FR-331 request `domains`). Empty for an
    /// item requested without bounds.
    pub fn domains(&self) -> &[ProofBound] {
        &self.domains
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
}

impl CatalogCoded for BoundRefusal {
    /// Every case is `invalid_runtime_input`/`invalid-value` (ADR-014 §4).
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::UnknownDomain { .. }
            | Self::UnboundableDomain { .. }
            | Self::KindMismatch { .. }
            | Self::MissingDomain { .. } => {
                CatalogCode::new("invalid_runtime_input", "invalid-value")
            }
        }
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

    /// Write the item for `node` with `requirements` and no proof bounds.
    /// Its classification comes from the checked extent alone.
    pub fn item(&mut self, node: WireNodeId, requirements: &Requirements) -> RequestIndex {
        let extent = ExtentClassification::of(requirements.extent());
        self.push(node, requirements.kind(), extent, Vec::new())
    }

    /// Write the bounded item for `node` with `requirements`, substituting
    /// `bounds` for its unbounded domains (ADR-014 §4 "Bounded request").
    /// Refuses, writing nothing, unless `bounds` names exactly the item's
    /// unbounded domains, each boundable, each with a bound of its kind.
    pub fn bounded_item(
        &mut self,
        node: WireNodeId,
        requirements: &Requirements,
        bounds: BTreeMap<DomainKey, FiniteBound>,
    ) -> Result<RequestIndex, BoundRefusal> {
        let domains = match requirements.extent() {
            ClaimExtent::Bounded => {
                if let Some(domain) = bounds.into_keys().next() {
                    return Err(BoundRefusal::UnknownDomain { domain });
                }
                Vec::new()
            }
            ClaimExtent::Unbounded(unbounded) => {
                for (domain, bound) in &bounds {
                    let Some(kind) = unbounded.kind(domain) else {
                        return Err(BoundRefusal::UnknownDomain {
                            domain: domain.clone(),
                        });
                    };
                    let Some(expected) = kind.finite_kind() else {
                        return Err(BoundRefusal::UnboundableDomain {
                            domain: domain.clone(),
                            kind,
                        });
                    };
                    if bound.kind() != expected {
                        return Err(BoundRefusal::KindMismatch {
                            domain: domain.clone(),
                            expected,
                            supplied: bound.kind(),
                        });
                    }
                }
                if let Some((domain, _)) = unbounded
                    .iter()
                    .find(|(domain, _)| !bounds.contains_key(domain))
                {
                    return Err(BoundRefusal::MissingDomain {
                        domain: domain.clone(),
                    });
                }
                bounds
                    .into_iter()
                    .map(|(domain, bound)| ProofBound { domain, bound })
                    .collect()
            }
        };
        Ok(self.push(
            node,
            requirements.kind(),
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
        node: WireNodeId,
        kind: Capability,
        extent: ExtentClassification,
        domains: Vec<ProofBound>,
    ) -> RequestIndex {
        let index = RequestIndex(self.items.len());
        self.items.push(RequestItem {
            index,
            node,
            kind,
            extent,
            domains,
        });
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn node(fill: u8) -> WireNodeId {
        WireNodeId::from_digest([fill; 32])
    }

    fn key(fill: u8, path: &[u32]) -> DomainKey {
        DomainKey::new(node(fill), path.to_vec())
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
        let bounded = writer.item(node(9), &claim(&[]));
        let set = writer.item(node(9), &set_claim());
        let set_and_loop = writer.item(
            node(9),
            &claim(&[
                (key(1, &[]), DomainKind::Collection),
                (key(2, &[]), DomainKind::Loop),
            ]),
        );
        let formula = writer.item(
            node(9),
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
        let unbounded = writer.item(node(9), &requirements);
        let eight = writer
            .bounded_item(
                node(9),
                &requirements,
                BTreeMap::from([
                    (key(1, &[]), FiniteBound::cardinality(8)),
                    (key(1, &[0]), depth(3)),
                ]),
            )
            .expect("one bound per domain, each of its kind");
        let four = writer
            .bounded_item(
                node(9),
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
                ProofBound {
                    domain: key(1, &[]),
                    bound: FiniteBound::cardinality(8),
                },
                ProofBound {
                    domain: key(1, &[0]),
                    bound: depth(3),
                },
            ]
        );
        // The proof bounds are part of the item: different bounds, different
        // items; neither is the unbounded item.
        assert_ne!(items[eight.get()].domains(), items[four.get()].domains());
        assert_ne!(items[eight.get()], items[unbounded.get()]);
        // A bounded claim needs no proof bound: an empty set is its item.
        let mut writer = RequestWriter::new();
        let index = writer
            .bounded_item(node(9), &claim(&[]), BTreeMap::new())
            .unwrap();
        assert_eq!(
            writer.items()[index.get()].extent(),
            ExtentClassification::Bounded
        );
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
                BoundRefusal::UnknownDomain {
                    domain: key(1, &[]),
                },
            ),
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
                BoundRefusal::MissingDomain {
                    domain: key(2, &[]),
                },
            ),
            (
                set_claim(),
                BTreeMap::new(),
                BoundRefusal::MissingDomain {
                    domain: key(1, &[]),
                },
            ),
        ];
        for (requirements, bounds, expected) in cases {
            let mut writer = RequestWriter::new();
            let refusal = writer
                .bounded_item(node(9), &requirements, bounds)
                .expect_err("the bounds refuse");
            assert_eq!(refusal, expected);
            assert_eq!(
                refusal.catalog_code(),
                CatalogCode::new("invalid_runtime_input", "invalid-value")
            );
            assert!(writer.items().is_empty(), "nothing is written");
        }
    }
}
