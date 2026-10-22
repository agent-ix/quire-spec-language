// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-125 Instancing over a model of several populations and a type
//! hierarchy: the `test/things` domain package declares `Thing`, its
//! subtypes `Lamp` and `Switch`, and the unrelated `Other`; each test
//! declares the populations it needs. An instance of `over (x: T)` is one
//! key of a population that admits `T`, derived from the subject's
//! universes, in the canonical order.
//!
//! Not covered: reading the bound parameter as the state's own reference
//! (FR-125-AC-9, AC-11, AC-12's reading half) is the FR-107 letter seam's,
//! which these tests supply themselves; no state-graph claim (FR-165) is
//! instantiated here.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::{NotSimulated, RequiresBound};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::WireNodeId;
use qsl_package::CheckedPackage;
use qsl_replay::{check_closure, CertificateRequest, ClosureCertificate, ClosureRefusal};
use qsl_replay::{CounterexampleKind, ModelCheckItem, ModelCheckLimits};
use qsl_replay::{ModelCheckOutcome, UndecidedCause};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};
use quire_exact::{Cancel, Incomplete, Meter};
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
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
        "origin": origin(&identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": supertypes.iter().map(|name| ident(name)).collect::<Vec<_>>(),
        "fields": [], "operations": [], "relationships": [],
    })
}

/// The domain package with `populations`, each `(name, member types)`.
fn document(populations: &[(&str, &[&str])]) -> Vec<u8> {
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
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
                .map(|(key, type_name)| json!({"key": key, "type": ident(type_name), "fields": {}}))
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
