// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-153-AC-3's `Toggle` subject for the annotated-quotient tests
//! (`crate::quotient`): a `test/toggle` domain package with object type
//! `Toggle` (`on: Boolean`, `fired: Boolean`) in the population `toggles`,
//! declared `symmetric`, and operations `flip()` (frame `modifies_receiver
//! on`) and `fire()` (frame `modifies_receiver fired`), whose contracts the
//! unit writes.
//!
//! A temporal clause always has an `over` parameter, so the claim AC-3
//! writes with none is bound here to the one object `w` of a second
//! population, `watchers` of the field-less `Watch`, which no symmetry
//! declaration names: its one instance runs under the full group of
//! `toggles`, as AC-3's claim does. The `holds` atom reads the toggles
//! alone, through the test's own letters.

use std::collections::BTreeMap;

use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest, Reductions};
use qsl_eval::automaton::translate;
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::AtomReading;
use qsl_package::CheckedPackage;
use qsl_replay::{ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, TemporalCounterexample};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{meaning, package_input};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::{Cancel, Incomplete, Meter, Value};
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;
use crate::units::{checked, clause, unit, Domain};

const PACKAGE: &str = "test/toggle";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The toggles' keys.
pub(crate) const TOGGLES: [&str; 2] = ["a", "b"];

/// `flip()` negates `on`; `fire()` needs `on` and not yet `fired`, and sets
/// `fired`.
const CONTRACTS: &str = "post Flip using v on M::Toggle::flip { self.on = not pre(self.on) }\n\
     pre Ready using v on M::Toggle::fire { self.on and not self.fired }\n\
     post Fired using v on M::Toggle::fire { self.fired }\n";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

/// The declaration key of `suffix` in the package.
pub(crate) fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

/// The domain package.
fn domain() -> Domain {
    let field = |name: &str| {
        let identity = ident(&format!("Toggle/{name}"));
        json!({
            "identity": identity, "name": name, "typeRef": "ix://quire/native/Boolean",
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin(&identity),
        })
    };
    let operation = |name: &str, modifies: &str| {
        let identity = ident(&format!("Toggle/{name}"));
        json!({
            "identity": identity, "name": name, "params": [], "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": [], "modifies_receiver": [ident(&format!("Toggle/{modifies}"))],
                "creates": [], "deletes": []},
        })
    };
    let object = |name: &str, fields: Vec<Json>, operations: Vec<Json>| {
        let identity = ident(name);
        json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
            "origin": origin(&identity), "constraints": [], "extensions": [],
            "unknownPolicy": "reject", "supertypes": [],
            "fields": fields, "operations": operations, "relationships": [],
        })
    };
    let population = |name: &str, member: &str, symmetric: bool| {
        let identity = ident(name);
        json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [ident(member)], "extent": "closed", "origin": origin(&identity),
            "symmetric": symmetric,
        })
    };
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
    let document = json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            object(
                "Toggle",
                vec![field("on"), field("fired")],
                vec![operation("flip", "on"), operation("fire", "fired")],
            ),
            object("Watch", Vec::new(), Vec::new()),
        ],
        "populations": [
            population("toggles", "Toggle", true),
            population("watchers", "Watch", false),
        ],
    });
    Domain {
        package: PACKAGE,
        population: "toggles",
        object: "Toggle",
        document: document.to_string().into_bytes(),
    }
}

/// The `toggles` population.
pub(crate) fn toggles() -> DeclarationKey {
    key("toggles")
}

/// The checked `Toggle` unit with one clause of `fairness` and `formula`,
/// both toggles initially all false, and its provisioned snapshot.
pub(crate) struct World {
    package: CheckedPackage,
    pub(crate) clauses: Vec<CheckedTemporalClause>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: Vec<DocumentRef>,
}

