// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 (TC-900): `encode_smt_query`, the canonical SMT-LIB 2 encoding of a
//! model subject's transition relation and of a temporal item's proof
//! queries, over the `Counter` and `Slot` subjects.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use qsl_eval::smt::{encode_smt_query, SmtEncodingRefusal, SmtItem, SmtQueryKind};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedTemporalClause, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::ScalarLimits;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

const PACKAGE: &str = "test/smt";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const BOUNDED: &str = "quire.temporal.event-position.false-extension/v1";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn wire_type(identity: &str, kind: &str, extra: Json) -> Json {
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
}

/// `Level`: `Int[0, 3]`.
fn level() -> Json {
    let identity = ident("Level");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    wire_type(
        &identity,
        "value_type",
        json!({"scalar": "integer",
            "constraints": [constraint("min", "0"), constraint("max", "3")]}),
    )
}

fn field(object: &str, name: &str, type_ref: &str, presence: &str) -> Json {
    let identity = ident(&format!("{object}/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": presence,
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&identity),
    })
}

fn operation(object: &str, name: &str, modifies: &[&str]) -> Json {
    let identity = ident(&format!("{object}/{name}"));
    json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": modifies.iter()
            .map(|field| ident(&format!("{object}/{field}"))).collect::<Vec<_>>(),
            "creates": [], "deletes": []},
    })
}

/// The domain package: one object type `object` with `fields` and
/// `operations`, and the population `objects` of it.
fn document(object: &str, fields: Vec<Json>, operations: Vec<Json>) -> Vec<u8> {
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
    let object_identity = ident(object);
    let population = ident("objects");
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            level(),
            wire_type(&object_identity, "object_type", json!({
                "supertypes": [], "fields": fields, "operations": operations,
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [object_identity], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// A checked package over a domain, its clauses and the snapshot of its
/// initial state.
struct World {
    package: CheckedPackage,
    clauses: Vec<CheckedTemporalClause>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: DocumentRef,
}

/// The unit selecting `document` as `M` with `contracts` and, under the
/// profile `profile`, the clauses `formulas` over `variable: M::object`,
/// from one initial snapshot of `start`, each `(key, fields)`.
fn world(
    (object, variable): (&str, &str),
    document: &[u8],
    contracts: &str,
    profile: &str,
    formulas: &[&str],
    start: &[(&str, Json)],
) -> World {
    let packages = package_input([document]);
    let digest = *packages.keys().next().expect("one model");
    let clauses: String = formulas
        .iter()
        .enumerate()
        .map(|(index, formula)| {
            format!(
                "temporal C{index} using t over ({variable}: M::{object}) clock \"model-steps\" on origin {{ {formula} }}\n"
            )
        })
        .collect();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{contracts}{clauses}",
        hex(&digest)
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "smt", "git", "1"),
        "smt.native",
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
    .unwrap_or_else(|refusals| panic!("{unit}: the unit checks: {refusals:?}"));
    let clauses: Vec<CheckedTemporalClause> = graph
        .temporal_clauses()
        .iter()
        .map(|node| node.clause().clone())
        .collect();
    assert_eq!(clauses.len(), formulas.len(), "{unit}");

    let objects: Vec<Json> = start
        .iter()
        .map(|(key, fields)| json!({"key": key, "type": ident(object), "fields": fields}))
        .collect();
    let header = json!({"identity": PACKAGE, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&digest))});
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("objects"), "complete": true,
            "objects": objects}],
    });
    let bytes = snapshot.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let snapshot_digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    World {
        package: CheckedPackage::link(graph),
        clauses,
        packages,
        snapshots: BTreeMap::from([(snapshot_digest, bytes)]),
        initial: DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest: snapshot_digest,
        },
    }
}

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
    fn subject(&self, keys: &[&str]) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: vec![self.initial.clone()],
                universes: vec![PopulationUniverse {
                    population: DeclarationKey {
                        package: PACKAGE.to_owned(),
                        node: ident("objects"),
                    },
                    keys: keys.iter().map(|key| (*key).to_owned()).collect(),
                }],
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: UNLIMITED,
            },
            proof_bounds: Vec::new(),
        }
    }

    fn query(&self, clause: usize, kind: SmtQueryKind) -> Result<String, SmtEncodingRefusal> {
        self.query_with(&["c"], clause, kind)
    }

    fn query_with(
        &self,
        keys: &[&str],
        clause: usize,
        kind: SmtQueryKind,
    ) -> Result<String, SmtEncodingRefusal> {
        encode_smt_query(
            &self.subject(keys),
            SmtItem::Clause(&self.clauses[clause]),
            kind,
        )
        .map(|script| String::from_utf8(script).expect("a script is UTF-8"))
    }
}

