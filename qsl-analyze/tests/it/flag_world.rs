// SPDX-License-Identifier: AGPL-3.0-or-later
//! The boolean-flag subjects of the annotated-quotient tests
//! (`crate::quotient`), each a `test/<name>` domain package with one object
//! type in a population declared `symmetric`, over the universe `{a, b}`:
//!
//! - FR-153-AC-3's `Toggle` (`on`, `fired`) in `toggles`, with `flip()`
//!   (frame `modifies_receiver on`) and `fire()` (frame `modifies_receiver
//!   fired`), both toggles initially all false;
//! - FR-161-AC-2's `Token`: `Holder` (`tok`) in `holders`, with `pass()`
//!   (frame `modifies tok`), `a.tok` false and `b.tok` true. FR-161-AC-2's
//!   postcondition ranges over `M::holders` (FR-391), which S3 admits but
//!   package emission refuses until FR-322 encodes the population
//!   reference (QSL-869), and the run reads the clause root from the
//!   emitted package; so each holder names the other in a reference field
//!   `next`, and `pass()` hands the token to `self.next`: with two holders,
//!   every other holder.
//!
//! The unit writes the contracts. A temporal clause always has an `over`
//! parameter, so a claim the ACs write with none is bound to the one object
//! `w` of a second population, `watchers` of the field-less `Watch`, which
//! no symmetry declaration names: its one instance runs under the full
//! group. A clause may instead be bound over the subject's own type, whose
//! one orbit then runs under a trivial stabiliser. The one `holds` atom is
//! read by the test's letters as "every object's flag is true", as the ACs
//! state the run supplies it.

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
use qsl_replay::{
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelTraceSettlement, ModelTraceSubject,
    TemporalCounterexample,
};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{meaning, package_input};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::{Cancel, Incomplete, Meter, Value};
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;
use crate::reduction_world::{clause_identity, replay_unit, ReplayUnit};
use crate::units::{checked, clause, unit, Domain};

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The keys of the subject's population.
pub(crate) const KEYS: [&str; 2] = ["a", "b"];

/// One of the boolean-flag subjects.
#[derive(Clone, Copy)]
pub(crate) enum Subject {
    /// FR-153-AC-3's toggles.
    Toggle,
    /// FR-161-AC-2's token.
    Token,
}

/// An operation: its name, the field its frame grants, and whether the
/// grant is scoped to the receiver.
type Operation = (&'static str, &'static str, bool);

