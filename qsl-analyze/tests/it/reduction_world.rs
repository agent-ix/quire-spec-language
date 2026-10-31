// SPDX-License-Identifier: AGPL-3.0-or-later
//! The subjects of ADR-021's worked examples for the reduction tests
//! (`crate::reduction`): a `test/sym` domain package with object type
//! `Counter` (`v: Int[0, max]`, optionally `link: Counter?`), operation
//! `bump` whose frame is scoped to its receiver (`modifies_receiver v`), and
//! population `counters`, optionally declared `symmetric`. A second
//! population, `spares` of the field-less `Spare`, is declared `symmetric`
//! and never has a universe.
//!
//! Each unit is compiled through S1 to S4 as `crate::units`'s are, and its
//! clauses' `holds` atoms are read through the test's own letters, as
//! `crate::model_world` notes.

use std::collections::BTreeMap;

use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_eval::automaton::translate;
use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::{
    replay_model_trace, settle_model_check, CertificateRequest, EnvelopeSource, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ModelTraceSettlement, ModelTraceSubject, QualifiedName,
    ReplayLimits, ReplayRequestWire, ReplaySource, StateEnvironment, TemporalCounterexample,
    Witness, WitnessEnvelope, WitnessPacket,
};
use qsl_route::reductions::Reductions;
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::{Cancel, Identifier, Meter, NodeKey, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;

const PACKAGE: &str = "test/sym";
/// The infinite-trace profile alias every unit declares.
pub(crate) const INFINITE: &str = "t";
/// The bounded event-position false-extension profile alias every unit
/// declares.
pub(crate) const BOUNDED: &str = "b";

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

/// The population `counters`.
pub(crate) fn counters() -> DeclarationKey {
    key("counters")
}

/// The population `spares`, which no subject gives a universe.
pub(crate) fn spares() -> DeclarationKey {
    key("spares")
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

/// The shape of the domain package.
#[derive(Clone, Copy)]
pub(crate) struct Shape {
    /// Whether `counters` is declared `symmetric`.
    pub(crate) symmetric: bool,
    /// The greatest value of `v`.
    pub(crate) max: u64,
    /// Whether `Counter` has the optional reference field `link`.
    pub(crate) link: bool,
}

/// The domain package of `shape`.
pub(crate) fn document(shape: Shape) -> Vec<u8> {
    let wire_type = |identity: &str, kind: &str, extra: Json| {
        let mut node = json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": PACKAGE, "name": kind}, "roles": [],
            "origin": origin(identity), "constraints": [], "extensions": [],
            "unknownPolicy": "reject",
        });
        for (name, value) in extra.as_object().expect("extra members") {
            node[name] = value.clone();
        }
        node
    };
    let level = ident("Level");
    let bound = |keyword: &str, value: String| {
        json!({
            "identity": format!("{level}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&level),
        })
    };
    let field = |name: &str, type_ref: &str, presence: &str| {
        let identity = ident(&format!("Counter/{name}"));
        json!({
            "identity": identity, "name": name, "typeRef": type_ref,
            "presence": presence, "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin(&identity),
        })
    };
    let counter = ident("Counter");
    let mut fields = vec![field("v", &level, "required")];
    if shape.link {
        fields.push(field("link", &counter, "optional"));
    }
    let bump = ident("Counter/bump");
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let population = |name: &str, member: &str, symmetric: bool| {
        let identity = ident(name);
        json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [member], "extent": "closed", "origin": origin(&identity),
            "symmetric": symmetric,
        })
    };
    let spare = ident("Spare");
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": PACKAGE, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            wire_type(&level, "value_type", json!({"scalar": "integer",
                "constraints": [bound("min", "0".into()), bound("max", shape.max.to_string())]})),
            wire_type(&counter, "object_type", json!({
                "supertypes": [],
                "fields": fields,
                "operations": [{
                    "identity": bump, "name": "bump", "params": [], "pre": [], "post": [],
                    "origin": {"source": {"sourceIdentity": ident("spec"),
                        "path": "spec.qspec", "startLine": 1, "startColumn": 1}},
                    "frame": {"modifies": [], "modifies_receiver": [ident("Counter/v")],
                        "creates": [], "deletes": []},
                }],
                "relationships": [],
            })),
            wire_type(&spare, "object_type", json!({
                "supertypes": [], "fields": [], "operations": [], "relationships": [],
            })),
        ],
        "populations": [
            population("counters", &counter, shape.symmetric),
            population("spares", &spare, true),
        ],
    })
    .to_string()
    .into_bytes()
}