/// `Counter`: `value: Int[0, 3]`, operation `inc` modifying it, guarded by
/// `value < 3`.
const COUNTER_CONTRACT: &str = "pre CanInc using v on M::Counter::inc { self.value < 3 }\n\
     post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";

fn counter(profile: &str, formulas: &[&str]) -> World {
    let document = document(
        "Counter",
        vec![field("Counter", "value", &ident("Level"), "required")],
        vec![operation("Counter", "inc", &["value"])],
    );
    world(
        ("Counter", "c"),
        &document,
        COUNTER_CONTRACT,
        profile,
        formulas,
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

fn slot(formulas: &[&str], held: Json) -> World {
    let document = document(
        "Slot",
        vec![field("Slot", "held", &ident("Level"), "optional")],
        vec![
            operation("Slot", "drop", &["held"]),
            operation("Slot", "take", &["held"]),
        ],
    );
    world(
        ("Slot", "s"),
        &document,
        "post Dropped using v on M::Slot::drop { not present(self.held) }\n\
         pre Has using v on M::Slot::take { present(self.held) implies value(self.held) <= 3 }\n\
         post Took using v on M::Slot::take { present(self.held) and value(self.held) = 1 }\n",
        INFINITE,
        formulas,
        &[("s", json!({ "held": held }))],
    )
}

fn has(script: &str, line: &str) {
    assert!(
        script.lines().any(|candidate| candidate == line),
        "missing `{line}` in\n{script}"
    );
}

fn declared(script: &str) -> Vec<&str> {
    script
        .lines()
        .filter_map(|line| line.strip_prefix("(declare-const "))
        .collect()
}

/// FR-315-AC-1: the bounded `Counter` item declares the four copies in
/// order, asserts the initial predicate, one transition per step and the
/// negated property; equal inputs give byte-equal scripts.
#[trace("TC-900", "FR-315-AC-1")]
#[test]
fn an_unrolling_declares_the_copies_and_asserts_the_negated_property() {
    let world = counter(BOUNDED, &["always[0,3] holds(c.value <= 3)"]);
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 3 })
        .expect("encodes");
    assert_eq!(
        declared(&script),
        [
            "s0.c.value Int)",
            "s1.c.value Int)",
            "s2.c.value Int)",
            "s3.c.value Int)"
        ]
    );
    assert!(script.starts_with("(set-logic QF_LIA)\n"));
    assert!(script.ends_with("(check-sat)\n"));
    has(&script, "(assert (= s0.c.value 0))");
    for step in 0..3 {
        has(
            &script,
            &format!(
                "(assert (and (< s{step}.c.value 3) (= (- s{next}.c.value s{step}.c.value) 1)))",
                next = step + 1
            ),
        );
    }
    has(
        &script,
        "(assert (or (not (<= s0.c.value 3)) (not (<= s1.c.value 3)) (not (<= s2.c.value 3)) (not (<= s3.c.value 3))))",
    );
    let again = world
        .query(0, SmtQueryKind::Unrolling { depth: 3 })
        .expect("encodes");
    assert_eq!(script, again);
}

