// SPDX-License-Identifier: AGPL-3.0-or-later
//! The subjects of ADR-021 §7.5 and §7.6 and FR-156-AC-3 for the
//! partial-order tests (`crate::partial_order`): a `test/por` domain package
//! of object types with boolean and bounded-integer fields, references and
//! receiver-scoped frames, a native unit that selects it as `M` with the
//! contracts of the subject, and the temporal clauses a test asks for.
//!
//! Each subject is a [`Fixture`]: the checked unit, one initial snapshot and
//! the universe of every population. The `holds` atoms of a clause are read
//! by the test's own closures over the state, and their footprints are
//! derived from the clause's checked conditions, so a test that spells an
//! atom's formula one way and its closure another disagrees with itself, not
//! with the engine.

use std::collections::{BTreeMap, BTreeSet};

use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest, Reductions};
use qsl_eval::automaton::translate;
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::{
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelTraceSettlement, ModelTraceSubject,
    TemporalCounterexample,
};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::{Cancel, Incomplete, Meter, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;
use crate::reduction_world::{clause_identity, replay_unit, ReplayUnit};

const PACKAGE: &str = "test/por";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const BOOLEAN: &str = "ix://quire/native/Boolean";

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

// ---------------------------------------------------------------------------
// The domain package
// ---------------------------------------------------------------------------

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn one() -> Json {
    json!({"lower": 1, "upper": 1, "ordered": false, "unique": true})
}

/// An object type with its fields and operations.
pub(crate) fn object_type(name: &str, fields: Vec<Json>, operations: Vec<Json>) -> Json {
    let identity = ident(name);
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
        "origin": origin(&identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": [], "fields": fields, "operations": operations, "relationships": [],
    })
}

/// A required field of `owner` of type `type_ref`.
pub(crate) fn field(owner: &str, name: &str, type_ref: &str) -> Json {
    field_of(owner, name, type_ref, "required")
}

/// An optional field of `owner` of type `type_ref`.
pub(crate) fn optional(owner: &str, name: &str, type_ref: &str) -> Json {
    field_of(owner, name, type_ref, "optional")
}

fn field_of(owner: &str, name: &str, type_ref: &str, presence: &str) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": presence,
        "nullable": false, "defaultKind": "none", "multiplicity": one(),
        "origin": origin(&identity),
    })
}

/// A boolean field of `owner`.
pub(crate) fn flag(owner: &str, name: &str) -> Json {
    field(owner, name, BOOLEAN)
}

/// A bounded integer type `0..=upper`.
pub(crate) fn level(name: &str, upper: u64) -> Json {
    let identity = ident(name);
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

/// The reference to the type `name` as a field's `typeRef`.
pub(crate) fn type_ref(name: &str) -> String {
    ident(name)
}

/// An operation of `owner` with no parameters, returning `returns` when it
/// names a type, with `frame`.
pub(crate) fn operation(owner: &str, name: &str, returns: Option<&str>, frame: &Json) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    let mut node = json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": frame,
    });
    if let Some(returns) = returns {
        node["returns"] = json!({"typeRef": returns, "nullable": false, "multiplicity": one()});
    }
    node
}

/// A frame whose receiver-scoped `modifies` names `fields` of `owner`.
pub(crate) fn writes(owner: &str, fields: &[&str]) -> Json {
    frame(&json!({"modifies_receiver":
        fields.iter().map(|name| ident(&format!("{owner}/{name}"))).collect::<Vec<_>>()}))
}

/// A frame from its members: the type-wide `modifies`, `creates` and
/// `deletes` entries and the receiver-scoped `modifies_receiver` and
/// `deletes_receiver`.
pub(crate) fn frame(members: &Json) -> Json {
    let mut frame = json!({"modifies": [], "creates": [], "deletes": []});
    for (name, value) in members.as_object().expect("frame members") {
        frame[name] = value.clone();
    }
    frame
}

/// The type-wide `modifies` entries `fields` of `owner`.
pub(crate) fn writes_any(owner: &str, fields: &[&str]) -> Json {
    frame(&json!({"modifies":
        fields.iter().map(|name| ident(&format!("{owner}/{name}"))).collect::<Vec<_>>()}))
}

