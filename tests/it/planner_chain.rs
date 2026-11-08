// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared helpers of TC-1329 and TC-1330: compile a unit from real source
//! through S1 to S4, derive the planner declaration, write the request items
//! with `qsl-route`'s writer, and build QSpec FR-492 settlement requests
//! from them.
//!
//! The request item JSON is built from the route-written [`RequestItem`]:
//! its kind, claim, extent classification, `correspondence`,
//! `stutter_closure` and `premises` are the route item's. The `subject` and
//! the fairness set are the producer's, not the checked package's, so they
//! come from the derived slot key.

use qsl_foundation::node_wire::node_key_to_json;
use qsl_package::CheckedPackage;
use qsl_replay::planner::{
    read_settlement_request, settle, subject_text, Budget, PlannerDeclaration, PlannerResult,
    ReadLimits, RequestRefusal, SettlementRequest,
};
use qsl_route::declaration::{DeclaredItem, ItemPremise};
use qsl_route::request::RequestItem;
use qsl_semantics::check::{ComponentGuaranteeKey, PackageDeclarations, SlotKey};
use quire_exact::NodeKey;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::fairness::FairnessConstraint;
use serde_json::{json, Value};

/// The checked package identity text every fixture unit is assembled under.
pub(crate) const PACKAGE: &str = "pkg";

/// Why a unit does not compile to a checked package.
#[derive(Debug)]
#[allow(dead_code, reason = "the refusal is read through `Debug` in a failure message")]
pub(crate) enum Failure {
    Syntax(Vec<qsl_cst::CompleteDiagnostic>),
    Assembly(qsl_semantics::check::AssemblyRefusal),
    Check(Vec<qsl_semantics::check::CheckRefusal>),
}

/// Compile `unit` over the domain package `documents` through S1 to S4.
pub(crate) fn compile_with(unit: &str, documents: &[Value]) -> Result<CheckedPackage, Failure> {
    let bytes: Vec<Vec<u8>> = documents
        .iter()
        .map(|document| serde_json::to_vec(document).expect("the fixture serializes"))
        .collect();
    let packages =
        qsl_semantics::model::intake::package_input(bytes.iter().map(Vec::as_slice)).unwrap();
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:planner-chain"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(Failure::Syntax(parsed.diagnostics().to_vec()));
    }
    let raw = parsed.source().reference().clone();
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the models are admitted");
    let assembled =
        PackageDeclarations::assemble(raw, PACKAGE.to_owned(), built, models, Vec::new())
            .map_err(Failure::Assembly)?;
    assembled
        .check(CheckingLimits::default())
        .map(CheckedPackage::link)
        .map_err(Failure::Check)
}

/// The FR-331 `fairness` entries of `constraints`, as the canonical writer
/// spells them.
fn fairness_wire(constraints: &[FairnessConstraint]) -> Vec<Value> {
    let mut entries: Vec<Value> = constraints
        .iter()
        .map(|constraint| {
            let bytes = quire_canonical::to_vec(constraint, quire_canonical::Limits::new(u64::MAX))
                .expect("a constraint writes");
            serde_json::from_slice(&bytes).expect("canonical JSON")
        })
        .collect();
    entries.sort_by_key(ToString::to_string);
    entries
}

fn node_wire(node: &NodeKey) -> Value {
    node_key_to_json(node).expect("a node writes")
}