/// FR-315-AC-2: the step case asserts no initial predicate, the property at
/// position 0 and its violation at position 1; the base case asserts the
/// initial predicate on copy 0.
#[trace("TC-900", "FR-315-AC-2")]
#[test]
fn the_step_case_has_no_initial_state_and_the_base_case_has_one() {
    let world = counter(INFINITE, &["always holds(c.value <= 3)"]);
    let step = world
        .query(0, SmtQueryKind::InductionStep { depth: 1 })
        .expect("encodes");
    assert!(!step.contains("(= s0.c.value 0)"), "{step}");
    has(&step, "(assert (<= s0.c.value 3))");
    has(&step, "(assert (not (<= s1.c.value 3)))");
    let base = world
        .query(0, SmtQueryKind::InductionBase { depth: 1 })
        .expect("encodes");
    has(&base, "(assert (= s0.c.value 0))");
    // Under infinite-trace the terminal stutter is one more disjunct.
    has(
        &base,
        "(assert (or (and (< s0.c.value 3) (= (- s1.c.value s0.c.value) 1)) (and (not (< s0.c.value 3)) (= s0.c.value s1.c.value))))",
    );
}

/// FR-315-AC-3: a field whose sort has no SMT form refuses, naming the
/// object and the field, and writes no script.
#[trace("TC-900", "FR-315-AC-3")]
#[test]
fn a_field_with_no_smt_sort_is_refused_by_name() {
    let document = document(
        "Counter",
        vec![
            field("Counter", "value", &ident("Level"), "required"),
            field("Counter", "label", "ix://quire/native/String", "required"),
        ],
        vec![operation("Counter", "inc", &["value"])],
    );
    let world = world(
        ("Counter", "c"),
        &document,
        COUNTER_CONTRACT,
        INFINITE,
        &["always holds(c.value <= 3)"],
        &[(
            "c",
            json!({"value": {"integer": "0"}, "label": {"kind": "string", "text": "a"}}),
        )],
    );
    assert_eq!(
        world.query(0, SmtQueryKind::Unrolling { depth: 1 }),
        Err(SmtEncodingRefusal::FieldSort {
            object: "c".to_owned(),
            field: "label".to_owned(),
            sort: "a String",
        })
    );
}

/// FR-315-AC-4 (the declarations and the presence reading): an optional field
/// is a value constant and a `Bool` presence constant, the frame and the
/// initial predicate read presence, and every term is linear. S3 refuses a
/// `value` read it cannot prove present (`Unproved(Presence)`), so the
/// spec's `always holds(value(s.held) <= 3)` does not check; the guarded
/// claim stands in for it.
#[trace("TC-900", "FR-315-AC-4")]
#[test]
fn an_optional_field_is_a_value_and_a_presence_constant() {
    let world = slot(
        &["always holds(present(s.held) implies value(s.held) <= 3)"],
        json!({"present": {"integer": "0"}}),
    );
    let script = world
        .query_with(&["s"], 0, SmtQueryKind::Unrolling { depth: 2 })
        .expect("encodes");
    assert_eq!(
        declared(&script),
        [
            "s0.s.held Int)",
            "s0.s.held.present Bool)",
            "s1.s.held Int)",
            "s1.s.held.present Bool)",
            "s2.s.held Int)",
            "s2.s.held.present Bool)"
        ]
    );
    has(&script, "(assert (and s0.s.held.present (= s0.s.held 0)))");
    has(&script, "(assert (and (<= 0 s1.s.held) (<= s1.s.held 3)))");
    has(
        &script,
        "(assert (or (not (or (not s0.s.held.present) (<= s0.s.held 3))) (not (or (not s1.s.held.present) (<= s1.s.held 3))) (not (or (not s2.s.held.present) (<= s2.s.held 3)))))",
    );
    assert!(!script.contains("(* "), "{script}");
}

