// SPDX-License-Identifier: AGPL-3.0-or-later
//! The probabilistic models of FR-196's tests (TC-631, TC-641, TC-630): each
//! a `test/<name>` domain package with one object type in one population
//! over the universe `{<key>}`, a unit selecting it as `M` that writes its
//! contracts, FR-185 declarations and claims, and one initial snapshot.
//!
//! The models are ADR-024 §7's and ADR-028 §15's. `Service`'s `phase` is
//! `Int[0, 2]` (`0` Idle, `1` Busy, `2` Done), its `d` is `Int[1, 12]` and
//! its `outcome` a Boolean `ok`, since FR-120 has no encoding for an
//! enumeration or quantity value; the ADR's `1 ms`, `3 ms` and `12 ms` are
//! `1`, `3` and `12`.

use std::collections::BTreeMap;

use qsl_analyze::exact_probabilistic::ProductRequest;
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use qsl_package::CheckedPackage;
use qsl_replay::ModelCheckLimits;
use qsl_semantics::check::{CheckedProbabilisticClaim, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::ScalarLimits;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The native Boolean type reference.
pub(crate) const BOOLEAN: &str = "ix://quire/native/Boolean";

/// One operation: its name, its parameters as `(name, type reference)` and
/// the fields its frame lets it write.
pub(crate) struct Operation {
    pub(crate) name: &'static str,
    pub(crate) parameters: Vec<(&'static str, String)>,
    pub(crate) modifies: Vec<&'static str>,
    /// Whether the frame lets it delete the object.
    pub(crate) deletes: bool,
}

/// One fixture model.
pub(crate) struct Model {
    /// The domain package's identity, `test/<name>`.
    pub(crate) package: &'static str,
    /// The object type's name.
    pub(crate) object: &'static str,
    /// The population's name.
    pub(crate) population: &'static str,
    /// The one object's key.
    pub(crate) key: &'static str,
    /// Bounded integer types: name, lower, upper.
    pub(crate) integers: Vec<(&'static str, i64, i64)>,
    /// The object type's fields: name and type reference.
    pub(crate) fields: Vec<(&'static str, String)>,
    /// The object type's operations.
    pub(crate) operations: Vec<Operation>,
    /// The unit's declarations after the model selection.
    pub(crate) body: String,
    /// The initial object's field values, as snapshot JSON.
    pub(crate) initial: Json,
}

impl Model {
    /// The identity of `suffix` in this package.
    pub(crate) fn ident(&self, suffix: &str) -> String {
        format!("ix://{}/{suffix}", self.package)
    }

    fn origin(identity: &str) -> Json {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    }

    fn definition(&self, name: &str, kind: &str, members: Json) -> Json {
        let identity = self.ident(name);
        let mut node = json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": self.package, "name": kind}, "roles": [],
            "origin": Self::origin(&identity), "constraints": [], "extensions": [],
            "unknownPolicy": "reject",
        });
        for (key, value) in members.as_object().expect("members") {
            node[key] = value.clone();
        }
        node
    }

    fn integer_type(&self, name: &str, lower: i64, upper: i64) -> Json {
        let identity = self.ident(name);
        let constraint = |keyword: &str, value: i64| {
            json!({
                "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
                "operands": {"value": value.to_string()}, "appliesTo": "ix://quire/native/Integer",
                "diagnosticCode": format!("bound.{keyword}"), "origin": Self::origin(&identity),
            })
        };
        self.definition(
            name,
            "value_type",
            json!({"scalar": "integer",
                "constraints": [constraint("min", lower), constraint("max", upper)]}),
        )
    }

    fn typed(identity: String, name: &str, type_ref: &str) -> Json {
        json!({
            "identity": identity.clone(), "name": name, "typeRef": type_ref,
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": Self::origin(&identity),
        })
    }

    fn operation(&self, operation: &Operation, receiver_scoped: bool) -> Json {
        let identity = self.ident(&format!("{}/{}", self.object, operation.name));
        let parameters: Vec<Json> = operation
            .parameters
            .iter()
            .map(|(name, type_ref)| Self::typed(format!("{identity}/{name}"), name, type_ref))
            .collect();
        let modifies: Vec<String> = operation
            .modifies
            .iter()
            .map(|field| self.ident(&format!("{}/{field}", self.object)))
            .collect();
        let deletes = if operation.deletes {
            vec![self.ident(self.object)]
        } else {
            vec![]
        };
        // A receiver-scoped frame lets the operation write its receiver's
        // fields only, so a population of several objects keeps the others.
        let frame = if receiver_scoped {
            json!({"modifies": [], "modifies_receiver": modifies, "creates": [],
                "deletes": deletes})
        } else {
            json!({"modifies": modifies, "creates": [], "deletes": deletes})
        };
        json!({
            "identity": identity, "name": operation.name, "params": parameters,
            "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": self.ident("spec"), "path": "spec.qspec",
                "startLine": 1, "startColumn": 1}},
            "frame": frame,
        })
    }

    /// The domain package document.
    fn document(&self, symmetric: bool) -> Vec<u8> {
        let mut kinds = vec![
            ("object_type", meaning::OBJECT_TYPE),
            ("population", meaning::POPULATION),
        ];
        if !self.integers.is_empty() {
            kinds.push(("value_type", meaning::VALUE_TYPE));
        }
        let constructs: Vec<Json> = kinds
            .into_iter()
            .map(|(name, meaning)| {
                json!({"kind": {"module": self.package, "name": name}, "moduleVersion": "1.0.0",
                "manifestDigest": PLACEHOLDER,
                "construct": {"identity": "none", "shape": "record", "members": {},
                    "meaning": meaning}})
            })
            .collect();
        let fields: Vec<Json> = self
            .fields
            .iter()
            .map(|(name, type_ref)| {
                Self::typed(
                    self.ident(&format!("{}/{name}", self.object)),
                    name,
                    type_ref,
                )
            })
            .collect();
        let mut types: Vec<Json> = self
            .integers
            .iter()
            .map(|(name, lower, upper)| self.integer_type(name, *lower, *upper))
            .collect();
        types.push(self.definition(
            self.object,
            "object_type",
            json!({
                "supertypes": [], "fields": fields,
                "operations": self.operations.iter().map(|operation| self.operation(operation, symmetric))
                    .collect::<Vec<_>>(),
                "relationships": [],
            }),
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
                "members": [self.ident(self.object)], "extent": "closed",
                "symmetric": symmetric,
                "origin": Self::origin(&population),
            }],
        })
        .to_string()
        .into_bytes()
    }
}

