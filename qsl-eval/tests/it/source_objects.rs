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
    /// Variant S-M: fixture S, which also selects D's package as `M`.
    Mixed,
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
            Twin::Mixed => (
                package_input([document().as_slice()]).expect("fixture D admits"),
                "model M = \"test/orders\";\n",
                OBJECTS,
                "",
            ),
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
            Twin::Source | Twin::Mixed => declaration.to_owned(),
            Twin::Model => ident(declaration),
        }
    }

    /// The `model` member a snapshot of this twin carries, if it does.
    fn model(&self, document: &mut Json) {
        if self.twin != Twin::Source {
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

/// `text` with the owner of each twin's declarations removed: the one thing
/// the twins' values do not share is who owns the type.
fn neutral(text: &str) -> String {
    text.replace("source/qsl-eval-integration-tests", "P")
        .replace("test/orders", "P")
}

/// V6 helper: the frame check of the invocation `invocation` of `Order::bump`:
/// the verdict, the witness and the ordered charge log.
fn frame_check(unit: &Unit, invocation: &DocumentRef) -> (String, Option<String>, Vec<String>) {
    use qsl_eval::value::FrameEvaluation;
    use qsl_semantics::model::observation::{admit_frame_invocation, FrameFacts};
    use quire_exact::Identifier;

    let name = |text: &str| Identifier::new(text).expect("an identifier");
    let graph = unit.graph();
    let selection = match unit.twin {
        Twin::Source | Twin::Mixed => graph.resolve_source_operation(&name("Order"), &name("bump")),
        Twin::Model => graph.resolve_operation(&name("M"), &name("Order"), &name("bump")),
    }
    .expect("the operation resolves");
    let (context, operation_frame) = graph.operation_frame(&selection).expect("the frame node");
    let frame = operation_frame.frame();
    let facts = FrameFacts {
        context,
        frame: frame.clone(),
        operation: OperationFacts {
            declaring: operation_frame.operation().declaring.clone(),
            declaration: operation_frame.operation().declaration.clone(),
        },
    };
    let admitted = admit_frame_invocation(
        graph.model_inputs(),
        graph.scope().types(),
        &facts,
        &unit.packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &unit.snapshots,
            invocations: &unit.invocations,
        },
        invocation,
        ObservationLimits::default(),
    )
    .expect("the invocation admits");
    let mut meter = Meter::new(UNLIMITED);
    let FrameEvaluation {
        evaluation,
        witness,
    } = unit
        .package
        .evaluate_frame(frame, &admitted, &mut meter)
        .expect("the frame evaluates");
    let log = meter
        .admitted_charges()
        .iter()
        .map(|charge| format!("{charge:?}"))
        .collect();
    (
        format!("{:?}", evaluation.outcome),
        witness.map(|witness| neutral(&format!("{witness:?}"))),
        log,
    )
}

/// V6: an invocation of `bump` on `o1` that also changes `o2.total` violates
/// the frame, which grants `o1.total` only; one that changes `o1.total`
/// alone does not. S and D give the same verdict, witness and charge log.
#[trace("FR-569-AC-10", "TC-1408")]
#[test]
fn v6_a_frame_check_matches_its_model_twin() {
    let mut results = Vec::new();
    for twin in [Twin::Source, Twin::Model] {
        let mut unit = Unit::new(twin, "");
        let pre = unit.snapshot("pre", "pre", &POPULATION);
        let ok = unit.snapshot("ok", "post", &[("o1", "Order", 4), ("o2", "Sub", 4)]);
        let bad = unit.snapshot("bad", "post", &[("o1", "Order", 4), ("o2", "Sub", 5)]);
        let allowed = unit.invocation("allowed", &pre, &ok);
        let violating = unit.invocation("violating", &pre, &bad);
        let allowed = frame_check(&unit, &allowed);
        let violating = frame_check(&unit, &violating);
        assert!(
            allowed.1.is_none(),
            "o1.total alone is granted: {allowed:?}"
        );
        let witness = violating.1.clone().expect("o2.total is not granted");
        assert!(witness.contains("o2"), "the witness names o2: {witness}");
        assert!(
            witness.contains("total"),
            "the witness names total: {witness}"
        );
        results.push((allowed, violating));
    }
    assert_eq!(results[0], results[1], "S and D agree on the frame check");
}

/// The refusal of `admitted` as its catalog code and cause.
fn refusal_of(admitted: Result<AdmittedObservations, AdmissionFailure>) -> (String, String) {
    match admitted {
        Err(AdmissionFailure::Refused(record)) => (
            record.catalog.code().to_owned(),
            record.catalog.cause().to_owned(),
        ),
        other => panic!("a refused admission: {other:?}"),
    }
}

impl Unit {
    /// A current snapshot `label` holding exactly `populations`.
    fn snapshot_of(&mut self, label: &str, populations: Json) -> DocumentRef {
        let mut document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {"authority": "test", "identity": label},
            "observation": "current",
            "anchor": {"kind": "handler", "name": "validate"},
            "populations": populations,
        });
        self.model(&mut document);
        self.provide(label, document)
    }
}

