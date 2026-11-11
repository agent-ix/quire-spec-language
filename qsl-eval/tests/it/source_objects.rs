// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569-AC-9 and AC-10 (TC-1408 steps 7 and 8): a snapshot names a source
//! declaration by its source identity, and every evaluation over a
//! source-declared object type gives the outcome and the ordered charge log
//! its model twin gives. Fixture S declares the object types in the unit;
//! fixture D, its twin, is the domain package `test/orders` selected as `M`.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedGraph, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input, PackageInput};
use qsl_semantics::model::observation::{
    admit_observations, AdmissionFailure, AdmittedObservations, ClauseFacts, ClauseSelection,
    ClauseSelectionInput, DocumentProvision, DocumentRef, ObservationLimits, OperationFacts,
    Provisions, SelectedAnchor, SelectedObject,
};
use quire_eval::FamilyOutcome;
use quire_exact::{Meter, Outcome, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

const MODEL: &str = "test/orders";

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

/// TC-1408 fixture S's object declarations.
const OBJECTS: &str = "\
object Order identity (total) {
  total: Int[0, 9];
  note: Text[0, 8; nfc]?;
  population orders;
  operation size(): Integer;
  operation count(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation size(): Integer redefines Order::size;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }
";

/// The clauses of fixture S, written for the type spelling `ty` (`""` for S,
/// `"M::"` for its twin D).
fn clauses(ty: &str) -> String {
    format!(
        "invariant I using P on {ty}Order at current {{ self.total >= 0 }}\n\
         pre Q using P on {ty}Order::bump {{ self.total < 9 }}\n\
         post R using P on {ty}Order::bump {{ self.total > pre(self.total) }}\n\
         body B using P on {ty}Sub::size() {{ 7 }}\n\
         body C using P on {ty}Order::size() {{ 1 }}\n\
         pre Qs using P on {ty}Order::size {{ self.total >= 0 }}\n\
         invariant Sized using P on {ty}Order at current {{ self.size() >= 0 }}\n\
         invariant V4 using P on {ty}Order at current \
         {{ forall(o in {pop} : deref(o).total >= 0) }}\n\
         invariant V5 using P on {ty}Order at current {{ self.size() = 7 }}\n",
        pop = if ty.is_empty() { "orders" } else { "M::orders" },
    )
}

// ---------------------------------------------------------------------------
// Fixture D: the domain package document.
// ---------------------------------------------------------------------------

fn ident(suffix: &str) -> String {
    format!("ix://{MODEL}/{suffix}")
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn one() -> Json {
    json!({"lower": 1, "upper": 1, "ordered": false, "unique": true})
}

fn constraint(of: &str, keyword: &str, operands: Json) -> Json {
    json!({
        "identity": format!("{}/constraints/{keyword}", ident(of)), "keyword": keyword,
        "operands": operands, "appliesTo": ident(of),
        "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&ident(of)),
    })
}

fn value_type(name: &str, scalar: &str, constraints: Vec<Json>) -> Json {
    json!({
        "identity": ident(name), "displayName": ident(name),
        "kind": {"module": MODEL, "name": "value_type"}, "roles": [],
        "origin": origin(&ident(name)), "extensions": [], "unknownPolicy": "reject",
        "scalar": scalar, "constraints": constraints,
    })
}

fn field(owner: &str, name: &str, type_ref: &str, presence: &str) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref,
        "presence": presence, "nullable": false, "defaultKind": "none",
        "multiplicity": one(), "origin": origin(&identity),
    })
}

fn parameter(owner: &str, operation: &str, name: &str) -> Json {
    let identity = ident(&format!("{owner}/{operation}/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": "ix://quire/native/Integer",
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": one(), "origin": origin(&identity),
    })
}