impl Subject {
    fn package(self) -> &'static str {
        match self {
            Self::Toggle => "test/toggle",
            Self::Token => "test/token",
        }
    }

    /// The object type's name.
    pub(crate) fn object(self) -> &'static str {
        match self {
            Self::Toggle => "Toggle",
            Self::Token => "Holder",
        }
    }

    fn population_name(self) -> &'static str {
        match self {
            Self::Toggle => "toggles",
            Self::Token => "holders",
        }
    }

    /// The fields: each a boolean, or (`true`) a reference to the object
    /// type.
    fn fields(self) -> &'static [(&'static str, bool)] {
        match self {
            Self::Toggle => &[("on", false), ("fired", false)],
            Self::Token => &[("tok", false), ("next", true)],
        }
    }

    fn operations(self) -> &'static [Operation] {
        match self {
            Self::Toggle => &[("flip", "on", true), ("fire", "fired", true)],
            Self::Token => &[("pass", "tok", false)],
        }
    }

    /// `flip()` negates `on`; `fire()` needs `on` and not yet `fired`, and
    /// sets `fired`. `pass()` needs the token and hands it to every other
    /// holder.
    fn contracts(self) -> &'static str {
        match self {
            Self::Toggle => {
                "post Flip using v on M::Toggle::flip { self.on = not pre(self.on) }\n\
                 pre Ready using v on M::Toggle::fire { self.on and not self.fired }\n\
                 post Fired using v on M::Toggle::fire { self.fired }\n"
            }
            Self::Token => {
                "pre Has using v on M::Holder::pass { self.tok }\n\
                 post Pass using v on M::Holder::pass { not self.tok and self.next.tok }\n"
            }
        }
    }

    /// The fields of `key` in each initial state, in subject order. The
    /// token's AC names one initial state, `b` holding the token, but
    /// symmetry admits only a set of initial states closed under the
    /// group (ADR-021 SYM-4), so its image under `(a b)` follows it.
    fn starts(self) -> Vec<Vec<(&'static str, Json)>> {
        match self {
            Self::Toggle => vec![KEYS
                .iter()
                .map(|key| {
                    (
                        *key,
                        json!({"on": {"boolean": false}, "fired": {"boolean": false}}),
                    )
                })
                .collect()],
            Self::Token => ["b", "a"]
                .into_iter()
                .map(|holder| {
                    KEYS.iter()
                        .map(|key| {
                            let other = if *key == "a" { "b" } else { "a" };
                            (
                                *key,
                                json!({"tok": {"boolean": *key == holder},
                                    "next": {"reference": {"population": self.ident("holders"),
                                        "key": other}}}),
                            )
                        })
                        .collect()
                })
                .collect(),
        }
    }

    /// The flag the claim's atom reads: true where every object's is.
    fn goal(self) -> &'static str {
        match self {
            Self::Toggle => "fired",
            Self::Token => "tok",
        }
    }

    fn ident(self, suffix: &str) -> String {
        format!("ix://{}/{suffix}", self.package())
    }

    fn key(self, suffix: &str) -> DeclarationKey {
        DeclarationKey {
            package: self.package().to_owned(),
            node: self.ident(suffix),
        }
    }

    /// The subject's symmetric population.
    pub(crate) fn population(self) -> DeclarationKey {
        self.key(self.population_name())
    }

    /// The boolean field `field` of object `object` at `state`.
    pub(crate) fn flag(self, state: &ModelSimState, object: &str, field: &str) -> bool {
        match state.state().populations[&self.population()][object]
            .fields
            .get(&self.key(&format!("{}/{field}", self.object())))
        {
            Some(Value::Boolean(value)) => *value,
            other => panic!("{object}.{field} is not a boolean: {other:?}"),
        }
    }

    fn origin(identity: &str) -> Json {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    }

    /// The domain package.
    fn domain(self) -> Domain {
        let package = self.package();
        let object_name = self.object();
        let field = |(name, reference): &(&str, bool)| {
            let identity = self.ident(&format!("{object_name}/{name}"));
            let type_ref = if *reference {
                self.ident(object_name)
            } else {
                "ix://quire/native/Boolean".to_owned()
            };
            json!({
                "identity": identity, "name": name, "typeRef": type_ref,
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": Self::origin(&identity),
            })
        };
        let operation = |(name, grants, receiver): &Operation| {
            let identity = self.ident(&format!("{object_name}/{name}"));
            let granted = vec![self.ident(&format!("{object_name}/{grants}"))];
            let (modifies, modifies_receiver) = if *receiver {
                (Vec::new(), granted)
            } else {
                (granted, Vec::new())
            };
            json!({
                "identity": identity, "name": name, "params": [], "pre": [], "post": [],
                "origin": {"source": {"sourceIdentity": self.ident("spec"),
                    "path": "spec.qspec", "startLine": 1, "startColumn": 1}},
                "frame": {"modifies": modifies, "modifies_receiver": modifies_receiver,
                    "creates": [], "deletes": []},
            })
        };
        let object = |name: &str, fields: Vec<Json>, operations: Vec<Json>| {
            let identity = self.ident(name);
            json!({
                "identity": identity, "displayName": identity,
                "kind": {"module": package, "name": "object_type"}, "roles": [],
                "origin": Self::origin(&identity), "constraints": [], "extensions": [],
                "unknownPolicy": "reject", "supertypes": [],
                "fields": fields, "operations": operations, "relationships": [],
            })
        };
        let population = |name: &str, member: &str, symmetric: bool| {
            let identity = self.ident(name);
            json!({
                "identity": identity, "displayName": identity,
                "kind": {"module": package, "name": "population"},
                "members": [self.ident(member)], "extent": "closed",
                "origin": Self::origin(&identity), "symmetric": symmetric,
            })
        };
        let constructs: Vec<Json> = [
            ("object_type", meaning::OBJECT_TYPE),
            ("population", meaning::POPULATION),
        ]
        .into_iter()
        .map(|(name, meaning)| {
            json!({"kind": {"module": package, "name": name}, "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER,
                "construct": {"identity": "none", "shape": "record", "members": {},
                    "meaning": meaning}})
        })
        .collect();
        let document = json!({
            "contractVersion": "2.0.0",
            "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
            "source": {"identity": self.ident("spec"), "version": "1.0.0",
                "dialect": "spec-bundle", "digest": PLACEHOLDER},
            "package": {"identity": package, "version": "1.0.0",
                "manifestDigest": PLACEHOLDER, "mappingVersions": [], "profileVersions": [],
                "lockDigest": PLACEHOLDER},
            "occurrences": [], "extensions": [],
            "constructs": constructs,
            "types": [
                object(
                    object_name,
                    self.fields().iter().map(field).collect(),
                    self.operations().iter().map(operation).collect(),
                ),
                object("Watch", Vec::new(), Vec::new()),
            ],
            "populations": [
                population(self.population_name(), object_name, true),
                population("watchers", "Watch", false),
            ],
        });
        Domain {
            package,
            population: self.population_name(),
            object: object_name,
            document: document.to_string().into_bytes(),
        }
    }
}

/// A checked unit over one subject with one clause, its provisioned
/// snapshot and the initial state.
pub(crate) struct World {
    subject: Subject,
    package: CheckedPackage,
    clauses: Vec<CheckedTemporalClause>,
    document: Vec<u8>,
    text: String,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: Vec<DocumentRef>,
}

