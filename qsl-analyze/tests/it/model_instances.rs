// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-125 Instancing over a model of several populations and a type
//! hierarchy: the `test/things` domain package declares `Thing`, its
//! subtypes `Lamp` and `Switch`, and the unrelated `Other`; each test
//! declares the populations it needs. An instance of `over (x: T)` is one
//! key of a population that admits `T`, derived from the subject's
//! universes, in the canonical order.
//!
//! FR-125-AC-11 and AC-12 read the bound parameter through the production
//! letters of the checked clause (`ModelSystem::clause_letters`).
//!
//! Not covered: FR-125-AC-9 and AC-15, whose reads are of the reference
//! itself; no state-graph claim (FR-165) is instantiated here.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, LettersRefusal, ModelSimState, ModelSimulationRequest, ModelSystem,
    PopulationUniverse,
};
use qsl_eval::simulation::{NotSimulated, RequiresBound};
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::WireNodeId;
use qsl_package::CheckedPackage;
use qsl_replay::{check_closure, CertificateRequest, ClosureCertificate, ClosureRefusal};
use qsl_replay::{CounterexampleKind, ModelCheckItem, ModelCheckLimits};
use qsl_replay::{ModelCheckOutcome, UndecidedCause};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNode, FormulaNodeId};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::{Cancel, Incomplete, Meter};
use quire_semantic_value::catalog::{UndefinedReason, UndefinedRecord};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::{declarations_in, document_digest, UNLIMITED};

const PACKAGE: &str = "test/things";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn object_type(name: &str, supertypes: &[&str]) -> Json {
    let identity = ident(name);
    let fields: Vec<Json> = if name == "Animal" {
        let age = ident("Animal/age");
        vec![json!({
            "identity": age, "name": "age", "typeRef": ident("Level"),
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin(&age),
        })]
    } else {
        Vec::new()
    };
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
        "origin": origin(&identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": supertypes.iter().map(|name| ident(name)).collect::<Vec<_>>(),
        "fields": fields, "operations": [], "relationships": [],
    })
}

/// `Int[0, upper]`, the type of `Animal`'s `age`.
fn level(upper: u64) -> Json {
    let identity = ident("Level");
    let constraint = |keyword: &str, value: String| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "value_type"}, "roles": [],
        "origin": origin(&identity), "extensions": [], "unknownPolicy": "reject",
        "scalar": "integer",
        "constraints": [constraint("min", "0".into()), constraint("max", upper.to_string())],
    })
}

/// The domain package with `populations`, each `(name, member types)`.
fn document(populations: &[(&str, &[&str])]) -> Vec<u8> {
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let populations: Vec<Json> = populations
        .iter()
        .map(|(name, members)| {
            let identity = ident(name);
            json!({
                "identity": identity, "displayName": identity,
                "kind": {"module": PACKAGE, "name": "population"},
                "members": members.iter().map(|name| ident(name)).collect::<Vec<_>>(),
                "extent": "closed", "origin": origin(&identity),
            })
        })
        .collect();
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            object_type("Thing", &[]),
            object_type("Lamp", &["Thing"]),
            object_type("Switch", &["Thing"]),
            object_type("Other", &[]),
            object_type("Animal", &[]),
            object_type("Dog", &["Animal"]),
            object_type("Cat", &["Animal"]),
            level(9),
        ],
        "populations": populations,
    })
    .to_string()
    .into_bytes()
}

/// One population of a snapshot: its name and its objects, each `(key,
/// type)`.
type Held<'a> = (&'a str, &'a [(&'a str, &'a str)]);

/// The model, its checked package, and one initial snapshot.
struct World {
    document: Vec<u8>,
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: Vec<DocumentRef>,
}

