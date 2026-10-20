// SPDX-License-Identifier: AGPL-3.0-or-later
//! The probabilistic models of FR-187's and FR-188's tests (TC-622,
//! TC-623): each a `test/<name>` domain package with one object type in one
//! population over the universe `{<key>}`, a unit selecting it as `M` that
//! writes its contracts and FR-185 declarations, and one initial snapshot.
//!
//! FR-120 encodes no enum value in a state key or transition identity, and
//! domain-package intake admits no `Quantity<…>` parameter (STD-113), so
//! ADR-024 §7.2's `Service` stands in integers and Booleans for them:
//! `phase` is `Phase = Int[0, 2]` (`0` Idle, `1` Busy, `2` Done), `d` is
//! `Millis = Int[1, 12]` (`1 ms` is `1`), and `outcome` is `ok: Boolean`
//! (`Ok` is `true`).

use std::collections::BTreeMap;

use qsl_package::CheckedPackage;
use qsl_semantics::check::{PackageDeclarations, Workload};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};
use quire_exact::ScalarLimits;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use crate::simulation::TransitionSystem;

const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The native Boolean type reference.
pub(crate) const BOOLEAN: &str = "ix://quire/native/Boolean";

/// One operation: its name, its parameters as `(name, type reference)` and
/// the fields its frame lets it write.
pub(crate) struct Operation {
    pub(crate) name: &'static str,
    pub(crate) parameters: Vec<(&'static str, String)>,
    pub(crate) modifies: Vec<&'static str>,
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

    fn operation(&self, operation: &Operation) -> Json {
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
        json!({
            "identity": identity, "name": operation.name, "params": parameters,
            "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": self.ident("spec"), "path": "spec.qspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": modifies, "creates": [], "deletes": []},
        })
    }

    /// The domain package document.
    fn document(&self) -> Vec<u8> {
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
                "operations": self.operations.iter().map(|operation| self.operation(operation))
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
    initial: DocumentRef,
    universe: PopulationUniverse,
}

/// The evaluation budget: ample for every fixture.
const UNLIMITED: ScalarLimits = ScalarLimits {
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
        let document = model.document();
        let packages = package_input([document.as_slice()]);
        let digest = *packages.keys().next().expect("one model");
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\n\
             model M = \"{}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n{}\n",
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
        let snapshot = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
                "revision": "fixture:1"},
            "observation": "current", "model": header,
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": model.ident(model.population), "complete": true,
                "objects": [{"key": model.key, "type": model.ident(model.object),
                    "fields": model.initial}]}],
        })
        .to_string()
        .into_bytes();
        let read = quire_canonical::read(&snapshot, u64::MAX).expect("JSON fixture");
        let snapshot_digest =
            *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
                .expect("canonical fixture")
                .as_bytes();
        let population = qsl_semantics::model::key::DeclarationKey {
            package: model.package.to_owned(),
            node: model.ident(model.population),
        };
        Self {
            package: CheckedPackage::link(graph),
            packages,
            snapshots: BTreeMap::from([(snapshot_digest, snapshot)]),
            initial: DocumentRef {
                authority: "test".to_owned(),
                identity: "s0".to_owned(),
                revision_namespace: "fixture".to_owned(),
                revision: "fixture:1".to_owned(),
                digest: snapshot_digest,
            },
            universe: PopulationUniverse {
                population,
                keys: vec![model.key.to_owned()],
            },
        }
    }

    /// The system over this world, or its admission failure.
    pub(crate) fn admit(&self) -> Result<ModelSystem<'_>, AdmissionFailure> {
        ModelSystem::new(ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: vec![self.initial.clone()],
            universes: vec![self.universe.clone()],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
        })
    }

    /// The system over this world.
    pub(crate) fn system(&self) -> ModelSystem<'_> {
        self.admit().expect("the system admits")
    }

    /// The workload declared `name`.
    pub(crate) fn workload(&self, name: &str) -> &Workload {
        self.package
            .graph()
            .probabilistic_model()
            .workload(name)
            .expect("the workload is declared")
    }
}

/// `system`'s one initial state.
pub(crate) fn initial(system: &ModelSystem<'_>) -> ModelSimState {
    let [state] = <[_; 1]>::try_from(system.initial()).expect("one initial state");
    state
}

/// The value of the one object's field `field` at `state`.
pub(crate) fn field(state: &ModelSimState, field: &str) -> quire_exact::Value {
    let suffix = format!("/{field}");
    state
        .state()
        .populations
        .values()
        .flat_map(BTreeMap::values)
        .flat_map(|object| object.fields.iter())
        .find(|(key, _)| key.node.ends_with(&suffix))
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| panic!("no field {field}"))
}

fn int(package: &'static str, name: &str) -> String {
    format!("ix://{package}/{name}")
}

