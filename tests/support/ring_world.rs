// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 §8's ring buffer refined to a queue (`RingIsQueue`), as a world
//! the refinement tests run: the abstract `example/ring-queue-spec` and the
//! concrete `example/ring-queue-impl` packages with the contracts of
//! `enq`, `deq`, `put` and `take`, one native unit that selects both and
//! declares the refinement, the initial snapshots and the two model systems.
//!
//! The abstract queue holds `items` only. [`Variant`] selects the broken
//! `take` of ADR-020 §8, which takes the slot after the head, and the `only`
//! row of FR-140-AC-3, which is undefined wherever the queue is not empty.

#![allow(
    dead_code,
    reason = "each test binary uses the part of the fixture it needs"
)]

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedGraph, CheckedRefinement, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use qsl_semantics::model::state::{ModelState, StateObject};
use quire_exact::{Integer, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

#[path = "ring_queue.rs"]
mod ring_queue;

pub use ring_queue::{key, node, QUEUE, RING};

const BOOLEAN: &str = "ix://quire/native/Boolean";

/// A budget no fixture evaluation exhausts.
pub const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

/// A budget that admits no charge.
pub const ZERO: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 0,
    work_units: 0,
    result_units: 0,
};

/// Which ring model a [`RingWorld`] is.
#[derive(Clone, Copy, Debug, Default)]
pub struct Variant {
    /// `take` precondition selects the slot after the head, so it returns
    /// the second item where the queue's `deq` needs the first.
    pub broken_take: bool,
    /// The `items` row's `only` condition is `s.ring = self`, which two
    /// slots satisfy: undefined wherever the queue is not empty.
    pub only_ring: bool,
}

/// `operation` with the frame that grants `modifies` on every object and
/// `receiver_modifies` on the receiver only, as field identities of
/// `package`.
fn framed(
    mut operation: Json,
    package: &str,
    modifies: &[&str],
    receiver_modifies: &[&str],
) -> Json {
    let identities = |paths: &[&str]| {
        paths
            .iter()
            .map(|path| node(package, path))
            .collect::<Vec<_>>()
    };
    operation["frame"]["modifies"] = json!(identities(modifies));
    operation["frame"]["modifies_receiver"] = json!(identities(receiver_modifies));
    operation
}

/// The abstract package: `Queue { items }` with `enq(x)` and `deq()`.
fn queue_document() -> Vec<u8> {
    let digit = node(QUEUE, "Digit");
    let enq = framed(
        ring_queue::operation(QUEUE, "Queue", "enq", &[("x", &digit)], None, &[]),
        QUEUE,
        &["Queue/items"],
        &[],
    );
    let deq = framed(
        ring_queue::operation(QUEUE, "Queue", "deq", &[], Some(&digit), &[]),
        QUEUE,
        &["Queue/items"],
        &[],
    );
    ring_queue::document(
        QUEUE,
        vec![
            ring_queue::bounded(QUEUE, "Digit", 1),
            ring_queue::object_type(
                QUEUE,
                "Queue",
                vec![ring_queue::field(
                    QUEUE,
                    "Queue",
                    "items",
                    &digit,
                    (0, 2, true),
                )],
                vec![enq, deq],
            ),
        ],
        vec![ring_queue::population(QUEUE, "queues", "Queue")],
    )
}