fn order(key: &str, ty: &str, total: i64) -> Json {
    json!({"key": key, "type": ty, "fields": {
        "total": {"integer": total.to_string()}, "note": {"absent": {}}}})
}

/// AC-9: a snapshot of S names `orders` and its objects' types by their
/// source identities. An object typed with an `ix://` name, which names a
/// model declaration only, and an object of the abstract `Shape` are refused
/// as the model twin refuses them.
#[trace("FR-569-AC-9", "TC-1408")]
#[test]
fn a_snapshot_names_source_declarations_by_their_source_identity() {
    let mut unit = Unit::new(Twin::Source, "");
    let listed = |objects: Vec<Json>, shapes: Vec<Json>| {
        json!([
            {"population": "orders", "complete": true, "objects": objects},
            {"population": "shapes", "complete": true, "objects": shapes},
        ])
    };
    let ok = unit.snapshot_of(
        "ok",
        listed(vec![order("o1", "Order", 3), order("o2", "Sub", 4)], vec![]),
    );
    unit.current("I", &ok, "o1")
        .expect("Order and Sub objects admit");

    let foreign = unit.snapshot_of(
        "foreign",
        listed(vec![order("o1", "ix://test/orders/Order", 3)], vec![]),
    );
    assert_eq!(
        refusal_of(unit.current("I", &foreign, "o1")),
        (
            "invalid_runtime_input".to_owned(),
            "wrong-role-mapping".to_owned()
        ),
        "an ix:// type names no source declaration"
    );

    let abstract_instance = unit.snapshot_of(
        "abstract",
        listed(
            vec![order("o1", "Order", 3)],
            vec![json!({"key": "s1", "type": "Shape", "fields": {}})],
        ),
    );
    assert_eq!(
        refusal_of(unit.current("I", &abstract_instance, "o1")),
        (
            "invalid_runtime_input".to_owned(),
            "abstract-instance".to_owned()
        )
    );

    let unnamed = unit.snapshot_of(
        "unnamed",
        json!([{"population": "ghosts", "complete": true, "objects": []}]),
    );
    assert_eq!(
        refusal_of(unit.current("I", &unnamed, "o1")).1,
        "wrong-role-mapping",
        "a population the package does not declare"
    );
}

/// AC-9: a package that selects no domain package admits no `model` member;
/// one that does requires it.
#[trace("FR-569-AC-9", "TC-1408")]
#[test]
fn the_model_member_is_required_exactly_when_a_domain_package_is_selected() {
    let populations = json!([
        {"population": "orders", "complete": true, "objects": [order("o1", "Order", 3)]},
        {"population": "shapes", "complete": true, "objects": []},
    ]);
    let mut source = Unit::new(Twin::Source, "");
    let mut with_model = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "with-model"},
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {"identity": MODEL},
        "populations": populations,
    });
    let snapshot = source.provide("with-model", with_model.clone());
    let refused = source.current("I", &snapshot, "o1");
    let Err(AdmissionFailure::Refused(record)) = refused else {
        panic!("a refusal: {refused:?}");
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("invalid_runtime_input", "unknown-member")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("model")
    );

    // S-M selects a domain package, so the member is required.
    let mut mixed = Unit::new(Twin::Mixed, "");
    with_model["populations"][0]["objects"] = json!([order("o1", "Order", 3)]);
    let admitted = mixed.provide("with-model", with_model.clone());
    mixed
        .current("I", &admitted, "o1")
        .expect("the model member is admitted");
    let mut without = with_model.clone();
    without.as_object_mut().expect("an object").remove("model");
    let missing = mixed.provide("without-model", without);
    let refused = mixed.current("I", &missing, "o1");
    let Err(AdmissionFailure::Refused(record)) = refused else {
        panic!("a refusal: {refused:?}");
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("invalid_runtime_input", "missing-member")
    );
}