/// ADR-024 §7.2's `Service` (see the module doc for its types), with
/// `extra` appended to its declarations.
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
            },
            Operation {
                name: "attempt",
                parameters: vec![("d", int(PACKAGE, "Millis")), ("ok", BOOLEAN.to_owned())],
                modifies: vec!["phase", "attempts"],
            },
            Operation {
                name: "reset",
                parameters: vec![],
                modifies: vec!["phase"],
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

/// FR-187-AC-2's `Link`.
pub(crate) fn link() -> Model {
    const PACKAGE: &str = "test/link";
    let send = |name| Operation {
        name,
        parameters: vec![("lost", BOOLEAN.to_owned())],
        modifies: vec!["delivered", "attempts"],
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
             terminal Stop using v on M when {{ self.delivered or self.attempts = 2 }};\n\
             workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; }}\n",
            contract("send_a"),
            contract("send_b")
        ),
        initial: json!({"delivered": {"boolean": false}, "attempts": {"integer": "0"}}),
    }
}

/// A variant of ADR-024 §7.1's `Health`: `tick(fault)` writes `healthy` and
/// a second Boolean `spare` under the postcondition `post`, with `fault`'s
/// weights `weights`.
pub(crate) fn health(post: &str, weights: &str) -> Model {
    Model {
        package: "test/health",
        object: "Node",
        population: "nodes",
        key: "n",
        integers: vec![],
        fields: vec![
            ("healthy", BOOLEAN.to_owned()),
            ("spare", BOOLEAN.to_owned()),
        ],
        operations: vec![Operation {
            name: "tick",
            parameters: vec![("fault", BOOLEAN.to_owned())],
            modifies: vec!["healthy", "spare"],
        }],
        body: format!(
            "post Tick using v on M::Node::tick {{ {post} }}\n\
             random Node::tick(fault) ~ {{ {weights} }};\n\
             workload Steady on M {{ weight Node::tick = 1; }}\n"
        ),
        initial: json!({"healthy": {"boolean": true}, "spare": {"boolean": false}}),
    }
}

/// `Health` whose `tick` leaves `spare` unconstrained: two post-states per
/// drawn `fault`.
pub(crate) fn loose_health() -> Model {
    health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9999999",
    )
}

/// `Health` whose `tick` has no post-state for `fault = true`.
pub(crate) fn blocked_health() -> Model {
    health(
        "not fault and self.healthy = pre(self.healthy) and self.spare = pre(self.spare)",
        "true: 1, false: 1",
    )
}

/// One object, one operation `name` with one Boolean parameter `x` whose
/// distribution is `support`, writing the field `up` to `x`, under a unit
/// workload.
fn one_flag(
    package: &'static str,
    object: &'static str,
    name: &'static str,
    support: &str,
) -> Model {
    Model {
        package,
        object,
        population: "items",
        key: "c",
        integers: vec![],
        fields: vec![("up", BOOLEAN.to_owned())],
        operations: vec![Operation {
            name,
            parameters: vec![("x", BOOLEAN.to_owned())],
            modifies: vec!["up"],
        }],
        body: format!(
            "post Writes using v on M::{object}::{name} {{ self.up = x }}\n\
             random {object}::{name}(x) ~ {{ {support} }};\n\
             workload Unit on M {{ weight {object}::{name} = 1; }}\n"
        ),
        initial: json!({"up": {"boolean": false}}),
    }
}

/// FR-188-AC-4's `Coin`: `flip(x)` with weights `true: 1, false: 3`.
pub(crate) fn coin() -> Model {
    one_flag("test/coin", "Coin", "flip", "true: 1, false: 3")
}

/// FR-188-AC-5's `Wide`: `spin(x)` with weights `true: 1, false: 2^256`.
pub(crate) fn wide() -> Model {
    one_flag(
        "test/wide",
        "Wide",
        "spin",
        "true: 1, false: \
         115792089237316195423570985008687907853269984665640564039457584007913129639936",
    )
}

/// FR-188-AC-4's `Lone`: one operation `ping` with no parameter.
pub(crate) fn lone() -> Model {
    Model {
        package: "test/lone",
        object: "Pinger",
        population: "pingers",
        key: "m",
        integers: vec![],
        fields: vec![("up", BOOLEAN.to_owned())],
        operations: vec![Operation {
            name: "ping",
            parameters: vec![],
            modifies: vec![],
        }],
        body: "workload Unit on M { weight Pinger::ping = 1; }\n".to_owned(),
        initial: json!({"up": {"boolean": false}}),
    }
}

/// FR-187-AC-4's `refund` model: `pay(n: Int[0, 5])` with the reward
/// `refund = n - 3`.
pub(crate) fn refund() -> Model {
    const PACKAGE: &str = "test/refund";
    Model {
        package: PACKAGE,
        object: "Account",
        population: "accounts",
        key: "a",
        integers: vec![("Amount", 0, 5)],
        fields: vec![("up", BOOLEAN.to_owned())],
        operations: vec![Operation {
            name: "pay",
            parameters: vec![("n", int(PACKAGE, "Amount"))],
            modifies: vec![],
        }],
        body: "reward refund on Account::pay = n - 3;\n\
               workload Unit on M { weight Account::pay = 1; }\n"
            .to_owned(),
        initial: json!({"up": {"boolean": false}}),
    }
}