/// The FR-331 request item of the route-written `item`, whose derived slot
/// key is `key`, checked under `assumptions`. `index` is its `request_index`.
pub(crate) fn item_wire(item: &RequestItem, key: Option<(&SlotKey, &[NodeKey])>) -> Value {
    let mut wire = json!({
        "request_index": item.index().get(),
        "kind": item.kind().to_wire(),
        "claim": {
            "node": node_wire(item.node()),
            "occurrence": {
                "role": item.occurrence().origin().role().as_str(),
                "ordinal": item.occurrence().origin().ordinal(),
            },
        },
        "extent": {"classification": item.extent().to_wire()},
        "candidates": {"type": "set", "backends": ["verus"]},
        "options": {},
    });
    if let (Some(kind), Some((key, assumptions))) = (item.item_kind(), key) {
        let subject = subject_text(&key.subject, assumptions).expect("a subject");
        let mut temporal = json!({
            "subject": serde_json::from_str::<Value>(&subject).expect("a subject"),
            "item_kind": kind.to_wire(),
            "fairness": fairness_wire(&key.fairness),
        });
        if item.stutter_closure() {
            temporal["stutter_closure"] = json!(true);
        }
        wire["temporal"] = temporal;
    }
    if let Some(correspondence) = item.correspondence() {
        let mut members = json!({"role": correspondence.role.as_str()});
        if let Some((context, operation)) = &correspondence.operation {
            members["operation"] =
                json!({"context": node_wire(context), "operation": operation.as_str()});
        }
        wire["correspondence"] = members;
    }
    if !item.premises().is_empty() {
        wire["premises"] = Value::Array(
            item.premises()
                .iter()
                .map(
                    |premise| json!({"role": premise.role().as_str(), "key": premise_key(premise)}),
                )
                .collect(),
        );
    }
    wire
}

/// The `key` of a `premises` entry: the FR-492 slot key of a
/// component-guarantee premise.
fn premise_key(premise: &ItemPremise) -> Value {
    match premise {
        ItemPremise::ComponentGuarantee(key) => guarantee_key_wire(key),
        ItemPremise::Slot(slot) => json!({"role": slot.role.as_str()}),
    }
}

/// The slot key of a component-guarantee slot as item-shaped members.
pub(crate) fn guarantee_key_wire(key: &ComponentGuaranteeKey) -> Value {
    let subject = subject_text(&key.key.subject, &key.assumptions).expect("a subject");
    let mut temporal = json!({
        "subject": serde_json::from_str::<Value>(&subject).expect("a subject"),
        "item_kind": "formula",
        "fairness": fairness_wire(&key.key.fairness),
    });
    if key.key.stutter_closure {
        temporal["stutter_closure"] = json!(true);
    }
    json!({
        "kind": key.key.kind.to_wire(),
        "claim": {
            "node": node_wire(&key.key.claim),
            "occurrence": {"role": "claim", "ordinal": key.key.ordinal},
        },
        "extent": {"classification": "unbounded"},
        "temporal": temporal,
    })
}

/// A proved result entry for the item at `index` of kind `kind`.
pub(crate) fn proved_entry(kind: &str, index: usize) -> Value {
    json!({
        "request_index": index,
        "backend": "verus",
        "kind": kind,
        "result": "proved",
        "basis": {"type": "inductive", "depth": 1},
        "certification": "certified",
    })
}

/// An entry for the item at `index` of kind `kind` with `extra` members.
pub(crate) fn entry_of(kind: &str, index: usize, extra: Value) -> Value {
    let mut entry = json!({"request_index": index, "backend": "verus", "kind": kind});
    for (name, value) in extra.as_object().expect("members").clone() {
        entry[name] = value;
    }
    entry
}

/// One declaration of a compiled package, with its planner declaration and
/// the request items `qsl-route` wrote for it.
pub(crate) struct Chain {
    pub(crate) package: CheckedPackage,
    pub(crate) declaration: PlannerDeclaration,
    /// The route-written items of every declaration, in write order.
    pub(crate) items: Vec<DeclaredItem>,
    /// The derived slots' S3 keys with their component-guarantee
    /// assumptions, by slot index.
    keys: Vec<Option<(SlotKey, Vec<NodeKey>)>>,
}

impl Chain {
    /// The chain of the declaration at `node`.
    pub(crate) fn of(
        package: CheckedPackage,
        node: &NodeKey,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let declaration = PlannerDeclaration::from_package(&package, node, PACKAGE)?;
        let mut writer = qsl_route::request::RequestWriter::new();
        let items = writer.declaration_items(package.graph(), PACKAGE);
        let keys = declared_keys(&package, node);
        Ok(Self {
            package,
            declaration,
            items,
            keys,
        })
    }

    /// The route-written item of the declaration's slot `slot`.
    pub(crate) fn item(&self, slot: usize) -> &DeclaredItem {
        self.items
            .iter()
            .find(|written| {
                written.declaration == self.declaration.node && written.slot == Some(slot)
            })
            .unwrap_or_else(|| panic!("slot {slot} has a route item"))
    }