/// AC-9, variant S-M: a name with no scheme names the source `Order` and
/// `orders`, an `ix://` name the model's, and an object of one placed in the
/// other's population is refused.
#[trace("FR-569-AC-9", "TC-1408")]
#[test]
fn a_name_without_a_scheme_is_the_source_declaration_and_an_ix_name_the_model_s() {
    let mut unit = Unit::new(Twin::Mixed, "");
    let source = unit.snapshot_of(
        "source",
        json!([
            {"population": "orders", "complete": true, "objects": [order("o1", "Order", 3)]},
            {"population": "shapes", "complete": true, "objects": []},
        ]),
    );
    unit.current("I", &source, "o1")
        .expect("source objects admit");

    let model = ident("orders");
    let model_snapshot = unit.snapshot_of(
        "model",
        json!([
            {"population": model, "complete": true,
             "objects": [order("m1", &ident("Order"), 3)]},
            {"population": ident("shapes"), "complete": true, "objects": []},
            {"population": "orders", "complete": true, "objects": [order("o1", "Order", 3)]},
            {"population": "shapes", "complete": true, "objects": []},
        ]),
    );
    unit.current("I", &model_snapshot, "o1")
        .expect("model M::Order objects admit beside the source populations");

    let crossed = unit.snapshot_of(
        "crossed",
        json!([
            {"population": ident("orders"), "complete": true,
             "objects": [order("o1", "Order", 3)]},
            {"population": ident("shapes"), "complete": true, "objects": []},
            {"population": "orders", "complete": true, "objects": []},
            {"population": "shapes", "complete": true, "objects": []},
        ]),
    );
    assert_eq!(
        refusal_of(unit.current("I", &crossed, "o1")),
        (
            "invalid_runtime_input".to_owned(),
            "wrong-role-mapping".to_owned()
        ),
        "a source Order is no member type of the model's orders"
    );

    // The model's abstract `Shape` is refused as the source's is.
    let abstract_model = unit.snapshot_of(
        "abstract-model",
        json!([
            {"population": ident("orders"), "complete": true, "objects": []},
            {"population": ident("shapes"), "complete": true,
             "objects": [json!({"key": "s1", "type": ident("Shape"), "fields": {}})]},
            {"population": "orders", "complete": true, "objects": [order("o1", "Order", 3)]},
            {"population": "shapes", "complete": true, "objects": []},
        ]),
    );
    assert_eq!(
        refusal_of(unit.current("I", &abstract_model, "o1")),
        (
            "invalid_runtime_input".to_owned(),
            "abstract-instance".to_owned()
        )
    );
}

/// AC-9, variant S-M: `I` reads only the source objects. A clause over the
/// source `Order` quantifies over the source `orders`, so a model `M::Order`
/// of total 8 in `M::orders` does not reach it; the control clause over
/// `M::orders` does read that object.
#[trace("FR-569-AC-9", "TC-1408")]
#[test]
fn a_source_clause_reads_only_the_source_objects_in_a_mixed_snapshot() {
    let mut unit = Unit::new(
        Twin::Mixed,
        "invariant Small using P on Order at current \
           { forall(o in orders : deref(o).total <= 5) }\n\
         invariant SmallModel using P on M::Order at current \
           { forall(o in M::orders : deref(o).total <= 5) }\n",
    );
    let snapshot = unit.snapshot_of(
        "mixed",
        json!([
            {"population": ident("orders"), "complete": true,
             "objects": [order("m1", &ident("Order"), 8)]},
            {"population": ident("shapes"), "complete": true, "objects": []},
            {"population": "orders", "complete": true, "objects": [order("o1", "Order", 3)]},
            {"population": "shapes", "complete": true, "objects": []},
        ]),
    );
    let admitted = unit
        .current("Small", &snapshot, "o1")
        .expect("the mixed snapshot admits");
    let (outcome, _) = unit.evaluate("Small", &admitted);
    assert!(
        holds(&outcome),
        "the source clause does not see the model's object of total 8"
    );
    let model_self = ClauseSelectionInput::Current {
        snapshot: snapshot.clone(),
        anchor: SelectedAnchor {
            kind: qsl_semantics::model::observation::AnchorKind::Handler,
            name: "validate".to_owned(),
        },
        self_object: Some(SelectedObject {
            population: ident("orders"),
            key: "m1".to_owned(),
        }),
    };
    let admitted = unit
        .admit("SmallModel", model_self)
        .expect("the model clause admits");
    let (outcome, _) = unit.evaluate("SmallModel", &admitted);
    assert!(
        !holds(&outcome),
        "the control clause over M::orders reads the object of total 8"
    );
}

