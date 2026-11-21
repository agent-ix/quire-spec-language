// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1044: the items a `compose` declaration selects through `analyze`.
//!
//! An item naming a checked `compose` declaration with `kind` `composition`
//! selects the declaration's composition. Its premise items, the component
//! guarantees and the cycle's `fixed-point` slot, are derived from the
//! declaration, never supplied, and settle in the same request after the
//! caller's items. A component guarantee is a clause of the component's own
//! checked package, so its item is resolved over that package and run over it
//! by the shared analyze path ([`CompositionClaim::package`]).
//!
//! No registered backend advertises `composition`, so a routed composition
//! settles `unsupported` at negotiation, and the `assume-guarantee` item,
//! which has no FSpec FR-331 request item, settles `unsupported` before it.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::planner::{composition_premises, CompositionPremise, ItemKey, SlotKey, SlotRole};
use fsl_replay::TerminalRecord;
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::Registry;
use fsl_semantics::check::{
    Capability, CheckedComposition, ClauseRef, CompositionRule, CompositionSlotKey,
};
use fsl_semantics::library::LibraryName;
use serde_json::{json, Map, Value};

use super::{
    candidate_list, invalid_kind, resolve_one, unknown_option, ItemSelection, ResolvedClaim,
    ResolvedItem, SubjectSelection,
};
use crate::analyze::{
    ClaimItem, CompositionItem, Diagnostic, ItemForm, ItemOptions, Payload, SubjectBinding,
};

/// A claim item the composition selection derives or settles (FR-1044),
/// beside the ordinary [`ResolvedItem::Claim`].
#[derive(Clone)]
pub struct CompositionClaim {
    /// The claim item and its candidates.
    pub claim: ResolvedClaim,
    /// The checked package of the component the claim is a clause of, which
    /// the item is analyzed over; `None` for a claim of the request's own
    /// package.
    pub package: Option<Arc<CheckedPackage>>,
    /// The record the item settles with before negotiation (FR-621 `S4`);
    /// `None` when negotiation settles it.
    pub settled: Option<Box<TerminalRecord>>,
    /// For a composition, the request index of the item of each of its
    /// component-guarantee premises, as assigned by [`premise_items`]: the
    /// items whose own settlements decide whether a provider's `proved` for
    /// the composition stands. Empty for every other claim.
    pub required: Vec<RequestIndex>,
}

/// What resolving one selection of a composition gives.
pub(super) struct Selected {
    /// The composition's item.
    pub item: ResolvedItem,
    /// The premise items the selection derives, in FSpec FR-491 slot order;
    /// empty when the selection settled before its slots were derived.
    pub premises: Vec<Premise>,
}

/// One premise item a selected composition derives.
pub(super) struct Premise {
    /// The slot's key, which identifies the item within the request.
    key: ItemKey,
    kind: PremiseKind,
}

enum PremiseKind {
    /// A `component-guarantee` slot.
    Guarantee {
        clause: ClauseRef,
        library: LibraryName,
        /// Whether the guarantee is checked under peer guarantees, which no
        /// registered engine reads.
        assumed: bool,
    },
    /// A cycle's `fixed-point` slot.
    FixedPoint {
        name: String,
        node: filament_exact::NodeKey,
        profile: String,
        premises: Vec<Value>,
    },
    /// A slot whose component or key the checked declaration does not
    /// resolve: an invariant break, settled as a typed `failed` record.
    Unresolved { composition: String },
}

/// What a selection is resolved against.
pub(super) struct Context<'a> {
    pub package: &'a CheckedPackage,
    pub subject: &'a SubjectSelection,
    pub registry: &'a Registry,
    pub binding: &'a SubjectBinding<'a>,
}

fn composition_named<'p>(
    package: &'p CheckedPackage,
    name: &str,
) -> Option<&'p CheckedComposition> {
    package
        .graph()
        .compositions()
        .iter()
        .find(|composition| composition.name() == name)
}