fn world(populations: &[(&str, &[&str])], held: &[Held<'_>]) -> World {
    let document = document(populations);
    let graph = declarations_in(PACKAGE, &document, "")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let package = CheckedPackage::link(graph);
    let populations: Vec<Json> = held
        .iter()
        .map(|(population, objects)| {
            let objects: Vec<Json> = objects
                .iter()
                .map(|(key, type_name)| {
                    let fields = if matches!(*type_name, "Animal" | "Dog" | "Cat") {
                        json!({"age": {"integer": "5"}})
                    } else {
                        json!({})
                    };
                    json!({"key": key, "type": ident(type_name), "fields": fields})
                })
                .collect();
            json!({"population": ident(population), "complete": true, "objects": objects})
        })
        .collect();
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current",
        "model": {"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&document_digest(&document)))},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": populations,
    })
    .to_string()
    .into_bytes();
    let read = quire_canonical::read(&snapshot, u64::MAX).expect("JSON fixture");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    World {
        packages: package_input([document.as_slice()]),
        snapshots: BTreeMap::from([(digest, snapshot)]),
        initial: vec![DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest,
        }],
        document,
        package,
    }
}

/// The universes, each `(population, keys)`.
fn universes(universes: &[(&str, &[&str])]) -> Vec<PopulationUniverse> {
    universes
        .iter()
        .map(|(population, keys)| PopulationUniverse {
            population: key(population),
            keys: keys.iter().map(|key| (*key).to_owned()).collect(),
        })
        .collect()
}