/// A checked fixture model with its initial snapshot.
pub(crate) struct World {
    pub(crate) package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: Vec<DocumentRef>,
    universe: PopulationUniverse,
}

/// The evaluation budget: ample for every fixture.
pub(crate) const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

impl World {
    /// `model`, checked.
    pub(crate) fn new(model: &Model) -> Self {
        Self::build(
            model,
            &[model.key],
            &[model.key],
            &[&model.initial],
            false,
            false,
        )
    }

    /// `model`, checked, over a universe of `keys`, every one of them live in
    /// the initial snapshot.
    pub(crate) fn with_objects(model: &Model, keys: &[&str]) -> Self {
        Self::build(model, keys, keys, &[&model.initial], false, false)
    }

    /// `model`, checked, over a universe of `keys`; only the object under
    /// `model.key` is in the initial snapshot.
    pub(crate) fn with_universe(model: &Model, keys: &[&str]) -> Self {
        Self::build(model, keys, &[model.key], &[&model.initial], false, false)
    }

    /// `model`, checked, with one initial snapshot per field set of
    /// `initials`, in order.
    pub(crate) fn with_initials(model: &Model, initials: &[Json]) -> Self {
        let initials: Vec<&Json> = initials.iter().collect();
        Self::build(model, &[model.key], &[model.key], &initials, false, false)
    }

    /// `model`, checked, with the integer operators of
    /// `quire.value.complete/v1` available as the profile `vc`.
    pub(crate) fn with_value_profile(model: &Model) -> Self {
        Self::build(
            model,
            &[model.key],
            &[model.key],
            &[&model.initial],
            false,
            true,
        )
    }

    /// `model`, checked, with its population declared `symmetric` and its
    /// operations scoped to their receivers, over a universe of `keys`,
    /// every one of them live in the initial snapshot.
    pub(crate) fn symmetric_objects(model: &Model, keys: &[&str]) -> Self {
        Self::build(model, keys, keys, &[&model.initial], true, false)
    }

