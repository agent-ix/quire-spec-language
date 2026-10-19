// SPDX-License-Identifier: AGPL-3.0-or-later
//! Units other than `Counter` for the model-check tests: ADR-018 §6's
//! ConfigVersion example and shipped units over `examples/config-version`'s
//! domain package, and FR-126-AC-4's `Health` unit over a domain package
//! written here. Each is compiled through S1 to S4 as `crate::counter`'s
//! units are, and read through the test's own letters.

use std::collections::BTreeMap;

use qsl_eval::automaton::translate;
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, PopulationUniverse,
};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::{Incomplete, Meter, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;

/// `examples/config-version`'s domain package and unit text, the datum the
/// example's own fixtures and `qsl-package`'s corpus read.
#[path = "../../../examples/config-version/spine_unit.rs"]
mod config_version;

pub(crate) use config_version::{unit_text as shipped_unit_text, PACKAGE_IDENTITY as CONFIG};

/// A domain package and where its objects live.
pub(crate) struct Domain {
    /// The package identity.
    pub(crate) package: &'static str,
    /// The one population's name.
    pub(crate) population: &'static str,
    /// The one object type's name.
    pub(crate) object: &'static str,
    /// The Semantic IR 2.0.0 document.
    pub(crate) document: Vec<u8>,
}

impl Domain {
    /// `examples/config-version`'s domain package with `attemptUpdate`'s
    /// frame scoped to its receiver (`modifies_receiver`), so an update of
    /// one object leaves the other as it is, as ADR-018 §6's subject
    /// requires: the shipped frame grants the field of every object, and
    /// with two objects over `Int[0, 1000]` that is a million candidates
    /// per step.
    pub(crate) fn config_version() -> Self {
        let mut document: Json = serde_json::from_str(config_version::DOMAIN_PACKAGE)
            .expect("the shipped domain package is JSON");
        let frame = &mut document["types"][1]["operations"][0]["frame"];
        assert_eq!(frame["modifies"].as_array().map(Vec::len), Some(1));
        let field = frame["modifies"][0].clone();
        *frame = json!({"modifies": [], "modifies_receiver": [field],
            "creates": [], "deletes": []});
        Self {
            package: CONFIG,
            population: "config_history",
            object: "ConfigVersion",
            document: document.to_string().into_bytes(),
        }
    }

    fn ident(&self, suffix: &str) -> String {
        format!("ix://{}/{suffix}", self.package)
    }

    fn key(&self, suffix: &str) -> DeclarationKey {
        DeclarationKey {
            package: self.package.to_owned(),
            node: self.ident(suffix),
        }
    }

    /// The `sha256-jcs` digest hex of the domain package, as a unit's
    /// `model` statement names it.
    pub(crate) fn digest_hex(&self) -> String {
        let digest = *package_input([self.document.as_slice()])
            .keys()
            .next()
            .expect("one model");
        hex(&digest)
    }
}

/// The unit text of a model header over `domain`, aliased `alias`, and
/// `body` after it.
pub(crate) fn unit(domain: &Domain, alias: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t0 = \"quire.temporal.infinite-trace/v1\";\n\
         model {alias} = \"{}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{body}",
        domain.package,
        domain.digest_hex()
    )
}

/// A temporal clause `C` over `over`, `on origin`, with `fairness` and
/// `formula`.
pub(crate) fn clause(over: &str, fairness: &str, formula: &str) -> String {
    format!(
        "temporal C using t0 over ({over}) clock \"model-steps\" on origin {{ {fairness} {formula} }}\n"
    )
}

/// The checked package of `text` over `domain`, and its declared clauses.
pub(crate) fn checked(domain: &Domain, text: &str) -> (CheckedPackage, Vec<CheckedTemporalClause>) {
    let packages = package_input([domain.document.as_slice()]);
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "units", "git", "1"),
        "units.native",
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
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    let graph = declarations
        .with_unwritten_terminals()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    let clauses = graph
        .temporal_clauses()
        .iter()
        .map(|node| node.clause().clone())
        .collect();
    (CheckedPackage::link(graph), clauses)
}

/// A world: a checked unit, its provisioned snapshots and initial states.
pub(crate) struct World {
    pub(crate) domain: Domain,
    pub(crate) package: CheckedPackage,
    pub(crate) clauses: Vec<CheckedTemporalClause>,
    pub(crate) packages: BTreeMap<[u8; 32], Vec<u8>>,
    pub(crate) snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    pub(crate) initial: Vec<DocumentRef>,
    pub(crate) keys: Vec<&'static str>,
}

