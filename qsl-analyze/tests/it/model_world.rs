// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model subjects for the EN-1 safety tests (`crate::model_check_safety`):
//! a domain package with one population, a native unit selecting it as `M`
//! with its contracts and temporal clauses, the initial snapshots, and the
//! letters of each clause's `holds` atoms.
//!
//! Two domain packages are used: FR-124-AC-1's `test/counter`
//! (`crate::counter`'s document) and FR-126-AC-4's `test/health`, written
//! here.

use std::collections::BTreeMap;

use qsl_eval::automaton::translate;
use qsl_eval::model_subject::{ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::TemporalCounterexample;
use qsl_semantics::check::{check_clauses, CheckedTemporalClause, FormulaNodeId, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{
    declared_type_universe_for, DocumentRef, ObservationLimits,
};
use quire_exact::{Incomplete, Meter, ObjectId, ObjectReference, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

/// The infinite-trace profile.
pub(crate) const INFINITE: &str = "quire.temporal.infinite-trace/v1";
/// The bounded event-position false-extension profile.
pub(crate) const BOUNDED: &str = "quire.temporal.event-position.false-extension/v1";

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// A domain package with one population of one object type.
pub(crate) struct Domain {
    /// The package identity.
    pub(crate) package: &'static str,
    /// The object type's name.
    pub(crate) object: &'static str,
    /// The population's name.
    pub(crate) population: &'static str,
    /// The package document.
    pub(crate) document: Vec<u8>,
}

impl Domain {
    fn node(&self, suffix: &str) -> String {
        format!("ix://{}/{suffix}", self.package)
    }

    fn key(&self, suffix: &str) -> DeclarationKey {
        DeclarationKey {
            package: self.package.to_owned(),
            node: self.node(suffix),
        }
    }

    /// The field `name` of the object `object` at `state`.
    pub(crate) fn field<'s>(&self, state: &'s ModelSimState, object: &str, name: &str) -> &'s Value {
        state.state().populations[&self.key(self.population)][object]
            .fields
            .get(&self.key(&format!("{}/{name}", self.object)))
            .unwrap_or_else(|| panic!("{object}.{name} is set"))
    }

    /// The integer field `name` of `object` at `state`.
    pub(crate) fn integer(&self, state: &ModelSimState, object: &str, name: &str) -> i64 {
        match self.field(state, object, name) {
            Value::Integer(value) => value.to_string().parse().expect("a small value"),
            other => panic!("{object}.{name} is not an integer: {other:?}"),
        }
    }

    /// The Boolean field `name` of `object` at `state`.
    pub(crate) fn boolean(&self, state: &ModelSimState, object: &str, name: &str) -> bool {
        match self.field(state, object, name) {
            Value::Boolean(value) => *value,
            other => panic!("{object}.{name} is not a Boolean: {other:?}"),
        }
    }
}

/// FR-124-AC-1's `Counter` package: `value: Int[0, 3]`, operation `inc`.
pub(crate) fn counter() -> Domain {
    Domain {
        package: "test/counter",
        object: "Counter",
        population: "counters",
        document: crate::counter::document(false),
    }
}

/// FR-126-AC-4's `Health` package: object `Health` with `healthy: Boolean`
/// and `failures: Int[0, 1]`, operation `fail` modifying both and `recover`
/// modifying `healthy`, population `systems`.
pub(crate) fn health() -> Domain {
    let package = "test/health";
    let ident = |suffix: &str| format!("ix://{package}/{suffix}");
    let origin = |identity: &str| {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    };
    let field = |name: &str, type_ref: &str| {
        let identity = ident(&format!("Health/{name}"));
        json!({
            "identity": identity, "name": name, "typeRef": type_ref,
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin(&identity),
        })
    };
    let operation = |name: &str, modifies: &[&str]| {
        json!({
            "identity": ident(&format!("Health/{name}")), "name": name, "params": [],
            "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": modifies.iter().map(|field| ident(&format!("Health/{field}")))
                .collect::<Vec<_>>(), "creates": [], "deletes": []},
        })
    };
    let wire_type = |identity: &str, kind: &str, extra: Json| {
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
    };
    let failures = ident("Failures");
    let bound = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{failures}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&failures),
        })
    };
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
    let object = ident("Health");
    let population = ident("systems");
    let document = json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": package, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            wire_type(&failures, "value_type", json!({"scalar": "integer",
                "constraints": [bound("min", "0"), bound("max", "1")]})),
            wire_type(&object, "object_type", json!({
                "supertypes": [],
                "fields": [field("healthy", "ix://quire/native/Boolean"),
                    field("failures", &failures)],
                "operations": [operation("fail", &["healthy", "failures"]),
                    operation("recover", &["healthy"])],
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": package, "name": "population"},
            "members": [object], "extent": "closed", "origin": origin(&population),
        }],
    });
    Domain {
        package,
        object: "Health",
        population: "systems",
        document: document.to_string().into_bytes(),
    }
}