impl World {
    /// The unit over `subject` with the clause `fairness formula`, bound to
    /// the watcher `w`.
    pub(crate) fn new(subject: Subject, fairness: &str, formula: &str) -> Self {
        Self::over(subject, "w: M::Watch", fairness, formula)
    }

    /// The unit over `subject` with the clause `fairness formula`, over
    /// `over`.
    pub(crate) fn over(subject: Subject, over: &str, fairness: &str, formula: &str) -> Self {
        let domain = subject.domain();
        let text = unit(
            &domain,
            "M",
            &format!("{}{}", subject.contracts(), clause(over, fairness, formula)),
        );
        let (package, clauses) = checked(&domain, &text);
        let packages = package_input([domain.document.as_slice()]);
        let header = json!({"identity": subject.package(), "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", domain.digest_hex())});
        let mut snapshots = BTreeMap::new();
        let mut initial = Vec::new();
        for (index, start) in subject.starts().into_iter().enumerate() {
            let name = format!("s{index}");
            let objects: Vec<Json> = start
                .into_iter()
                .map(|(key, fields)| {
                    json!({"key": key, "type": subject.ident(subject.object()),
                        "fields": fields})
                })
                .collect();
            let document = json!({
                "format": "quire.state.snapshot/v1",
                "identity": {"authority": "test", "identity": name,
                    "revision_namespace": "fixture", "revision": "fixture:1"},
                "observation": "current", "model": header,
                "anchor": {"kind": "initialization", "name": "start"},
                "populations": [
                    {"population": subject.ident(subject.population_name()), "complete": true,
                        "objects": objects},
                    {"population": subject.ident("watchers"), "complete": true,
                        "objects": [{"key": "w", "type": subject.ident("Watch"),
                            "fields": {}}]},
                ],
            });
            let bytes = document.to_string().into_bytes();
            let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
            let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
                .expect("canonical fixture")
                .as_bytes();
            snapshots.insert(digest, bytes);
            initial.push(DocumentRef {
                authority: "test".to_owned(),
                identity: name,
                revision_namespace: "fixture".to_owned(),
                revision: "fixture:1".to_owned(),
                digest,
            });
        }
        Self {
            subject,
            package,
            clauses,
            document: domain.document,
            text,
            packages,
            snapshots,
            initial,
        }
    }

    fn universes(&self) -> Vec<PopulationUniverse> {
        vec![
            PopulationUniverse {
                population: self.subject.population(),
                keys: KEYS.iter().map(|key| (*key).to_owned()).collect(),
            },
            PopulationUniverse {
                population: self.subject.key("watchers"),
                keys: vec!["w".to_owned()],
            },
        ]
    }

    fn model_subject(&self) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes: self.universes(),
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: UNLIMITED,
            },
            proof_bounds: Vec::new(),
        }
    }

    /// The clause's letters: its one atom is true where every object's goal
    /// flag is.
    fn letters(&self) -> AllTrue {
        AllTrue::new(self.subject, &self.clauses[0])
    }

    /// The run of the clause under `reductions`.
    pub(crate) fn run(
        &self,
        reductions: Reductions,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let letters = self.letters();
        check_model(
            ModelCheckRequest {
                subject: self.model_subject(),
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
        let system = ModelSystem::new(self.model_subject().simulation).expect("the subject admits");
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

    /// FR-128: replay `payload` as the clause's counterexample.
    pub(crate) fn replay(&self, payload: TemporalCounterexample) -> ModelTraceSettlement {
        replay_unit(
            &ReplayUnit {
                identity: "units",
                path: "units.native",
                text: self.text.as_bytes(),
                document: &self.document,
                snapshots: &self.snapshots,
            },
            &ModelTraceSubject {
                initial: self.initial.clone(),
                universes: self.universes(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
            },
            payload,
            Some(clause_identity(&self.package, 0)),
            &self.letters(),
        )
    }
}

/// The letters of a clause with one `holds` atom, true exactly where every
/// object's goal flag is true.
struct AllTrue {
    subject: Subject,
    atom: FormulaNodeId,
}

impl AllTrue {
    fn new(subject: Subject, clause: &CheckedTemporalClause) -> Self {
        let atoms = translate(clause, 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        let [atom] = atoms.as_slice() else {
            panic!("one `holds` atom");
        };
        Self {
            subject,
            atom: *atom,
        }
    }
}

impl ModelLetters for AllTrue {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        atom: FormulaNodeId,
        _instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        assert_eq!(atom, self.atom, "the clause's own atom");
        let goal = self.subject.goal();
        Ok(
            if KEYS.iter().all(|key| self.subject.flag(state, key, goal)) {
                AtomReading::True
            } else {
                AtomReading::False
            },
        )
    }
}
