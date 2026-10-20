// SPDX-License-Identifier: AGPL-3.0-or-later
//! The refinements of FR-138 and FR-141's auxiliary state (ADR-020 AX-1,
//! AX-2): `RegisterHistory`, whose abstract register keeps the value its
//! concrete register has overwritten in a history field, and `Coin`, whose
//! abstract coin fixes a hidden `side` at `toss` that the concrete `reveal`
//! shows later.
//!
//! A [`Unit`] is a pair of domain packages with their contracts; its
//! [`World`] holds the checked unit with one refinement, the initial
//! snapshots and the two model systems the tests read it through.

#![allow(
    dead_code,
    reason = "each test binary uses the part of the fixture it needs"
)]

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckRefusal, CheckedGraph, CheckedRefinement, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use qsl_semantics::model::state::{ModelState, StateObject};
use quire_exact::{Integer, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::ring_world::ring_queue::{
    self, bounded, document, node, object_type, operation, population, single,
};

const BOOLEAN: &str = "ix://quire/native/Boolean";

/// A budget no fixture evaluation exhausts.
pub const UNLIMITED: ScalarLimits = ScalarLimits {
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

/// A refusal of the unit with the text it is located at.
#[derive(Debug)]
pub struct Refused {
    /// The refusal.
    pub refusal: CheckRefusal,
    /// The text of the unit at its region.
    pub at: String,
}

/// Two domain packages, the contracts over them and the objects they hold.
pub struct Unit {
    /// The abstract package.
    pub abstract_package: &'static str,
    /// The concrete package.
    pub concrete_package: &'static str,
    /// The object type both models declare, and its population.
    pub object: &'static str,
    /// The population of that object type.
    pub population: &'static str,
    documents: (Vec<u8>, Vec<u8>),
    contracts: String,
    abstract_fields: Json,
    concrete_initials: Vec<Json>,
}

const REGISTER_SPEC: &str = "example/register-spec";
const REGISTER_IMPL: &str = "example/register-impl";
const COIN_SPEC: &str = "example/coin-spec";
const COIN_IMPL: &str = "example/coin-impl";

fn integer_field(value: i64) -> Json {
    json!({"integer": value.to_string()})
}

impl Unit {
    /// FR-138's `Register`: the abstract register keeps `value` and the
    /// `prev` the concrete register does not; `write(x)` of either.
    pub fn register() -> Self {
        let bit = |package: &str| node(package, "Bit");
        let spec = {
            let package = REGISTER_SPEC;
            document(
                package,
                vec![
                    bounded(package, "Bit", 1),
                    object_type(
                        package,
                        "Register",
                        vec![
                            single(package, "Register", "value", &bit(package)),
                            single(package, "Register", "prev", &bit(package)),
                        ],
                        vec![operation(
                            package,
                            "Register",
                            "write",
                            &[("x", &bit(package))],
                            None,
                            &["value", "prev"],
                        )],
                    ),
                ],
                vec![population(package, "regs", "Register")],
            )
        };
        let concrete = {
            let package = REGISTER_IMPL;
            document(
                package,
                vec![
                    bounded(package, "Bit", 1),
                    object_type(
                        package,
                        "Register",
                        vec![single(package, "Register", "value", &bit(package))],
                        vec![operation(
                            package,
                            "Register",
                            "write",
                            &[("x", &bit(package))],
                            None,
                            &["value"],
                        )],
                    ),
                ],
                vec![population(package, "regs", "Register")],
            )
        };
        Self {
            abstract_package: REGISTER_SPEC,
            concrete_package: REGISTER_IMPL,
            object: "Register",
            population: "regs",
            documents: (spec, concrete),
            contracts: "post SpecWrite using v on Spec::Register::write { self.value = x and self.prev = pre(self.value) }\n\
                        post ImplWrite using v on Impl::Register::write { self.value = x }\n"
                .to_owned(),
            abstract_fields: json!({"value": integer_field(0), "prev": integer_field(0)}),
            concrete_initials: vec![json!({"value": integer_field(0)})],
        }
    }

    /// FR-141-AC-4's `Coin`, with `initials` concrete initial states: the
    /// first has every field false and 0; with three, the second has `face`
    /// 1 and the third has `tossed` true (FR-141-AC-11's `Coin3`).
    pub fn coin(initials: usize) -> Self {
        let bit = |package: &str| node(package, "Bit");
        let spec = {
            let package = COIN_SPEC;
            document(
                package,
                vec![
                    bounded(package, "Bit", 1),
                    object_type(
                        package,
                        "Coin",
                        vec![
                            single(package, "Coin", "tossed", BOOLEAN),
                            single(package, "Coin", "side", &bit(package)),
                            single(package, "Coin", "shown", BOOLEAN),
                            single(package, "Coin", "face", &bit(package)),
                        ],
                        vec![
                            operation(package, "Coin", "toss", &[], None, &["tossed", "side"]),
                            operation(package, "Coin", "show", &[], None, &["shown", "face"]),
                        ],
                    ),
                ],
                vec![population(package, "coins", "Coin")],
            )
        };
        let concrete = {
            let package = COIN_IMPL;
            document(
                package,
                vec![
                    bounded(package, "Bit", 1),
                    object_type(
                        package,
                        "Coin",
                        vec![
                            single(package, "Coin", "tossed", BOOLEAN),
                            single(package, "Coin", "shown", BOOLEAN),
                            single(package, "Coin", "face", &bit(package)),
                        ],
                        vec![
                            operation(package, "Coin", "toss", &[], None, &["tossed"]),
                            operation(package, "Coin", "reveal", &[], None, &["shown", "face"]),
                        ],
                    ),
                ],
                vec![population(package, "coins", "Coin")],
            )
        };
        let state = |tossed: bool, face: i64| {
            json!({"tossed": {"boolean": tossed}, "shown": {"boolean": false},
                "face": integer_field(face)})
        };
        let mut concrete_initials = vec![state(false, 0)];
        if initials > 1 {
            concrete_initials.push(state(false, 1));
            concrete_initials.push(state(true, 0));
        }
        Self {
            abstract_package: COIN_SPEC,
            concrete_package: COIN_IMPL,
            object: "Coin",
            population: "coins",
            documents: (spec, concrete),
            contracts: "pre SpecToss using v on Spec::Coin::toss { not self.tossed }\n\
                        post SpecTossed using v on Spec::Coin::toss { self.tossed }\n\
                        pre SpecShow using v on Spec::Coin::show { self.tossed and not self.shown }\n\
                        post SpecShown using v on Spec::Coin::show { self.shown and self.face = self.side }\n\
                        pre ImplToss using v on Impl::Coin::toss { not self.tossed }\n\
                        post ImplTossed using v on Impl::Coin::toss { self.tossed }\n\
                        pre ImplReveal using v on Impl::Coin::reveal { self.tossed and not self.shown }\n\
                        post ImplRevealed using v on Impl::Coin::reveal { self.shown }\n"
                .to_owned(),
            abstract_fields: json!({"tossed": {"boolean": false}, "side": integer_field(0),
                "shown": {"boolean": false}, "face": integer_field(0)}),
            concrete_initials,
        }
    }

    /// The declaration key of `path` in the abstract package.
    pub fn abstract_key(&self, path: &str) -> DeclarationKey {
        ring_queue::key(self.abstract_package, path)
    }

    /// The declaration key of `path` in the concrete package.
    pub fn concrete_key(&self, path: &str) -> DeclarationKey {
        ring_queue::key(self.concrete_package, path)
    }

    fn packages(&self) -> BTreeMap<[u8; 32], Vec<u8>> {
        package_input([self.documents.0.as_slice(), self.documents.1.as_slice()])
    }

    /// The unit's text: both models, the contracts and `body`.
    fn text(&self, packages: &BTreeMap<[u8; 32], Vec<u8>>, body: &str) -> String {
        let digest = |package: &str| {
            packages
                .iter()
                .find(|(_, bytes)| {
                    serde_json::from_slice::<Json>(bytes)
                        .is_ok_and(|json| json["package"]["identity"] == package)
                })
                .map(|(digest, _)| hex(digest))
                .expect("the package is provisioned")
        };
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             profile inf = \"quire.temporal.infinite-trace/v1\";\n\
             model Spec = \"{}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
             model Impl = \"{}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
             {}{body}\n",
            self.abstract_package,
            digest(self.abstract_package),
            self.concrete_package,
            digest(self.concrete_package),
            self.contracts,
        )
    }

    /// The checked graph of the unit with `body`, or its refusals.
    ///
    /// # Errors
    ///
    /// Every refusal, each with the text at its region.
    pub fn check(&self, body: &str) -> Result<CheckedGraph, Vec<Refused>> {
        let packages = self.packages();
        let text = self.text(&packages, body);
        let parsed = qsl_cst::parse(
            SourceIdentity::new("agent-ix", "refinement-aux", "git", "1"),
            "aux.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 parses the unit");
        assert_eq!(parsed.diagnostics(), [], "the unit is admissible: {text}");
        let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let models = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("the model selections admit")
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
        .expect("the unit assembles")
        .with_unwritten_refinements();
        let regions = declarations.regions();
        declarations
            .check(CheckingLimits::default())
            .map_err(|refusals| {
                refusals
                    .into_iter()
                    .map(|refusal| {
                        let region = regions
                            .refusal_region(&refusal)
                            .unwrap_or_else(|| panic!("{refusal:?} names a region"));
                        let (start, end) = (
                            usize::try_from(region.start()).expect("a start"),
                            usize::try_from(region.end()).expect("an end"),
                        );
                        Refused {
                            at: text[start..end].to_owned(),
                            refusal,
                        }
                    })
                    .collect()
            })
    }

    /// The refusals of the unit with `body`, which must be refused.
    pub fn refused(&self, body: &str) -> Vec<Refused> {
        match self.check(body) {
            Ok(_) => panic!("{body}: expected a refusal"),
            Err(refusals) => refusals,
        }
    }

    /// The world of the unit with `body`, which must check.
    pub fn world(&self, body: &str) -> World<'_> {
        let graph = self.check(body).unwrap_or_else(|refusals| {
            panic!("{body}: the unit checks: {refusals:?}");
        });
        World::new(self, graph)
    }
}