    fn build(
        model: &Model,
        keys: &[&str],
        live: &[&str],
        initials: &[&Json],
        symmetric: bool,
        value_profile: bool,
    ) -> Self {
        let document = model.document(symmetric);
        let packages = package_input([document.as_slice()]);
        let digest = *packages.keys().next().expect("one model");
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\n\
             profile ev = \"quire.temporal.event-position.false-extension/v1\";\n\
             profile inf = \"quire.temporal.infinite-trace/v1\";\n{}\
             model M = \"{}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n{}\n",
            if value_profile {
                "profile vc = \"quire.value.complete/v1\";\n"
            } else {
                ""
            },
            model.package,
            hex(&digest),
            model.body
        );
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("test", "fr-188", "fixture", "1"),
            "fr-188.native",
            unit.as_bytes(),
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
        .expect("the kept operation clauses parse");
        let graph = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            built,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .with_unwritten_terminals()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:#?}"));

        let header = json!({"identity": model.package, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&digest))});
        let mut snapshots = BTreeMap::new();
        let mut initial = Vec::new();
        for (index, fields) in initials.iter().enumerate() {
            let identity = format!("s{index}");
            let snapshot = json!({
                "format": "quire.state.snapshot/v1",
                "identity": {"authority": "test", "identity": identity,
                    "revision_namespace": "fixture", "revision": "fixture:1"},
                "observation": "current", "model": header,
                "anchor": {"kind": "initialization", "name": "start"},
                "populations": [{"population": model.ident(model.population), "complete": true,
                    "objects": live.iter().map(|key| json!({"key": key,
                        "type": model.ident(model.object), "fields": fields}))
                        .collect::<Vec<_>>()}],
            })
            .to_string()
            .into_bytes();
            let read = quire_canonical::read(&snapshot, u64::MAX).expect("JSON fixture");
            let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
                .expect("canonical fixture")
                .as_bytes();
            snapshots.insert(digest, snapshot);
            initial.push(DocumentRef {
                authority: "test".to_owned(),
                identity,
                revision_namespace: "fixture".to_owned(),
                revision: "fixture:1".to_owned(),
                digest,
            });
        }
        let population = qsl_semantics::model::key::DeclarationKey {
            package: model.package.to_owned(),
            node: model.ident(model.population),
        };
        Self {
            package: CheckedPackage::link(graph),
            packages,
            snapshots,
            initial,
            universe: PopulationUniverse {
                population,
                keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            },
        }
    }

    fn simulation(&self) -> ModelSimulationRequest<'_> {
        ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: vec![self.universe.clone()],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
        }
    }

    /// The claim declared `name`.
    pub(crate) fn claim(&self, name: &str) -> &CheckedProbabilisticClaim {
        self.package
            .graph()
            .probabilistic_model()
            .claim(name)
            .unwrap_or_else(|| panic!("the claim {name} checks"))
    }

    /// The model subject over the initial snapshot.
    pub(crate) fn subject(&self) -> ModelSubject<'_> {
        ModelSubject {
            simulation: self.simulation(),
            proof_bounds: Vec::new(),
        }
    }

    /// The product request of the claim `name` under `limits`.
    pub(crate) fn request(&self, name: &str, limits: ModelCheckLimits) -> ProductRequest<'_> {
        ProductRequest {
            subject: self.subject(),
            claim: self.claim(name),
            limits,
        }
    }
}

fn int(package: &'static str, name: &str) -> String {
    format!("ix://{package}/{name}")
}