fn operation(
    owner: &str,
    name: &str,
    parameter_name: Option<&str>,
    result: &str,
    redefines: Option<&str>,
    modifies: &[&str],
) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    let mut operation = json!({
        "identity": identity, "name": name,
        "params": parameter_name.map(|p| parameter(owner, name, p)).into_iter().collect::<Vec<_>>(),
        "returns": {"typeRef": result, "multiplicity": one(), "nullable": false},
        "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": [], "creates": [], "deletes": [],
            "modifies_receiver": modifies.iter()
                .map(|field| ident(&format!("Order/{field}"))).collect::<Vec<_>>()},
    });
    if let Some(redefines) = redefines {
        operation["redefines"] = json!(ident(redefines));
    }
    operation
}

fn object_type(
    name: &str,
    supertypes: &[&str],
    is_abstract: bool,
    fields: Vec<Json>,
    operations: Vec<Json>,
) -> Json {
    let identity = ident(name);
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": MODEL, "name": "object_type"}, "roles": [],
        "origin": origin(&identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject", "abstract": is_abstract,
        "supertypes": supertypes.iter().map(|s| ident(s)).collect::<Vec<_>>(),
        "fields": fields, "operations": operations, "relationships": [],
    })
}

fn population(name: &str, member: &str) -> Json {
    json!({
        "identity": ident(name), "displayName": ident(name),
        "kind": {"module": MODEL, "name": "population"},
        "members": [ident(member)], "extent": "closed", "origin": origin(&ident(name)),
    })
}

/// Fixture D's package document.
fn document() -> Vec<u8> {
    use qsl_semantics::model::intake::meaning;
    let integer = "ix://quire/native/Integer";
    let boolean = "ix://quire/native/Boolean";
    let types = vec![
        value_type(
            "Total",
            "integer",
            vec![
                constraint("Total", "min", json!({"value": "0"})),
                constraint("Total", "max", json!({"value": "9"})),
            ],
        ),
        value_type(
            "Note",
            "string",
            vec![
                constraint("Note", "minLength", json!({"value": 0})),
                constraint("Note", "maxLength", json!({"value": 8})),
                constraint("Note", "format", json!({"name": "text-profile:nfc"})),
            ],
        ),
        object_type(
            "Order",
            &[],
            false,
            vec![
                field("Order", "total", &ident("Total"), "required"),
                field("Order", "note", &ident("Note"), "optional"),
            ],
            vec![
                operation("Order", "size", None, integer, None, &[]),
                operation("Order", "count", None, integer, None, &[]),
                operation("Order", "scaled", Some("n"), integer, None, &[]),
                operation("Order", "bump", None, boolean, None, &["total"]),
            ],
        ),
        object_type(
            "Sub",
            &["Order"],
            false,
            vec![],
            vec![
                operation("Sub", "size", None, integer, Some("Order/size"), &[]),
                operation(
                    "Sub",
                    "scaled",
                    Some("m"),
                    integer,
                    Some("Order/scaled"),
                    &[],
                ),
            ],
        ),
        object_type("Shape", &[], true, vec![], vec![]),
    ];
    let populations = vec![population("orders", "Order"), population("shapes", "Shape")];
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": MODEL, "name": name}, "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": MODEL, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [],
        "constructs": constructs, "types": types, "populations": populations,
    })
    .to_string()
    .into_bytes()
}

// ---------------------------------------------------------------------------
// The twins.
// ---------------------------------------------------------------------------

/// Which twin a unit is.
#[derive(Clone, Copy, PartialEq)]
enum Twin {
    /// Fixture S: the unit declares the object types.
    Source,
    /// Fixture D: the unit selects the domain package that declares them.
    Model,
}

/// A checked twin, its packages and the documents its tests add.
struct Unit {
    twin: Twin,
    package: CheckedPackage,
    packages: PackageInput,
    snapshots: DocumentProvision,
    invocations: DocumentProvision,
}

fn document_ref(label: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
    }
}