impl World {
    /// The unit with the clause `fairness formula`, over `w: M::Watch`.
    pub(crate) fn new(fairness: &str, formula: &str) -> Self {
        let domain = domain();
        let text = unit(
            &domain,
            "M",
            &format!("{CONTRACTS}{}", clause("w: M::Watch", fairness, formula)),
        );
        let (package, clauses) = checked(&domain, &text);
        let packages = package_input([domain.document.as_slice()]);
        let toggle = json!({"on": {"boolean": false}, "fired": {"boolean": false}});
        let objects: Vec<Json> = TOGGLES
            .iter()
            .map(|key| json!({"key": key, "type": ident("Toggle"), "fields": toggle}))
            .collect();
        let header = json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", domain.digest_hex())});
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [
                {"population": ident("toggles"), "complete": true, "objects": objects},
                {"population": ident("watchers"), "complete": true,
                    "objects": [{"key": "w", "type": ident("Watch"), "fields": {}}]},
            ],
        });
        let bytes = document.to_string().into_bytes();
        let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
        let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
            .expect("canonical fixture")
            .as_bytes();
        let snapshots = BTreeMap::from([(digest, bytes)]);
        let initial = vec![DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest,
        }];
        Self {
            package,
            clauses,
            packages,
            snapshots,
            initial,
        }
    }

    fn subject(&self) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes: vec![
                    PopulationUniverse {
                        population: toggles(),
                        keys: TOGGLES.iter().map(|key| (*key).to_owned()).collect(),
                    },
                    PopulationUniverse {
                        population: key("watchers"),
                        keys: vec!["w".to_owned()],
                    },
                ],
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: UNLIMITED,
            },
            proof_bounds: Vec::new(),
        }
    }

    /// The run of the clause, its one `holds` atom true where every toggle
    /// has fired, under `reductions`.
    pub(crate) fn run(
        &self,
        reductions: Reductions,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let letters = AllFired::new(&self.clauses[0]);
        check_model(
            ModelCheckRequest {
                subject: self.subject(),
                item: ModelCheckItem::Clause {
                    clause: &self.clauses[0],
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions,
            },
            &Cancel::new(),
        )
    }

    /// The model state at each position of `counterexample`, read by
    /// re-executing its steps from its initial state.
    pub(crate) fn positions(&self, counterexample: &TemporalCounterexample) -> Vec<ModelSimState> {
        let system = ModelSystem::new(self.subject().simulation).expect("the subject admits");
        let mut current = system.initial()[counterexample.initial_state].clone();
        let mut states = vec![current.clone()];
        for step in &counterexample.prefix {
            current = system
                .successors(&current)
                .expect("the state expands")
                .successors
                .into_iter()
                .find(|(transition, state)| {
                    *transition == step.transition
                        && system.state_key(state).expect("encodes").1 == step.post_state
                })
                .map(|(_, state)| state)
                .expect("the recorded step is a subject step");
            states.push(current.clone());
        }
        states
    }
}

/// The boolean field `field` of toggle `object` at `state`.
pub(crate) fn flag(state: &ModelSimState, object: &str, field: &str) -> bool {
    match state.state().populations[&toggles()][object]
        .fields
        .get(&key(&format!("Toggle/{field}")))
    {
        Some(Value::Boolean(value)) => *value,
        other => panic!("{object}.{field} is not a boolean: {other:?}"),
    }
}

/// The letters of the clause: its one `holds` atom is true exactly where
/// every toggle has fired.
struct AllFired {
    atom: FormulaNodeId,
}

impl AllFired {
    fn new(clause: &CheckedTemporalClause) -> Self {
        let atoms = translate(clause, 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        let [atom] = atoms.as_slice() else {
            panic!("one `holds` atom");
        };
        Self { atom: *atom }
    }
}

impl ModelLetters for AllFired {
    fn atom(
        &self,
        atom: FormulaNodeId,
        _instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        assert_eq!(atom, self.atom, "the clause's own atom");
        Ok(
            if TOGGLES.iter().all(|toggle| flag(state, toggle, "fired")) {
                AtomReading::True
            } else {
                AtomReading::False
            },
        )
    }
}