/// The composition of `package` declared at `node`.
pub(super) fn composition_at<'p>(
    package: &'p CheckedPackage,
    node: &filament_exact::NodeKey,
) -> Option<&'p CheckedComposition> {
    package
        .graph()
        .compositions()
        .iter()
        .find(|composition| composition.node() == node)
}

fn occurrence(node: &filament_exact::NodeKey) -> OccurrenceKey {
    OccurrenceKey::new(
        node.clone(),
        filament_exact::Origin::new(filament_exact::Role::new("claim"), 0),
    )
}

pub(super) fn failed(index: RequestIndex, name: &str) -> ResolvedItem {
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_failed(
            index,
            filament_semantic_value::catalog::InternalFault::new(
                "composition-selection",
                "slots-are-derived",
            )
            .catalog_code(),
        )),
    }
}

fn unsupported(index: RequestIndex, kind: Capability) -> TerminalRecord {
    let mut detail = Map::new();
    detail.insert("kind".to_owned(), json!(kind.to_wire()));
    TerminalRecord::disposition_unsupported(
        index,
        fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
        detail,
    )
}

/// Resolve `selection` when it names a checked `compose` declaration of
/// `ctx.package`; `None` for any other name.
pub(super) fn resolve(
    ctx: &Context<'_>,
    index: RequestIndex,
    selection: &ItemSelection,
) -> Option<Selected> {
    let name = selection.claim.as_deref()?;
    let composition = composition_named(ctx.package, name)?;
    let settled = |item: ResolvedItem| {
        Some(Selected {
            item,
            premises: Vec::new(),
        })
    };
    let Some(label) = selection.kind.as_deref() else {
        return settled(invalid_kind(index, Some(name), "absent-kind"));
    };
    let kind = match Capability::from_wire(label) {
        Ok(kind) => kind,
        Err(_) => return settled(invalid_kind(index, Some(name), "unknown-kind")),
    };
    let planner_local = composition.rule() == CompositionRule::AssumeGuarantee;
    if kind != Capability::Composition {
        // FR-507: only a composition applies to its own claim. An
        // `assume-guarantee` claim has no family.
        let diagnostic = Diagnostic {
            code: CatalogCode::new("invalid_capability", "inapplicable-kind"),
            payload: Payload::Inapplicable {
                request_index: index,
                capability: kind,
                family: if planner_local {
                    None
                } else {
                    crate::claim_family::claim_family(
                        ctx.package,
                        ItemKind::Formula,
                        &occurrence(composition.node()),
                    )
                },
            },
        };
        return settled(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(TerminalRecord::disposition_invalid_request(
                index,
                diagnostic.code,
                diagnostic.payload.detail(),
            )),
        });
    }
    if let Some(refusal) = composition.item_refusal() {
        let cycles: Vec<Vec<String>> = refusal
            .cycles
            .iter()
            .map(|cycle| cycle.iter().map(ToString::to_string).collect())
            .collect();
        let mut detail = Map::new();
        detail.insert("cycles".to_owned(), json!(cycles));
        return settled(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(TerminalRecord::disposition_invalid_request(
                index,
                CatalogCode::new("invalid_model_binding", refusal.cause()),
                detail,
            )),
        });
    }
    let slots = match composition_premises(ctx.package, composition.node()) {
        Ok(slots) => slots,
        Err(_) => return settled(failed(index, name)),
    };
    let Ok(listed) = list(&slots) else {
        return settled(failed(index, name));
    };
    if let Some(caller) = selection.premises.as_ref() {
        if *caller != Value::Array(listed.clone()) {
            let mut detail = Map::new();
            detail.insert("member".to_owned(), json!("premises"));
            detail.insert(
                "pointer".to_owned(),
                json!(format!("/items/{}/premises", index.get())),
            );
            return settled(ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(TerminalRecord::disposition_invalid_request(
                    index,
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    detail,
                )),
            });
        }
    }
    let premises = premises_of(composition, &slots);
    let item = if planner_local {
        // FSpec FR-491: the planner settles an `assume-guarantee` item, so it
        // has no request item to negotiate or echo.
        ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(unsupported(index, Capability::Composition)),
        }
    } else {
        routed(ctx, index, selection, name, composition, listed)
    };
    Some(Selected { item, premises })
}