/// The concrete package: `Ring { head, size }` and `Slot { ring, index,
/// value }` with `put(x)` and `take()`.
fn ring_document() -> Vec<u8> {
    let digit = node(RING, "Digit");
    let put = framed(
        ring_queue::operation(RING, "Slot", "put", &[("x", &digit)], None, &[]),
        RING,
        &["Ring/size"],
        &["Slot/value"],
    );
    let take = framed(
        ring_queue::operation(RING, "Slot", "take", &[], Some(&digit), &[]),
        RING,
        &["Ring/head", "Ring/size"],
        &[],
    );
    ring_queue::document(
        RING,
        vec![
            ring_queue::bounded(RING, "Digit", 1),
            ring_queue::bounded(RING, "Two", 2),
            ring_queue::object_type(
                RING,
                "Ring",
                vec![
                    ring_queue::single(RING, "Ring", "head", &digit),
                    ring_queue::single(RING, "Ring", "size", &node(RING, "Two")),
                ],
                Vec::new(),
            ),
            ring_queue::object_type(
                RING,
                "Slot",
                vec![
                    ring_queue::single(RING, "Slot", "ring", &node(RING, "Ring")),
                    ring_queue::single(RING, "Slot", "index", &digit),
                    ring_queue::single(RING, "Slot", "value", &digit),
                ],
                vec![put, take],
            ),
        ],
        vec![
            ring_queue::population(RING, "rings", "Ring"),
            ring_queue::population(RING, "slots", "Slot"),
        ],
    )
}

/// The contracts of both models.
fn contracts(variant: Variant) -> String {
    let at = if variant.broken_take {
        "(self.ring.head + 1) mod 2"
    } else {
        "self.ring.head"
    };
    format!(
        "pre CanEnq using v on Q::Queue::enq {{ size(self.items) < 2 }}\n\
         post Enq using v on Q::Queue::enq {{ (size(pre(self.items)) = 0 and self.items = sequence[x]) or (pre(self.items) = sequence[0] and self.items = sequence[0, x]) or (pre(self.items) = sequence[1] and self.items = sequence[1, x]) }}\n\
         pre CanDeq using v on Q::Queue::deq {{ size(self.items) > 0 }}\n\
         post Deq using v on Q::Queue::deq {{ (pre(self.items) = sequence[0] and result = 0 and size(self.items) = 0) or (pre(self.items) = sequence[1] and result = 1 and size(self.items) = 0) or (pre(self.items) = sequence[0, 0] and result = 0 and self.items = sequence[0]) or (pre(self.items) = sequence[0, 1] and result = 0 and self.items = sequence[1]) or (pre(self.items) = sequence[1, 0] and result = 1 and self.items = sequence[0]) or (pre(self.items) = sequence[1, 1] and result = 1 and self.items = sequence[1]) }}\n\
         pre CanPut using v on R::Slot::put {{ self.ring.size < 2 and self.index + 0 = (self.ring.head + self.ring.size) mod 2 }}\n\
         post Put using v on R::Slot::put {{ self.value = x and pre(self.ring.size) < 2 and self.ring.size = pre(self.ring.size) + 1 }}\n\
         pre CanTake using v on R::Slot::take {{ self.ring.size > 0 and self.index + 0 = {at} }}\n\
         post Take using v on R::Slot::take {{ result = pre(self.value) and pre(self.ring.size) > 0 and self.ring.size = pre(self.ring.size) - 1 and self.ring.head + 0 = (pre(self.ring.head) + 1) mod 2 }}\n"
    )
}

/// The `items` row of ADR-020 §8; with `only_ring` the `only` condition is
/// `s.ring = self`.
fn items_row(variant: Variant) -> String {
    let condition = if variant.only_ring {
        "s.ring = self".to_owned()
    } else {
        "s.ring = self and s.index + 0 = (self.head + i) mod 2".to_owned()
    };
    format!("items = seq(i in 0 .. self.size - 1: only(s in R::slots where {condition}: s.value));")
}

/// `RingIsQueue` with the `items` row `items`.
pub fn ring_is_queue(items: &str) -> String {
    format!(
        "invariant Anything using v on R::Ring at current {{ true }}\n\
         refinement RingIsQueue using inf abstract Q concrete R {{\n\
         population Q::queues from R::rings;\n\
         object Q::Queue from R::Ring {{ {items} }}\n\
         step R::Slot::put -> Q::Queue::enq(self.ring, x);\n\
         step R::Slot::take -> Q::Queue::deq(self.ring);\n\
         }}"
    )
}