/// FR-315-AC-5: the step case asserts the copies pairwise distinct; a TP-3
/// item declares a monitor state per position and asserts its steps, the
/// property at a position being the monitor not rejecting there.
#[trace("TC-900", "FR-315-AC-5")]
#[test]
fn the_step_case_asserts_distinct_copies_and_the_monitor_steps() {
    let world = counter(
        INFINITE,
        &[
            "always holds(c.value <= 3)",
            "always[0,1] holds(c.value <= 3)",
        ],
    );
    let plain = world
        .query(0, SmtQueryKind::InductionStep { depth: 2 })
        .expect("encodes");
    for assertion in [
        "(assert (not (= s0.c.value s1.c.value)))",
        "(assert (not (= s0.c.value s2.c.value)))",
        "(assert (not (= s1.c.value s2.c.value)))",
    ] {
        has(&plain, assertion);
    }
    let monitored = world
        .query(1, SmtQueryKind::InductionStep { depth: 2 })
        .expect("encodes");
    assert_eq!(
        declared(&monitored)
            .into_iter()
            .filter(|line| line.starts_with('m'))
            .collect::<Vec<_>>(),
        ["m0 Int)", "m1 Int)", "m2 Int)"]
    );
    assert!(monitored.contains("(=> (and (= m0 "), "{monitored}");
    has(
        &monitored,
        "(assert (or (not (= s0.c.value s1.c.value)) (not (= m0 m1))))",
    );
    // The property at a position is the monitor not rejecting there.
    has(&monitored, "(assert (not (= m0 1)))");
    has(&monitored, "(assert (= m2 1))");
}

/// A TP-3 base case steps the initial monitor state over the letter at
/// position 0.
#[trace("TC-900", "FR-315-AC-5")]
#[test]
fn the_base_case_steps_the_initial_monitor_over_position_zero() {
    let world = counter(INFINITE, &["always[0,1] holds(c.value <= 3)"]);
    let base = world
        .query(0, SmtQueryKind::InductionBase { depth: 1 })
        .expect("encodes");
    has(&base, "(assert (=> (not (<= s0.c.value 3)) (= m0 1)))");
    has(&base, "(assert (=> (<= s0.c.value 3) (= m0 2)))");
}

/// An expression outside `QF_LIA` refuses, naming the construct and the
/// clause it is in.
#[trace("TC-900", "FR-315-AC-3")]
#[test]
fn a_nonlinear_atom_is_refused_with_its_locus() {
    let world = counter(INFINITE, &["always holds(c.value * c.value <= 3)"]);
    match world.query(0, SmtQueryKind::Unrolling { depth: 1 }) {
        Err(SmtEncodingRefusal::Expression { form, locus }) => {
            assert_eq!(form, "nonlinear multiplication");
            assert_eq!(locus.clause, "C0");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Items and kinds the proof queries do not cover refuse without a script.
#[trace("TC-900", "FR-315-AC-3")]
#[test]
fn items_without_a_proof_query_are_refused() {
    let world = counter(
        BOUNDED,
        &[
            "always[0,3] holds(c.value <= 3)",
            "eventually[0,3] holds(c.value = 3)",
        ],
    );
    assert!(matches!(
        world.query(0, SmtQueryKind::InductionStep { depth: 1 }),
        Err(SmtEncodingRefusal::QueryKind {
            kind: "induction-step",
            ..
        })
    ));
    let subject = world.subject(&["c"]);
    assert!(matches!(
        encode_smt_query(
            &subject,
            SmtItem::DeadlockFreedom,
            SmtQueryKind::Unrolling { depth: 1 }
        ),
        Err(SmtEncodingRefusal::ItemForm { .. })
    ));
    let live = counter(INFINITE, &["eventually holds(c.value = 3)"]);
    assert!(matches!(
        live.query(0, SmtQueryKind::Unrolling { depth: 1 }),
        Err(SmtEncodingRefusal::ItemForm { .. })
    ));
}

/// A bounded `eventually` is violated where its atom is false at every
/// position of the horizon.
#[trace("TC-900", "FR-315-AC-1")]
#[test]
fn a_bounded_eventually_is_violated_where_every_position_is_false() {
    let world = counter(BOUNDED, &["eventually[0,2] holds(c.value = 3)"]);
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 2 })
        .expect("encodes");
    has(
        &script,
        "(assert (and (not (= s0.c.value 3)) (not (= s1.c.value 3)) (not (= s2.c.value 3))))",
    );
}
