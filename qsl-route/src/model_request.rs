// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336-AC-4 (ADR-013 O-09): the request writer's temporal items over
//! model subjects, each joined to its obligation identity, and the items it
//! derives from them (FR-124, FR-232, QSpec FR-368).
//!
//! An item's obligation identity is its subject and its kind
//! ([`ItemObligation`]), and the subject names its universes, so two
//! requests for one clause over different universes are two obligations and
//! two request indices. A result carries the [`RequestIndex`] of the item it
//! answers (QSpec FR-360), so [`WrittenRequest::obligation`] joins a result
//! to its own obligation and to no other.
//!
//! **Derived items** (QSpec FR-368 "Position of derived items in the
//! request"). A [`ModelRequest`] holds the caller's items, authored and
//! `refinement`, in the caller's order. [`ModelRequest::derived`] is the one
//! way to finish it: it appends the derived items after them, grouped by
//! subject in the order each subject first occurs among the caller's items
//! and, within one subject, `deadlock-freedom` then `time-lock-freedom`, so
//! adding or removing a derived item never moves a caller's item. Each
//! derived item's claim is its subject's anchor; a subject that reads
//! several `model_selections` rows has none, and its derived items are
//! settled `unsupported` with no request item ([`UnanchoredItem`]).

use std::collections::BTreeSet;

use qsl_foundation::{BackendId, RequestIndex};
use qsl_semantics::check::Capability;
use qsl_semantics::family::Requirements;
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::fairness::PropertyForm;

use crate::deadlock::{
    by_subject, ItemObligation, ModelRefinementItem, ModelStateGraphItem, ModelTemporalItem,
    ObligationKind, SubjectReading,
};
use crate::reductions::{ObligationReductions, Reductions};
use crate::request::{AbstractSubject, DerivedLimits, ItemKind, RequestItem, RequestWriter};
use crate::Registry;

/// The caller's items of one request over model subjects `S`, in index
/// order, each with its obligation identity and its subject's reading.
#[derive(Clone, Debug)]
pub struct ModelRequest<S> {
    writer: RequestWriter,
    /// `obligations[i]` is the obligation of the item at index `i`.
    obligations: Vec<ItemObligation<S>>,
    /// Each caller item's subject and reading, in caller order.
    readings: Vec<(S, SubjectReading)>,
}

impl<S> Default for ModelRequest<S> {
    fn default() -> Self {
        Self {
            writer: RequestWriter::new(),
            obligations: Vec::new(),
            readings: Vec::new(),
        }
    }
}

impl<S: Clone + Eq> ModelRequest<S> {
    /// An empty request.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty request selecting `reductions` (FR-159): one selection for
    /// every item, so two items of one request never differ by it.
    pub fn reduced(reductions: Reductions) -> Self {
        Self {
            writer: RequestWriter::reduced(reductions),
            ..Self::default()
        }
    }

    /// Write the `formula` item for `item` with its `requirements`, and no
    /// proof bounds, over a subject that reads the `model_selections` rows
    /// whose identities are `selections`. The requirement record is the
    /// checked clause's alone (FR-336-AC-1), so the subject's universes
    /// reach only the item's obligation identity. An item whose obligation
    /// is already written is not written twice: the existing item's index
    /// is returned, so one obligation has one index. The item carries the
    /// request's reductions, and its obligation identity holds the symmetry
    /// declarations and the partial-order selection beside the subject,
    /// never the constraint (FR-159 "Identity", FR-158).
    pub fn authored(
        &mut self,
        item: &ModelTemporalItem<'_, S>,
        requirements: &Requirements,
        selections: &BTreeSet<&str>,
    ) -> RequestIndex {
        let obligation = item.obligation().reduced(self.writer.reductions());
        if let Some(index) = self.index_of(&obligation) {
            return index;
        }
        let index = self.writer.model_item(
            item.occurrence.clone(),
            requirements,
            ItemKind::Formula,
            None,
        );
        self.obligations.push(obligation);
        let reading = SubjectReading::read(item.package, |identity| selections.contains(identity));
        self.readings.push((item.subject.clone(), reading));
        index
    }