impl World {
    fn subject<'w>(
        &'w self,
        package: &'w CheckedPackage,
        universes: Vec<PopulationUniverse>,
    ) -> ModelSubject<'w> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes,
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: UNLIMITED,
            },
            proof_bounds: Vec::new(),
        }
    }

    /// The package declaring the TP-1 clause `C`, `always holds(true)` over
    /// `x: M::<over>`, and that clause.
    fn clause(&self, over: &str) -> (CheckedPackage, CheckedTemporalClause) {
        let body = format!(
            "temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds(true) }}\n"
        );
        let graph = declarations_in(PACKAGE, &self.document, &body)
            .with_unwritten_terminals()
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
        let [node] = graph.temporal_clauses() else {
            panic!("one declared clause");
        };
        let clause = node.clause().clone();
        (CheckedPackage::link(graph), clause)
    }

    /// The instances `ModelSystem` derives for `over` under `universes`.
    fn instances(
        &self,
        over: &str,
        universes: Vec<PopulationUniverse>,
    ) -> Result<Vec<ModelInstance>, NotSimulated> {
        let (_, clause) = self.clause(over);
        ModelSystem::new(self.subject(&self.package, universes).simulation)
            .expect("the subject admits")
            .instances(&clause.over().1)
    }

    /// EN-1's run of `over`'s clause under `universes`, its atom read as
    /// `holds`.
    fn run(
        &self,
        over: &str,
        universes: Vec<PopulationUniverse>,
        holds: bool,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let (package, clause) = self.clause(over);
        let letters = Constant(holds);
        check_model(
            ModelCheckRequest {
                subject: self.subject(&package, universes),
                item: ModelCheckItem::Clause {
                    clause: &clause,
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
    }

    /// The core's closure check of `over`'s clause under `universes`, of an
    /// empty certificate.
    fn check(
        &self,
        over: &str,
        universes: Vec<PopulationUniverse>,
    ) -> Result<qsl_replay::TemporalCertificateCheck, ClosureRefusal> {
        let (package, clause) = self.clause(over);
        let letters = Constant(true);
        check_closure(
            &CertificateRequest {
                subject: self.subject(&package, universes),
                item: ModelCheckItem::Clause {
                    clause: &clause,
                    letters: &letters,
                },
            },
            &ClosureCertificate::default(),
            &ModelCheckLimits::default(),
            &Cancel::new(),
        )
    }
}

/// A letter whose one atom reads the same truth for every instance.
struct Constant(bool);

impl ModelLetters for Constant {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(if self.0 {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

fn instance(population: &str, key: &str) -> ModelInstance {
    ModelInstance {
        population: self::key(population),
        key: key.to_owned(),
    }
}

/// The requires-bound root of `population`: the digest of its declaration
/// identity, as FR-120 names a population root.
fn population_root(population: &str) -> DomainKey {
    let naming = json!({"digest_domain": "sha256-jcs", "node": ident(population),
        "package": PACKAGE});
    let read = quire_canonical::read(naming.to_string().as_bytes(), u64::MAX).expect("JSON");
    let digest =
        quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX)).expect("canonical");
    DomainKey::Node {
        node: WireNodeId::from_digest(*digest.as_bytes()),
        path: Vec::new(),
    }
}

/// One declared population: its name and its member types.
type Declared<'a> = (&'a str, &'a [&'a str]);

const P1: (&str, &[&str]) = ("P1", &["Lamp", "Switch"]);
const P2: (&str, &[&str]) = ("P2", &["Thing"]);
const Q: (&str, &[&str]) = ("Q", &["Other"]);

// A checked clause names the one population that covers its `over` type, a
// member type that is `T` or a proper supertype of it (FR-104), and S3 refuses
// one that several cover (FR-336-AC-1). Admission is wider: a member type that
// is `T`, a subtype or a supertype of it (FR-125). `over (x: Thing)` is covered
// by `P2` alone and admitted by both `P1` (by its subtypes) and `P2`, so it is
// the clause these tests read several admitting populations through; `P1` and
// `P2` together both cover `Lamp`, so an `over (x: Lamp)` clause meets each of
// them in a model without the other.

/// FR-125-AC-8 and FR-125-AC-10: `P1` declares `Lamp` and `Switch`, `P2`
/// declares their supertype `Thing`, `Q` declares the unrelated `Other`.
/// Under universes `P1: [d, c]`, `P2: [b, a]`, `Q: [z]`, `over (x: Thing)`
/// has the instances `P1/d`, `P1/c`, `P2/b`, `P2/a`: one per key, though two
/// of `P1`'s member types are comparable with `Thing`, ordered by population
/// identity text then universe order, not by key. `Q` admits neither, and
/// `over (x: Other)` has `Q/z` alone. `over (x: Lamp)` has each key of `P1`,
/// which declares `Lamp`, and each key of `P2`, which admits `Lamp` by its
/// supertype. A violation names the first instance in that order.
#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10")]
#[test]
fn instances_are_each_key_of_each_admitting_population_in_canonical_order() {
    let world = world(
        &[P1, P2, Q],
        &[
            ("P1", &[("d", "Lamp"), ("c", "Switch")]),
            ("P2", &[("b", "Lamp")]),
            ("Q", &[]),
        ],
    );
    let bounded = || universes(&[("Q", &["z"]), ("P2", &["b", "a"]), ("P1", &["d", "c"])]);
    assert_eq!(
        world.instances("Thing", bounded()).expect("bounded"),
        vec![
            instance("P1", "d"),
            instance("P1", "c"),
            instance("P2", "b"),
            instance("P2", "a"),
        ]
    );
    assert_eq!(
        world.instances("Other", bounded()).expect("bounded"),
        vec![instance("Q", "z")]
    );

    let declared = self::world(
        &[P1, Q],
        &[("P1", &[("d", "Lamp"), ("c", "Switch")]), ("Q", &[])],
    );
    assert_eq!(
        declared
            .instances("Lamp", universes(&[("Q", &["z"]), ("P1", &["d", "c"])]))
            .expect("bounded"),
        vec![instance("P1", "d"), instance("P1", "c")]
    );
    let supertype = self::world(&[P2, Q], &[("P2", &[("b", "Lamp")]), ("Q", &[])]);
    assert_eq!(
        supertype
            .instances("Lamp", universes(&[("Q", &["z"]), ("P2", &["b", "a"])]))
            .expect("bounded"),
        vec![instance("P2", "b"), instance("P2", "a")]
    );

    let outcome = world.run("Thing", bounded(), false).expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected Violated, got {outcome:?}");
    };
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(counterexample.over_binding, Some(instance("P1", "d")));
    assert!(matches!(
        world.run("Thing", bounded(), true).expect("checked"),
        ModelCheckOutcome::Holds { .. }
    ));
}

