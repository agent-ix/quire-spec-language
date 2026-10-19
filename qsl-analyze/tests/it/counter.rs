// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124-AC-1's `Counter` subject: the `test/counter` domain package
//! (population `counters` of `Counter`, field `value: Int[0, 3]`, operation
//! `inc` modifying `value`), a native unit that selects it as `M` with the
//! contract `pre CanInc { self.value < 3 }` and `post Inc { self.value =
//! pre(self.value) + 1 }`, and the snapshots, requests and letters the
//! model-check tests build from it.

use std::collections::BTreeMap;

use qsl_eval::model_subject::{ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, PopulationUniverse,
};
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    check_clauses, CheckedTemporalClause, FormulaNodeId, PackageDeclarations,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{
    declared_type_universe_for, DocumentRef, ObservationLimits,
};
use quire_exact::{Incomplete, Meter, ObjectId, ObjectReference, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

pub(crate) const PACKAGE: &str = "test/counter";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The unit's contract on `inc`.
pub(crate) const CONTRACT: &str = "pre CanInc using v on M::Counter::inc { self.value < 3 }\n\
     post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
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

fn level(upper: u64) -> Json {
    let identity = ident("Level");
    let constraint = |keyword: &str, value: String| {
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
            "constraints": [constraint("min", "0".into()), constraint("max", upper.to_string())]}),
    )
}

/// `inc()`, or with `parameter` an operation `setTo(n: Integer)` whose
/// parameter no finite domain bounds.
fn operation(name: &str, parameter: Option<(&str, &str)>) -> Json {
    let identity = ident(&format!("Counter/{name}"));
    let params: Vec<Json> = parameter
        .into_iter()
        .map(|(parameter, type_ref)| {
            let parameter_identity = format!("{identity}/{parameter}");
            json!({
                "identity": parameter_identity, "name": parameter,
                "typeRef": type_ref,
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin(&parameter_identity),
            })
        })
        .collect();
    json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": [ident("Counter/value")], "creates": [], "deletes": []},
    })
}

const NATIVE_INTEGER: &str = "ix://quire/native/Integer";

/// The domain package; with `unbounded`, `Counter` also has `setTo(n:
/// Integer)`.
pub(crate) fn document(unbounded: bool) -> Vec<u8> {
    document_with(unbounded.then_some(NATIVE_INTEGER.to_owned()))
}

/// The domain package whose `Counter` also has `setTo(n: Int[0, 3])`: the
/// parameter is bounded by its declared type.
pub(crate) fn document_with_ranged_parameter() -> Vec<u8> {
    document_with(Some(ident("Level")))
}