/// One temporal clause over `c` of the domain's object type: its activation
/// and formula.
pub(crate) struct Clause<'c> {
    pub(crate) activation: &'c str,
    pub(crate) formula: &'c str,
}

/// A checked package over a domain, its clauses, and the snapshots of its
/// initial states.
pub(crate) struct World {
    pub(crate) domain: Domain,
    pub(crate) package: CheckedPackage,
    pub(crate) clauses: Vec<CheckedTemporalClause>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    pub(crate) initial: Vec<DocumentRef>,
}

impl World {
    /// The unit selecting `domain` as `M` with `body` (its contracts) and
    /// `clauses` under the temporal profile `profile` (a unit selects one),
    /// from one initial snapshot of the objects `start`, each `(key,
    /// fields)`.
    pub(crate) fn new(
        domain: Domain,
        body: &str,
        profile: &str,
        clauses: &[Clause<'_>],
        start: &[(&str, Json)],
    ) -> Self {
        let packages = package_input([domain.document.as_slice()]);
        let digest = *packages.keys().next().expect("one model");
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             profile t = \"{profile}\";\n\
             model M = \"{}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n{body}",
            domain.package,
            hex(&digest)
        );
        let declared = clauses
            .iter()
            .enumerate()
            .fold(String::new(), |text, (index, clause)| {
                text + &format!(
                    "temporal C{index} using t over (c: M::{}) clock \"model-steps\" {} {{ {} }}\n",
                    domain.object, clause.activation, clause.formula
                )
            });
        let declarations = |text: &str| {
            let parsed = qsl_cst::parse(
                SourceIdentity::new("agent-ix", "safety", "git", "1"),
                "safety.native",
                text.as_bytes(),
                qsl_cst::Limits::default(),
            )
            .expect("S1 parses the unit");
            assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
            let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
            let models = admit_unit(
                &built.selections().models,
                &packages,
                ModelNormalizationLimits::default(),
            )
            .expect("the model selection admits")
            .with_parsed_operation_clauses(|text| {
                qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
            })
            .expect("the operation clauses parse");
            PackageDeclarations::assemble(
                parsed.source().reference().clone(),
                built,
                models,
                Vec::new(),
            )
            .expect("the unit assembles")
        };
        // S4 emits no `on each` clause (FR-337), so the package check refuses
        // one: a world with one checks its clauses by the family alone, and
        // its package holds none of them.
        let emitted = clauses.iter().all(|clause| clause.activation == "on origin");
        let unit = if emitted {
            format!("{text}{declared}")
        } else {
            text.clone()
        };
        let graph = declarations(&unit)
            .with_unwritten_terminals()
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{unit}: the unit checks: {refusals:?}"));
        let checked: Vec<CheckedTemporalClause> = if emitted {
            graph
                .temporal_clauses()
                .iter()
                .map(|node| node.clause().clone())
                .collect()
        } else {
            check_clauses(&declarations(&format!("{text}{declared}")))
                .into_iter()
                .map(|clause| clause.unwrap_or_else(|why| panic!("{declared}: {why:?}")))
                .collect()
        };
        assert_eq!(checked.len(), clauses.len(), "{declared}");
        let mut world = Self {
            domain,
            package: CheckedPackage::link(graph),
            clauses: checked,
            packages,
            snapshots: BTreeMap::new(),
            initial: Vec::new(),
        };
        let start = world.snapshot(start);
        world.initial.push(start);
        world
    }