fn document(types: Vec<Json>, populations: &[(&str, &str)]) -> Vec<u8> {
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .filter(|(name, _)| {
        *name == "population" || types.iter().any(|node| node["kind"]["name"] == *name)
    })
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let populations: Vec<Json> = populations
        .iter()
        .map(|(name, member)| {
            let identity = ident(name);
            json!({
                "identity": identity, "displayName": identity,
                "kind": {"module": PACKAGE, "name": "population"},
                "members": [ident(member)], "extent": "closed", "origin": origin(&identity),
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
        "constructs": constructs, "types": types, "populations": populations,
    })
    .to_string()
    .into_bytes()
}

// ---------------------------------------------------------------------------
// Snapshot values
// ---------------------------------------------------------------------------

/// One object of the initial snapshot.
pub(crate) struct Obj {
    pub(crate) population: &'static str,
    pub(crate) key: &'static str,
    pub(crate) object_type: &'static str,
    pub(crate) fields: Json,
}

pub(crate) fn integer(value: i64) -> Json {
    json!({"integer": value.to_string()})
}

pub(crate) fn boolean(value: bool) -> Json {
    json!({"boolean": value})
}

pub(crate) fn reference(population: &str, key: &str) -> Json {
    json!({"reference": {"population": ident(population), "key": key}})
}

pub(crate) fn present(value: &Json) -> Json {
    json!({"present": value})
}

pub(crate) fn absent() -> Json {
    json!({"absent": {}})
}

// ---------------------------------------------------------------------------
// The fixture
// ---------------------------------------------------------------------------

/// How one `holds` atom reads a state for the bound object's key: `Some`
/// truth, or `None` for an undefined evaluation.
pub(crate) type Atom = Box<dyn Fn(&ModelSimState, &str) -> Option<bool>>;

/// A checked unit over a `test/por` package, its initial snapshot and its
/// universes.
pub(crate) struct Fixture {
    package: CheckedPackage,
    clauses: Vec<CheckedTemporalClause>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: Vec<DocumentRef>,
    universes: Vec<(String, Vec<String>)>,
    digest: [u8; 32],
    document: Vec<u8>,
    text: String,
}

/// A temporal clause over `over`, `on origin`, with its `body`: the
/// fairness constraints then the formula.
pub(crate) struct Temporal<'t> {
    pub(crate) over: &'t str,
    pub(crate) body: &'t str,
}

impl Fixture {
    /// The package of `types` and `populations`, the unit with `contracts`
    /// and `clauses`, started from `objects`, with `universes` giving each
    /// population's keys.
    pub(crate) fn new(
        types: Vec<Json>,
        populations: &[(&str, &str)],
        contracts: &[&str],
        clauses: &[Temporal<'_>],
        objects: &[Obj],
        universes: &[(&str, &[&str])],
    ) -> Self {
        let document = document(types, populations);
        let packages = package_input([document.as_slice()]);
        let digest = *packages.keys().next().expect("one model");
        let declared = clauses
            .iter()
            .enumerate()
            .fold(String::new(), |text, (index, clause)| {
                text + &format!(
                    "temporal C{index} using t over ({}) clock \"model-steps\" \
                     on origin {{ {} }}\n",
                    clause.over, clause.body
                )
            });
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             profile t = \"quire.temporal.infinite-trace/v1\";\n\
             model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
             function Noop using v(): Boolean pure {{ true }}\n{}\n{declared}",
            hex(&digest),
            contracts.join("\n"),
        );
        let parsed = qsl_cst::parse(
            SourceIdentity::new("agent-ix", "por", "git", "1"),
            "por.native",
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
        let graph = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            built,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .with_unwritten_terminals()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
        let checked: Vec<CheckedTemporalClause> = graph
            .temporal_clauses()
            .iter()
            .map(|node| node.clause().clone())
            .collect();
        assert_eq!(checked.len(), clauses.len(), "{declared}");
        let mut fixture = Self {
            package: CheckedPackage::link(graph),
            clauses: checked,
            packages,
            snapshots: BTreeMap::new(),
            initial: Vec::new(),
            universes: universes
                .iter()
                .map(|(population, keys)| {
                    (
                        (*population).to_owned(),
                        keys.iter().map(|key| (*key).to_owned()).collect(),
                    )
                })
                .collect(),
            digest,
            document,
            text,
        };
        let populations: Vec<&str> = universes
            .iter()
            .map(|(population, _)| *population)
            .collect();
        let start = fixture.snapshot("s0", &populations, objects);
        fixture.initial.push(start);
        fixture
    }

    fn snapshot(&mut self, name: &str, populations: &[&str], objects: &[Obj]) -> DocumentRef {
        let listed: Vec<Json> = populations
            .iter()
            .map(|population| {
                let members: Vec<Json> = objects
                    .iter()
                    .filter(|object| object.population == *population)
                    .map(|object| {
                        json!({"key": object.key, "type": ident(object.object_type),
                            "fields": object.fields})
                    })
                    .collect();
                json!({"population": ident(population), "complete": true, "objects": members})
            })
            .collect();
        let header = json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&self.digest))});
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": listed,
        });
        let bytes = document.to_string().into_bytes();
        let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
        let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
            .expect("canonical fixture")
            .as_bytes();
        self.snapshots.insert(digest, bytes);
        DocumentRef {
            authority: "test".to_owned(),
            identity: name.to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest,
        }
    }

    fn universes(&self) -> Vec<PopulationUniverse> {
        self.universes
            .iter()
            .map(|(population, keys)| PopulationUniverse {
                population: key(population),
                keys: keys.clone(),
            })
            .collect()
    }

    /// The subject: the initial state and the universes.
    pub(crate) fn subject(&self) -> ModelSubject<'_> {
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

    /// The subject's model system.
    pub(crate) fn system(&self) -> ModelSystem<'_> {
        ModelSystem::new(self.subject().simulation).expect("the subject admits")
    }

    /// The initial state.
    pub(crate) fn start(&self) -> ModelSimState {
        self.system().initial()[0].clone()
    }

    /// Clause `index`.
    pub(crate) fn clause(&self, index: usize) -> &CheckedTemporalClause {
        &self.clauses[index]
    }

    /// The letters of clause `index`, its atoms read by `atoms`, and the
    /// atoms in the order the clause's automaton numbers them.
    pub(crate) fn letters(&self, index: usize, atoms: Vec<Atom>) -> (Letters, Vec<FormulaNodeId>) {
        let clause = &self.clauses[index];
        let nodes = translate(clause, 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        assert_eq!(nodes.len(), atoms.len(), "one predicate per `holds` atom");
        let system = self.system();
        let node = &self.package.graph().temporal_clauses()[index];
        let reads = nodes
            .iter()
            .map(|atom| {
                let (condition, _) = node.condition(*atom).expect("a `holds` atom");
                system.atom_reads(clause.over(), condition)
            })
            .collect();
        (
            Letters {
                nodes: nodes.clone(),
                atoms,
                reads,
            },
            nodes,
        )
    }

    /// The run of clause `clause` (`None`: the deadlock-freedom item), its
    /// atoms read by `atoms`, under `reductions`.
    pub(crate) fn run(
        &self,
        clause: Option<usize>,
        atoms: Vec<Atom>,
        reductions: Reductions,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        self.run_to(clause, atoms, reductions, None, ModelCheckLimits::default())
    }

    /// [`Self::run`] with a search horizon and limits.
    pub(crate) fn run_to(
        &self,
        clause: Option<usize>,
        atoms: Vec<Atom>,
        reductions: Reductions,
        max_depth: Option<usize>,
        limits: ModelCheckLimits,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let letters = clause.map(|index| self.letters(index, atoms).0);
        let item = match (clause, &letters) {
            (Some(index), Some(letters)) => ModelCheckItem::Clause {
                clause: &self.clauses[index],
                letters,
            },
            _ => ModelCheckItem::DeadlockFreedom,
        };
        check_model(
            ModelCheckRequest {
                subject: self.subject(),
                item,
                max_depth,
                limits,
                reductions,
            },
            &Cancel::new(),
        )
    }

    /// The model state at each position of `counterexample`, read by
    /// re-executing its steps from its initial state.
    pub(crate) fn positions(&self, counterexample: &TemporalCounterexample) -> Vec<ModelSimState> {
        let system = self.system();
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
                .expect("the recorded step is a subject step");
            states.push(current.clone());
        }
        states
    }

    /// FR-128: replay `payload`, a counterexample of clause `clause` (`None`:
    /// of the deadlock-freedom item), its atoms read by `atoms`.
    pub(crate) fn replay(
        &self,
        clause: Option<usize>,
        atoms: Vec<Atom>,
        payload: TemporalCounterexample,
    ) -> ModelTraceSettlement {
        let (letters, _) = match clause {
            Some(index) => self.letters(index, atoms),
            None => (
                Letters {
                    nodes: Vec::new(),
                    atoms: Vec::new(),
                    reads: Vec::new(),
                },
                Vec::new(),
            ),
        };
        replay_unit(
            &ReplayUnit {
                identity: "por",
                path: "por.native",
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
            clause.map(|index| clause_identity(&self.package, index)),
            &letters,
        )
    }
}