impl Unit {
    /// The twin over `extra` declarations after the shared clauses.
    fn new(twin: Twin, extra: &str) -> Self {
        let (packages, selection, objects, ty) = match twin {
            Twin::Source => (PackageInput::new(), "", OBJECTS, ""),
            Twin::Model => (
                package_input([document().as_slice()]).expect("fixture D admits"),
                "model M = \"test/orders\";\n",
                "",
                "M::",
            ),
        };
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile P = \"quire.model.complete/v1\";\n\
             {selection}{objects}{}{extra}\n",
            clauses(ty),
        );
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("test", "tc-1408"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
        let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let models = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("the model selection admits");
        let mut declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            crate::TEST_PACKAGE_NAME.to_owned(),
            built,
            models,
            Vec::new(),
        )
        .unwrap_or_else(|refusal| panic!("the unit assembles: {:?}", refusal.errors));
        let pending = declarations
            .link_dispatch(ModelNormalizationLimits::default())
            .expect("dispatch links");
        let graph: CheckedGraph = declarations
            .check(CheckingLimits::default())
            .map_err(|refusals| pending.report(refusals))
            .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
        Self {
            twin,
            package: CheckedPackage::link(graph),
            packages,
            snapshots: BTreeMap::new(),
            invocations: BTreeMap::new(),
        }
    }

    /// How a snapshot of this twin names a declaration.
    fn name(&self, declaration: &str) -> String {
        match self.twin {
            Twin::Source => declaration.to_owned(),
            Twin::Model => ident(declaration),
        }
    }

    /// The `model` member a snapshot of this twin carries, if it does.
    fn model(&self, document: &mut Json) {
        if self.twin == Twin::Model {
            document["model"] = json!({"identity": MODEL});
        }
    }

    /// A snapshot `label` at `observation` holding `orders` (key, type,
    /// total) in `orders` and nothing in `shapes`.
    fn snapshot(
        &mut self,
        label: &str,
        observation: &str,
        orders: &[(&str, &str, i64)],
    ) -> DocumentRef {
        let objects: Vec<Json> = orders
            .iter()
            .map(|(key, ty, total)| {
                json!({"key": key, "type": self.name(ty), "fields": {
                    "total": {"integer": total.to_string()}, "note": {"absent": {}}}})
            })
            .collect();
        let mut document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": label},
            "observation": observation,
            "populations": [
                {"population": self.name("orders"), "complete": true, "objects": objects},
                {"population": self.name("shapes"), "complete": true, "objects": []},
            ],
        });
        self.model(&mut document);
        if observation == "current" {
            document["anchor"] = json!({"kind": "handler", "name": "validate"});
        }
        self.provide(label, document)
    }

    fn provide(&mut self, label: &str, document: Json) -> DocumentRef {
        let reference = document_ref(label);
        self.snapshots.insert(
            reference.clone(),
            serde_json::to_vec(&document).expect("the snapshot serializes"),
        );
        reference
    }

    /// The invocation `label` of `Order::bump` on `o1`, between `pre` and
    /// `post`.
    fn invocation(&mut self, label: &str, pre: &DocumentRef, post: &DocumentRef) -> DocumentRef {
        let reference = |reference: &DocumentRef| json!({"identity": {"authority": reference.authority, "identity": reference.identity}});
        let mut document = json!({
            "format": "quire.state.invocation/v1",
            "identity": {"authority": "test", "identity": label},
            "context": self.name("Order"),
            "operation": "bump",
            "self": {"population": self.name("orders"), "key": "o1"},
            "pre": reference(pre),
            "post": reference(post),
            "parameters": {},
            "result": {"boolean": true},
            "created": [],
            "deleted": [],
        });
        self.model(&mut document);
        let reference = document_ref(label);
        self.invocations.insert(
            reference.clone(),
            serde_json::to_vec(&document).expect("the invocation serializes"),
        );
        reference
    }

    /// The precondition `name` over the pre-call observation of `snapshot`,
    /// `self` the order `key`.
    fn pre_call(
        &self,
        name: &str,
        snapshot: &DocumentRef,
        key: &str,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        self.admit(
            name,
            ClauseSelectionInput::PreCall {
                snapshot: snapshot.clone(),
                self_object: Some(SelectedObject {
                    population: self.name("orders"),
                    key: key.to_owned(),
                }),
                parameters: BTreeMap::new(),
            },
        )
    }

    /// The clause `name` over the invocation `invocation`.
    fn invoked(
        &self,
        name: &str,
        invocation: &DocumentRef,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        self.admit(
            name,
            ClauseSelectionInput::Invocation {
                invocation: invocation.clone(),
            },
        )
    }

    fn graph(&self) -> &CheckedGraph {
        self.package.graph()
    }

    fn admit(
        &self,
        name: &str,
        input: ClauseSelectionInput,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let clause = self.graph().state_clause(name).expect("the clause");
        let facts = ClauseFacts {
            identity: clause.identity(),
            kind: clause.kind(),
            context: clause.context(),
            operation: clause.operation().map(|operation| OperationFacts {
                declaring: operation.declaring.clone(),
                declaration: operation.declaration.clone(),
            }),
            operations: self.graph().scope().operations().clone(),
            named_populations: clause.named_reads(),
        };
        admit_observations(
            self.graph().model_inputs(),
            self.graph().scope().types(),
            &facts,
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &self.snapshots,
                invocations: &self.invocations,
            },
            &ClauseSelection {
                name: name.to_owned(),
                input,
            },
            ObservationLimits::default(),
        )
    }

    /// The invariant `name` over the current snapshot, `self` the order `key`.
    fn current(
        &self,
        name: &str,
        snapshot: &DocumentRef,
        key: &str,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        self.admit(
            name,
            ClauseSelectionInput::Current {
                snapshot: snapshot.clone(),
                anchor: SelectedAnchor {
                    kind: qsl_semantics::model::observation::AnchorKind::Handler,
                    name: "validate".to_owned(),
                },
                self_object: Some(SelectedObject {
                    population: self.name("orders"),
                    key: key.to_owned(),
                }),
            },
        )
    }

    /// The outcome and the ordered charge log of the clause `name`.
    fn evaluate(
        &self,
        name: &str,
        observations: &AdmittedObservations,
    ) -> (FamilyOutcome<Value>, Vec<String>) {
        let mut meter = Meter::new(UNLIMITED);
        let outcome = self
            .package
            .evaluate_clause(
                &QualifiedName::unqualified(name).expect("a name"),
                observations,
                &mut meter,
            )
            .expect("the observations evaluate")
            .evaluation
            .outcome;
        let log = meter
            .admitted_charges()
            .iter()
            .map(|charge| format!("{charge:?}"))
            .collect();
        (outcome, log)
    }
}