/// The derived slot list a request's `premises` member spells: every slot
/// but the `fixed-point` slot, as `{role, key}`.
fn list(slots: &[CompositionPremise]) -> Result<Vec<Value>, fsl_replay::planner::KeyWireError> {
    slots
        .iter()
        .filter(|premise| premise.derived.role != SlotRole::FixedPoint)
        .map(|premise| {
            Ok(json!({
                "role": premise.derived.role.as_str(),
                "key": premise.derived.key.to_wire()?,
            }))
        })
        .collect()
}

/// The routed composition item (FSpec FR-331 `composition`).
fn routed(
    ctx: &Context<'_>,
    index: RequestIndex,
    selection: &ItemSelection,
    name: &str,
    composition: &CheckedComposition,
    listed: Vec<Value>,
) -> ResolvedItem {
    let named = selection
        .backend
        .as_deref()
        .map(fsl_foundation::BackendId::new);
    let backend_options = selection.options.clone().unwrap_or_default();
    if let Some(refused) = unknown_option(
        index,
        Some(name),
        ctx.registry,
        named.as_ref(),
        &backend_options,
    ) {
        return refused;
    }
    let unrouted = ItemSelection {
        premises: None,
        ..selection.clone()
    }
    .unrouted_routing_options(false);
    claim_item(
        ctx,
        index,
        (name, composition.node()),
        named,
        (backend_options, selection.required.unwrap_or(true)),
        listed,
        unrouted,
    )
}

/// A composition item anchored at `node`.
fn claim_item(
    ctx: &Context<'_>,
    index: RequestIndex,
    (name, node): (&str, &filament_exact::NodeKey),
    named: Option<fsl_foundation::BackendId>,
    (backend_options, required): (BTreeMap<String, String>, bool),
    premises: Vec<Value>,
    unrouted: bool,
) -> ResolvedItem {
    let candidates = ctx
        .registry
        .candidates(Capability::Composition, named.as_ref());
    let item = ClaimItem {
        request_index: index,
        claim: occurrence(node),
        kind: Capability::Composition,
        extent: ExtentClassification::Unbounded {
            finite_bound_available: false,
        },
        backend: named,
        candidates: candidate_list(&candidates),
        options: ItemOptions {
            required,
            max_cases: None,
            max_task_product_work: None,
            trace: None,
            traces: None,
            recording: None,
            population: None,
            backend_options,
            protocol_analysis: None,
        },
        form: Some(ItemForm::Composition(CompositionItem { premises })),
    };
    ResolvedItem::Composition(Box::new(CompositionClaim {
        claim: ResolvedClaim {
            item,
            claim_name: name.to_owned(),
            candidates,
            subject: ctx.subject.clone(),
            unrouted,
        },
        package: None,
        settled: None,
        required: Vec::new(),
    }))
}

/// The premise items of `composition` in FSpec FR-491 order: each
/// `component-guarantee` slot, then each `fixed-point` slot.
fn premises_of(composition: &CheckedComposition, slots: &[CompositionPremise]) -> Vec<Premise> {
    let of_role = |fixed: bool| {
        slots
            .iter()
            .filter(move |premise| (premise.derived.role == SlotRole::FixedPoint) == fixed)
            .filter_map(|premise| premise_of(composition, premise))
    };
    of_role(false).chain(of_role(true)).collect()
}