/// One temporal clause over `x: M::Counter`, `on origin`: its profile
/// alias and formula.
pub(crate) struct Clause<'c> {
    pub(crate) profile: &'c str,
    pub(crate) formula: &'c str,
}

/// A checked unit over a `test/sym` package, with its snapshot provision,
/// its initial states and its universe.
pub(crate) struct World {
    pub(crate) package: CheckedPackage,
    pub(crate) clauses: Vec<CheckedTemporalClause>,
    packages: qsl_semantics::model::intake::PackageInput,
    snapshots: qsl_semantics::model::observation::DocumentProvision,
    pub(crate) initial: Vec<DocumentRef>,
    /// The universe of `counters`; `None` gives the subject no universe.
    pub(crate) keys: Option<Vec<String>>,
    /// The subject's evaluation meter.
    pub(crate) meter: ScalarLimits,
    document: Vec<u8>,
    text: String,
}

impl World {
    /// The unit over `shape`'s package with `body` (contracts and state
    /// clauses) and `clauses`, started from each of `starts`, each a list of
    /// `(key, fields)`, over the universe `keys`.
    pub(crate) fn new(
        shape: Shape,
        body: &str,
        clauses: &[Clause<'_>],
        starts: &[Vec<(&str, Json)>],
        keys: &[&str],
    ) -> Self {
        let document = document(shape);
        let packages = package_input([document.as_slice()]).unwrap();
        let declared = clauses
            .iter()
            .enumerate()
            .fold(String::new(), |text, (index, clause)| {
                text + &format!(
                    "temporal C{index} using {} over (x: M::Counter) clock \"model-steps\" \
                     on origin {{ {} }}\n",
                    clause.profile, clause.formula
                )
            });
        // A unit selects one temporal profile.
        let profile = match clauses.first().map(|clause| clause.profile) {
            Some(BOUNDED) => format!(
                "profile {BOUNDED} = \"quire.temporal.event-position.false-extension/v1\";\n"
            ),
            _ => format!("profile {INFINITE} = \"quire.temporal.infinite-trace/v1\";\n"),
        };
        assert!(
            clauses
                .windows(2)
                .all(|pair| pair[0].profile == pair[1].profile),
            "one temporal profile per unit"
        );
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n{profile}\
             model M = \"{PACKAGE}\";\n\
             function Noop using v(): Boolean pure {{ true }}\n{body}{declared}"
        );
        let parsed = qsl_cst::parse(
            SourceIdentity::new("agent-ix", "reduction", "git", "1"),
            "reduction.native",
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
            "package".to_owned(),
            built,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
        let checked: Vec<CheckedTemporalClause> = graph
            .temporal_clauses()
            .iter()
            .map(|node| node.clause().clone())
            .collect();
        assert_eq!(checked.len(), clauses.len(), "{declared}");
        let mut world = Self {
            package: CheckedPackage::link(graph),
            clauses: checked,
            packages,
            snapshots: BTreeMap::new(),
            initial: Vec::new(),
            keys: Some(keys.iter().map(|key| (*key).to_owned()).collect()),
            meter: UNLIMITED,
            document,
            text,
        };
        for (index, objects) in starts.iter().enumerate() {
            let start = world.snapshot(&format!("s{index}"), objects);
            world.initial.push(start);
        }
        world
    }

    fn snapshot(&mut self, name: &str, objects: &[(&str, Json)]) -> DocumentRef {
        let objects: Vec<Json> = objects
            .iter()
            .map(|(key, fields)| json!({"key": key, "type": ident("Counter"), "fields": fields}))
            .collect();
        let header = json!({"identity": PACKAGE});
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": ident("counters"), "complete": true,
                "objects": objects}],
        });
        let bytes = document.to_string().into_bytes();
        self.snapshots.insert(
            qsl_semantics::model::observation::document_reference(&bytes)
                .expect("a document names itself"),
            bytes,
        );
        DocumentRef {
            authority: "test".to_owned(),
            identity: name.to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
        }
    }

    /// The subject: the initial states and the universe, if any.
    pub(crate) fn subject(&self) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes: self
                    .keys
                    .iter()
                    .map(|keys| PopulationUniverse {
                        population: counters(),
                        keys: keys.clone(),
                    })
                    .collect(),
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: self.meter,
            },
            proof_bounds: Vec::new(),
        }
    }

    /// The letters of clause `index`, its atoms read by `atoms`, and the
    /// atoms in the order the clause's automaton numbers them.
    pub(crate) fn letters(&self, index: usize, atoms: Vec<Atom>) -> (Letters, Vec<FormulaNodeId>) {
        let nodes = translate(&self.clauses[index], 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        (Letters::new(self, index, atoms), nodes)
    }

    /// The subject's model system.
    pub(crate) fn system(&self) -> ModelSystem<'_> {
        ModelSystem::new(self.subject().simulation).expect("the subject admits")
    }

    /// The run of clause `clause` (`None`: the deadlock-freedom item), its
    /// atoms read by `atoms`, under `reductions`, `max_depth` and `limits`.
    pub(crate) fn run(
        &self,
        clause: Option<usize>,
        atoms: Vec<Atom>,
        reductions: Reductions,
        max_depth: Option<usize>,
        limits: ModelCheckLimits,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let letters = clause.map(|index| Letters::new(self, index, atoms));
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
    /// re-executing its steps from its initial state: each step's
    /// post-state is the successor of its transition identity whose
    /// state key it records.
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
                        && system.state_key(state).expect("encodes").0.as_slice()
                            == step.post_state.as_bytes()
                })
                .map(|(_, state)| state)
                .expect("the recorded step is a subject step");
            states.push(current.clone());
        }
        states
    }
}