/// The letters of one clause: each `holds` atom, in the formula's node
/// order, read by its own closure, with the footprint derived from its
/// checked condition.
pub(crate) struct Letters {
    nodes: Vec<FormulaNodeId>,
    atoms: Vec<Atom>,
    reads: Vec<Result<BTreeSet<SymbolicLocation>, FootprintRefusal>>,
}

use qsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};

impl Letters {
    fn index(&self, atom: FormulaNodeId) -> usize {
        self.nodes
            .iter()
            .position(|node| *node == atom)
            .expect("the engine reads only the clause's atoms")
    }
}

impl ModelLetters for Letters {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(match (self.atoms[self.index(atom)])(state, &instance.key) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )),
        })
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        self.reads[self.index(atom)].clone()
    }
}

// ---------------------------------------------------------------------------
// Reading a state
// ---------------------------------------------------------------------------

/// The field `field` of `owner`'s object `object` in `population` at
/// `state`; `None` when the state holds no such object.
pub(crate) fn read<'s>(
    state: &'s ModelSimState,
    population: &str,
    object: &str,
    field: &str,
) -> Option<&'s Value> {
    state
        .state()
        .populations
        .get(&key(population))?
        .get(object)?
        .fields
        .get(&key(field))
}

/// A boolean field, `None` when the object is gone.
pub(crate) fn truth(
    state: &ModelSimState,
    population: &str,
    object: &str,
    field: &str,
) -> Option<bool> {
    match read(state, population, object, field)? {
        Value::Boolean(value) => Some(*value),
        other => panic!("{object}.{field} is not a boolean: {other:?}"),
    }
}