    /// Write the `refinement` item for `item` with its record's
    /// `requirements` (FR-144), as [`Self::authored`] writes a clause's: the
    /// item is keyed by the refinement's occurrence, carries
    /// `abstract_subject` (QSpec FR-368-AC-19), and its obligation is the
    /// concrete subject, the occurrence and the abstract subject's initial
    /// snapshots. `None` for a refinement with no record.
    pub fn refinement(
        &mut self,
        item: &ModelRefinementItem<'_, S>,
        requirements: &Requirements,
        abstract_subject: AbstractSubject,
    ) -> Option<RequestIndex> {
        let occurrence = item.occurrence()?.clone();
        let obligation = item.obligation()?.reduced(self.writer.reductions());
        if let Some(index) = self.index_of(&obligation) {
            return Some(index);
        }
        let index = self.writer.model_item(
            occurrence,
            requirements,
            ItemKind::Refinement,
            Some(abstract_subject),
        );
        self.obligations.push(obligation);
        self.readings.push((item.subject.clone(), item.reading()));
        Some(index)
    }

    /// Join the subject of the state-graph item `item` (FR-165), over a
    /// subject reading the `model_selections` rows `selections`, to the
    /// derivation: state-graph claims share their model subject's derived
    /// items with temporal claims. The state-graph item's own request item
    /// is written from its record, so nothing is written here.
    pub fn state_graph(&mut self, item: &ModelStateGraphItem<'_, S>, selections: &BTreeSet<&str>) {
        let reading = SubjectReading::read(item.package, |identity| selections.contains(identity));
        self.readings.push((item.subject.clone(), reading));
    }

    /// The obligation of the item at `index`: the join of a result that
    /// carries `index` to the obligation it answers. `None` for an index
    /// this request did not write.
    pub fn obligation(&self, index: RequestIndex) -> Option<&ItemObligation<S>> {
        self.obligations.get(index.get())
    }

    /// The index of the item with `obligation`, when this request wrote it.
    pub fn index_of(&self, obligation: &ItemObligation<S>) -> Option<RequestIndex> {
        index_of(&self.obligations, obligation)
    }

    /// The caller's items written so far, in index order.
    pub fn items(&self) -> &[RequestItem] {
        self.writer.items()
    }

    /// The finished request: the caller's items, then the derived items of
    /// their subjects (FR-124, FR-232, QSpec FR-368 "Position of derived
    /// items in the request"). Each subject that a model it reads does not
    /// opt out by `terminal any` gets a `deadlock-freedom` item, and each
    /// timed subject a `time-lock-freedom` item. A derived item's
    /// candidates are the `temporal-satisfaction` backends under `registry`
    /// for the backend `named`, restricted to EN-6's zone search, the
    /// backends deciding timed subjects, for a timed subject (FR-243) and to those advertising the request's
    /// reductions (FR-159); it carries `limits`, the caller's settings, and
    /// no `max_depth`. A subject with no stated anchor gets no request item
    /// and an [`UnanchoredItem`] per derived item instead.
    pub fn derived(
        self,
        registry: &Registry,
        named: Option<&BackendId>,
        limits: &DerivedLimits,
    ) -> WrittenRequest<S> {
        let Self {
            mut writer,
            mut obligations,
            readings,
        } = self;
        let selected: BTreeSet<_> = writer.reductions().selected().into_iter().collect();
        let mut unanchored = Vec::new();
        for (subject, reading) in by_subject(readings) {
            let kinds = [
                (!reading.opted_out()).then_some(Derived::DeadlockFreedom),
                reading.timed().then_some(Derived::TimeLockFreedom),
            ];
            for derived in kinds.into_iter().flatten() {
                let kind = derived.item_kind();
                let claim = match reading.anchor() {
                    Ok(claim) => claim,
                    Err(rows) => {
                        unanchored.push(UnanchoredItem {
                            subject: subject.clone(),
                            kind,
                            rows,
                        });
                        continue;
                    }
                };
                let candidates = registry.restrict_to_reductions(
                    registry.restrict_to_dense_time(
                        registry.candidates_for_temporal_dynamics(
                            Capability::TemporalSatisfaction,
                            named,
                            &BTreeSet::new(),
                            &BTreeSet::new(),
                            false,
                        ),
                        reading.timed(),
                    ),
                    &selected,
                );
                writer.derived_item(
                    claim,
                    kind,
                    derived.property_form(),
                    candidates,
                    limits.clone(),
                );
                obligations.push(
                    ItemObligation {
                        subject: subject.clone(),
                        kind: derived.obligation_kind(),
                        reductions: ObligationReductions::default(),
                    }
                    .reduced(writer.reductions()),
                );
            }
        }
        WrittenRequest {
            items: writer.finish(),
            obligations,
            unanchored,
        }
    }
}

