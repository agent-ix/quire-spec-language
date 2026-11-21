// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model subjects for the EN-1 safety tests (`crate::model_check_safety`):
//! a domain package with one population, a native unit selecting it as `M`
//! with its contracts and temporal clauses, the initial snapshots, and the
//! letters of each clause's `holds` atoms.
//!
//! Two domain packages are used: FR-124-AC-1's `test/counter`
//! (`crate::counter`'s document) and FR-126-AC-4's `test/health`, written
//! here.

use filament_semantic_value::checking::CheckMode;
use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_exact::{Meter, ScalarLimits, Value};
use filament_semantic_value::checking::CheckingLimits;
use fsl_eval::automaton::translate;
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters, ModelSubject};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::TransitionSystem;
use fsl_eval::temporal::{AtomReading, UndefinedCause};
use fsl_foundation::SourceIdentity;
use fsl_package::CheckedPackage;
use fsl_replay::TemporalCounterexample;
use fsl_semantics::check::{CheckedTemporalClause, FormulaNodeId, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, meaning, package_input};
use fsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use serde_json::{json, Value as Json};

/// The infinite-trace profile.
pub(crate) const INFINITE: &str = "filament.temporal.infinite-trace/v1";
/// The bounded event-position false-extension profile.
pub(crate) const BOUNDED: &str = "filament.temporal.event-position.false-extension/v1";

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

    pub(crate) fn key(&self, suffix: &str) -> DeclarationKey {
        DeclarationKey {
            package: self.package.to_owned(),
            node: self.node(suffix),
        }
    }

    /// The field `name` of the object `object` at `state`.
    pub(crate) fn field<'s>(
        &self,
        state: &'s ModelSimState,
        object: &str,
        name: &str,
    ) -> &'s Value {
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