/// An initialization snapshot of the one object `r` of `unit`'s population
/// in `package`, whose fields are `fields`.
fn snapshot(
    unit: &Unit,
    (package, document): (&str, &[u8]),
    name: &str,
    fields: &Json,
    snapshots: &mut BTreeMap<[u8; 32], Vec<u8>>,
) -> DocumentRef {
    let model_digest = *package_input([document])
        .keys()
        .next()
        .expect("one package");
    let identity = format!("{package}/{name}");
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": identity, "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current",
        "model": {"identity": package, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&model_digest))},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": node(package, unit.population), "complete": true,
            "objects": [{"key": "r", "type": node(package, unit.object), "fields": fields}]}],
    });
    let bytes = snapshot.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    snapshots.insert(digest, bytes);
    DocumentRef {
        authority: "test".to_owned(),
        identity,
        revision_namespace: "fixture".to_owned(),
        revision: "fixture:1".to_owned(),
        digest,
    }
}

/// The checked unit, the two models' documents and their initial states.
pub struct World<'u> {
    unit: &'u Unit,
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    abstract_initial: DocumentRef,
    concrete_initials: Vec<DocumentRef>,
}

impl<'u> World<'u> {
    fn new(unit: &'u Unit, graph: CheckedGraph) -> Self {
        let mut snapshots = BTreeMap::new();
        let abstract_initial = snapshot(
            unit,
            (unit.abstract_package, &unit.documents.0),
            "start",
            &unit.abstract_fields,
            &mut snapshots,
        );
        let concrete_initials = unit
            .concrete_initials
            .iter()
            .enumerate()
            .map(|(index, fields)| {
                snapshot(
                    unit,
                    (unit.concrete_package, &unit.documents.1),
                    &format!("start-{index}"),
                    fields,
                    &mut snapshots,
                )
            })
            .collect();
        Self {
            unit,
            package: CheckedPackage::link(graph),
            packages: unit.packages(),
            snapshots,
            abstract_initial,
            concrete_initials,
        }
    }