const AUTHORITY: &str = "agent-ix";

/// The fixture package manifest's top-level name.
const PACKAGE_NAME: &str = "package";

type SourceWire = (String, String, String, String);

fn source_wire(identity: &str) -> SourceWire {
    (
        AUTHORITY.to_owned(),
        identity.to_owned(),
        "git".to_owned(),
        "1".to_owned(),
    )
}

fn witness_source() -> ReplaySource {
    ReplaySource::Witness(
        Witness::parse("<<<assertion|model_check|deadlock|>>>").expect("a transcript"),
    )
}

fn envelope_witness_source() -> EnvelopeSource {
    EnvelopeSource::Witness(
        Witness::parse("<<<assertion|model_check|deadlock|>>>").expect("a transcript"),
    )
}

fn selected() -> QualifiedName {
    QualifiedName::new(vec![Identifier::new("Noop").expect("an identifier")]).expect("a name")
}

/// A compiled unit as FR-128 recompiles it: its source identity,
/// its text, its one domain package and the snapshots it provisions.
pub(crate) struct ReplayUnit<'u> {
    pub(crate) identity: &'u str,
    pub(crate) text: &'u [u8],
    pub(crate) document: &'u [u8],
    pub(crate) snapshots: &'u qsl_semantics::model::observation::DocumentProvision,
}

/// The node and occurrence of `package`'s temporal clause `index`, as an
/// envelope carries them.
pub(crate) fn clause_identity(package: &CheckedPackage, index: usize) -> (NodeKey, OccurrenceKey) {
    let node = &package.graph().temporal_clauses()[index];
    let id = node.identity().clone();
    (id.clone(), OccurrenceKey::new(id, node.claim().clone()))
}

/// FR-128: replay `payload` through `replay_model_trace`: `unit` is
/// recompiled from its source bytes, the subject re-admitted from the byte
/// provision, and the item is the clause `clause` names, or the
/// deadlock-freedom item when it names none.
pub(crate) fn replay_unit(
    unit: &ReplayUnit<'_>,
    subject: &ModelTraceSubject,
    payload: TemporalCounterexample,
    clause: Option<(NodeKey, OccurrenceKey)>,
    letters: &dyn ModelLetters,
) -> ModelTraceSettlement {
    replay_unit_result(unit, subject, payload, clause, letters)
        .expect("the counterexample replays")
        .settlement
}