impl World {
    /// The unit `text` over `domain`, started from the objects `starts`:
    /// each `(key, fields)`, the fields as snapshot values.
    pub(crate) fn new(domain: Domain, text: &str, starts: &[(&'static str, Json)]) -> Self {
        let (package, clauses) = checked(&domain, text);
        let packages = package_input([domain.document.as_slice()]);
        let mut world = Self {
            package,
            clauses,
            packages,
            snapshots: BTreeMap::new(),
            initial: Vec::new(),
            keys: starts.iter().map(|(key, _)| *key).collect(),
            domain,
        };
        let reference = world.start("s0", starts);
        world.initial.push(reference);
        world
    }

    fn start(&mut self, name: &str, objects: &[(&str, Json)]) -> DocumentRef {
        let objects: Vec<Json> = objects
            .iter()
            .map(|(key, fields)| {
                json!({"key": key, "type": self.domain.ident(self.domain.object),
                    "fields": fields})
            })
            .collect();
        let header = json!({"identity": self.domain.package, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", self.domain.digest_hex())});
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": self.domain.ident(self.domain.population),
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
            identity: name.to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest,
        }
    }

    pub(crate) fn subject(&self, meter: ScalarLimits) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes: vec![PopulationUniverse {
                    population: self.domain.key(self.domain.population),
                    keys: self.keys.iter().map(|key| (*key).to_owned()).collect(),
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

    pub(crate) fn unlimited(&self) -> ModelSubject<'_> {
        self.subject(UNLIMITED)
    }

    /// The field `field` of the object `object` at `state`.
    pub(crate) fn field<'s>(
        &self,
        state: &'s ModelSimState,
        object: &str,
        field: &str,
    ) -> &'s Value {
        state.state().populations[&self.domain.key(self.domain.population)][object]
            .fields
            .get(&self.domain.key(&format!("{}/{field}", self.domain.object)))
            .unwrap_or_else(|| panic!("{object}.{field} is not set"))
    }

    pub(crate) fn integer(&self, state: &ModelSimState, object: &str, field: &str) -> i64 {
        match self.field(state, object, field) {
            Value::Integer(value) => value.to_string().parse().expect("a small value"),
            other => panic!("{object}.{field} is not an integer: {other:?}"),
        }
    }

    pub(crate) fn boolean(&self, state: &ModelSimState, object: &str, field: &str) -> bool {
        match self.field(state, object, field) {
            Value::Boolean(value) => *value,
            other => panic!("{object}.{field} is not a boolean: {other:?}"),
        }
    }
}

/// How one atom reads a state for a bound object's key.
pub(crate) type Read<'w> = Box<dyn Fn(&ModelSimState, &str) -> bool + 'w>;

/// Letters over a world: atom `i` of the clause (in clause-node order) is
/// read by `reads[i]` from the state and the bound object's key.
pub(crate) struct Atoms<'w> {
    pub(crate) atoms: Vec<FormulaNodeId>,
    pub(crate) reads: Vec<Read<'w>>,
}

impl<'w> Atoms<'w> {
    /// Letters for `world`'s first clause: instances are the universe's.
    pub(crate) fn new(world: &World, reads: Vec<Read<'w>>) -> Self {
        let automaton = translate(&world.clauses[0], 1 << 20).expect("the clause translates");
        assert_eq!(automaton.atoms().len(), reads.len(), "one read per atom");
        Self {
            atoms: automaton.atoms().to_vec(),
            reads,
        }
    }
}

impl ModelLetters for Atoms<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let index = self
            .atoms
            .iter()
            .position(|known| *known == atom)
            .expect("the clause's own atom");
        Ok(if (self.reads[index])(state, &instance.key) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

/// `object`'s snapshot fields for `ConfigVersion`: its version, no parent.
pub(crate) fn config_object(version: i64) -> Json {
    json!({"versionNumber": {"integer": version.to_string()},
        "parent": {"absent": {}}})
}

/// `Health`'s package (FR-126-AC-4): object `Health` (fields `healthy:
/// Bool`, `failures: Int[0, 1]`) in population `systems`, with operations
/// `fail` (modifies both) and `recover` (modifies `healthy`).
pub(crate) fn health() -> Domain {
    const PACKAGE: &str = "test/health";
    let ident = |suffix: &str| format!("ix://{PACKAGE}/{suffix}");
    let origin = |identity: &str| {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    };
    let multiplicity = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
    let field = |name: &str, type_ref: &str| {
        let identity = ident(&format!("Health/{name}"));
        json!({
            "identity": identity, "name": name, "typeRef": type_ref,
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": multiplicity, "origin": origin(&identity),
        })
    };
    let operation = |name: &str, modifies: &[&str]| {
        let identity = ident(&format!("Health/{name}"));
        json!({
            "identity": identity, "name": name, "params": [], "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": modifies.iter()
                .map(|field| ident(&format!("Health/{field}"))).collect::<Vec<_>>(),
                "creates": [], "deletes": []},
        })
    };
    let level = {
        let identity = ident("Level");
        let constraint = |keyword: &str, value: &str| {
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
            "constraints": [constraint("min", "0"), constraint("max", "1")],
        })
    };
    let health = ident("Health");
    let population = ident("systems");
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
            level,
            {
                "identity": health, "displayName": health,
                "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
                "origin": origin(&health), "constraints": [], "extensions": [],
                "unknownPolicy": "reject", "supertypes": [],
                "fields": [
                    field("healthy", "ix://quire/native/Boolean"),
                    field("failures", &ident("Level")),
                ],
                "operations": [
                    operation("fail", &["healthy", "failures"]),
                    operation("recover", &["healthy"]),
                ],
                "relationships": [],
            },
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [health], "extent": "closed", "origin": origin(&population),
        }],
    });
    Domain {
        package: PACKAGE,
        population: "systems",
        object: "Health",
        document: document.to_string().into_bytes(),
    }
}

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