/// The premise item `premise` derives, `None` for a slot no item backs.
fn premise_of(composition: &CheckedComposition, premise: &CompositionPremise) -> Option<Premise> {
    let SlotKey::Item(key) = &premise.derived.key else {
        return None;
    };
    let unresolved = || PremiseKind::Unresolved {
        composition: composition.name().to_owned(),
    };
    let kind = match &premise.slot.key {
        Some(CompositionSlotKey::ComponentGuarantee(guarantee)) => premise
            .slot
            .guarantee
            .as_ref()
            .and_then(|clause| {
                let component = composition
                    .components()
                    .iter()
                    .find(|component| component.name == clause.component)?;
                Some(PremiseKind::Guarantee {
                    clause: clause.clone(),
                    library: LibraryName::new(component.subject.package.as_str()).ok()?,
                    assumed: !guarantee.assumptions.is_empty(),
                })
            })
            .unwrap_or_else(unresolved),
        Some(CompositionSlotKey::FixedPoint { node, profile, .. }) => key
            .to_wire()
            .ok()
            .and_then(|wire| wire.get("premises")?.as_array().cloned())
            .map_or_else(unresolved, |premises| PremiseKind::FixedPoint {
                name: composition.name().to_owned(),
                node: node.clone(),
                profile: profile.clone(),
                premises,
            }),
        None => return None,
    };
    Some(Premise {
        key: key.clone(),
        kind,
    })
}

/// The premise items of `groups`, one group per selected composition at the
/// request index it is paired with, each written once however many
/// selections derive it and numbered after `resolved`. A composition's item
/// in `resolved` records the request index of each of its component
/// guarantees, which is the item its own slot key (assumptions included)
/// identifies, so a guarantee checked under other peers is never read as
/// this one.
pub(super) fn premise_items(
    ctx: &Context<'_>,
    groups: Vec<(RequestIndex, Vec<Premise>)>,
    resolved: &mut [ResolvedItem],
    first: usize,
) -> Vec<ResolvedItem> {
    let mut written: HashMap<ItemKey, RequestIndex> = HashMap::new();
    let mut items = Vec::new();
    for (composition, premises) in groups {
        let mut required = Vec::new();
        for premise in premises {
            let guarantee = !matches!(premise.kind, PremiseKind::FixedPoint { .. });
            let index = match written.get(&premise.key) {
                Some(index) => *index,
                None => {
                    let index = RequestIndex::new(first + items.len());
                    written.insert(premise.key.clone(), index);
                    items.push(premise_item(ctx, index, premise));
                    index
                }
            };
            if guarantee {
                required.push(index);
            }
        }
        if let Some(ResolvedItem::Composition(claim)) = resolved.get_mut(composition.get()) {
            claim.required = required;
        }
    }
    items
}

fn premise_item(ctx: &Context<'_>, index: RequestIndex, premise: Premise) -> ResolvedItem {
    match premise.kind {
        PremiseKind::Unresolved { composition } => failed(index, &composition),
        PremiseKind::FixedPoint {
            name,
            node,
            profile,
            premises,
        } => claim_item(
            ctx,
            index,
            (&name, &node),
            None,
            (BTreeMap::from([("profile".to_owned(), profile)]), true),
            premises,
            false,
        ),
        PremiseKind::Guarantee {
            clause,
            library,
            assumed,
        } => {
            let Some(component) = ctx.package.dependencies().get(&library).cloned() else {
                return failed(index, &clause.name);
            };
            let selection = ItemSelection {
                claim: Some(clause.name.clone()),
                kind: Some(Capability::TemporalSatisfaction.to_wire().to_owned()),
                ..ItemSelection::default()
            };
            let resolved = resolve_one(
                &component,
                ctx.subject,
                index,
                &selection,
                ctx.registry,
                ctx.binding,
            );
            let ResolvedItem::Claim(mut claim) = resolved else {
                return resolved;
            };
            let settled = assumed.then(|| {
                // FSpec FR-491: the slot is checked under peer guarantees, an
                // assumption set no registered engine reads. Running the
                // clause alone would settle another obligation under this
                // slot's key.
                claim.item.candidates.clear();
                Box::new(unsupported(index, Capability::TemporalSatisfaction))
            });
            ResolvedItem::Composition(Box::new(CompositionClaim {
                claim: *claim,
                package: Some(component),
                settled,
                required: Vec::new(),
            }))
        }
    }
}