/// The `Counter` package with no operation: its object type keeps its field
/// and population and declares nothing a step applies, so every state of a
/// model over it deadlocks (FR-124).
pub(crate) fn still_counter() -> Domain {
    let mut document: Json = serde_json::from_slice(&crate::counter::document(false))
        .expect("the Counter document is JSON");
    for declared in document["types"]
        .as_array_mut()
        .expect("the document declares types")
    {
        if declared.get("operations").is_some() {
            declared["operations"] = Json::Array(Vec::new());
        }
    }
    Domain {
        document: document.to_string().into_bytes(),
        ..counter()
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
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.fspec",
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
            "operands": {"value": value}, "appliesTo": "ix://filament/native/Integer",
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
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let object = ident("Health");
    let population = ident("systems");
    let document = json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": package, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            wire_type(&failures, "value_type", json!({"scalar": "integer",
                "constraints": [bound("min", "0"), bound("max", "1")]})),
            wire_type(&object, "object_type", json!({
                "supertypes": [],
                "fields": [field("healthy", "ix://filament/native/Boolean"),
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
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: fsl_semantics::model::observation::DocumentProvision,
    pub(crate) initial: Vec<DocumentRef>,
    /// The native unit the package was checked from.
    pub(crate) unit: Vec<u8>,
    pub(crate) provision: fsl_analyze::analyze::SourceProvision,
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
        Self::build(domain, body, "", CheckMode::Linked, profile, clauses, start)
    }

    /// As [`World::new`], with `functions` (unit-level declarations placed
    /// before the clauses) checked in `CheckMode::Kernel`, so a function
    /// whose divisor may be zero is checked by the Kernel (FR-495-AC-3).
    pub(crate) fn with_kernel_functions(
        domain: Domain,
        body: &str,
        functions: &str,
        profile: &str,
        clauses: &[Clause<'_>],
        start: &[(&str, Json)],
    ) -> Self {
        Self::build(
            domain,
            body,
            functions,
            CheckMode::Kernel,
            profile,
            clauses,
            start,
        )
    }

    fn build(
        domain: Domain,
        body: &str,
        functions: &str,
        mode: CheckMode,
        profile: &str,
        clauses: &[Clause<'_>],
        start: &[(&str, Json)],
    ) -> Self {
        let packages = package_input([domain.document.as_slice()]).unwrap();
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             profile t = \"{profile}\";\n\
             model M = \"{}\";\n{body}{functions}",
            domain.package
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
            let parsed = fsl_cst::parse(
                SourceIdentity::new("agent-ix", "safety"),
                "safety.native",
                text.as_bytes(),
                fsl_cst::Limits::default(),
            )
            .expect("S1 parses the unit");
            assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
            let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
            let models = admit_unit(
                &built.selections().models,
                &packages,
                ModelNormalizationLimits::default(),
            )
            .expect("the model selection admits")
            .with_parsed_operation_clauses(|text| {
                fsl_forms::parse_clause_text(text, fsl_cst::Limits::default())
            })
            .expect("the operation clauses parse");
            PackageDeclarations::assemble(
                parsed.source().reference().clone(),
                "package".to_owned(),
                built,
                models,
                Vec::new(),
            )
            .expect("the unit assembles")
        };
        let unit = format!("{text}{declared}");
        let graph = declarations(&unit)
            .check(
                mode,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("{unit}: the unit checks: {refusals:?}"));
        let checked: Vec<CheckedTemporalClause> = graph
            .temporal_clauses()
            .iter()
            .map(|node| node.clause().clone())
            .collect();
        assert_eq!(checked.len(), clauses.len(), "{declared}");
        let mut world = Self {
            provision: fsl_analyze::analyze::SourceProvision::single(
                graph.source(),
                unit.as_bytes(),
            ),
            domain,
            package: CheckedPackage::link(graph),
            clauses: checked,
            packages,
            snapshots: BTreeMap::new(),
            initial: Vec::new(),
            unit: unit.into_bytes(),
        };
        let start = world.snapshot(start);
        world.initial.push(start);
        world
    }

    /// The binding `analyze` takes over this world's subject, with the
    /// universe `keys` of its population.
    pub(crate) fn binding(&self, keys: &[&str]) -> fsl_analyze::analyze::SubjectBinding<'_> {
        fsl_analyze::analyze::SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: vec![PopulationUniverse {
                population: self.domain.key(self.domain.population),
                keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            }],
        }
    }

    /// The FR-517 handoff this world's package is written to.
    pub(crate) fn handoff(&self) -> WorldHandoff {
        let emission = fsl_package::emit_checked(&self.package, fsl_package::EmitLimits::fixture())
            .expect("the package emits");
        let bytes = emission.package().bytes().to_vec();
        let admitted = fsl_package::admit_checked_package(
            &bytes,
            &self.packages,
            fsl_package::V2ReadLimits::fixture(),
        )
        .expect("the strict reader admits the bytes");
        let mut budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
        let identities = emission
            .identity_rows(&admitted, &mut budget)
            .expect("the identity rows");
        let equality = emission
            .equality_witness(&admitted, &mut budget)
            .expect("the equality rows");
        WorldHandoff {
            identity_bytes: identities.to_bytes(&admitted).expect("identities.json"),
            equality_bytes: equality.to_bytes(&admitted).expect("equality.json"),
            bytes,
            admitted,
            identities,
            equality,
        }
    }

    /// The bytes of each provisioned snapshot, by label.
    pub(crate) fn snapshot_bytes(
        &self,
    ) -> impl Iterator<Item = (&fsl_semantics::model::observation::DocumentRef, &Vec<u8>)> {
        self.snapshots.iter()
    }

    /// An initialization snapshot of `objects`, provisioned here.
    fn snapshot(&mut self, objects: &[(&str, Json)]) -> DocumentRef {
        let object_type = self.domain.node(self.domain.object);
        let objects: Vec<Json> = objects
            .iter()
            .map(|(key, fields)| json!({"key": key, "type": object_type, "fields": fields}))
            .collect();
        let header = json!({"identity": self.domain.package});
        let document = json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": self.domain.node(self.domain.population),
                "complete": true, "objects": objects}],
        });
        let bytes = document.to_string().into_bytes();
        self.snapshots.insert(
            fsl_semantics::model::observation::document_reference(&bytes)
                .expect("a document names itself"),
            bytes,
        );
        DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
        }
    }

    /// The subject from the initial snapshot with universe `keys`.
    pub(crate) fn subject(&self, keys: &[&str], meter: ScalarLimits) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                emit: fsl_package::EmitLimits::fixture(),
                input: fsl_eval::simulation::model::ModelCheckInput::source(&self.package),
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

    /// The model state at each position of `counterexample`, read by
    /// re-executing its steps: each step's post-state is the successor of its
    /// transition identity whose state key it records.
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
                        && system.state_key(state).expect("encodes").0.as_slice()
                            == step.post_state.as_bytes()
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
    holds: Vec<FormulaNodeId>,
    atoms: Vec<Atom>,
    /// The reading of a guarded `on each` activation's `when` condition.
    guard: Option<Atom>,
    /// What the guard reads (FR-156), as the test declares it.
    guard_reads: crate::units::Reads,
}