/// An integer field, `None` when the object is gone.
pub(crate) fn number(
    state: &ModelSimState,
    population: &str,
    object: &str,
    field: &str,
) -> Option<i64> {
    match read(state, population, object, field)? {
        Value::Integer(value) => Some(value.to_string().parse().expect("a small value")),
        other => panic!("{object}.{field} is not an integer: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The subjects
// ---------------------------------------------------------------------------

/// Where §7.5's `Gate` guards `go`: in the precondition, or in the
/// postcondition (§7.6's second vector).
#[derive(Clone, Copy)]
pub(crate) enum Guard {
    Pre,
    Post,
}

/// FR-157's `Gate`: `a`, `c`, `d` and `done`, one object `g` all false, and
/// operations `arm`, `arm2`, `go` and `toggle`, each with a receiver-scoped
/// frame; `spawn` (framed `creates Gate`) when `spawn` is set, over the
/// universe `{g}` or `{g, h}`. `fairness` is written before the formula of
/// each of `claims`.
pub(crate) fn gate(guard: Guard, spawn: bool, claims: &[&str]) -> Fixture {
    gate_with(guard, spawn, false, claims)
}

/// [`gate`] with `setd()` (framed `modifies self.d`, pre `not self.d`) when
/// `setd` is set: it reads nothing `arm` writes, and writes what `arm2`'s
/// and `go`'s enablings read.
pub(crate) fn gate_with(guard: Guard, spawn: bool, setd: bool, claims: &[&str]) -> Fixture {
    let mut operations = vec![
        operation("Gate", "arm", None, &writes("Gate", &["c"])),
        operation("Gate", "arm2", None, &writes("Gate", &["d"])),
        operation("Gate", "go", None, &writes("Gate", &["done"])),
        operation("Gate", "toggle", None, &writes("Gate", &["a"])),
    ];
    if setd {
        operations.push(operation("Gate", "setd", None, &writes("Gate", &["d"])));
    }
    if spawn {
        operations.push(operation(
            "Gate",
            "spawn",
            None,
            &frame(&json!({"creates": [ident("Gate")]})),
        ));
    }
    let go = match guard {
        Guard::Pre => {
            "pre GoP using v on M::Gate::go { self.c and self.d and not self.done }\n\
             post GoQ using v on M::Gate::go { self.done }"
        }
        Guard::Post => {
            "pre GoP using v on M::Gate::go { not self.done }\n\
             post GoQ using v on M::Gate::go { self.done and pre(self.c) and pre(self.d) }"
        }
    };
    let mut contracts = vec![
        "pre ArmP using v on M::Gate::arm { not self.c }",
        "post ArmQ using v on M::Gate::arm { self.c }",
        "pre Arm2P using v on M::Gate::arm2 { self.c and not self.d }",
        "post Arm2Q using v on M::Gate::arm2 { self.d }",
        go,
        "post ToggleQ using v on M::Gate::toggle { self.a = not pre(self.a) }",
    ];
    if setd {
        contracts.push("pre SetdP using v on M::Gate::setd { not self.d }");
        contracts.push("post SetdQ using v on M::Gate::setd { self.d }");
    }
    let clauses: Vec<Temporal<'_>> = claims
        .iter()
        .map(|body| Temporal {
            over: "x: M::Gate",
            body,
        })
        .collect();
    let keys: &[&str] = if spawn { &["g", "h"] } else { &["g"] };
    Fixture::new(
        vec![object_type(
            "Gate",
            vec![
                flag("Gate", "a"),
                flag("Gate", "c"),
                flag("Gate", "d"),
                flag("Gate", "done"),
            ],
            operations,
        )],
        &[("gates", "Gate")],
        &contracts,
        &clauses,
        &[Obj {
            population: "gates",
            key: "g",
            object_type: "Gate",
            fields: json!({"a": boolean(false), "c": boolean(false), "d": boolean(false),
                "done": boolean(false)}),
        }],
        &[("gates", keys)],
    )
}

/// `(a, c, d, done)` of `g` at `state`.
pub(crate) fn gate_state(state: &ModelSimState) -> (bool, bool, bool, bool) {
    let flag = |field: &str| truth(state, "gates", "g", &format!("Gate/{field}")).expect("g");
    (flag("a"), flag("c"), flag("d"), flag("done"))
}

/// FR-156-AC-3's `Ring` and `Counter`: object `r` of `Ring` (`v: Int[0, 2]`,
/// `step()` framed `modifies self.v`, wrapping at 2) and object `q` of
/// `Counter` (`v: Int[0, 2]`, `bump()` below 2), both at 0.
pub(crate) fn ring(claims: &[&str]) -> Fixture {
    let clauses: Vec<Temporal<'_>> = claims
        .iter()
        .map(|body| Temporal {
            over: "x: M::Counter",
            body,
        })
        .collect();
    let level = type_ref("Level");
    Fixture::new(
        vec![
            level_type(),
            object_type(
                "Ring",
                vec![field("Ring", "v", &level)],
                vec![operation("Ring", "step", None, &writes("Ring", &["v"]))],
            ),
            object_type(
                "Counter",
                vec![field("Counter", "v", &level)],
                vec![operation(
                    "Counter",
                    "bump",
                    None,
                    &writes("Counter", &["v"]),
                )],
            ),
        ],
        &[("rings", "Ring"), ("counters", "Counter")],
        &[
            "post Wrap using v on M::Ring::step { \
             (pre(self.v) < 2 and self.v - pre(self.v) = 1) or (pre(self.v) = 2 and self.v = 0) }",
            "pre Below using v on M::Counter::bump { self.v < 2 }",
            "post Up using v on M::Counter::bump { self.v - pre(self.v) = 1 }",
        ],
        &clauses,
        &[
            Obj {
                population: "rings",
                key: "r",
                object_type: "Ring",
                fields: json!({"v": integer(0)}),
            },
            Obj {
                population: "counters",
                key: "q",
                object_type: "Counter",
                fields: json!({"v": integer(0)}),
            },
        ],
        &[("rings", &["r"]), ("counters", &["q"])],
    )
}

fn level_type() -> Json {
    level("Level", 2)
}

/// `(r.v, q.v)` of the ring subject at `state`.
pub(crate) fn ring_state(state: &ModelSimState) -> (i64, i64) {
    (
        number(state, "rings", "r", "Ring/v").expect("r"),
        number(state, "counters", "q", "Counter/v").expect("q"),
    )
}

// ---------------------------------------------------------------------------
// ADR-021 §7.6's vectors
// ---------------------------------------------------------------------------

fn bit() -> Json {
    level("Bit", 1)
}

/// A gate over cells: the object types and clauses a vector writes. `fields`
/// are the gate's fields beyond `armed` and `alarm`.
fn gate_cells(
    gate_fields: Vec<Json>,
    gate_operations: Vec<Json>,
    cell_operations: Vec<Json>,
    contracts: &[&str],
    objects: &[Obj],
    claim: &str,
) -> Fixture {
    let mut fields = vec![flag("Gate", "armed"), flag("Gate", "alarm")];
    fields.extend(gate_fields);
    let bit_ref = type_ref("Bit");
    Fixture::new(
        vec![
            bit(),
            object_type("Cell", vec![field("Cell", "v", &bit_ref)], cell_operations),
            object_type("Gate", fields, gate_operations),
        ],
        &[("cells", "Cell"), ("gates", "Gate")],
        contracts,
        &[Temporal {
            over: "x: M::Gate",
            body: claim,
        }],
        objects,
        &[("cells", &["a", "b"]), ("gates", &["g"])],
    )
}

fn set() -> Json {
    operation("Cell", "set", None, &writes("Cell", &["v"]))
}

fn cell(key: &'static str, v: i64) -> Obj {
    Obj {
        population: "cells",
        key,
        object_type: "Cell",
        fields: json!({"v": integer(v)}),
    }
}

/// `not g.alarm`, read of the bound gate.
pub(crate) fn no_alarm() -> Vec<Atom> {
    vec![Box::new(|state, key| {
        truth(state, "gates", key, "Gate/alarm").map(|alarm| !alarm)
    })]
}

/// §7.6's guard in a postcondition: `boom()` is enabled by `armed`, and its
/// postcondition reads `pre(self.cell.v)`; refuted by `set, boom`.
pub(crate) fn guard_in_postcondition() -> Fixture {
    gate_cells(
        vec![field("Gate", "cell", &type_ref("Cell"))],
        vec![
            operation("Gate", "disarm", None, &writes("Gate", &["armed"])),
            operation("Gate", "boom", None, &writes("Gate", &["alarm"])),
        ],
        vec![set()],
        &[
            "post Disarmed using v on M::Gate::disarm { not self.armed }",
            "pre BoomArmed using v on M::Gate::boom { self.armed }",
            "post BoomRaised using v on M::Gate::boom { self.alarm and pre(self.cell.v) = 1 }",
            "pre SetZero using v on M::Cell::set { self.v = 0 }",
            "post SetOne using v on M::Cell::set { self.v = 1 }",
        ],
        &[
            cell("a", 0),
            Obj {
                population: "gates",
                key: "g",
                object_type: "Gate",
                fields: json!({"armed": boolean(true), "alarm": boolean(false),
                    "cell": reference("cells", "a")}),
            },
        ],
        "always holds(not x.alarm)",
    )
}

/// §7.6's navigation through a rewritten reference: `point()` repoints
/// `cell` at `alt`, and `boom()` reads `self.cell.v`; refuted by `point,
/// boom`.
pub(crate) fn navigation() -> Fixture {
    gate_cells(
        vec![
            field("Gate", "cell", &type_ref("Cell")),
            field("Gate", "alt", &type_ref("Cell")),
        ],
        vec![
            operation("Gate", "disarm", None, &writes("Gate", &["armed"])),
            operation("Gate", "point", None, &writes("Gate", &["cell"])),
            operation("Gate", "boom", None, &writes("Gate", &["alarm"])),
        ],
        Vec::new(),
        &[
            "post Disarmed using v on M::Gate::disarm { not self.armed }",
            "pre PointMoves using v on M::Gate::point { self.cell != self.alt }",
            "post PointTo using v on M::Gate::point { self.cell = pre(self.alt) }",
            "pre BoomNav using v on M::Gate::boom { self.armed and self.cell.v = 1 }",
            "post BoomRaised using v on M::Gate::boom { self.alarm }",
        ],
        &[
            cell("a", 0),
            cell("b", 1),
            Obj {
                population: "gates",
                key: "g",
                object_type: "Gate",
                fields: json!({"armed": boolean(true), "alarm": boolean(false),
                    "cell": reference("cells", "a"), "alt": reference("cells", "b")}),
            },
        ],
        "always holds(not x.alarm)",
    )
}

/// §7.6's postcondition reading through `result`: `pick(): Reference<Cell>`
/// is enabled only when some cell has `v = 1`; refuted by `set, pick`.
pub(crate) fn result_read() -> Fixture {
    let cell_ref = type_ref("Cell");
    gate_cells(
        Vec::new(),
        vec![
            operation("Gate", "disarm", None, &writes("Gate", &["armed"])),
            operation("Gate", "pick", Some(&cell_ref), &writes("Gate", &["alarm"])),
        ],
        vec![set()],
        &[
            "post Disarmed using v on M::Gate::disarm { not self.armed }",
            "pre PickArmed using v on M::Gate::pick { self.armed }",
            "post PickRaised using v on M::Gate::pick { self.alarm and result.v = 1 }",
            "pre SetZero using v on M::Cell::set { self.v = 0 }",
            "post SetOne using v on M::Cell::set { self.v = 1 }",
        ],
        &[
            cell("a", 0),
            Obj {
                population: "gates",
                key: "g",
                object_type: "Gate",
                fields: json!({"armed": boolean(true), "alarm": boolean(false)}),
            },
        ],
        "always holds(not x.alarm)",
    )
}

/// The deletion vectors' subject: `Target` (`bad`, `cancellable`, `x`) in
/// population `targets`, which the claim ranges over so that its one
/// instance is the one object the vector names, and `Job` (`armed`,
/// `cancellable` and the optional references `peer` to a target and `target`
/// to a job) in population `jobs`, with the operations and contracts a
/// vector chooses.
fn deletions(
    target_operations: Vec<Json>,
    job_operations: Vec<Json>,
    contracts: &[&str],
    objects: &[Obj],
    claim: &str,
) -> Fixture {
    Fixture::new(
        vec![
            level("Bit", 1),
            object_type(
                "Target",
                vec![
                    flag("Target", "bad"),
                    flag("Target", "cancellable"),
                    field("Target", "x", &type_ref("Bit")),
                ],
                target_operations,
            ),
            object_type(
                "Job",
                vec![
                    flag("Job", "armed"),
                    flag("Job", "cancellable"),
                    optional("Job", "peer", &type_ref("Target")),
                    optional("Job", "target", &type_ref("Job")),
                ],
                job_operations,
            ),
        ],
        &[("targets", "Target"), ("jobs", "Job")],
        contracts,
        &[Temporal {
            over: "x: M::Target",
            body: claim,
        }],
        objects,
        &[("targets", &["k"]), ("jobs", &["j", "m"])],
    )
}

fn target(cancellable: bool) -> Obj {
    Obj {
        population: "targets",
        key: "k",
        object_type: "Target",
        fields: json!({"bad": boolean(false), "cancellable": boolean(cancellable),
            "x": integer(0)}),
    }
}

fn job(key: &'static str, armed: bool, cancellable: bool, peer: bool, target: Option<&str>) -> Obj {
    Obj {
        population: "jobs",
        key,
        object_type: "Job",
        fields: json!({"armed": boolean(armed), "cancellable": boolean(cancellable),
        "peer": if peer { present(&reference("targets", "k")) } else { absent() },
        "target": match target {
            Some(to) => present(&reference("jobs", to)),
            None => absent(),
        }}),
    }
}

/// `not x.bad`, read of the bound target.
pub(crate) fn not_bad() -> Vec<Atom> {
    vec![Box::new(|state, key| {
        truth(state, "targets", key, "Target/bad").map(|bad| !bad)
    })]
}

/// `x.x = 0`, read of the bound target; undefined once the target is
/// deleted.
pub(crate) fn zero_x() -> Vec<Atom> {
    vec![Box::new(|state, key| {
        number(state, "targets", key, "Target/x").map(|x| x == 0)
    })]
}

/// §7.6's deleting-the-receiver vector: `boom()` (every-object `modifies
/// bad`) reads `self.peer.bad`, `cancel()` deletes its receiver. Job `j`
/// (armed and cancellable, `peer = k`) and target `k`; refuted by `boom(j)`.
pub(crate) fn deleting_the_receiver() -> Fixture {
    deletions(
        Vec::new(),
        vec![
            operation("Job", "boom", None, &writes_any("Target", &["bad"])),
            operation(
                "Job",
                "cancel",
                None,
                &frame(&json!({"deletes_receiver": true})),
            ),
        ],
        &[
            "pre BoomArmed using v on M::Job::boom { self.armed }",
            "post BoomBad using v on M::Job::boom { present(self.peer) and value(self.peer).bad }",
            "pre Cancellable using v on M::Job::cancel { self.cancellable }",
        ],
        &[job("j", true, true, true, None), target(false)],
        "always holds(not x.bad)",
    )
}

/// §7.6's blocked-delete vector: target `k` (cancellable) has `lock()`
/// (clears `cancellable`) and `cancel()` (deletes its receiver), and job `j`
/// (`peer = k`) has `unlink()` (clears `peer`). FR-120 admits no post-state
/// with a dangling reference, so `cancel(k)` is enabled only after
/// `unlink(j)`; refuted by `unlink, cancel` with an undefined evaluation once
/// `k` is deleted.
pub(crate) fn blocked_delete() -> Fixture {
    deletions(
        vec![
            operation("Target", "lock", None, &writes("Target", &["cancellable"])),
            operation(
                "Target",
                "cancel",
                None,
                &frame(&json!({"deletes_receiver": true})),
            ),
        ],
        vec![operation("Job", "unlink", None, &writes("Job", &["peer"]))],
        &[
            "pre LockPre using v on M::Target::lock { self.cancellable }",
            "post LockPost using v on M::Target::lock { not self.cancellable }",
            "pre CancelPre using v on M::Target::cancel { self.cancellable }",
            "pre UnlinkPre using v on M::Job::unlink { present(self.peer) }",
            "post UnlinkPost using v on M::Job::unlink { not present(self.peer) }",
        ],
        &[job("j", false, false, true, None), target(true)],
        "always holds(x.x = 0)",
    )
}

/// §7.6's type-wide-delete vector: the previous vector with a job `m`
/// (cancellable, `target = j`) whose `cancel()` is framed `deletes Job` and
/// `modifies self.target`. A type-wide `deletes` fixes no deleted key, so
/// `cancel(m)` writes the membership of any job; refuted by `boom(j)`. It has
/// no postcondition, so it does not pin which job is deleted or what
/// `target` becomes: S3 accepts a population name only in a refinement, so
/// the ADR's "every other job stays" cannot be written as a state clause yet,
/// and `cancel(m)` may delete any set of jobs and leave `target` anywhere.
pub(crate) fn deleting_any_job() -> Fixture {
    deletions(
        Vec::new(),
        vec![
            operation("Job", "boom", None, &writes_any("Target", &["bad"])),
            operation(
                "Job",
                "cancel",
                None,
                &frame(&json!({"deletes": [ident("Job")],
                    "modifies_receiver": [ident("Job/target")]})),
            ),
        ],
        &[
            "pre BoomArmed using v on M::Job::boom { self.armed }",
            "post BoomBad using v on M::Job::boom { present(self.peer) and value(self.peer).bad }",
            "pre CancelPre using v on M::Job::cancel { self.cancellable and present(self.target) }",
        ],
        &[
            job("j", true, false, true, None),
            job("m", false, true, false, Some("j")),
            target(false),
        ],
        "always holds(not x.bad)",
    )
}

// ---------------------------------------------------------------------------
// Transitions over objects no state holds yet (FR-155-AC-7, FR-162)
// ---------------------------------------------------------------------------

/// An operation of `owner` over `(name, type)` reference parameters.
fn operation_over(owner: &str, name: &str, params: &[(&str, &str)], frame: &Json) -> Json {
    let mut node = operation(owner, name, None, frame);
    let identity = ident(&format!("{owner}/{name}"));
    node["params"] = json!(params
        .iter()
        .map(|(parameter, of)| {
            let parameter_identity = format!("{identity}/{parameter}");
            json!({
                "identity": parameter_identity, "name": parameter, "typeRef": type_ref(of),
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": one(), "origin": origin(&parameter_identity),
            })
        })
        .collect::<Vec<_>>());
    node
}

/// `x` of `Job` (`cancel()` deletes its receiver), `y` of `Maker` (`spawn()`
/// creates an `Item`; `spin()`; `probe(it, j)` over an item and a job) and the
/// `Item` universe `{d}`, which starts empty. `probe(y, d, x)` reads the
/// membership of `d` and of `x`, though `d` is held by no state until `spawn`
/// creates it. `Item` has `touch()`.
pub(crate) fn makers(claims: &[&str]) -> Fixture {
    let clauses: Vec<Temporal<'_>> = claims
        .iter()
        .map(|body| Temporal {
            over: "x: M::Maker",
            body,
        })
        .collect();
    Fixture::new(
        vec![
            object_type(
                "Job",
                vec![flag("Job", "f")],
                vec![operation(
                    "Job",
                    "cancel",
                    None,
                    &frame(&json!({"deletes_receiver": true})),
                )],
            ),
            object_type(
                "Maker",
                vec![flag("Maker", "f"), flag("Maker", "g")],
                vec![
                    operation(
                        "Maker",
                        "spawn",
                        None,
                        &frame(&json!({"creates": [ident("Item")]})),
                    ),
                    operation("Maker", "spin", None, &writes("Maker", &["g"])),
                    operation_over(
                        "Maker",
                        "probe",
                        &[("it", "Item"), ("j", "Job")],
                        &writes("Maker", &["f"]),
                    ),
                ],
            ),
            object_type(
                "Item",
                vec![flag("Item", "h")],
                vec![operation("Item", "touch", None, &writes("Item", &["h"]))],
            ),
        ],
        &[("jobs", "Job"), ("makers", "Maker"), ("items", "Item")],
        &[
            "pre SpinP using v on M::Maker::spin { not self.f }",
            "post SpinQ using v on M::Maker::spin { self.g = not pre(self.g) }",
            "pre ProbeP using v on M::Maker::probe { not self.f }",
            "post ProbeQ using v on M::Maker::probe { self.f }",
            "pre TouchP using v on M::Item::touch { not self.h }",
            "post TouchQ using v on M::Item::touch { self.h }",
        ],
        &clauses,
        &[
            Obj {
                population: "jobs",
                key: "x",
                object_type: "Job",
                fields: json!({"f": boolean(false)}),
            },
            Obj {
                population: "makers",
                key: "y",
                object_type: "Maker",
                fields: json!({"f": boolean(false), "g": boolean(false)}),
            },
        ],
        &[("jobs", &["x"]), ("makers", &["y"]), ("items", &["d"])],
    )
}

/// `not x.f`, read of the bound maker.
pub(crate) fn no_f() -> Vec<Atom> {
    vec![Box::new(|state, key| {
        truth(state, "makers", key, "Maker/f").map(|f| !f)
    })]
}

/// A maker whose `spin()` has a function call in its precondition, which
/// FR-383 gives no footprint rule: unreduced the claim `always holds(not
/// x.f)` is refuted by `set()`.
pub(crate) fn call_in_a_precondition() -> Fixture {
    Fixture::new(
        vec![object_type(
            "Maker",
            vec![flag("Maker", "f"), flag("Maker", "g")],
            vec![
                operation("Maker", "spin", None, &writes("Maker", &["g"])),
                operation("Maker", "set", None, &writes("Maker", &["f"])),
            ],
        )],
        &[("makers", "Maker")],
        &[
            "pre SpinP using v on M::Maker::spin { Noop() }",
            "post SpinQ using v on M::Maker::spin { self.g = not pre(self.g) }",
            "post SetQ using v on M::Maker::set { self.f }",
        ],
        &[Temporal {
            over: "x: M::Maker",
            body: "always holds(not x.f)",
        }],
        &[Obj {
            population: "makers",
            key: "y",
            object_type: "Maker",
            fields: json!({"f": boolean(false), "g": boolean(false)}),
        }],
        &[("makers", &["y"])],
    )
}