/// [`replay_unit`]'s whole result, as the replay `settle_model_check` takes.
pub(crate) fn replay_unit_result(
    unit: &ReplayUnit<'_>,
    subject: &ModelTraceSubject,
    payload: TemporalCounterexample,
    clause: Option<(NodeKey, OccurrenceKey)>,
    letters: &dyn ModelLetters,
) -> Result<qsl_replay::ModelTraceReplayResult, qsl_replay::ReplayRefusal> {
    let byte_provision = vec![(
        AUTHORITY.to_owned(),
        unit.identity.to_owned(),
        unit.text.to_vec(),
    )];
    let mut documents = vec![unit.document.to_vec()];
    for bytes in unit.snapshots.values() {
        documents.push(bytes.clone());
    }
    let wire = ReplayRequestWire {
        package_name: PACKAGE_NAME.to_owned(),
        profile_selections: Vec::new(),
        sources: vec![source_wire(unit.identity)],
        dependencies: Vec::new(),
        replay_target: qsl_replay::RequestTargetWire::function(
            selected(),
            qsl_replay::OccurrenceKey::new(
                crate::fixture_ids::node_key([7]),
                qsl_replay::Origin::new(qsl_replay::Role::new("declaration"), 0),
            ),
        ),
        source: witness_source(),
        obligation_identity: crate::fixture_ids::obligation_identity(),
        backend: "qsl-en-1".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents,
    };
    let (node, occurrence) = clause.unwrap_or_else(|| crate::model_settle::model_anchor(PACKAGE));
    let envelope = WitnessEnvelope::reconstruct(
        WitnessPacket {
            obligation_identity: Some(crate::fixture_ids::obligation_identity()),
            occurrence_key: Some(occurrence),
            clause_node: Some(node),
            replay_target: qsl_replay::ReplayTargetMembers::function(selected()),
            profile_selections: Some(Vec::new()),
            run_limits: Some(UNLIMITED),
            declared_domains: Some(Vec::new()),
            backend: Some("qsl-en-1".to_owned()),
            trace_position: Some(None),
            source: Some(envelope_witness_source()),
            family_payload: Some(payload),
        },
        ReplayLimits::default(),
    )
    .expect("a complete packet reconstructs");
    replay_model_trace(wire, subject, &envelope, letters, ReplayLimits::default())
}

impl World {
    fn replay_unit(&self) -> ReplayUnit<'_> {
        ReplayUnit {
            identity: "reduction",
            text: self.text.as_bytes(),
            document: &self.document,
            snapshots: &self.snapshots,
        }
    }

    fn trace_subject(&self) -> ModelTraceSubject {
        ModelTraceSubject {
            timed: None,
            initial: self.initial.clone(),
            universes: self
                .keys
                .iter()
                .map(|keys| PopulationUniverse {
                    population: counters(),
                    keys: keys.clone(),
                })
                .collect(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
        }
    }

    /// FR-128: replay `payload`, a deadlock-freedom counterexample.
    pub(crate) fn replay(&self, payload: TemporalCounterexample) -> ModelTraceSettlement {
        replay_unit(
            &self.replay_unit(),
            &self.trace_subject(),
            payload,
            None,
            &NoAtoms,
        )
    }

    /// FR-127: `outcome` of clause `clause` (its atoms read by `atoms`)
    /// settled through `settle_model_check`, a refutation by replaying its
    /// counterexample through FR-128.
    pub(crate) fn settle_clause(
        &self,
        clause: usize,
        atoms: Vec<Atom>,
        outcome: &ModelCheckOutcome,
    ) -> qsl_replay::ModelCheckRecord {
        let letters = Letters::new(self, clause, atoms);
        let unit = self.replay_unit();
        settle_model_check(
            outcome,
            &ModelCheckLimits::default(),
            &CertificateRequest {
                subject: self.subject(),
                item: ModelCheckItem::Clause {
                    clause: &self.clauses[clause],
                    letters: &letters,
                },
                smt_encoding: qsl_replay::SmtEncodingLimits::default(),
            },
            &Cancel::new(),
            |counterexample| {
                replay_unit_result(
                    &unit,
                    &self.trace_subject(),
                    counterexample.clone(),
                    Some(clause_identity(&self.package, clause)),
                    &letters,
                )
            },
        )
    }

    /// FR-128: replay `payload` as clause `clause`'s counterexample, its
    /// atoms read by `atoms`.
    pub(crate) fn replay_clause(
        &self,
        clause: usize,
        atoms: Vec<Atom>,
        payload: TemporalCounterexample,
    ) -> ModelTraceSettlement {
        let letters = Letters::new(self, clause, atoms);
        replay_unit(
            &self.replay_unit(),
            &self.trace_subject(),
            payload,
            Some(clause_identity(&self.package, clause)),
            &letters,
        )
    }
}