fn holds(outcome: &FamilyOutcome<Value>) -> bool {
    match outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds))) => *holds,
        other => panic!("a decided clause: {other:?}"),
    }
}

/// V1: `I` over `o1` gives the same outcome and charge log over S and D.
#[trace("FR-569-AC-10", "TC-1408")]
#[test]
fn v1_an_invariant_over_a_source_object_matches_its_model_twin() {
    let mut results = Vec::new();
    for twin in [Twin::Source, Twin::Model] {
        let mut unit = Unit::new(twin, "");
        let current = unit.snapshot(
            "current",
            "current",
            &[("o1", "Order", 3), ("o2", "Sub", 4)],
        );
        let admitted = unit.current("I", &current, "o1").unwrap_or_else(|e| {
            panic!(
                "the snapshot admits twin source={}: {e:?}",
                twin == Twin::Source
            )
        });
        let (outcome, log) = unit.evaluate("I", &admitted);
        assert!(holds(&outcome));
        assert!(!log.is_empty(), "the evaluation is charged");
        results.push((format!("{outcome:?}"), log));
    }
    assert_eq!(results[0], results[1], "S and D agree on I");
}

/// Both twins, each evaluated by `run`: the outcomes and charge logs must be
/// equal, and the outcome is returned for the oracle to judge.
fn on_both(run: impl Fn(&mut Unit) -> (FamilyOutcome<Value>, Vec<String>)) -> bool {
    let results: Vec<_> = [Twin::Source, Twin::Model]
        .into_iter()
        .map(|twin| {
            let mut unit = Unit::new(twin, "");
            let (outcome, log) = run(&mut unit);
            let decided = holds(&outcome);
            assert!(!log.is_empty(), "the evaluation is charged");
            (decided, format!("{outcome:?}"), log)
        })
        .collect();
    assert_eq!(results[0], results[1], "S and D agree");
    results[0].0
}

