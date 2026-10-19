// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 §8's `CasRefinesCounter`: the abstract `example/counter-spec` and
//! the concrete `example/counter-cas` domain packages, one native unit that
//! selects both, carries their contracts and declares the refinement, and the
//! two model systems and states the refinement tests read it through.
//!
//! [`Variant`] selects the ADR-020 §8 variants: the lost-update model (both
//! commit preconditions `self.busyX and self.tmpX < 2`, no `retry`
//! operations), a concrete initial `value`, and a `SubCounter` subtype that
//! `counters` admits and no object row names.

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
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};
use qsl_semantics::model::state::{ModelState, StateObject};
use quire_exact::{Integer, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

pub const SPEC: &str = "example/counter-spec";
pub const CAS: &str = "example/counter-cas";
/// A third model, a copy of the abstract one, which the unit selects as
/// `Third` and no refinement names.
pub const THIRD: &str = "example/counter-third";

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const INTEGER: &str = "ix://quire/native/Integer";
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

fn ident(package: &str, suffix: &str) -> String {
    format!("ix://{package}/{suffix}")
}

/// The declaration key of `suffix` in `package`.
pub fn key(package: &str, suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: ident(package, suffix),
    }
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn wire_type(package: &str, identity: &str, kind: &str, extra: Json) -> Json {
    let mut node = json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": package, "name": kind}, "roles": [],
        "origin": origin(identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
    });
    for (name, value) in extra.as_object().expect("extra members") {
        node[name] = value.clone();
    }
    node
}

/// `Level`, the integer type `Int[0, 2]`.
fn level(package: &str) -> Json {
    let identity = ident(package, "Level");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": INTEGER,
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    wire_type(
        package,
        &identity,
        "value_type",
        json!({"scalar": "integer",
            "constraints": [constraint("min", "0"), constraint("max", "2")]}),
    )
}

fn field(package: &str, name: &str, type_ref: &str) -> Json {
    let identity = ident(package, &format!("Counter/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&identity),
    })
}

/// A parameterless operation of `Counter` modifying `modifies`.
fn operation(package: &str, name: &str, modifies: &[&str]) -> Json {
    let identity = ident(package, &format!("Counter/{name}"));
    json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident(package, "spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {
            "modifies": modifies
                .iter()
                .map(|field| ident(package, &format!("Counter/{field}")))
                .collect::<Vec<_>>(),
            "creates": [], "deletes": [],
        },
    })
}