impl Letters {
    /// `clause`'s letters, its `holds` atoms read in node order by `atoms`
    /// for the object under the instance's key. The atoms' node order is the
    /// translation's letter order (`PropertyAutomaton::atoms`).
    pub(crate) fn new(clause: &CheckedTemporalClause, atoms: Vec<Atom>) -> Self {
        let holds = translate(clause, 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        assert_eq!(holds.len(), atoms.len(), "one predicate per `holds` atom");
        Self {
            holds,
            atoms,
            guard: None,
            guard_reads: crate::units::unread(),
        }
    }

    /// These letters, with the `when` condition of the clause's guarded `on
    /// each` activation read by `guard`, which reads `reads`.
    pub(crate) fn guarded(self, guard: Atom, reads: crate::units::Reads) -> Self {
        Self {
            guard: Some(guard),
            guard_reads: reads,
            ..self
        }
    }
}

/// An atom's reading as the engine takes it: `None` is an undefined
/// division by zero.
fn reading(truth: Option<bool>) -> AtomReading {
    match truth {
        Some(true) => AtomReading::True,
        Some(false) => AtomReading::False,
        None => AtomReading::Undefined(UndefinedCause::Kernel(
            filament_exact::Undefined::DivisionByZero,
        )),
    }
}

impl ModelLetters for Letters {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
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
        Ok(reading((self.atoms[index])(state, &instance.key)))
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let guard = self
            .guard
            .as_ref()
            .expect("the engine reads only a guarded clause's condition");
        Ok(reading(guard(state, &instance.key)))
    }

    fn activation_reads(&self) -> crate::units::Reads {
        self.guard_reads.clone()
    }
}

/// The FR-517 handoff a [`World`]'s package is written to: the package
/// bytes, their strict read-back, and the identity and equality rows with
/// the bytes of `identities.json` and `equality.json`.
pub(crate) struct WorldHandoff {
    pub(crate) bytes: Vec<u8>,
    admitted: fsl_package::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: fsl_package::ReadEqualityRows,
    identity_bytes: Vec<u8>,
    equality_bytes: Vec<u8>,
}

impl WorldHandoff {
    /// The admitted package with the handoff's rows attached.
    pub(crate) fn bare(&self) -> fsl_package::BarePackage<'_> {
        fsl_package::BarePackage::new(&self.admitted)
            .with_supply(
                fsl_package::BareSupply {
                    identities: &self.identities,
                    equality: Some(&self.equality),
                },
                &mut fsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the supply joins")
    }

    /// The handoff's identity and equality file bytes.
    pub(crate) fn supply(&self) -> fsl_replay::HandoffSupply<'_> {
        fsl_replay::HandoffSupply {
            dependencies: &[],
            identities: &self.identity_bytes,
            equality: Some(&self.equality_bytes),
        }
    }
}
