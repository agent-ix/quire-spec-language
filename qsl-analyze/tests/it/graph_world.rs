// SPDX-License-Identifier: AGPL-3.0-or-later
//! The state-graph tests' worlds: a one-type domain package whose fields are
//! bounded integers and whose operations take no parameter, a native unit
//! selecting it with a contract and state-graph claims, and the snapshots,
//! subjects and bindings a run reads.
//!
//! ADR-022 §7's worked examples are built here: §7.1's `ConfigVersion`
//! ([`config_version`]), §7.2's game ([`game`]), §7.3's job ([`job`]) and
//! FR-168-AC-9's `Fork` ([`fork_world`]); FR-168-AC-10's `Ladder`
//! ([`ladder_world`]) and `Detour` ([`detour_world`]).

use std::collections::BTreeMap;

use qsl_analyze::model_check::{StateGraphItem, StateGraphRequest};
use qsl_eval::model_subject::{ModelInstance, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, PopulationUniverse,
};
use qsl_foundation::{BackendId, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_replay::{
    GraphEvidence, GraphEvidenceEnvelope, GraphReplayResult, ModelCheckLimits, ModelPath,
    ModelTraceSubject, ReplayLimits, ReplayRequestWire, ReplayResult, ReplaySource,
    RequestTargetWire, StateEnvironment, WitnessPath, WitnessSource,
};
use qsl_semantics::check::{CheckedStateGraphClaim, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::observation::{
    declared_type_universe_for, DocumentRef, ObservationLimits,
};
use qsl_semantics::value::{admit_integer_division, DefinitionRole};
use quire_checked::key::DeclarationKey;
use quire_exact::{ObjectId, ObjectReference, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// An integer field `name`: `Int[low, high]`, or the unbounded native
/// `Integer` for `None`.
pub(crate) struct Field {
    pub(crate) name: &'static str,
    pub(crate) range: Option<(i64, i64)>,
}

/// One parameterless operation and the fields of its receiver its frame
/// modifies.
pub(crate) struct Operation {
    pub(crate) name: &'static str,
    pub(crate) modifies: &'static [&'static str],
}

/// A one-type domain package.
pub(crate) struct Model {
    /// The package identity.
    pub(crate) package: &'static str,
    /// The model alias the unit selects it as.
    pub(crate) alias: &'static str,
    /// The object type.
    pub(crate) object: &'static str,
    /// The population of the type.
    pub(crate) population: &'static str,
    /// Whether the population is declared symmetric.
    pub(crate) symmetric: bool,
    pub(crate) fields: Vec<Field>,
    pub(crate) operations: Vec<Operation>,
}

impl Model {
    fn ident(&self, suffix: &str) -> String {
        format!("ix://{}/{suffix}", self.package)
    }

    fn key(&self, suffix: &str) -> DeclarationKey {
        DeclarationKey {
            package: self.package.to_owned(),
            node: self.ident(suffix),
        }
    }

    fn origin(identity: &str) -> Json {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    }

    fn wire_type(&self, identity: &str, kind: &str, extra: Json) -> Json {
        let mut node = json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": self.package, "name": kind}, "roles": [],
            "origin": Self::origin(identity), "constraints": [], "extensions": [],
            "unknownPolicy": "reject",
        });
        for (name, value) in extra.as_object().expect("extra members") {
            node[name] = value.clone();
        }
        node
    }

    fn range(&self, field: &Field, low: i64, high: i64) -> Json {
        let identity = self.ident(&format!("{}Range", field.name));
        let constraint = |keyword: &str, value: String| {
            json!({
                "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
                "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
                "diagnosticCode": format!("bound.{keyword}"), "origin": Self::origin(&identity),
            })
        };
        self.wire_type(
            &identity,
            "value_type",
            json!({"scalar": "integer", "constraints": [
                constraint("min", low.to_string()),
                constraint("max", high.to_string()),
            ]}),
        )
    }

    fn operation(&self, operation: &Operation) -> Json {
        let identity = self.ident(&format!("{}/{}", self.object, operation.name));
        let modifies: Vec<String> = operation
            .modifies
            .iter()
            .map(|field| self.ident(&format!("{}/{field}", self.object)))
            .collect();
        json!({
            "identity": identity, "name": operation.name, "params": [], "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": self.ident("spec"), "path": "spec.qspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": [], "modifies_receiver": modifies, "creates": [],
                "deletes": []},
        })
    }

    /// The domain package document.
    pub(crate) fn document(&self) -> Vec<u8> {
        // The schema refuses a construct no type is of.
        let ranged = self.fields.iter().any(|field| field.range.is_some());
        let constructs: Vec<Json> = [
            ("object_type", meaning::OBJECT_TYPE),
            ("population", meaning::POPULATION),
            ("value_type", meaning::VALUE_TYPE),
        ]
        .into_iter()
        .filter(|(name, _)| ranged || *name != "value_type")
        .map(|(name, meaning)| {
            json!({"kind": {"module": self.package, "name": name}, "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER,
                "construct": {"identity": "none", "shape": "record", "members": {},
                    "meaning": meaning}})
        })
        .collect();
        let object = self.ident(self.object);
        let fields: Vec<Json> = self
            .fields
            .iter()
            .map(|field| {
                let identity = self.ident(&format!("{}/{}", self.object, field.name));
                json!({
                    "identity": identity, "name": field.name,
                    "typeRef": match field.range {
                        Some(_) => self.ident(&format!("{}Range", field.name)),
                        None => "ix://quire/native/Integer".to_owned(),
                    },
                    "presence": "required", "nullable": false, "defaultKind": "none",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                    "origin": Self::origin(&identity),
                })
            })
            .collect();
        let operations: Vec<Json> = self
            .operations
            .iter()
            .map(|operation| self.operation(operation))
            .collect();
        let mut types: Vec<Json> = self
            .fields
            .iter()
            .filter_map(|field| field.range.map(|(low, high)| self.range(field, low, high)))
            .collect();
        types.push(self.wire_type(
            &object,
            "object_type",
            json!({"supertypes": [], "fields": fields, "operations": operations,
                "relationships": []}),
        ));
        let population = self.ident(self.population);
        json!({
            "contractVersion": "2.0.0",
            "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
            "source": {"identity": self.ident("spec"), "version": "1.0.0",
                "dialect": "spec-bundle", "digest": PLACEHOLDER},
            "package": {"identity": self.package, "version": "1.0.0",
                "manifestDigest": PLACEHOLDER, "mappingVersions": [], "profileVersions": [],
                "lockDigest": PLACEHOLDER},
            "occurrences": [], "extensions": [],
            "constructs": constructs,
            "types": types,
            "populations": [{
                "identity": population, "displayName": population,
                "kind": {"module": self.package, "name": "population"},
                "members": [object], "extent": "closed", "origin": Self::origin(&population),
            "symmetric": self.symmetric,
            }],
        })
        .to_string()
        .into_bytes()
    }
}