fn document(package: &str, fields: Vec<Json>, operations: Vec<Json>, sub_counter: bool) -> Vec<u8> {
    let counter = ident(package, "Counter");
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": package, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let population = ident(package, "counters");
    let sub = ident(package, "SubCounter");
    let mut types = vec![
        level(package),
        wire_type(
            package,
            &counter,
            "object_type",
            json!({
                "supertypes": [], "fields": fields, "operations": operations,
                "relationships": [],
            }),
        ),
    ];
    let mut members = vec![counter.clone()];
    if sub_counter {
        types.push(wire_type(
            package,
            &sub,
            "object_type",
            json!({
                "supertypes": [counter], "fields": [], "operations": [],
                "relationships": [],
            }),
        ));
        members.push(sub);
    }
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident(package, "spec"), "version": "1.0.0",
            "dialect": "spec-bundle", "digest": PLACEHOLDER},
        "package": {"identity": package, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": types,
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": package, "name": "population"},
            "members": members, "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// Which ADR-020 §8 model a [`World`] is.
#[derive(Clone, Copy, Debug)]
pub struct Variant {
    /// Both commit preconditions are `self.busyX and self.tmpX < 2`, the
    /// compare-and-set guard dropped, and there are no `retry` operations.
    pub lost_update: bool,
    /// The concrete package also declares `SubCounter`, a subtype of
    /// `Counter` that `counters` admits and no object row names.
    pub sub_counter: bool,
    /// The concrete initial snapshot's `value`.
    pub concrete_value: i64,
}

impl Variant {
    /// The compare-and-set model.
    pub const CAS: Self = Self {
        lost_update: false,
        sub_counter: false,
        concrete_value: 0,
    };
    /// The lost-update model.
    pub const LOST_UPDATE: Self = Self {
        lost_update: true,
        ..Self::CAS
    };
}

/// `Counter { value: Int[0, 2] }` with `inc()`, under the package identity
/// `package`.
fn spec_document(package: &str) -> Vec<u8> {
    document(
        package,
        vec![field(package, "value", &ident(package, "Level"))],
        vec![operation(package, "inc", &["value"])],
        false,
    )
}

/// The compare-and-set counter of ADR-020 §8.
fn cas_document(variant: Variant) -> Vec<u8> {
    let level = ident(CAS, "Level");
    let step = |name: &str, modifies: &[&str]| operation(CAS, name, modifies);
    let mut operations = vec![
        step("beginA", &["tmpA", "busyA"]),
        step("commitA", &["value", "busyA"]),
        step("beginB", &["tmpB", "busyB"]),
        step("commitB", &["value", "busyB"]),
        step("peek", &[]),
    ];
    if !variant.lost_update {
        operations.push(step("retryA", &["busyA"]));
        operations.push(step("retryB", &["busyB"]));
    }
    document(
        CAS,
        vec![
            field(CAS, "value", &level),
            field(CAS, "tmpA", &level),
            field(CAS, "busyA", BOOLEAN),
            field(CAS, "tmpB", &level),
            field(CAS, "busyB", BOOLEAN),
        ],
        operations,
        variant.sub_counter,
    )
}

/// The contracts of one worker (`A` or `B`) of the compare-and-set model.
fn worker(name: &str, variant: Variant) -> String {
    let commit = if variant.lost_update {
        format!("self.busy{name} and self.tmp{name} < 2")
    } else {
        format!("self.busy{name} and self.tmp{name} = self.value and self.value < 2")
    };
    let retry = if variant.lost_update {
        String::new()
    } else {
        format!(
            "pre Retry{name}Pre using v on Impl::Counter::retry{name} {{ self.busy{name} and not (self.tmp{name} = self.value) }}\n\
             post Retry{name}Post using v on Impl::Counter::retry{name} {{ not self.busy{name} }}\n"
        )
    };
    format!(
        "pre Begin{name}Pre using v on Impl::Counter::begin{name} {{ not self.busy{name} }}\n\
         post Begin{name}Post using v on Impl::Counter::begin{name} {{ self.busy{name} and self.tmp{name} = pre(self.value) }}\n\
         pre Commit{name}Pre using v on Impl::Counter::commit{name} {{ {commit} }}\n\
         post Commit{name}Post using v on Impl::Counter::commit{name} {{ pre(self.tmp{name}) < 2 and self.value = pre(self.tmp{name}) + 1 and not self.busy{name} }}\n\
         {retry}"
    )
}

/// ADR-020 §8's `CasRefinesCounter`.
pub fn cas_refines_counter() -> String {
    "refinement CasRefinesCounter using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     step Impl::Counter::commitA -> Spec::Counter::inc(self);\n\
     step Impl::Counter::commitB -> Spec::Counter::inc(self);\n\
     step Impl::Counter::beginA -> stutter;\n\
     step Impl::Counter::beginB -> stutter;\n\
     step Impl::Counter::retryA -> stutter;\n\
     step Impl::Counter::retryB -> stutter;\n\
     step Impl::Counter::peek -> stutter;\n\
     assume fair weak each Impl::Counter::beginA;\n\
     assume fair weak each Impl::Counter::beginB;\n\
     assume fair weak each Impl::Counter::commitA;\n\
     assume fair weak each Impl::Counter::commitB;\n\
     assume fair weak each Impl::Counter::retryA;\n\
     assume fair weak each Impl::Counter::retryB;\n\
     ensure fair weak each Spec::Counter::inc;\n\
     }"
    .to_owned()
}

/// The lost-update refinement: `CasRefinesCounter` without its `retry` rows.
pub fn lost_update_refinement() -> String {
    cas_refines_counter()
        .lines()
        .filter(|line| !line.contains("retry"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `body` with the one occurrence of `from` replaced by `to`.
pub fn mutated(body: &str, from: &str, to: &str) -> String {
    assert_eq!(body.matches(from).count(), 1, "{from} occurs once");
    body.replace(from, to)
}

/// The unit text: both models, their contracts and `refinement`.
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
         model Spec = \"{SPEC}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         model Impl = \"{CAS}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         model Third = \"{THIRD}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         pre CanInc using v on Spec::Counter::inc {{ self.value < 2 }}\n\
         post Inc using v on Spec::Counter::inc {{ pre(self.value) < 2 and self.value = pre(self.value) + 1 }}\n\
         {}{}{refinement}\n",
        digest(SPEC),
        digest(CAS),
        digest(THIRD),
        worker("A", variant),
        worker("B", variant),
    )
}

/// The two domain packages' documents, by digest.
pub fn packages(variant: Variant) -> BTreeMap<[u8; 32], Vec<u8>> {
    package_input([
        spec_document(SPEC).as_slice(),
        cas_document(variant).as_slice(),
        spec_document(THIRD).as_slice(),
    ])
}

/// The checked graph of the unit with `refinement`, or its refusals.
pub fn check(variant: Variant, refinement: &str) -> Result<CheckedGraph, String> {
    let documents = packages(variant);
    let text = unit_text(&documents, refinement, variant);
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "refinement", "git", "1"),
        "refinement.native",
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

/// An initialization snapshot of the one counter `c` of `package`, whose
/// fields are `fields`.
fn snapshot(
    package: &str,
    document: &[u8],
    fields: Json,
    snapshots: &mut BTreeMap<[u8; 32], Vec<u8>>,
) -> DocumentRef {
    let model_digest = *package_input([document])
        .keys()
        .next()
        .expect("one package");
    let header = json!({"identity": package, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&model_digest))});
    let name = format!("{package}/start");
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident(package, "counters"), "complete": true,
            "objects": [{"key": "c", "type": ident(package, "Counter"), "fields": fields}]}],
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

