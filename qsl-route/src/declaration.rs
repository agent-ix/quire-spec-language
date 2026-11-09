// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-529 and FR-530 "Authoring and compile": the request items an
//! implementation-guarantee declaration and a component-composition
//! declaration derive (QSpec FR-490 "Derived items", FR-491 "Derived item
//! and slots").
//!
//! [`RequestWriter::declaration_items`] writes one item for every
//! item-backed premise slot of a checked package, in the declarations' order
//! and, within one declaration, in derived slot order:
//!
//! - an `operation-contract` item carries QSpec FR-331's `correspondence`
//!   member ([`RequestItem::correspondence`]);
//! - the model-claim `temporal-satisfaction` item of a liveness clause
//!   carries `stutter_closure` ([`RequestItem::stutter_closure`]);
//! - a `composition` item carries `premises`, its premise slots in order
//!   ([`RequestItem::premises`]): the item of a composition under a rule
//!   other than `assume-guarantee` holds every premise slot but the
//!   `fixed-point` slots, and a `fixed-point` slot's own item holds the
//!   component-guarantee keys of its cycle.
//!
//! A declaration whose item is refused (QSpec FR-490-AC-6, FR-491 "Dependency
//! graph and cycles") writes no item. Routing is unchanged: an item is
//! negotiated by its [`kind`](RequestItem::kind) as every other item is, so a
//! `composition` item with no composition-capable backend registered has an
//! empty candidate set (QSpec FR-348-AC-4).

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::{
    Capability, CheckedComposition, CheckedGraph, CheckedImplementationGuarantee,
    ComponentGuaranteeKey, CompositionRule, CompositionSlot, CompositionSlotKey, Correspondence,
    ExtentClass, SlotBacking, SlotKey, SlotRole,
};
use quire_exact::{NodeKey, Origin, Role};
use quire_semantic_value::fairness::PropertyForm;

use crate::request::{ItemKind, RequestItem, RequestWriter};

/// One entry of a `composition` item's `premises` member (QSpec FR-331
/// "Items"): a premise slot, spelled as QSpec FR-492's `slots`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemPremise {
    /// A `component-guarantee` slot, by its slot key.
    ComponentGuarantee(ComponentGuaranteeKey),
    /// Any other premise slot (`rely`, `liveness-dependency`, `fairness` or
    /// `environment`): not item-backed, so its key is its role and the
    /// members that produced it.
    Slot(Box<CompositionSlot>),
}

impl ItemPremise {
    /// The premise slot's role.
    pub fn role(&self) -> SlotRole {
        match self {
            Self::ComponentGuarantee(_) => SlotRole::ComponentGuarantee,
            Self::Slot(slot) => slot.role,
        }
    }

    fn of(slot: &CompositionSlot) -> Self {
        match &slot.key {
            Some(CompositionSlotKey::ComponentGuarantee(key)) => {
                Self::ComponentGuarantee(key.clone())
            }
            Some(CompositionSlotKey::FixedPoint { .. }) | None => {
                Self::Slot(Box::new(slot.clone()))
            }
        }
    }
}

/// A request item written for a declaration (FR-529, FR-530).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredItem {
    /// The declaration's node.
    pub declaration: NodeKey,
    /// The declaration's derived slot the item answers; `None` for the
    /// `composition` item of a rule other than `assume-guarantee`, which is
    /// the declaration's own item.
    pub slot: Option<usize>,
    /// The request item.
    pub item: RequestItem,
}

/// The members of a declared item beside the claim's.
pub(crate) struct Declared {
    pub(crate) occurrence: OccurrenceKey,
    pub(crate) kind: Capability,
    pub(crate) extent: ExtentClass,
    pub(crate) item_kind: Option<ItemKind>,
    pub(crate) property_form: Option<PropertyForm>,
    pub(crate) correspondence: Option<Correspondence>,
    pub(crate) stutter_closure: bool,
    pub(crate) premises: Vec<ItemPremise>,
}