/// A derived item over a model subject the writer adds (FR-124, FR-232),
/// in FR-368's order within one subject.
#[derive(Clone, Copy, Debug)]
enum Derived {
    DeadlockFreedom,
    TimeLockFreedom,
}

impl Derived {
    const fn item_kind(self) -> ItemKind {
        match self {
            Self::DeadlockFreedom => ItemKind::DeadlockFreedom,
            Self::TimeLockFreedom => ItemKind::TimeLockFreedom,
        }
    }

    /// The obligation identity's fixed kind (ADR-013 O-09).
    const fn obligation_kind(self) -> ObligationKind {
        match self {
            Self::DeadlockFreedom => ObligationKind::DeadlockFreedom,
            Self::TimeLockFreedom => ObligationKind::TimeLockFreedom,
        }
    }

    /// The property form the item is routed and settled as: the
    /// deadlock-freedom item's `ReachableInvariant` (FR-124); the
    /// time-lock-freedom item is settled by FR-235 and has none.
    const fn property_form(self) -> Option<PropertyForm> {
        match self {
            Self::DeadlockFreedom => Some(PropertyForm::ReachableInvariant),
            Self::TimeLockFreedom => None,
        }
    }
}

fn index_of<S: Eq>(
    obligations: &[ItemObligation<S>],
    obligation: &ItemObligation<S>,
) -> Option<RequestIndex> {
    obligations
        .iter()
        .position(|written| written == obligation)
        .map(RequestIndex::new)
}

/// A finished request over model subjects `S`: the caller's items, then
/// the derived items, in index order, each with its obligation identity.
#[derive(Clone, Debug)]
pub struct WrittenRequest<S> {
    items: Vec<RequestItem>,
    obligations: Vec<ItemObligation<S>>,
    unanchored: Vec<UnanchoredItem<S>>,
}

impl<S: Eq> WrittenRequest<S> {
    /// Every item, in index order.
    pub fn items(&self) -> &[RequestItem] {
        &self.items
    }

    /// The obligation of the item at `index`; `None` for an index this
    /// request did not write.
    pub fn obligation(&self, index: RequestIndex) -> Option<&ItemObligation<S>> {
        self.obligations.get(index.get())
    }

    /// The index of the item with `obligation`, when this request wrote it.
    pub fn index_of(&self, obligation: &ItemObligation<S>) -> Option<RequestIndex> {
        index_of(&self.obligations, obligation)
    }

    /// The derived items the writer settled `unsupported` with no request
    /// item, in derivation order.
    pub fn unanchored(&self) -> &[UnanchoredItem<S>] {
        &self.unanchored
    }
}

/// A derived item over a model subject with no stated anchor (QSpec
/// FR-368-AC-22): the subject reads `rows` `model_selections` rows, not
/// one, so the producer settles the item `unsupported` with
/// `unsupported_projection`/`unsupported-requested-capability` and writes
/// no request item for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnanchoredItem<S> {
    /// The subject.
    pub subject: S,
    /// The derived item's kind.
    pub kind: ItemKind,
    /// The number of `model_selections` rows the subject reads.
    pub rows: usize,
}

impl<S> UnanchoredItem<S> {
    /// The settlement's diagnostic code.
    pub const fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("unsupported_projection", "unsupported-requested-capability")
    }

    /// The diagnostic's `detail`: the item kind and that the subject reads
    /// several model selections.
    pub fn detail(&self) -> String {
        format!(
            "the {} item's subject reads {} model selections, so it has no anchor",
            self.kind.to_wire(),
            self.rows
        )
    }
}