const POPULATION: [(&str, &str, i64); 2] = [("o1", "Order", 3), ("o2", "Sub", 4)];

/// V2: `Q` over `o1` (`total` 3 < 9), and false over an order at the bound.
#[trace("FR-569-AC-10", "TC-1408")]
#[test]
fn v2_a_precondition_matches_its_model_twin() {
    assert!(on_both(|unit| {
        let pre = unit.snapshot("pre", "pre", &POPULATION);
        let admitted = unit
            .pre_call("Q", &pre, "o1")
            .expect("the pre snapshot admits");
        unit.evaluate("Q", &admitted)
    }));
    assert!(!on_both(|unit| {
        let pre = unit.snapshot("pre", "pre", &[("o1", "Order", 9), ("o2", "Sub", 4)]);
        let admitted = unit
            .pre_call("Q", &pre, "o1")
            .expect("the pre snapshot admits");
        unit.evaluate("Q", &admitted)
    }));
}

/// V3: `R` over the invocation of `bump` on `o1` that takes `total` from 3
/// to 4, and false when it does not grow.
#[trace("FR-569-AC-10", "TC-1408")]
#[test]
fn v3_a_postcondition_over_pre_and_post_snapshots_matches_its_model_twin() {
    for (post_total, expected) in [(4, true), (3, false)] {
        assert_eq!(
            on_both(|unit| {
                let pre = unit.snapshot("pre", "pre", &POPULATION);
                let post = unit.snapshot(
                    "post",
                    "post",
                    &[("o1", "Order", post_total), ("o2", "Sub", 4)],
                );
                let invocation = unit.invocation("call", &pre, &post);
                let admitted = unit
                    .invoked("R", &invocation)
                    .expect("the invocation admits");
                unit.evaluate("R", &admitted)
            }),
            expected,
            "o1.total 3 -> {post_total}"
        );
    }
}

/// V4: the population domain `forall(o in orders : deref(o).total >= 0)`.
#[trace("FR-569-AC-10", "TC-1408")]
#[test]
fn v4_a_population_domain_matches_its_model_twin() {
    assert!(on_both(|unit| {
        let current = unit.snapshot("current", "current", &POPULATION);
        let admitted = unit
            .current("V4", &current, "o1")
            .expect("the snapshot admits");
        unit.evaluate("V4", &admitted)
    }));
}

/// V5: `self.size()` dispatches to `Sub.size` (body `B`, 7) over `o2` and to
/// `Order.size` (body `C`, 1) over `o1`.
#[trace("FR-569-AC-10", "TC-1408")]
#[test]
fn v5_a_dispatched_query_matches_its_model_twin() {
    for (key, expected) in [("o2", true), ("o1", false)] {
        assert_eq!(
            on_both(|unit| {
                let current = unit.snapshot("current", "current", &POPULATION);
                let admitted = unit
                    .current("V5", &current, key)
                    .expect("the snapshot admits");
                unit.evaluate("V5", &admitted)
            }),
            expected,
            "self.size() = 7 over {key}"
        );
    }
}