/// The unit text: both models, the contracts of `variant` and `refinement`.
fn unit_text(
    documents: &BTreeMap<[u8; 32], Vec<u8>>,
    refinement: &str,
    variant: Variant,
) -> String {
    let digest = |package: &str| {
        documents
            .iter()
            .find(|(_, bytes)| {
                serde_json::from_slice::<Json>(bytes)
                    .is_ok_and(|json| json["package"]["identity"] == package)
            })
            .map(|(digest, _)| hex(digest))
            .expect("the package is provisioned")
    };
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model Q = \"{QUEUE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         model R = \"{RING}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {}{refinement}\n",
        digest(QUEUE),
        digest(RING),
        contracts(variant),
    )
}

/// The two domain packages' documents, by digest.
fn packages() -> BTreeMap<[u8; 32], Vec<u8>> {
    package_input([queue_document().as_slice(), ring_document().as_slice()])
}

/// The checked graph of the unit with `refinement`, or its refusals.
pub fn check(variant: Variant, refinement: &str) -> Result<CheckedGraph, String> {
    let documents = packages();
    let text = unit_text(&documents, refinement, variant);
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "ring", "git", "1"),
        "ring.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &built.selections().models,
        &documents,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selections admit")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("the operation clauses parse");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    declarations
        .with_unwritten_refinements()
        .check(CheckingLimits::default())
        .map_err(|refusals| format!("{refusals:?}"))
}

/// An initialization snapshot of `package` holding `populations`
/// (population name, objects as (key, type name, fields)).
fn snapshot(
    package: &str,
    document: &[u8],
    populations: Vec<Json>,
    snapshots: &mut BTreeMap<[u8; 32], Vec<u8>>,
) -> DocumentRef {
    let model_digest = *package_input([document])
        .keys()
        .next()
        .expect("one package");
    let name = format!("{package}/start");
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current",
        "model": {"identity": package, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&model_digest))},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": populations,
    });
    let bytes = snapshot.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    snapshots.insert(digest, bytes);
    DocumentRef {
        authority: "test".to_owned(),
        identity: name,
        revision_namespace: "fixture".to_owned(),
        revision: "fixture:1".to_owned(),
        digest,
    }
}

/// The unit's package, the two models' documents and their initial states.
pub struct RingWorld {
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    queue_initial: DocumentRef,
    ring_initial: DocumentRef,
}

impl RingWorld {
    /// The world of the unit of `variant` with `refinement`.
    pub fn with(variant: Variant, refinement: &str) -> Self {
        let graph = check(variant, refinement).unwrap_or_else(|refusals| {
            panic!("{refinement}: the unit checks: {refusals}");
        });
        let mut snapshots = BTreeMap::new();
        let queue_initial = snapshot(
            QUEUE,
            &queue_document(),
            vec![
                json!({"population": node(QUEUE, "queues"), "complete": true,
                "objects": [{"key": "r", "type": node(QUEUE, "Queue"),
                    "fields": {"items": {"sequence": []}}}]}),
            ],
            &mut snapshots,
        );
        let slot = |key: &str, index: u8| {
            json!({"key": key, "type": node(RING, "Slot"), "fields": {
                "ring": {"reference": {"population": node(RING, "rings"), "key": "r"}},
                "index": {"integer": index.to_string()}, "value": {"integer": "0"},
            }})
        };
        let ring_initial = snapshot(
            RING,
            &ring_document(),
            vec![
                json!({"population": node(RING, "rings"), "complete": true,
                    "objects": [{"key": "r", "type": node(RING, "Ring"),
                        "fields": {"head": {"integer": "0"}, "size": {"integer": "0"}}}]}),
                json!({"population": node(RING, "slots"), "complete": true,
                    "objects": [slot("s0", 0), slot("s1", 1)]}),
            ],
            &mut snapshots,
        );
        Self {
            package: CheckedPackage::link(graph),
            packages: packages(),
            snapshots,
            queue_initial,
            ring_initial,
        }
    }