/// A source member of a model-owned type: its value is held to the model
/// type's bound at admission.
#[trace("FR-569-AC-9", "TC-1408")]
#[test]
fn a_source_member_of_a_model_type_is_admitted_by_that_types_bound() {
    let mut unit = Unit::new(
        Twin::Mixed,
        "object Z { t: M::Total; population zs; }\n\
         invariant ZI using P on Z at current { self.t >= 0 }\n",
    );
    let zs = |total: i64| {
        json!([
            {"population": "zs", "complete": true, "objects": [
                {"key": "z1", "type": "Z", "fields": {"t": {"integer": total.to_string()}}}]},
            {"population": "orders", "complete": true, "objects": []},
            {"population": "shapes", "complete": true, "objects": []},
        ])
    };
    let select = |snapshot: &DocumentRef| ClauseSelectionInput::Current {
        snapshot: snapshot.clone(),
        anchor: SelectedAnchor {
            kind: qsl_semantics::model::observation::AnchorKind::Handler,
            name: "validate".to_owned(),
        },
        self_object: Some(SelectedObject {
            population: "zs".to_owned(),
            key: "z1".to_owned(),
        }),
    };
    let within = unit.snapshot_of("within", zs(3));
    let admitted = unit
        .admit("ZI", select(&within))
        .expect("t = 3 is within M::Total's bound");
    let (outcome, _) = unit.evaluate("ZI", &admitted);
    assert!(holds(&outcome), "ZI holds for t = 3");
    let beyond = unit.snapshot_of("beyond", zs(99));
    assert_eq!(
        refusal_of(unit.admit("ZI", select(&beyond))),
        (
            "invalid_runtime_input".to_owned(),
            "invalid-value".to_owned()
        ),
        "t = 99 exceeds M::Total's bound"
    );
}

/// FR-106: an `ix://` name resolves only among model declarations, so one
/// that spells a source declaration (`ix://source/<package>/Order`) names
/// nothing, in a population position and as an object's type, whether or not
/// the package also selects a model.
#[trace("FR-106", "FR-569-AC-9", "TC-1408")]
#[test]
fn an_ix_name_spelling_a_source_declaration_is_refused() {
    let source_ix = |name: &str| format!("ix://source/{}/{name}", crate::TEST_PACKAGE_NAME);
    for twin in [Twin::Source, Twin::Mixed] {
        let mut unit = Unit::new(twin, "");
        let typed = unit.snapshot_of(
            "typed",
            json!([
                {"population": "orders", "complete": true,
                 "objects": [order("o1", &source_ix("Order"), 3)]},
                {"population": "shapes", "complete": true, "objects": []},
            ]),
        );
        assert_eq!(
            refusal_of(unit.current("I", &typed, "o1")),
            (
                "invalid_runtime_input".to_owned(),
                "wrong-role-mapping".to_owned()
            ),
            "an object typed by the ix:// spelling of a source declaration"
        );
        let populated = unit.snapshot_of(
            "populated",
            json!([
                {"population": source_ix("orders"), "complete": true,
                 "objects": [order("o1", "Order", 3)]},
                {"population": "shapes", "complete": true, "objects": []},
            ]),
        );
        assert_eq!(
            refusal_of(unit.current("I", &populated, "o1")).1,
            "wrong-role-mapping",
            "a population named by the ix:// spelling of a source declaration"
        );
    }
}