    /// The declaration key of `path` in the abstract package.
    pub fn abstract_key_of(&self, path: &str) -> DeclarationKey {
        self.unit.abstract_key(path)
    }

    /// The unit's checked package.
    pub fn package(&self) -> &CheckedPackage {
        &self.package
    }

    /// The unit's one refinement.
    pub fn refinement(&self) -> &CheckedRefinement {
        let [refinement] = self.package.graph().refinements() else {
            panic!("one refinement");
        };
        refinement
    }

    /// The abstract subject's initial snapshot.
    pub fn abstract_initial(&self) -> DocumentRef {
        self.abstract_initial.clone()
    }

    /// The request over `package` with `initial` states, the universe
    /// `{r}`, and each clause evaluated under `meter`.
    fn request(
        &self,
        package: &str,
        initial: Vec<DocumentRef>,
        meter: ScalarLimits,
    ) -> ModelSimulationRequest<'_> {
        ModelSimulationRequest {
            package: &self.package,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial,
            universes: vec![PopulationUniverse {
                population: ring_queue::key(package, self.unit.population),
                keys: vec!["r".to_owned()],
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter,
        }
    }

    /// The concrete subject's request.
    pub fn concrete_request(&self, meter: ScalarLimits) -> ModelSimulationRequest<'_> {
        self.request(
            self.unit.concrete_package,
            self.concrete_initials.clone(),
            meter,
        )
    }