    /// An initialization snapshot of `objects`, provisioned here.
    fn snapshot(&mut self, objects: &[(&str, Json)]) -> DocumentRef {
        let object_type = self.domain.node(self.domain.object);
        let objects: Vec<Json> = objects
            .iter()
            .map(|(key, fields)| json!({"key": key, "type": object_type, "fields": fields}))
            .collect();
        let digest = *self.packages.keys().next().expect("one model");
        let header = json!({"identity": self.domain.package, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&digest))});
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": self.domain.node(self.domain.population),
                "complete": true, "objects": objects}],
        });
        let bytes = document.to_string().into_bytes();
        let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
        let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
            .expect("canonical fixture")
            .as_bytes();
        self.snapshots.insert(digest, bytes);
        DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest,
        }
    }

    /// The subject from the initial snapshot with universe `keys`.
    pub(crate) fn subject(&self, keys: &[&str], meter: ScalarLimits) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes: vec![PopulationUniverse {
                    population: self.domain.key(self.domain.population),
                    keys: keys.iter().map(|key| (*key).to_owned()).collect(),
                }],
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter,
            },
            proof_bounds: Vec::new(),
        }
    }

    /// The reference of `object` in the package's universe rule.
    pub(crate) fn reference(&self, object: &str) -> ObjectReference {
        let (effective, universe) = declared_type_universe_for(
            self.package.graph().model_selections(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &self.domain.key(self.domain.object),
        )
        .expect("the universe derives")
        .expect("the object type is declared");
        ObjectReference::new(universe, effective, ObjectId::new(object).expect("an id"))
    }

    /// The model state at each position of `counterexample`, read by
    /// re-executing its steps: each step's post-state is the successor of its
    /// transition identity whose state-key digest it records.
    pub(crate) fn positions(
        &self,
        keys: &[&str],
        counterexample: &TemporalCounterexample,
    ) -> Vec<ModelSimState> {
        let system = ModelSystem::new(self.subject(keys, crate::counter::UNLIMITED).simulation)
            .expect("the subject admits");
        let mut current = system.initial()[counterexample.initial_state].clone();
        let mut states = vec![current.clone()];
        for step in &counterexample.prefix {
            let expansion = system.successors(&current).expect("the state expands");
            current = expansion
                .successors
                .into_iter()
                .find(|(transition, state)| {
                    *transition == step.transition
                        && system.state_key(state).expect("encodes").1 == step.post_state
                })
                .map(|(_, state)| state)
                .expect("the recorded step is a successor");
            states.push(current.clone());
        }
        states
    }
}

/// One `holds` atom's reading of the bound object at a state: `Some(truth)`,
/// or `None` for an undefined division by zero.
pub(crate) type Atom = Box<dyn Fn(&ModelSimState, &str) -> Option<bool>>;

/// The letters of one clause: each `holds` atom, in the formula's node
/// order, read by its own predicate.
pub(crate) struct Letters {
    bindings: Vec<ObjectReference>,
    holds: Vec<FormulaNodeId>,
    atoms: Vec<Atom>,
}

impl Letters {
    /// `clause`'s letters over the instances `bindings`, its `holds` atoms
    /// read in node order by `atoms`. The atoms' node order is the
    /// translation's letter order (`PropertyAutomaton::atoms`).
    pub(crate) fn new(
        clause: &CheckedTemporalClause,
        bindings: Vec<ObjectReference>,
        atoms: Vec<Atom>,
    ) -> Self {
        let holds = translate(clause, 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        assert_eq!(holds.len(), atoms.len(), "one predicate per `holds` atom");
        Self {
            bindings,
            holds,
            atoms,
        }
    }
}

impl ModelLetters for Letters {
    fn bindings(&self) -> Vec<ObjectReference> {
        self.bindings.clone()
    }

    fn atom(
        &self,
        atom: FormulaNodeId,
        binding: &ObjectReference,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let index = self
            .holds
            .iter()
            .position(|node| *node == atom)
            .expect("the engine reads only the clause's atoms");
        Ok(match (self.atoms[index])(state, binding.object().as_str()) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )),
        })
    }
}