/// The unit's package, the models' documents and their initial states.
pub struct World {
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    spec_initial: DocumentRef,
    cas_initial: DocumentRef,
    third_initial: DocumentRef,
}

impl World {
    /// The world of the compare-and-set unit with `refinement`.
    pub fn new(refinement: &str) -> Self {
        Self::with(Variant::CAS, refinement)
    }

    /// The world of the unit of `variant` with `refinement`.
    pub fn with(variant: Variant, refinement: &str) -> Self {
        let graph = check(variant, refinement).unwrap_or_else(|refusals| {
            panic!("{refinement}: the unit checks: {refusals}");
        });
        let mut snapshots = BTreeMap::new();
        let spec_initial = snapshot(
            SPEC,
            &spec_document(SPEC),
            json!({"value": {"integer": "0"}}),
            &mut snapshots,
        );
        let third_initial = snapshot(
            THIRD,
            &spec_document(THIRD),
            json!({"value": {"integer": "0"}}),
            &mut snapshots,
        );
        let cas_initial = snapshot(
            CAS,
            &cas_document(variant),
            json!({
                "value": {"integer": variant.concrete_value.to_string()},
                "tmpA": {"integer": "0"}, "busyA": {"boolean": false},
                "tmpB": {"integer": "0"}, "busyB": {"boolean": false},
            }),
            &mut snapshots,
        );
        Self {
            package: CheckedPackage::link(graph),
            packages: packages(variant),
            snapshots,
            spec_initial,
            cas_initial,
            third_initial,
        }
    }

    /// The unit's checked package.
    pub fn package(&self) -> &CheckedPackage {
        &self.package
    }