    /// The abstract subject's system, evaluating each clause under `meter`.
    pub fn abstract_system(&self, meter: ScalarLimits) -> ModelSystem<'_> {
        ModelSystem::over_model(
            self.request(
                self.unit.abstract_package,
                vec![self.abstract_initial.clone()],
                meter,
            ),
            self.unit.abstract_package,
        )
        .expect("the abstract system admits")
    }

    /// The concrete subject's system, evaluating each clause under `meter`.
    pub fn concrete_system(&self, meter: ScalarLimits) -> ModelSystem<'_> {
        ModelSystem::over_model(self.concrete_request(meter), self.unit.concrete_package)
            .expect("the concrete system admits")
    }

    /// The concrete state with the one object `r` holding `fields`.
    pub fn concrete_state(&self, fields: &[(&str, Value)]) -> ModelState {
        self.state(self.unit.concrete_package, fields)
    }

    /// The abstract state with the one object `r` holding `fields`.
    pub fn abstract_state(&self, fields: &[(&str, Value)]) -> ModelState {
        self.state(self.unit.abstract_package, fields)
    }

    fn state(&self, package: &str, fields: &[(&str, Value)]) -> ModelState {
        let owner = self.unit.object;
        ModelState {
            populations: BTreeMap::from([(
                ring_queue::key(package, self.unit.population),
                BTreeMap::from([(
                    "r".to_owned(),
                    StateObject {
                        type_identity: ring_queue::key(package, owner),
                        fields: fields
                            .iter()
                            .map(|(name, value)| {
                                (
                                    ring_queue::key(package, &format!("{owner}/{name}")),
                                    value.clone(),
                                )
                            })
                            .collect(),
                    },
                )]),
            )]),
            extents: Vec::new(),
        }
    }
}

/// The integer `value` as a state field.
pub fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

/// The boolean `value` as a state field.
pub fn flag(value: bool) -> Value {
    Value::Boolean(value)
}

/// `RegisterHistory` with the history rows `history` (each a full `history`
/// row) and the field row `prev = self.last`.
pub fn register_history(history: &str) -> String {
    format!(
        "refinement RegisterHistory using inf abstract Spec concrete Impl {{\n\
         population Spec::regs from Impl::regs;\n\
         object Spec::Register from Impl::Register {{ prev = self.last; value = self.value; }}\n\
         {history}\n\
         step Impl::Register::write -> Spec::Register::write(self, x);\n\
         }}"
    )
}

/// FR-138's one history row.
pub const LAST: &str =
    "history Impl::Register.last : Int[0, 1] = 0 { on Impl::Register::write : pre(self.value); }";

/// The `writes` history field of FR-138-AC-4, which no row reads.
pub const WRITES: &str = "history Impl::Register.writes : Int[0, 1] = 0 { on Impl::Register::write : pre(self.writes) + 1; }";

/// The `inv` history field of FR-138-AC-5: 1 for `x` = 1 and undefined for
/// `x` = 0. The spec writes `1 / x`, which no refinement row can type as an
/// `Int[0, 1]` (`/` of integers needs a `Rational` context, and a row admits
/// no integer-division law); `mod` is Euclidean in every profile.
pub const INV: &str =
    "history Impl::Register.inv : Int[0, 1] = 0 { on Impl::Register::write : (1 mod x) + 1; }";

/// The coin refinement, with the object row's field rows `rows`.
pub fn coin_refines(rows: &str) -> String {
    format!(
        "refinement CoinRefines using inf abstract Spec concrete Impl {{\n\
         population Spec::coins from Impl::coins;\n\
         object Spec::Coin from Impl::Coin {{ tossed = self.tossed; shown = self.shown; face = self.face; {rows} }}\n\
         step Impl::Coin::toss -> Spec::Coin::toss(self);\n\
         step Impl::Coin::reveal -> Spec::Coin::show(self);\n\
         }}"
    )
}