/// ADR-024 §7.2's `Service` as FR-187-AC-1 declares it, with `extra`
/// appended to its declarations. The object is `s`; `phase` is `0` Idle, `1`
/// Busy, `2` Done.
pub(crate) fn service(extra: &str) -> Model {
    const PACKAGE: &str = "test/service";
    Model {
        package: PACKAGE,
        object: "Server",
        population: "servers",
        key: "s",
        integers: vec![("Phase", 0, 2), ("Attempts", 0, 2), ("Millis", 1, 12)],
        fields: vec![
            ("phase", int(PACKAGE, "Phase")),
            ("attempts", int(PACKAGE, "Attempts")),
        ],
        operations: vec![
            Operation {
                name: "request",
                parameters: vec![],
                modifies: vec!["phase", "attempts"],
                deletes: false,
            },
            Operation {
                name: "attempt",
                parameters: vec![("d", int(PACKAGE, "Millis")), ("ok", BOOLEAN.to_owned())],
                modifies: vec!["phase", "attempts"],
                deletes: false,
            },
            Operation {
                name: "reset",
                parameters: vec![],
                modifies: vec!["phase"],
                deletes: false,
            },
        ],
        body: format!(
            "pre Idle using v on M::Server::request {{ self.phase = 0 }}\n\
             post Started using v on M::Server::request {{ self.phase = 1 and self.attempts = 0 }}\n\
             pre Busy using v on M::Server::attempt {{ self.phase = 1 }}\n\
             post Attempted using v on M::Server::attempt {{ \
                self.attempts - pre(self.attempts) = 1 and \
                ((ok and self.phase = 2) or \
                 (not ok and pre(self.attempts) = 1 and self.phase = 2) or \
                 (not ok and pre(self.attempts) = 0 and self.phase = 1)) }}\n\
             pre Done using v on M::Server::reset {{ self.phase = 2 }}\n\
             post Reset using v on M::Server::reset {{ self.phase = 0 }}\n\
             random Server::attempt(d) ~ {{ 1: 90, 3: 7, 12: 3 }};\n\
             random Server::attempt(ok) ~ {{ true: 98, false: 2 }};\n\
             reward duration on Server::attempt = d;\n\
             workload Steady on M {{ weight Server::request = 1; weight Server::attempt = 1; \
                weight Server::reset = 1; }}\n{extra}"
        ),
        initial: json!({"phase": {"integer": "0"}, "attempts": {"integer": "0"}}),
    }
}

/// ADR-028 §15.4's `Link`, with `extra` appended: two sends that lose with
/// probability `1/10` and `1/5`, a `cost` of 1 on each, terminal once
/// delivered or after two attempts.
pub(crate) fn link(extra: &str) -> Model {
    const PACKAGE: &str = "test/link";
    let send = |name| Operation {
        name,
        parameters: vec![("lost", BOOLEAN.to_owned())],
        modifies: vec!["delivered", "attempts"],
        deletes: false,
    };
    let contract = |name: &str| {
        format!(
            "pre Ready_{name} using v on M::Msg::{name} {{ not self.delivered and self.attempts < 2 }}\n\
             post Sent_{name} using v on M::Msg::{name} {{ \
                self.attempts - pre(self.attempts) = 1 and self.delivered = (not lost) }}\n"
        )
    };
    Model {
        package: PACKAGE,
        object: "Msg",
        population: "msgs",
        key: "m",
        integers: vec![("Attempts", 0, 2)],
        fields: vec![
            ("delivered", BOOLEAN.to_owned()),
            ("attempts", int(PACKAGE, "Attempts")),
        ],
        operations: vec![send("send_a"), send("send_b")],
        body: format!(
            "{}{}random Msg::send_a(lost) ~ {{ true: 1, false: 9 }};\n\
             random Msg::send_b(lost) ~ {{ true: 1, false: 4 }};\n\
             reward cost on Msg::send_a = 1;\n\
             reward cost on Msg::send_b = 1;\n\
             terminal Stop using v on M when {{ self.delivered or self.attempts = 2 }};\n\
             workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; }}\n{extra}",
            contract("send_a"),
            contract("send_b")
        ),
        initial: json!({"delivered": {"boolean": false}, "attempts": {"integer": "0"}}),
    }
}

/// ADR-024 §7.1's `Health`: `tick(fault)` writes `healthy` under the
/// postcondition `post` with `fault`'s weights `weights`, and, with `spare`,
/// a second Boolean `spare` that the FR-187-AC-3 variants leave loose or
/// pin. `extra` is appended to the declarations.
pub(crate) fn health(post: &str, weights: &str, spare: bool, extra: &str) -> Model {
    let mut fields = vec![("healthy", BOOLEAN.to_owned())];
    let mut modifies = vec!["healthy"];
    let mut initial = json!({"healthy": {"boolean": true}});
    if spare {
        fields.push(("spare", BOOLEAN.to_owned()));
        modifies.push("spare");
        initial["spare"] = json!({"boolean": false});
    }
    Model {
        package: "test/health",
        object: "Node",
        population: "nodes",
        key: "n",
        integers: vec![],
        fields,
        operations: vec![Operation {
            name: "tick",
            parameters: vec![("fault", BOOLEAN.to_owned())],
            modifies,
            deletes: false,
        }],
        body: format!(
            "post Tick using v on M::Node::tick {{ {post} }}\n\
             random Node::tick(fault) ~ {{ {weights} }};\n\
             workload Steady on M {{ weight Node::tick = 1; }}\n{extra}"
        ),
        initial,
    }
}