    /// The abstract subject's initial snapshot.
    pub fn abstract_initial(&self) -> DocumentRef {
        self.spec_initial.clone()
    }

    /// The request that builds the system over `model` with `initial`
    /// states, evaluating each clause under `meter`.
    pub fn request(
        &self,
        model: &str,
        initial: Vec<DocumentRef>,
        meter: ScalarLimits,
    ) -> ModelSimulationRequest<'_> {
        ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial,
            universes: vec![PopulationUniverse {
                population: key(model, "counters"),
                keys: vec!["c".to_owned()],
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter,
        }
    }

    /// The concrete subject's request.
    pub fn concrete_request(&self, meter: ScalarLimits) -> ModelSimulationRequest<'_> {
        self.request(CAS, vec![self.cas_initial.clone()], meter)
    }

    /// The unit's one refinement.
    pub fn refinement(&self) -> &CheckedRefinement {
        let [refinement] = self.package.graph().refinements() else {
            panic!("one refinement");
        };
        refinement
    }

    /// The abstract subject's system, evaluating each clause under `meter`.
    pub fn abstract_system(&self, meter: ScalarLimits) -> ModelSystem<'_> {
        self.system(SPEC, &self.spec_initial, meter)
    }

    /// The concrete subject's system, evaluating each clause under `meter`.
    pub fn concrete_system(&self, meter: ScalarLimits) -> ModelSystem<'_> {
        self.system(CAS, &self.cas_initial, meter)
    }

    fn system(&self, model: &str, initial: &DocumentRef, meter: ScalarLimits) -> ModelSystem<'_> {
        self.try_system(model, initial, meter)
            .expect("the model system admits")
    }

    /// A system over the model `model`, or why it does not admit.
    pub fn try_system(
        &self,
        model: &str,
        initial: &DocumentRef,
        meter: ScalarLimits,
    ) -> Result<ModelSystem<'_>, AdmissionFailure> {
        ModelSystem::over_model(self.request(model, vec![initial.clone()], meter), model)
    }

    /// The system over the unit's third model.
    pub fn third_system(&self) -> ModelSystem<'_> {
        self.system(THIRD, &self.third_initial, UNLIMITED)
    }

    /// A system over `model`, a model the unit does not select.
    pub fn unselected_system(&self, model: &str) -> Result<ModelSystem<'_>, AdmissionFailure> {
        self.try_system(model, &self.cas_initial, UNLIMITED)
    }
}

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

/// The abstract state with the one counter `c` at `value`.
pub fn spec_state(value: i64) -> ModelState {
    ModelState {
        populations: BTreeMap::from([(
            key(SPEC, "counters"),
            BTreeMap::from([(
                "c".to_owned(),
                StateObject {
                    type_identity: key(SPEC, "Counter"),
                    fields: BTreeMap::from([(key(SPEC, "Counter/value"), integer(value))]),
                },
            )]),
        )]),
        extents: Vec::new(),
    }
}

/// The concrete state `(value, tmpA, busyA, tmpB, busyB)` of the one counter
/// `c`.
pub fn cas_state(value: i64, tmp_a: i64, busy_a: bool, tmp_b: i64, busy_b: bool) -> ModelState {
    let field = |name: &str, value: Value| (key(CAS, &format!("Counter/{name}")), value);
    ModelState {
        populations: BTreeMap::from([(
            key(CAS, "counters"),
            BTreeMap::from([(
                "c".to_owned(),
                StateObject {
                    type_identity: key(CAS, "Counter"),
                    fields: BTreeMap::from([
                        field("value", integer(value)),
                        field("tmpA", integer(tmp_a)),
                        field("busyA", Value::Boolean(busy_a)),
                        field("tmpB", integer(tmp_b)),
                        field("busyB", Value::Boolean(busy_b)),
                    ]),
                },
            )]),
        )]),
        extents: Vec::new(),
    }
}