fn document_with(parameter_type: Option<String>) -> Vec<u8> {
    let counter = ident("Counter");
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
    let value = ident("Counter/value");
    let mut operations = vec![operation("inc", None)];
    if let Some(type_ref) = &parameter_type {
        operations.push(operation("setTo", Some(("n", type_ref))));
    }
    let population = ident("counters");
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
            level(3),
            wire_type(&counter, "object_type", json!({
                "supertypes": [],
                "fields": [{
                    "identity": value, "name": "value", "typeRef": ident("Level"),
                    "presence": "required", "nullable": false, "defaultKind": "none",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                    "origin": origin(&value),
                }],
                "operations": operations,
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [counter], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// The digest of the domain package document.
pub(crate) fn document_digest(document: &[u8]) -> [u8; 32] {
    *package_input([document]).keys().next().expect("one model")
}

/// The native unit selecting `document` as `M`, with `body` after the
/// header.
pub(crate) fn unit_text(document: &[u8], body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t0 = \"quire.temporal.infinite-trace/v1\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{body}",
        hex(&document_digest(document))
    )
}

fn declarations(document: &[u8], body: &str) -> PackageDeclarations {
    let text = unit_text(document, body);
    let packages = package_input([document]);
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "counter", "git", "1"),
        "counter.native",
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
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
}

/// The checked package of `body` over `document`. A `terminal` declaration
/// is checked into the S3 graph the subject reads, though no S4 emission
/// writes it yet.
pub(crate) fn package(document: &[u8], body: &str) -> CheckedPackage {
    let graph = declarations(document, body)
        .with_unwritten_terminals()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// The one temporal clause `C` over `c: M::Counter`, `on origin`, whose
/// formula is `formula` under infinite-trace.
pub(crate) fn clause(document: &[u8], fairness: &str, formula: &str) -> CheckedTemporalClause {
    let body = format!(
        "temporal C using t0 over (c: M::Counter) clock \"model-steps\" on origin {{ {fairness} {formula} }}\n"
    );
    let mut checked = check_clauses(&declarations(document, &body));
    match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        other => panic!("{formula}: one checked clause, not {other:?}"),
    }
}

/// A subject's documents: the domain package and the snapshots.
pub(crate) struct Documents {
    pub(crate) document: Vec<u8>,
    pub(crate) packages: BTreeMap<[u8; 32], Vec<u8>>,
    pub(crate) snapshots: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Documents {
    pub(crate) fn new(document: Vec<u8>) -> Self {
        Self {
            packages: package_input([document.as_slice()]),
            document,
            snapshots: BTreeMap::new(),
        }
    }

    /// An initialization snapshot named `name` of the counters `objects`,
    /// each `(key, value)`, provisioned here.
    pub(crate) fn start(&mut self, name: &str, objects: &[(&str, i64)]) -> DocumentRef {
        let objects: Vec<Json> = objects
            .iter()
            .map(|(key, value)| {
                json!({"key": key, "type": ident("Counter"),
                    "fields": {"value": {"integer": value.to_string()}}})
            })
            .collect();
        let header = json!({"identity": PACKAGE, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", hex(&document_digest(&self.document)))});
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

    /// The bytes of each provisioned snapshot, by digest.
    pub(crate) fn snapshot_bytes(&self) -> impl Iterator<Item = (&[u8; 32], &Vec<u8>)> {
        self.snapshots.iter()
    }
}

/// The universe `{keys}` of `counters`.
pub(crate) fn universe(keys: &[&str]) -> Vec<PopulationUniverse> {
    vec![PopulationUniverse {
        population: key("counters"),
        keys: keys.iter().map(|key| (*key).to_owned()).collect(),
    }]
}

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

pub(crate) const ZERO: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 0,
    work_units: 0,
    result_units: 0,
};

/// The subject over `package` from `initial` with universe `keys`.
pub(crate) fn subject<'a>(
    package: &'a CheckedPackage,
    documents: &'a Documents,
    initial: Vec<DocumentRef>,
    keys: &[&str],
    meter: ScalarLimits,
) -> ModelSubject<'a> {
    ModelSubject {
        simulation: ModelSimulationRequest {
            package,
            packages: &documents.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &documents.snapshots,
            initial,
            universes: universe(keys),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter,
        },
        proof_bounds: Vec::new(),
    }
}

/// The reference of the counter `object` in `package`'s universe rule.
pub(crate) fn reference(
    package: &CheckedPackage,
    documents: &Documents,
    object: &str,
) -> ObjectReference {
    let (effective, universe) = declared_type_universe_for(
        package.graph().model_selections(),
        &documents.packages,
        ModelNormalizationLimits::default(),
        &key("Counter"),
    )
    .expect("the universe derives")
    .expect("Counter is declared");
    ObjectReference::new(universe, effective, ObjectId::new(object).expect("an id"))
}

/// The `value` of the counter `object` at `state`.
pub(crate) fn value(state: &ModelSimState, object: &str) -> i64 {
    match state.state().populations[&key("counters")][object]
        .fields
        .get(&key("Counter/value"))
    {
        Some(Value::Integer(value)) => value.to_string().parse().expect("a small value"),
        other => panic!("{object}.value is not an integer: {other:?}"),
    }
}

/// The one atom of a TP-1 clause over `c`, as the predicate `atom` of the
/// bound counter's `value`: `Some(truth)`, or `None` for an undefined
/// division by zero.
pub(crate) struct Letters {
    pub(crate) bindings: Vec<ObjectReference>,
    pub(crate) atom: fn(i64) -> Option<bool>,
}

impl ModelLetters for Letters {
    fn bindings(&self) -> Vec<ObjectReference> {
        self.bindings.clone()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        binding: &ObjectReference,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(match (self.atom)(value(state, binding.object().as_str())) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )),
        })
    }
}