    /// The compare-the-ring world: the broken-`take` and `only` variants are
    /// built by [`Self::with`].
    pub fn new() -> Self {
        Self::with(
            Variant::default(),
            &ring_is_queue(&items_row(Variant::default())),
        )
    }

    /// `RingIsQueue` of `variant`.
    pub fn variant(variant: Variant) -> Self {
        Self::with(variant, &ring_is_queue(&items_row(variant)))
    }

    /// The unit's checked package.
    pub fn package(&self) -> &CheckedPackage {
        &self.package
    }

    /// The unit's one refinement.
    pub fn refinement(&self) -> &CheckedRefinement {
        let [refinement] = self.package.graph().refinements() else {
            panic!("one refinement");
        };
        refinement
    }

    /// The abstract subject's initial snapshot.
    pub fn abstract_initial(&self) -> DocumentRef {
        self.queue_initial.clone()
    }

    /// The request over `model` with `initial` states and the universes
    /// `rings = {r}`, `slots = {s0, s1}` (the abstract model's derived).
    pub fn request(
        &self,
        model: &str,
        initial: Vec<DocumentRef>,
        meter: ScalarLimits,
    ) -> ModelSimulationRequest<'_> {
        let universes = if model == QUEUE {
            vec![PopulationUniverse {
                population: key(QUEUE, "queues"),
                keys: vec!["r".to_owned()],
            }]
        } else {
            vec![
                PopulationUniverse {
                    population: key(RING, "rings"),
                    keys: vec!["r".to_owned()],
                },
                PopulationUniverse {
                    population: key(RING, "slots"),
                    keys: vec!["s0".to_owned(), "s1".to_owned()],
                },
            ]
        };
        ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial,
            universes,
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter,
        }
    }

    /// The concrete subject's request.
    pub fn concrete_request(&self, meter: ScalarLimits) -> ModelSimulationRequest<'_> {
        self.request(RING, vec![self.ring_initial.clone()], meter)
    }

    /// The abstract subject's system, evaluating each clause under `meter`.
    pub fn abstract_system(&self, meter: ScalarLimits) -> ModelSystem<'_> {
        ModelSystem::over_model(
            self.request(QUEUE, vec![self.queue_initial.clone()], meter),
            QUEUE,
        )
        .expect("the abstract system admits")
    }

    /// The concrete subject's system, evaluating each clause under `meter`.
    pub fn concrete_system(&self, meter: ScalarLimits) -> ModelSystem<'_> {
        ModelSystem::over_model(self.concrete_request(meter), RING)
            .expect("the concrete system admits")
    }
}

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

/// The concrete state with the ring `r` at `head` holding `size` items and
/// the slots `s0` and `s1` (indices 0 and 1) holding `values`.
pub fn ring_state(system: &ModelSystem<'_>, head: i64, size: i64, values: [i64; 2]) -> ModelState {
    let ring = |field: &str| key(RING, &format!("Ring/{field}"));
    let slot = |field: &str| key(RING, &format!("Slot/{field}"));
    let reference = system
        .object_reference(&key(RING, "Ring"), "r")
        .expect("the ring is referenced");
    let slots = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (
                format!("s{index}"),
                StateObject {
                    type_identity: key(RING, "Slot"),
                    fields: BTreeMap::from([
                        (slot("ring"), Value::Reference(reference.clone())),
                        (
                            slot("index"),
                            integer(i64::try_from(index).expect("an index")),
                        ),
                        (slot("value"), integer(*value)),
                    ]),
                },
            )
        })
        .collect();
    ModelState {
        populations: BTreeMap::from([
            (
                key(RING, "rings"),
                BTreeMap::from([(
                    "r".to_owned(),
                    StateObject {
                        type_identity: key(RING, "Ring"),
                        fields: BTreeMap::from([
                            (ring("head"), integer(head)),
                            (ring("size"), integer(size)),
                        ]),
                    },
                )]),
            ),
            (key(RING, "slots"), slots),
        ]),
        extents: Vec::new(),
    }
}