/// `{"v": value}`, with `link` absent when the shape has one.
pub(crate) fn counter(value: i64) -> Json {
    json!({"v": {"integer": value.to_string()}})
}

/// `{"v": value, "link": ...}`: `link` names `target`, or is absent.
pub(crate) fn linked(value: i64, target: Option<&str>) -> Json {
    let link = match target {
        Some(key) => json!({"present": {"reference": {"population": ident("counters"),
            "key": key}}}),
        None => json!({"absent": {}}),
    };
    json!({"v": {"integer": value.to_string()}, "link": link})
}

/// One initial state over `a`, `b` and `c` with the values `values`.
pub(crate) fn start(values: [i64; 3]) -> Vec<(&'static str, Json)> {
    ["a", "b", "c"]
        .into_iter()
        .zip(values)
        .map(|(key, value)| (key, counter(value)))
        .collect()
}

/// `v` of `object` at `state`.
pub(crate) fn value(state: &ModelSimState, object: &str) -> i64 {
    match state.state().populations[&counters()][object]
        .fields
        .get(&key("Counter/v"))
    {
        Some(Value::Integer(value)) => value.to_string().parse().expect("a small value"),
        other => panic!("{object}.v is not an integer: {other:?}"),
    }
}

/// `(v(a), v(b), v(c))` at `state`.
pub(crate) fn values(state: &ModelSimState) -> (i64, i64, i64) {
    (value(state, "a"), value(state, "b"), value(state, "c"))
}

/// One `holds` atom's reading of the bound object `x` at a state:
/// `Some(truth)`, or `None` for an undefined division by zero.
pub(crate) type Atom = Box<dyn Fn(&ModelSimState, &str) -> Option<bool>>;

/// `x.v` read by `test`.
pub(crate) fn atom(test: fn(i64) -> Option<bool>) -> Atom {
    Box::new(move |state, object| test(value(state, object)))
}

/// The letters of one clause: each `holds` atom, in the formula's node
/// order, read by its own predicate.
pub(crate) struct Letters {
    holds: Vec<FormulaNodeId>,
    atoms: Vec<Atom>,
    /// What each atom reads, derived from its checked condition (FR-156).
    reads: Vec<crate::units::Reads>,
    /// The guard of a guarded `on each` clause, and what it reads.
    guard: Option<(Atom, crate::units::Reads)>,
}

impl Letters {
    /// The letters of clause `index` of `world`, its atoms read by `atoms`
    /// and their footprints derived from the checked conditions.
    fn new(world: &World, index: usize, atoms: Vec<Atom>) -> Self {
        let clause = &world.clauses[index];
        let holds = translate(clause, 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        assert_eq!(holds.len(), atoms.len(), "one predicate per `holds` atom");
        let system = world.system();
        let node = &world.package.graph().temporal_clauses()[index];
        let reads = holds
            .iter()
            .map(|atom| {
                let (condition, _) = node.condition(*atom).expect("a `holds` atom");
                system.atom_reads(clause.over(), condition)
            })
            .collect();
        Self {
            holds,
            atoms,
            reads,
            guard: None,
        }
    }

    /// These letters with a guard `guard` that reads `reads`.
    pub(crate) fn guarded(self, guard: Atom, reads: crate::units::Reads) -> Self {
        Self {
            guard: Some((guard, reads)),
            ..self
        }
    }
}

impl ModelLetters for Letters {
    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let (guard, _) = self
            .guard
            .as_ref()
            .expect("the engine reads only a guarded clause's condition");
        Ok(match guard(state, &instance.key) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )),
        })
    }

    fn activation_reads(&self) -> crate::units::Reads {
        let (_, reads) = self
            .guard
            .as_ref()
            .expect("the engine reads only a guarded clause's footprint");
        reads.clone()
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        let index = self
            .holds
            .iter()
            .position(|node| *node == atom)
            .expect("the engine reads only the clause's atoms");
        self.reads[index].clone()
    }

    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let index = self
            .holds
            .iter()
            .position(|node| *node == atom)
            .expect("the engine reads only the clause's atoms");
        Ok(match (self.atoms[index])(state, &instance.key) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )),
        })
    }
}

/// The letters of a deadlock-freedom replay, which reads no `holds` atom.
struct NoAtoms;

impl ModelLetters for NoAtoms {
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
    ) -> Result<AtomReading, AtomStop> {
        Ok(AtomReading::False)
    }
}