/// A cell whose `inc` postcondition divides by zero at a candidate post-state
/// (`v = 2`), so the contract conjunction is undecided at every state
/// (FR-189-AC-5's undecided successor, FR-120's `ContractUndetermined`).
pub(crate) fn undetermined(extra: &str) -> Model {
    const PACKAGE: &str = "test/undetermined";
    Model {
        package: PACKAGE,
        object: "Cell",
        population: "cells",
        key: "c",
        integers: vec![("Level", 0, 2)],
        fields: vec![("v", int(PACKAGE, "Level"))],
        operations: vec![Operation {
            name: "inc",
            parameters: vec![],
            modifies: vec!["v"],
            deletes: false,
        }],
        body: format!(
            "post Inc using vc on M::Cell::inc {{ \
                self.v - pre(self.v) = 1 and 6 mod (2 - self.v) >= 0 }}\n\
             workload Even on M {{ weight Cell::inc = 1; }}\n{extra}"
        ),
        initial: json!({"v": {"integer": "0"}}),
    }
}

/// `Health` as ADR-024 §7.1 has it: one fault in `10^7` ticks ends health.
pub(crate) fn no_fault(extra: &str) -> Model {
    health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9999999",
        false,
        extra,
    )
}

/// Checks FR-196-AC-5's `Coin`, adapted to what S3 types today: one object `c` with
/// `v: Int[0, 1]`, initially 0, and `drop(b: Int[0, 1])`, which draws `b`
/// with equal weights and deletes the coin when `b = 0`. A claim bound to `c`
/// reads `c.v` as undefined (`absent-key`) once the coin is gone.
pub(crate) fn coin(extra: &str) -> Model {
    const PACKAGE: &str = "test/coin";
    Model {
        package: PACKAGE,
        object: "Coin",
        population: "coins",
        key: "c",
        integers: vec![("Bit", 0, 1)],
        fields: vec![("v", int(PACKAGE, "Bit"))],
        operations: vec![Operation {
            name: "drop",
            parameters: vec![("b", int(PACKAGE, "Bit"))],
            modifies: vec![],
            deletes: true,
        }],
        body: format!(
            "bound B on M::coins [1]\n\
             post Drop using v on M::Coin::drop {{ count(o in M::coins where true) = b }}\n\
             random Coin::drop(b) ~ {{ 0: 1, 1: 1 }};\n\
             workload Even on M {{ weight Coin::drop = 1; }}\n{extra}"
        ),
        initial: json!({"v": {"integer": "0"}}),
    }
}

/// A coin that is tossed with `toss(b: Int[0, 1])`: `b` is drawn with equal
/// weights and the frame may delete the coin, with no postcondition, so each
/// draw has two post-states, the coin kept and the coin gone.
pub(crate) fn tossed_coin(extra: &str) -> Model {
    const PACKAGE: &str = "test/tossed";
    Model {
        package: PACKAGE,
        object: "Coin",
        population: "coins",
        key: "c",
        integers: vec![("Bit", 0, 1)],
        fields: vec![("v", int(PACKAGE, "Bit"))],
        operations: vec![Operation {
            name: "toss",
            parameters: vec![("b", int(PACKAGE, "Bit"))],
            modifies: vec![],
            deletes: true,
        }],
        body: format!(
            "random Coin::toss(b) ~ {{ 0: 1, 1: 1 }};\n\
             workload Even on M {{ weight Coin::toss = 1; }}\n{extra}"
        ),
        initial: json!({"v": {"integer": "0"}}),
    }
}

/// A coin that must be armed before it can be dropped: `arm` sets `armed`,
/// and `drop(b: Int[0, 1])` then deletes the coin when `b = 0`. A claim
/// bound to `c` reads `c.armed` as undefined once the coin is gone, two
/// steps in.
pub(crate) fn armed_coin(extra: &str) -> Model {
    const PACKAGE: &str = "test/armed";
    Model {
        package: PACKAGE,
        object: "Coin",
        population: "coins",
        key: "c",
        integers: vec![("Bit", 0, 1)],
        fields: vec![("armed", BOOLEAN.to_owned())],
        operations: vec![
            Operation {
                name: "arm",
                parameters: vec![],
                modifies: vec!["armed"],
                deletes: false,
            },
            Operation {
                name: "drop",
                parameters: vec![("b", int(PACKAGE, "Bit"))],
                modifies: vec![],
                deletes: true,
            },
        ],
        body: format!(
            "bound B on M::coins [1]\n\
             pre Loose using v on M::Coin::arm {{ not self.armed }}\n\
             post Armed using v on M::Coin::arm {{ self.armed }}\n\
             pre Ready using v on M::Coin::drop {{ self.armed }}\n\
             post Drop using v on M::Coin::drop {{ count(o in M::coins where true) = b }}\n\
             random Coin::drop(b) ~ {{ 0: 1, 1: 1 }};\n\
             workload Even on M {{ weight Coin::arm = 1; weight Coin::drop = 1; }}\n{extra}"
        ),
        initial: json!({"armed": {"boolean": false}}),
    }
}