/// One object of a snapshot: its key and its `(field, value)` pairs.
pub(crate) type Object<'a> = (&'a str, &'a [(&'a str, i64)]);

/// A checked world: the model, its package with `body`'s contract and
/// claims, and the provisioned documents and initial snapshots.
pub(crate) struct World {
    pub(crate) model: Model,
    pub(crate) package: CheckedPackage,
    document: Vec<u8>,
    source: Vec<u8>,
    packages: qsl_semantics::model::intake::PackageInput,
    snapshots: qsl_semantics::model::observation::DocumentProvision,
    pub(crate) initial: Vec<DocumentRef>,
}

impl World {
    /// The world of `model` with `body` after the unit header, and one
    /// initial snapshot per entry of `starts`, each `(key, [(field,
    /// value)])` per object.
    pub(crate) fn new(model: Model, body: &str, starts: &[&[Object<'_>]]) -> Self {
        Self::with_profiles(model, "", body, starts)
    }

    /// As [`Self::new`], with `profiles` (header `profile` lines) after the
    /// value profile.
    pub(crate) fn with_profiles(
        model: Model,
        profiles: &str,
        body: &str,
        starts: &[&[Object<'_>]],
    ) -> Self {
        let document = model.document();
        let packages = package_input([document.as_slice()]).unwrap();
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n{profiles}\
             model {} = \"{}\";\n{body}",
            model.alias, model.package
        );
        let parsed = qsl_cst::parse(
            SourceIdentity::new("agent-ix", "graph", "git", "1"),
            "graph.native",
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
        let mut declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "package".to_owned(),
            built,
            models,
            Vec::new(),
        )
        .expect("the unit assembles");
        // `div` reads truncating integer division.
        declarations.integer_division = Some(
            admit_integer_division(
                &[DefinitionRole::IntegerDivisionTruncating.reference()],
                None,
            )
            .expect("the division profile admits"),
        );
        let graph = declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
        let package = CheckedPackage::link(graph);
        let mut world = Self {
            model,
            package,
            document,
            source: text.into_bytes(),
            packages,
            snapshots: BTreeMap::new(),
            initial: Vec::new(),
        };
        world.initial = starts
            .iter()
            .enumerate()
            .map(|(index, objects)| world.start(&format!("s{index}"), objects))
            .collect();
        world
    }

    fn start(&mut self, name: &str, objects: &[Object<'_>]) -> DocumentRef {
        let object_type = self.model.ident(self.model.object);
        let objects: Vec<Json> = objects
            .iter()
            .map(|(key, fields)| {
                let fields: serde_json::Map<String, Json> = fields
                    .iter()
                    .map(|(field, value)| {
                        ((*field).to_owned(), json!({"integer": value.to_string()}))
                    })
                    .collect();
                json!({"key": key, "type": object_type, "fields": fields})
            })
            .collect();
        let header = json!({"identity": self.model.package});
        let document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": self.model.ident(self.model.population),
                "complete": true, "objects": objects}],
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

    /// The subject over the world's initial snapshots with universe `keys`.
    pub(crate) fn subject(&self, keys: &[&str], meter: ScalarLimits) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes: vec![PopulationUniverse {
                    population: self.model.key(self.model.population),
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

    /// Recompile and replay one state-graph witness through FR-170.
    pub(crate) fn replay_witness(
        &self,
        claim: &CheckedStateGraphClaim,
        over: Option<ModelInstance>,
        path: ModelPath,
        source: WitnessSource,
        keys: &[&str],
    ) -> GraphReplayResult {
        let source_wire: (String, String, String, String) = (
            "agent-ix".to_owned(),
            "graph".to_owned(),
            "git".to_owned(),
            "1".to_owned(),
        );
        let byte_provision = vec![(
            "agent-ix".to_owned(),
            "graph".to_owned(),
            self.source.clone(),
        )];
        let mut documents = vec![self.document.clone()];
        documents.extend(self.snapshots.values().cloned());
        let obligation = crate::fixture_ids::obligation_identity();
        let occurrence = claim
            .occurrence()
            .expect("a checked claim has an occurrence");
        let envelope = GraphEvidenceEnvelope::new(
            obligation.clone(),
            occurrence.clone(),
            occurrence.node().clone(),
            Vec::new(),
            UNLIMITED,
            Vec::new(),
            BackendId::new("qsl-en-1"),
            GraphEvidence::Witness {
                paths: vec![(WitnessPath::Path(path), source)],
                over,
            },
        );
        let wire = ReplayRequestWire {
            package_name: "package".to_owned(),
            profile_selections: Vec::new(),
            sources: vec![source_wire],
            dependencies: Vec::new(),
            replay_target: RequestTargetWire::default(),
            source: ReplaySource::ModelGraph(Box::new(envelope)),
            obligation_identity: obligation,
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
        let subject = ModelTraceSubject {
            timed: None,
            initial: self.initial.clone(),
            universes: vec![PopulationUniverse {
                population: self.model.key(self.model.population),
                keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            }],
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
        };
        let ReplayResult::ModelGraph(replayed) =
            qsl_replay::replay_model_graph(wire, &subject, ReplayLimits::default())
                .expect("the state-graph witness replays")
        else {
            panic!("state-graph replay returns its typed result arm")
        };
        replayed
    }

    /// The reference of the object `key` of the world's type.
    pub(crate) fn reference(&self, key: &str) -> ObjectReference {
        let (effective, universe) = declared_type_universe_for(
            self.package.graph().model_selections(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &self.model.key(self.model.object),
        )
        .expect("the universe derives")
        .expect("the type is declared");
        ObjectReference::new(universe, effective, ObjectId::new(key).expect("an id"))
    }

    /// The checked claim `name`.
    pub(crate) fn claim(&self, name: &str) -> &CheckedStateGraphClaim {
        self.package
            .graph()
            .state_graph_claims()
            .iter()
            .find(|claim| claim.name() == name)
            .unwrap_or_else(|| panic!("`{name}` checked"))
    }

    /// The item `name`; its instances are the subject's universe keys.
    pub(crate) fn item(&self, name: &str) -> StateGraphItem<'_> {
        StateGraphItem::new(self.claim(name))
    }

    /// A request over universe `keys` for `items`, with no deadlock item, no
    /// horizon, `limits` and no seed.
    pub(crate) fn request<'w>(
        &'w self,
        keys: &[&str],
        items: Vec<StateGraphItem<'w>>,
        limits: ModelCheckLimits,
    ) -> StateGraphRequest<'w> {
        StateGraphRequest {
            subject: self.subject(keys, UNLIMITED),
            items,
            reductions: Default::default(),
            deadlock_freedom: false,
            max_depth: None,
            limits,
            seed: None,
        }
    }

    /// The value of `field` of the object `key` at `state`.
    pub(crate) fn value(&self, state: &ModelSimState, key: &str, field: &str) -> i64 {
        match state.state().populations[&self.model.key(self.model.population)][key]
            .fields
            .get(&self.model.key(&format!("{}/{field}", self.model.object)))
        {
            Some(Value::Integer(value)) => value.to_string().parse().expect("a small value"),
            other => panic!("{key}.{field} is not an integer: {other:?}"),
        }
    }
}

/// ADR-022 §7.1's `ConfigVersion`: `attemptUpdate` advances `versionNumber`
/// modulo 3. The field's domain is `Int[0, 3]`: the post clause keeps every
/// reachable value in `{0, 1, 2}`, so the reachable graph is §7.1's.
pub(crate) fn config_version() -> Model {
    Model {
        package: "test/config",
        alias: "Config",
        object: "ConfigVersion",
        population: "config_history",
        symmetric: false,
        fields: vec![Field {
            name: "versionNumber",
            range: Some((0, 3)),
        }],
        operations: vec![Operation {
            name: "attemptUpdate",
            modifies: &["versionNumber"],
        }],
    }
}

pub(crate) fn symmetric_config_version() -> Model {
    Model {
        symmetric: true,
        ..config_version()
    }
}

/// §7.1's contract.
pub(crate) const CONFIG_CONTRACT: &str =
    "post Advance using v on Config::ConfigVersion::attemptUpdate { \
     (pre(self.versionNumber) < 2 and self.versionNumber = pre(self.versionNumber) + 1) or \
     (pre(self.versionNumber) = 2 and self.versionNumber = 0) }\n";

/// §7.1's claims.
pub(crate) const REACHES_TWO: &str =
    "possible ReachesTwo using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }\n";
pub(crate) const REACHES_THREE: &str =
    "possible ReachesThree using v over (c: Config::ConfigVersion) { c.versionNumber = 3 }\n";
/// FR-166-AC-5's and FR-168-AC-5's target with no value at `versionNumber`
/// 2.
pub(crate) const DIVIDES: &str = "possible Divides using v over (c: Config::ConfigVersion) { \
     2 div (2 - c.versionNumber) = 2 }\n";

/// §7.1's world: universe `{a, b}`, one snapshot per entry of `starts`, each
/// `(va, vb)`.
pub(crate) fn config_world(claims: &str, starts: &[(i64, i64)]) -> World {
    let starts: Vec<Vec<i64>> = starts.iter().map(|(a, b)| vec![*a, *b]).collect();
    config_world_over("", claims, &["a", "b"], &starts)
}

/// ADR-021 §7.2's three-counter POR instance, with `ReachesTwo` bound to
/// `x = a` and all three counters initially zero.
pub(crate) fn por_reachestwo_world() -> World {
    let model = Model {
        package: "test/por",
        alias: "M",
        object: "Counter",
        population: "counters",
        symmetric: false,
        fields: vec![Field {
            name: "v",
            range: Some((0, 2)),
        }],
        operations: vec![Operation {
            name: "bump",
            modifies: &["v"],
        }],
    };
    let source = concat!(
        "pre CanBump using v on M::Counter::bump { self.v < 2 }\n",
        "post Bumped using v on M::Counter::bump { (pre(self.v) = 0 and self.v = 1) or (pre(self.v) = 1 and self.v = 2) }\n",
        "possible ReachesTwo using v over (x: M::Counter) { x.v = 2 }\n",
    );
    let counters: [Object<'_>; 3] = [("a", &[("v", 0)]), ("b", &[("v", 0)]), ("c", &[("v", 0)])];
    World::new(model, source, &[&counters])
}

/// §7.1's world with `extra` added to its contract, universe `keys`, and one
/// snapshot per entry of `starts`, each the version of every key in order.
pub(crate) fn config_world_over(
    extra: &str,
    claims: &str,
    keys: &[&str],
    starts: &[Vec<i64>],
) -> World {
    config_world_over_model(config_version(), extra, claims, keys, starts)
}

pub(crate) fn symmetric_config_world_over(
    extra: &str,
    claims: &str,
    keys: &[&str],
    starts: &[Vec<i64>],
) -> World {
    config_world_over_model(symmetric_config_version(), extra, claims, keys, starts)
}

fn config_world_over_model(
    model: Model,
    extra: &str,
    claims: &str,
    keys: &[&str],
    starts: &[Vec<i64>],
) -> World {
    let fields: Vec<Vec<[(&str, i64); 1]>> = starts
        .iter()
        .map(|values| {
            values
                .iter()
                .map(|value| [("versionNumber", *value)])
                .collect()
        })
        .collect();
    let objects: Vec<Vec<Object<'_>>> = fields
        .iter()
        .map(|values| {
            keys.iter()
                .zip(values)
                .map(|(key, field)| (*key, field.as_slice()))
                .collect()
        })
        .collect();
    let starts: Vec<&[Object<'_>]> = objects.iter().map(Vec::as_slice).collect();
    World::new(model, &format!("{CONFIG_CONTRACT}{extra}{claims}"), &starts)
}

/// §7.2's game: `phase` is `Start` 0, `Mid` 1, `Lost` 2 or `Won` 3, so
/// the operation names alone give `lose` before `win`; with `count`, the
/// field `n: Int[0, 50]` and the operation `celebrate` (FR-168-AC-4); with
/// `restart`, the operation `restart`.
pub(crate) fn game(count: bool, restart: bool) -> Model {
    let mut fields = vec![Field {
        name: "phase",
        range: Some((0, 3)),
    }];
    let mut operations = vec![
        Operation {
            name: "play",
            modifies: &["phase"],
        },
        Operation {
            name: "win",
            modifies: &["phase"],
        },
        Operation {
            name: "lose",
            modifies: &["phase"],
        },
    ];
    if count {
        fields.push(Field {
            name: "n",
            range: Some((0, 50)),
        });
        operations.push(Operation {
            name: "celebrate",
            modifies: &["n"],
        });
    }
    if restart {
        operations.push(Operation {
            name: "restart",
            modifies: &["phase"],
        });
    }
    Model {
        package: "test/game",
        alias: "G",
        object: "Game",
        population: "games",
        symmetric: false,
        fields,
        operations,
    }
}

/// §7.2's contract: `play` `Start -> Mid`, `win` `Mid -> Won`, `lose`
/// `Mid -> Lost`.
pub(crate) const GAME_CONTRACT: &str = "\
    pre CanPlay using v on G::Game::play { self.phase = 0 }\n\
    post Played using v on G::Game::play { self.phase = 1 }\n\
    pre CanWin using v on G::Game::win { self.phase = 1 }\n\
    post Won using v on G::Game::win { self.phase = 3 }\n\
    pre CanLose using v on G::Game::lose { self.phase = 1 }\n\
    post Lost using v on G::Game::lose { self.phase = 2 }\n";

/// FR-168-AC-4's `celebrate`.
pub(crate) const CELEBRATE: &str = "\
    pre CanCelebrate using v on G::Game::celebrate { self.phase = 3 and self.n < 50 }\n\
    post Celebrated using v on G::Game::celebrate { self.n - pre(self.n) = 1 }\n";

/// §7.2's `restart`: `Lost -> Start`.
pub(crate) const RESTART: &str = "\
    pre CanRestart using v on G::Game::restart { self.phase = 2 }\n\
    post Restarted using v on G::Game::restart { self.phase = 0 }\n";

/// §7.2's `CanStillWin`.
pub(crate) const CAN_STILL_WIN: &str =
    "always possible CanStillWin using v over (x: G::Game) { x.phase = 3 }\n";

/// §7.2's game world over `{g}`, initially `Start` (and `n` 0).
pub(crate) fn game_world(count: bool, restart: bool, body: &str) -> World {
    let mut contract = GAME_CONTRACT.to_owned();
    if count {
        contract.push_str(CELEBRATE);
    }
    if restart {
        contract.push_str(RESTART);
    }
    let fields: &[(&str, i64)] = if count {
        &[("phase", 0), ("n", 0)]
    } else {
        &[("phase", 0)]
    };
    World::new(
        game(count, restart),
        &format!("{contract}{body}"),
        &[&[("g", fields)]],
    )
}

/// §7.3's job: `a`, `b` and `done`, each `Int[0, 1]` for `false` and `true`.
pub(crate) fn job(reset: bool) -> Model {
    let mut operations = vec![
        Operation {
            name: "stepA",
            modifies: &["a"],
        },
        Operation {
            name: "stepB",
            modifies: &["b"],
        },
        Operation {
            name: "finish",
            modifies: &["done"],
        },
    ];
    if reset {
        operations.push(Operation {
            name: "reset",
            modifies: &["a"],
        });
    }
    let flag = |name| Field {
        name,
        range: Some((0, 1)),
    };
    Model {
        package: "test/job",
        alias: "M",
        object: "Job",
        population: "jobs",
        symmetric: false,
        fields: vec![flag("a"), flag("b"), flag("done")],
        operations,
    }
}

/// §7.3's contract; `sequenced` gives `stepB` the precondition `a and not
/// b`; `reset` adds `reset` (pre `a and not b`, post `not a`).
pub(crate) fn job_contract(sequenced: bool, reset: bool) -> String {
    let step_b = if sequenced {
        "self.a = 1 and self.b = 0"
    } else {
        "self.b = 0"
    };
    let mut contract = format!(
        "pre CanStepA using v on M::Job::stepA {{ self.a = 0 }}\n\
         post SteppedA using v on M::Job::stepA {{ self.a = 1 }}\n\
         pre CanStepB using v on M::Job::stepB {{ {step_b} }}\n\
         post SteppedB using v on M::Job::stepB {{ self.b = 1 }}\n\
         pre CanFinish using v on M::Job::finish {{ self.a = 1 and self.b = 1 and self.done = 0 }}\n\
         post Finished using v on M::Job::finish {{ self.done = 1 }}\n"
    );
    if reset {
        contract.push_str(
            "pre CanReset using v on M::Job::reset { self.a = 1 and self.b = 0 }\n\
             post Reset using v on M::Job::reset { self.a = 0 }\n",
        );
    }
    contract
}

/// §7.3's `InOneWay`.
pub(crate) const IN_ONE_WAY: &str = "unique path InOneWay using v over (j: M::Job) \
     from (j.a = 0 and j.b = 0) to (j.done = 1);\n";

/// §7.3's job world over `{j}`, initially all false.
pub(crate) fn job_world(sequenced: bool, reset: bool) -> World {
    job_world_with_claims(sequenced, reset, "")
}

/// §7.3's job with additional state-graph claims for shared-exploration tests.
pub(crate) fn job_world_with_claims(sequenced: bool, reset: bool, claims: &str) -> World {
    World::new(
        job(reset),
        &format!("{}{IN_ONE_WAY}{claims}", job_contract(sequenced, reset)),
        &[&[("j", &[("a", 0), ("b", 0), ("done", 0)])]],
    )
}

/// FR-168-AC-9's `Fork`: `pos: Int[0, 4]`, initially 0.
pub(crate) fn fork_world(body: &str) -> World {
    moves_world(
        ["test/fork", "F", "Fork", "forks"],
        4,
        &[
            ("toA", 0, 1),
            ("toB", 0, 2),
            ("fromA", 1, 3),
            ("fromB", 2, 4),
            ("back", 3, 0),
        ],
        body,
    )
}

/// FR-168-AC-10's `Ladder`: `pos: Int[0, 3]`, initially 0;
/// `a` (0 to 1), `b` (0 to 2), `c` and `d` (each 1 to 3) and `e` (2 to 3).
pub(crate) fn ladder_world(body: &str) -> World {
    moves_world(
        ["test/ladder", "L", "Ladder", "ladders"],
        3,
        &[
            ("a", 0, 1),
            ("b", 0, 2),
            ("c", 1, 3),
            ("d", 1, 3),
            ("e", 2, 3),
        ],
        body,
    )
}

/// FR-168-AC-10's `Detour`: `pos: Int[0, 5]`, initially 0;
/// `s` (0 to 1), `t` (0 to 2), `u` (1 to 4), `w` (2 to 5), `y` (2 to 4)
/// and `x` (4 to 5).
pub(crate) fn detour_world(body: &str) -> World {
    moves_world(
        ["test/detour", "D", "Detour", "detours"],
        5,
        &[
            ("s", 0, 1),
            ("t", 0, 2),
            ("u", 1, 4),
            ("w", 2, 5),
            ("y", 2, 4),
            ("x", 4, 5),
        ],
        body,
    )
}

/// A world over the one object `f` of `[package, alias, object,
/// population]`, with `pos: Int[0, high]`, initially 0, and one operation
/// per `(name, from, to)`: pre `pos = from`, post `pos = to`, each framed
/// `modifies self.pos`.
fn moves_world(
    [package, alias, object, population]: [&'static str; 4],
    high: i64,
    moves: &[(&'static str, i64, i64)],
    body: &str,
) -> World {
    let model = Model {
        package,
        alias,
        object,
        population,
        symmetric: false,
        fields: vec![Field {
            name: "pos",
            range: Some((0, high)),
        }],
        operations: moves
            .iter()
            .map(|&(name, _, _)| Operation {
                name,
                modifies: &["pos"],
            })
            .collect(),
    };
    let contract = moves
        .iter()
        .map(|(name, from, to)| {
            format!(
                "pre Can{name} using v on {alias}::{object}::{name} {{ self.pos = {from} }}\n\
                 post Did{name} using v on {alias}::{object}::{name} {{ self.pos = {to} }}\n"
            )
        })
        .collect::<String>();
    World::new(
        model,
        &format!("{contract}{body}"),
        &[&[("f", &[("pos", 0)])]],
    )
}