/// FR-125-AC-8: a population that admits `T` and has no universe is
/// `RequiresBound` naming it, though no initial state holds it and it is not
/// the population the clause names; one that does not admit `T` needs none.
/// When no admitting population has a key, or no population admits `T`, the
/// item is `NoInstance` before any state is explored.
#[trace("TC-520", "FR-125-AC-8")]
#[test]
fn an_unbounded_admitting_population_requires_a_bound_and_no_key_is_no_instance() {
    let world = world(&[P1, P2, Q], &[("P2", &[("b", "Lamp")])]);
    let refusal = world
        .run("Thing", universes(&[("P2", &["b"])]), true)
        .expect_err("P1 admits Thing and has no universe");
    let ModelCheckRefusal::RequiresBound(bound) = &refusal else {
        panic!("expected RequiresBound, got {refusal:?}");
    };
    let roots: Vec<_> = bound
        .domains
        .iter()
        .map(|(key, kind)| (key.clone(), kind))
        .collect();
    assert_eq!(roots, [(population_root("P1"), DomainKind::Population)]);

    let world = self::world(&[P1, Q], &[("P1", &[("d", "Lamp")])]);
    assert_eq!(
        world
            .instances("Switch", universes(&[("P1", &["d"])]))
            .expect("Q does not admit Switch, so it needs no universe"),
        vec![instance("P1", "d")]
    );

    let empty: [(&[Declared<'_>], &str); 3] = [
        (&[P1, P2, Q], "Thing"),
        (&[P1, Q], "Lamp"),
        (&[P2, Q], "Lamp"),
    ];
    for (populations, over) in empty {
        let held: Vec<Held<'_>> = populations
            .iter()
            .map(|(name, _)| (*name, &[][..]))
            .collect();
        let world = self::world(populations, &held);
        let keys: Vec<(&str, &[&str])> = populations
            .iter()
            .map(|(name, _)| (*name, if *name == "Q" { &["z"][..] } else { &[][..] }))
            .collect();
        assert!(
            matches!(
                world.run(over, universes(&keys), true).expect("checked"),
                ModelCheckOutcome::Undecided {
                    cause: UndecidedCause::NoInstance,
                    stats,
                } if stats.product_states == 0
            ),
            "over {over} in {populations:?}"
        );
    }
    let world = self::world(&[P1, P2], &[("P1", &[]), ("P2", &[])]);
    let none = universes(&[("P1", &["d"]), ("P2", &["b"])]);
    assert_eq!(world.instances("Other", none.clone()), Ok(Vec::new()));
    assert!(matches!(
        world.run("Other", none, true).expect("checked"),
        ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            ..
        }
    ));
}

/// FR-125 and FR-338: the closure checker derives the same instances as
/// EN-1, so a population that admits `T` and has no universe, held by no
/// initial state and not the population the clause names, refuses the check
/// `NotSimulated(RequiresBound)` naming that population, before any
/// certificate state is read.
#[trace("TC-520", "FR-125-AC-8")]
#[test]
fn the_closure_checker_requires_a_bound_for_an_unbounded_admitting_population() {
    let world = world(&[P1, P2, Q], &[("P2", &[("b", "Lamp")])]);
    let refusal = world
        .check("Thing", universes(&[("P2", &["b"])]))
        .expect_err("P1 admits Thing and has no universe");
    let ClosureRefusal::NotSimulated(refusal) = &refusal else {
        panic!("expected NotSimulated, got {refusal:?}");
    };
    let NotSimulated::RequiresBound(RequiresBound { domains }) = refusal.as_ref() else {
        panic!("expected RequiresBound, got {refusal:?}");
    };
    let roots: Vec<_> = domains
        .iter()
        .map(|(key, kind)| (key.clone(), kind))
        .collect();
    assert_eq!(roots, [(population_root("P1"), DomainKind::Population)]);
}

/// A universe key is an object identity, so the empty key is a caller's
/// invalid input: EN-1 and the closure checker both refuse the subject's
/// admission `invalid_runtime_input`/`invalid-value` naming the population
/// and the key, never an internal fault of instancing.
#[trace("TC-520")]
#[test]
fn an_empty_universe_key_is_refused_as_invalid_input_not_a_fault() {
    let world = world(&[P1], &[("P1", &[("d", "Lamp")])]);
    let bad = || universes(&[("P1", &["d", ""])]);
    let invalid = |failure: &AdmissionFailure| match failure {
        AdmissionFailure::Refused(record) => {
            assert_eq!(record.code, Code::InvalidRuntimeInput);
            assert_eq!(record.cause, "invalid-value");
            assert_eq!(record.fields.get("key").map(String::as_str), Some(""));
        }
        other => panic!("expected Refused, got {other:?}"),
    };
    match world.run("Lamp", bad(), true) {
        Err(ModelCheckRefusal::Admission(failure)) => invalid(&failure),
        other => panic!("expected an admission refusal, got {other:?}"),
    }
    match world.check("Lamp", bad()) {
        Err(ClosureRefusal::Admission(failure)) => invalid(&failure),
        other => panic!("expected an admission refusal, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Reading the bound parameter (FR-125-AC-11, AC-12)
// ---------------------------------------------------------------------------

impl World {
    /// The package declaring the clause `C`, `on origin`, over `x: M::<over>`
    /// with `formula`, and its checked node index.
    fn formula(&self, over: &str, formula: &str) -> CheckedPackage {
        self.clause_text(&format!("M::{over}"), "on origin", formula)
    }

    /// The package declaring the clause `C` over `x: <over>` with the
    /// activation `activation` and `formula`.
    fn clause_text(&self, over: &str, activation: &str, formula: &str) -> CheckedPackage {
        let body = format!(
            "temporal C using t0 over (x: {over}) clock \"model-steps\" {activation} {{ {formula} }}\n"
        );
        let graph = declarations_in(PACKAGE, &self.document, &body)
            .with_unwritten_terminals()
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
        CheckedPackage::link(graph)
    }

    /// EN-1's run of `formula` over `x: M::<over>` through the production
    /// letters of its checked clause.
    fn run_formula(
        &self,
        over: &str,
        formula: &str,
        universes: Vec<PopulationUniverse>,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let package = self.formula(over, formula);
        let system = ModelSystem::new(self.subject(&package, universes.clone()).simulation)
            .expect("the subject admits");
        let node = &package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("an unguarded clause");
        check_model(
            ModelCheckRequest {
                subject: self.subject(&package, universes),
                item: ModelCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
    }

    /// Each `holds` atom of `formula` over `x: M::<over>`, read at the
    /// initial state for each of `keys`' instances: `readings[atom][key]`.
    fn atom_readings(
        &self,
        over: &str,
        formula: &str,
        universes: Vec<PopulationUniverse>,
        instances: &[ModelInstance],
    ) -> Vec<Vec<AtomReading>> {
        let package = self.formula(over, formula);
        let system = ModelSystem::new(self.subject(&package, universes).simulation)
            .expect("the subject admits");
        let node = &package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("an unguarded clause");
        let start = system.initial_states()[0].clone();
        let formula = node.clause().formula();
        (0..formula.nodes().len())
            .filter(|index| matches!(formula.nodes()[*index], FormulaNode::Holds { .. }))
            .map(|index| {
                let atom = formula.node_id(index).expect("a node");
                instances
                    .iter()
                    .map(|instance| {
                        letters
                            .atom(atom, instance, &start, &mut Meter::new(UNLIMITED))
                            .expect("the meter is not exhausted")
                    })
                    .collect()
            })
            .collect()
    }
}

/// FR-125-AC-11: the bound parameter reads as the state's own reference to
/// the object under the instance's key, with that object's type, whichever
/// member type the population lists first: `x = r` for `r` the reference
/// `allInstances` yields, and `contains(allInstances<Thing>(P), x)`, are true
/// for a `Switch` and for a `Lamp` under a population listing `Lamp` before
/// `Thing`. The object is found by its key across the populations that
/// overlap `Thing`: with `c` listed by `P1` and `P2` and the object held by
/// `P2` alone, the instance (at `P1`, its first listing) reads
/// `contains(allInstances<Thing>(P2), x)` true and the same of `P1` false.
#[trace("TC-520", "FR-125-AC-11")]
#[test]
fn the_bound_parameter_reads_as_the_reference_of_the_object_held_under_its_key() {
    const PM: Declared<'_> = ("PM", &["Lamp", "Thing"]);
    let world = world(&[PM], &[("PM", &[("l", "Lamp"), ("s", "Switch")])]);
    let bounded = || universes(&[("PM", &["l", "s"])]);
    let instances = [instance("PM", "l"), instance("PM", "s")];
    let readings = world.atom_readings(
        "Thing",
        "always (holds(exists(o in allInstances<M::Thing>(M::PM) : o = x)) \
            and holds(contains(allInstances<M::Thing>(M::PM), x)))",
        bounded(),
        &instances,
    );
    assert_eq!(
        readings,
        vec![vec![AtomReading::True; 2]; 2],
        "x equals the state's reference and is a member, for Lamp l and Switch s"
    );
    assert!(matches!(
        world
            .run_formula(
                "Thing",
                "always holds(contains(allInstances<M::Thing>(M::PM), x))",
                bounded()
            )
            .expect("checked"),
        ModelCheckOutcome::Holds { .. }
    ));

    let shared = self::world(&[P1, P2], &[("P1", &[]), ("P2", &[("c", "Switch")])]);
    let both = || universes(&[("P1", &["c"]), ("P2", &["c"])]);
    let readings = shared.atom_readings(
        "Thing",
        "always (holds(contains(allInstances<M::Thing>(M::P2), x)) \
            and holds(exists(o in allInstances<M::Thing>(M::P2) : o = x)))",
        both(),
        &[instance("P1", "c")],
    );
    assert_eq!(readings, [[AtomReading::True], [AtomReading::True]]);
}

/// FR-125-AC-12, with `animals` (`Animal`, universe `[k, t]`, a `Dog` under
/// `k` and a `Cat` under `t`) the one population that covers `Dog`: the
/// instances of `over (x: Dog)` are `k` and `t`; `k` reads as the `Dog`
/// reference, and the object `t` holds, a `Cat`, reads `absent-key`
/// undefined, so the unguarded `x.age` refutes at `t` with an undefined
/// evaluation. A subject whose only population holds only a `Cat` still has
/// its instance and is not `NoInstance`. Not covered: the guard exception, and
/// the half of the criterion that needs `dogs` beside `animals`, which S3
/// refuses for a clause over `Dog` (two populations cover it).
#[trace("TC-520")]
#[test]
fn a_key_holding_no_object_of_the_type_reads_absent_key() {
    let world = world(
        &[("animals", &["Animal"])],
        &[("animals", &[("k", "Dog"), ("t", "Cat")])],
    );
    let bounded = || universes(&[("animals", &["k", "t"])]);
    let instances = world.instances("Dog", bounded()).expect("bounded");
    assert_eq!(
        instances,
        vec![instance("animals", "k"), instance("animals", "t")]
    );

    let readings = world.atom_readings(
        "Dog",
        "always (holds(exists(o in allInstances<M::Dog>(M::animals) : o = x)) \
         and holds(x.age >= 0))",
        bounded(),
        &instances,
    );
    let absent_key = AtomReading::Undefined(UndefinedCause::Family(UndefinedRecord {
        reason: UndefinedReason::AbsentKey,
        fields: BTreeMap::new(),
    }));
    assert_eq!(
        readings,
        vec![vec![AtomReading::True, absent_key.clone()]; 2],
        "k reads as the Dog reference; t's Cat reads absent-key in every atom"
    );

    let unguarded = world
        .run_formula("Dog", "always holds(x.age >= 0)", bounded())
        .expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = unguarded else {
        panic!("expected Violated, got {unguarded:?}");
    };
    assert_eq!(counterexample.over_binding, Some(instance("animals", "t")));
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(AtomReading::Undefined(evaluation.cause.clone()), absent_key);

    let cats = self::world(&[("animals", &["Animal"])], &[("animals", &[("t", "Cat")])]);
    let only = || universes(&[("animals", &["t"])]);
    assert_eq!(
        cats.instances("Dog", only()).expect("bounded"),
        vec![instance("animals", "t")]
    );
    assert!(matches!(
        cats.run_formula("Dog", "always holds(x.age >= 0)", only())
            .expect("checked"),
        ModelCheckOutcome::Violated { .. }
    ));
}

/// `ModelSystem::clause_letters` refuses a clause whose `over` parameter is
/// no object reference.
#[trace("TC-520")]
#[test]
fn letters_refuse_a_non_reference_over() {
    let world = world(&[P1], &[("P1", &[("l", "Lamp")])]);
    let package = world.clause_text("Integer", "on origin", "always holds(true)");
    let system = ModelSystem::new(
        world
            .subject(&package, universes(&[("P1", &["l"])]))
            .simulation,
    )
    .expect("the subject admits");
    let node = &package.graph().temporal_clauses()[0];
    assert_eq!(
        system.clause_letters(node).err(),
        Some(LettersRefusal::OverNotReference)
    );
}