    /// The FR-331 request item of slot `slot`.
    pub(crate) fn item_json(&self, slot: usize) -> Value {
        let key = self.keys[slot]
            .as_ref()
            .map(|(key, assumptions)| (key, assumptions.as_slice()));
        item_wire(&self.item(slot).item, key)
    }

    /// The `kind` spelling of slot `slot`'s item.
    pub(crate) fn kind(&self, slot: usize) -> &'static str {
        self.item(slot).item.kind().to_wire()
    }

    /// A proved entry for slot `slot`.
    pub(crate) fn proved(&self, slot: usize) -> Value {
        proved_entry(self.kind(slot), self.item(slot).item.index().get())
    }

    /// An entry for slot `slot` with `extra` members.
    pub(crate) fn entry(&self, slot: usize, extra: Value) -> Value {
        entry_of(self.kind(slot), self.item(slot).item.index().get(), extra)
    }

    /// The request settling the declaration with every item slot `proved`
    /// and the record of every listed environment member.
    pub(crate) fn request(&self, environment: &dyn Fn(&str) -> Value) -> Value {
        let slots: Vec<Value> = self
            .declaration
            .slots
            .iter()
            .enumerate()
            .map(|(index, derived)| {
                let mut wire = json!({
                    "role": derived.role.as_str(),
                    "identity": derived.key.to_wire().expect("an identity"),
                });
                match &derived.backing {
                    qsl_replay::planner::SlotBacking::Item => {
                        wire["result"] = json!({
                            "request": self.item_json(index),
                            "result": self.proved(index),
                        });
                    }
                    qsl_replay::planner::SlotBacking::Environment { member: Some(name) } => {
                        wire["record"] = environment(name);
                    }
                    _ => {}
                }
                wire
            })
            .collect();
        json!({
            "format": qsl_replay::planner::FORMAT,
            "package": PACKAGE,
            "item": {
                "role": self.declaration.role.as_str(),
                "node": node_wire(&self.declaration.node),
            },
            "slots": slots,
        })
    }

    /// Admit `request`.
    pub(crate) fn read(&self, request: &Value) -> Result<SettlementRequest, RequestRefusal> {
        self.read_with(request, &ReadLimits::default())
    }

    /// Admit `request` under `limits`.
    pub(crate) fn read_with(
        &self,
        request: &Value,
        limits: &ReadLimits,
    ) -> Result<SettlementRequest, RequestRefusal> {
        read_settlement_request(
            &self.declaration,
            &serde_json::to_vec(request).expect("bytes"),
            limits,
        )
    }

    /// Admit and settle `request`.
    pub(crate) fn settled(&self, request: &Value) -> PlannerResult {
        let read = self.read(request).expect("the request is admitted");
        settle(&self.declaration, &read, Budget::default()).expect("the item settles")
    }

    /// The refusal of `request`: code, cause and pointer.
    pub(crate) fn refusal(&self, request: &Value) -> (&'static str, &'static str, String) {
        let refusal = self.read(request).expect_err("the request is refused");
        (refusal.code(), refusal.cause(), refusal.pointer())
    }
}

/// The S3 slot keys of the declaration at `node`, by slot index.
fn declared_keys(package: &CheckedPackage, node: &NodeKey) -> Vec<Option<(SlotKey, Vec<NodeKey>)>> {
    let graph = package.graph();
    if let Some(guarantee) = graph
        .implementation_guarantees()
        .iter()
        .find(|guarantee| guarantee.node() == node)
    {
        return guarantee
            .slots(PACKAGE)
            .into_iter()
            .map(|slot| slot.key.map(|key| (key, Vec::new())))
            .collect();
    }
    graph
        .compositions()
        .iter()
        .find(|composition| composition.node() == node)
        .map(|composition| {
            composition
                .slots()
                .into_iter()
                .map(|slot| match slot.key {
                    Some(qsl_semantics::check::CompositionSlotKey::ComponentGuarantee(key)) => {
                        Some((key.key, key.assumptions))
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}