/// ADR-028 §15.6's `Coin` process: `flip(heads)` ends it with probability
/// one half, `wait` leaves it as it is, and it is terminal once `done`.
pub(crate) fn process(extra: &str) -> Model {
    Model {
        package: "test/proc",
        object: "Proc",
        population: "procs",
        key: "p",
        integers: vec![],
        fields: vec![("done", BOOLEAN.to_owned())],
        operations: vec![
            Operation {
                name: "flip",
                parameters: vec![("heads", BOOLEAN.to_owned())],
                modifies: vec!["done"],
                deletes: false,
            },
            Operation {
                name: "wait",
                parameters: vec![],
                modifies: vec!["done"],
                deletes: false,
            },
        ],
        body: format!(
            "pre Live_flip using v on M::Proc::flip {{ not self.done }}\n\
             post Flipped using v on M::Proc::flip {{ self.done = heads }}\n\
             pre Live_wait using v on M::Proc::wait {{ not self.done }}\n\
             post Waited using v on M::Proc::wait {{ self.done = pre(self.done) }}\n\
             random Proc::flip(heads) ~ {{ true: 1, false: 1 }};\n\
             terminal Done using v on M when {{ self.done }};\n\
             workload Even on M {{ weight Proc::flip = 1; weight Proc::wait = 1; }}\n{extra}"
        ),
        initial: json!({"done": {"boolean": false}}),
    }
}

/// FR-189-AC-7's `Wide`: `spin(x: Boolean)` writes `v = x`, with `x` drawn
/// by weights whose total, `2^256 + 1`, is above the draw range.
pub(crate) fn wide(extra: &str) -> Model {
    Model {
        package: "test/wide",
        object: "Wide",
        population: "wides",
        key: "w",
        integers: vec![],
        fields: vec![("v", BOOLEAN.to_owned())],
        operations: vec![Operation {
            name: "spin",
            parameters: vec![("x", BOOLEAN.to_owned())],
            modifies: vec!["v"],
            deletes: false,
        }],
        body: format!(
            "post Spun using v on M::Wide::spin {{ self.v = x }}\n\
             random Wide::spin(x) ~ {{ true: 1, false: \
             115792089237316195423570985008687907853269984665640564039457584007913129639936 }};\n\
             workload Unit on M {{ weight Wide::spin = 1; }}\n{extra}"
        ),
        initial: json!({"v": {"boolean": false}}),
    }
}

/// FR-189-AC-4's activation variant of `Service`: `request(go: Boolean)`
/// starts the server (`phase = 1`) when `go` and leaves it `Idle`
/// otherwise, with `go ~ {true: 1, false: 1}`.
pub(crate) fn activated_service(extra: &str) -> Model {
    let mut model = service("");
    model.package = "test/activated";
    for operation in &mut model.operations {
        if operation.name == "request" {
            operation.parameters = vec![("go", BOOLEAN.to_owned())];
        }
        for (_, type_ref) in &mut operation.parameters {
            *type_ref = type_ref.replace("test/service", "test/activated");
        }
    }
    model.integers = vec![("Phase", 0, 2), ("Attempts", 0, 2), ("Millis", 1, 12)];
    model.fields = vec![
        ("phase", int("test/activated", "Phase")),
        ("attempts", int("test/activated", "Attempts")),
    ];
    model.body = model
        .body
        .replace(
            "post Started using v on M::Server::request { self.phase = 1 and self.attempts = 0 }",
            "post Started using v on M::Server::request { \
                (go and self.phase = 1 and self.attempts = 0) or \
                (not go and self.phase = 0 and self.attempts = pre(self.attempts)) }",
        )
        .replace(
            "random Server::attempt(d)",
            "random Server::request(go) ~ { true: 1, false: 1 };\nrandom Server::attempt(d)",
        );
    model.body.push_str(extra);
    model
}