impl Declared {
    /// The item an S3 slot key derives.
    fn of_key(key: &SlotKey, property_form: Option<PropertyForm>) -> Self {
        let temporal = key.kind == Capability::TemporalSatisfaction;
        Self {
            occurrence: OccurrenceKey::new(
                key.claim.clone(),
                Origin::new(Role::new("claim"), u64::from(key.ordinal)),
            ),
            kind: key.kind,
            extent: key.extent,
            item_kind: temporal.then_some(ItemKind::Formula),
            property_form: temporal.then_some(property_form).flatten(),
            correspondence: key.correspondence.clone(),
            stutter_closure: key.stutter_closure,
            premises: Vec::new(),
        }
    }

    /// A `composition` item anchored at `node`.
    fn composition(node: &NodeKey, premises: Vec<ItemPremise>) -> Self {
        Self {
            occurrence: OccurrenceKey::new(node.clone(), Origin::new(Role::new("claim"), 0)),
            kind: Capability::Composition,
            extent: ExtentClass::Unbounded,
            item_kind: None,
            property_form: None,
            correspondence: None,
            stutter_closure: false,
            premises,
        }
    }
}

impl RequestWriter {
    /// Write the items the guarantee and composition declarations of `graph`
    /// derive, each with its own [`qsl_foundation::RequestIndex`], and return them in write
    /// order. The package identity that fixes each slot key is the checked
    /// package's own ([`CheckedGraph::package_name`]).
    pub fn declaration_items(&mut self, graph: &CheckedGraph) -> Vec<DeclaredItem> {
        let package = graph.package_name();
        let mut written = Vec::new();
        for guarantee in graph.implementation_guarantees() {
            self.guarantee_items(guarantee, package, &mut written);
        }
        for composition in graph.compositions() {
            self.composition_items(composition, &mut written);
        }
        written
    }

    fn guarantee_items(
        &mut self,
        guarantee: &CheckedImplementationGuarantee,
        package: &str,
        written: &mut Vec<DeclaredItem>,
    ) {
        if guarantee.item_refusal().is_some() {
            return;
        }
        let form = Some(guarantee.form());
        for slot in guarantee.slots(package) {
            let (SlotBacking::Item, Some(key)) = (slot.backing, &slot.key) else {
                continue;
            };
            let item = self.declared_item(Declared::of_key(key, form));
            written.push(DeclaredItem {
                declaration: guarantee.node().clone(),
                slot: Some(slot.index),
                item,
            });
        }
    }

    fn composition_items(
        &mut self,
        composition: &CheckedComposition,
        written: &mut Vec<DeclaredItem>,
    ) {
        if composition.item_refusal().is_some() {
            return;
        }
        let declaration = composition.node().clone();
        let item = composition.item();
        if composition.rule() != CompositionRule::AssumeGuarantee {
            let premises = item.premises.iter().map(ItemPremise::of).collect();
            let own = self.declared_item(Declared::composition(&declaration, premises));
            written.push(DeclaredItem {
                declaration: declaration.clone(),
                slot: None,
                item: own,
            });
        }
        for slot in composition.slots() {
            let declared = match &slot.key {
                Some(CompositionSlotKey::ComponentGuarantee(key)) => {
                    let form = composition
                        .clauses()
                        .iter()
                        .find(|facts| facts.reference.node == key.key.claim)
                        .map(|facts| facts.form);
                    Declared::of_key(&key.key, form)
                }
                Some(CompositionSlotKey::FixedPoint { node, premises, .. }) => {
                    Declared::composition(
                        node,
                        premises
                            .iter()
                            .cloned()
                            .map(ItemPremise::ComponentGuarantee)
                            .collect(),
                    )
                }
                None => continue,
            };
            let item = self.declared_item(declared);
            written.push(DeclaredItem {
                declaration: declaration.clone(),
                slot: Some(slot.index),
                item,
            });
        }
    }
}
