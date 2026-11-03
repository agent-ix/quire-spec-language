// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1565 (FR-049-AC-2/AC-3): an object environment FR-106 check 6.5
//! admitted with out-of-range post-state witnesses evaluates through
//! `CheckedPackage::evaluate`: the entry admission and every
//! attribute read recognise the witnesses at the paths FR-106 itself records
//! them (an optional attribute's or record member's present value one
//! `OptionValue` step down, a sequence element by index, a record member by
//! field), and refuse an out-of-range integer no witness names.
//!
//! The witnesses are produced by FR-106 admission of real snapshot and
//! invocation documents, never built by hand.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::CheckedPackageEvaluation;
use qsl_forms::Expression;
use qsl_package::CheckedPackage;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::observation::{
    admit_observations, AdmittedObservations, ClauseFacts, ClauseSelection, ClauseSelectionInput,
    DocumentRef, ObservationLimits, OperationFacts, PathStep, PostStateRange, Provisions,
};
use quire_eval::{CallFailure, FamilyOutcome};
use quire_exact::{Meter, Outcome, ScalarLimits, Value, ValueType};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use serde_json::{json, Value as Json};

#[path = "../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code, reason = "this test uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "other test modules also load the K2 builder"
)]
mod k2;

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// The attributes of `Gauge` other than its identity, each modified by
/// `tick`.
const ATTRIBUTES: [&str; 5] = ["level", "reading", "levels", "readings", "last"];

fn field(owner: &str, name: &str, type_ref: &str) -> Json {
    k2::field(owner, name, type_ref)
}

fn optional(owner: &str, name: &str, type_ref: &str) -> Json {
    let mut field = field(owner, name, type_ref);
    field["presence"] = json!("optional");
    field
}

fn many(owner: &str, name: &str, type_ref: &str) -> Json {
    let mut field = field(owner, name, type_ref);
    field["multiplicity"] = json!({"lower": 0, "upper": 8, "ordered": true, "unique": false});
    field
}

/// `Digit`, the integers `0..=9`: a bounded `value` type over the native
/// Integer (the FR-359 fixture's own declaration).
fn digit_type() -> Json {
    let identity = k2::id("Digit");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": k2::origin("Digit"),
        })
    };
    let mut digit = k2::declaration("Digit", "bounded", json!({}));
    digit["scalar"] = json!("integer");
    digit["constraints"] = json!([constraint("min", "0"), constraint("max", "9")]);
    digit
}

/// A domain document holding `Digit`, `Reading { value?: Digit }` and the
/// object type `Gauge` whose `Digit` attributes sit at every FR-106 step
/// kind: `level?: Digit`, `reading: Reading`, `levels: Digit*`,
/// `readings: Reading*` and `last?: Reading`, with the operation
/// `tick(): Integer` modifying them all, in the closed population `gauges`.
fn document() -> Json {
    let digit = k2::id("Digit");
    let reading = k2::id("Reading");
    let reading_type = k2::declaration(
        "Reading",
        "money",
        json!({"supertypes": [], "fields": [optional("Reading", "value", &digit)]}),
    );
    let mut gauge = k2::declaration(
        "Gauge",
        "thing",
        json!({
            "identityFields": [k2::id("Gauge/gauge_id")],
            "fields": [
                field("Gauge", "gauge_id", "ix://quire/native/Integer"),
                optional("Gauge", "level", &digit),
                field("Gauge", "reading", &reading),
                many("Gauge", "levels", &digit),
                many("Gauge", "readings", &reading),
                optional("Gauge", "last", &reading),
            ],
        }),
    );
    let tick = "Gauge/tick";
    gauge["operations"] = json!([{
        "identity": k2::id(tick), "name": "tick", "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [], "post": [],
        "frame": {
            "modifies": ATTRIBUTES
                .iter()
                .map(|name| k2::id(&format!("Gauge/{name}")))
                .collect::<Vec<_>>(),
            "creates": [], "deletes": [],
        },
        "origin": k2::origin(tick),
    }]);
    let constructs = vec![
        k2::construct("money", meaning::RECORD_VALUE_TYPE),
        k2::construct("thing", meaning::OBJECT_TYPE),
        k2::construct("population", meaning::POPULATION),
        {
            let mut bounded = k2::construct("bounded", meaning::VALUE_TYPE);
            bounded["construct"]["members"] = json!({});
            bounded["construct"]["shape"] = json!("record");
            bounded
        },
    ];
    let mut document = k2::domain_document(vec![digit_type(), reading_type, gauge], constructs);
    document["populations"] = json!([{
        "identity": k2::id("gauges"), "displayName": k2::id("gauges"),
        "kind": {"module": k2::BUSINESS, "name": "population"},
        "members": [k2::id("Gauge")], "extent": "closed",
        "origin": k2::origin("gauges"),
    }]);
    document
}

const CLAUSE: &str = "post Ticked using v on M::Gauge::tick { true }";

/// The unit over [`document`], declaring [`CLAUSE`].
fn package() -> CheckedPackage {
    let bytes = serde_json::to_vec(&document()).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]).unwrap();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         {CLAUSE}\n",
        package = k2::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "witnessed"),
        "witnessed.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the source unit");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the fixture package");
    let declarations = qsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("assembly succeeds");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}")),
    )
}

fn document_ref(label: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
    }
}

fn identity_json(reference: &DocumentRef) -> Json {
    json!({
    "authority": reference.authority, "identity": reference.identity,
    })
}

fn integer(value: i64) -> Json {
    json!({"integer": value.to_string()})
}

fn reading(value: i64) -> Json {
    json!({"record": {"type": k2::id("Reading"),
                      "fields": {"value": {"present": integer(value)}}}})
}

/// A `Gauge` whose `Digit` integers are, in [`ATTRIBUTES`] order with
/// `levels` and `readings` holding two elements, the seven `digits`.
fn gauge(digits: [i64; 7]) -> Json {
    let [level, reading_value, first, second, first_reading, second_reading, last] = digits;
    json!({
        "key": "g1", "type": k2::id("Gauge"),
        "fields": {
            "gauge_id": integer(1),
            "level": {"present": integer(level)},
            "reading": reading(reading_value),
            "levels": {"sequence": [integer(first), integer(second)]},
            "readings": {"sequence": [reading(first_reading), reading(second_reading)]},
            "last": {"present": reading(last)},
        },
    })
}

/// FR-106 admission, under `Witness`, of a `tick` invocation from an
/// in-range pre state to the post state `post`.
fn admit(package: &CheckedPackage, post: [i64; 7]) -> AdmittedObservations {
    let bytes = serde_json::to_vec(&document()).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]).unwrap();
    let model = json!({"identity": k2::PACKAGE, "version": "1"});
    let snapshot = |label: &str, observation: &str, digits: [i64; 7]| {
        let identity = document_ref(label);
        let bytes = serde_json::to_vec(&json!({
            "format": "quire.state.snapshot/v1",
            "identity": identity_json(&identity),
            "observation": observation,
            "model": model,
            "populations": [{
                "population": k2::id("gauges"), "complete": true,
                "objects": [gauge(digits)],
            }],
        }))
        .expect("the snapshot serializes");
        (identity, bytes)
    };
    let (pre, pre_bytes) = snapshot("pre", "pre", [1; 7]);
    let (post_ref, post_bytes) = snapshot("post", "post", post);
    let invocation = document_ref("call");
    let invocation_bytes = serde_json::to_vec(&json!({
        "format": "quire.state.invocation/v1",
        "identity": identity_json(&invocation),
        "model": model,
        "context": k2::id("Gauge"),
        "operation": "tick",
        "self": {"population": k2::id("gauges"), "key": "g1"},
        "pre": {"identity": identity_json(&pre)},
        "post": {"identity": identity_json(&post_ref)},
        "parameters": {},
        "result": integer(0),
        "created": [],
        "deleted": [],
    }))
    .expect("the invocation serializes");
    let snapshots = BTreeMap::from([(pre, pre_bytes), (post_ref, post_bytes)]);
    let invocations = BTreeMap::from([(invocation.clone(), invocation_bytes)]);
    let checked = package.graph().state_clause("Ticked").expect("the clause");
    let facts = ClauseFacts {
        identity: checked.identity(),
        kind: checked.kind(),
        context: checked.context(),
        operation: checked.operation().map(|operation| OperationFacts {
            declaring: operation.declaring.clone(),
            declaration: operation.declaration.clone(),
        }),
        operations: Default::default(),
        named_populations: checked.named_reads(),
    };
    admit_observations(
        package.graph().model_selections(),
        package.graph().scope().types(),
        &facts,
        &packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &snapshots,
            invocations: &invocations,
        },
        &ClauseSelection {
            name: "Ticked".to_owned(),
            input: ClauseSelectionInput::Invocation { invocation },
        },
        ObservationLimits {
            post_state: PostStateRange::Witness,
            ..ObservationLimits::default()
        },
    )
    .unwrap_or_else(|failure| panic!("FR-106 admits the witnessed post state: {failure:?}"))
}

/// Each attribute of `self` read through `evaluate` over the post
/// environment, as `Ok(completed)` or the failure.
fn reads(
    package: &CheckedPackage,
    observations: &AdmittedObservations,
) -> Vec<(String, Result<bool, CallFailure>)> {
    let post = observations.post.as_ref().expect("the post observation");
    let reference = observations.self_object.clone();
    let parameters = vec![(
        "g".to_owned(),
        ValueType::Reference(reference.object_type().clone()),
    )];
    let completed = |result: Result<quire_eval::Evaluation, CallFailure>| {
        result.map(|evaluation| {
            matches!(
                evaluation.outcome,
                FamilyOutcome::Evaluated(Outcome::Completed(_))
            )
        })
    };
    ATTRIBUTES
        .iter()
        .map(|name| {
            let expression = Expression::field(Expression::deref(Expression::name("g")), *name);
            let checked = package
                .graph()
                .check_expression(
                    parameters.clone(),
                    &expression,
                    None,
                    CheckMode::Kernel,
                    CheckingLimits::default(),
                )
                .unwrap_or_else(|refusal| panic!("{name} checks: {refusal:?}"));
            let result = package.evaluate(
                &checked,
                vec![Value::Reference(reference.clone())],
                &post.environment,
                &mut Meter::new(UNLIMITED),
            );
            ((*name).to_owned(), completed(result))
        })
        .collect()
}

/// QSL-1565 (FR-049-AC-2/AC-3): a post state whose every `Digit` is out of
/// range -- an optional attribute, a required record's optional member, two
/// sequence elements, two sequence elements' optional members and an
/// optional record's optional member -- is admitted by FR-106 with one
/// witness each, at FR-106's own paths, and every read of it through
/// `evaluate` completes.
#[trace("TC-136", "FR-049-AC-2", "FR-049-AC-3")]
#[test]
fn witnesses_fr106_records_admit_their_integers_at_entry_and_on_read() {
    let package = package();
    let observations = admit(&package, [10, 11, 12, 13, 14, 15, 16]);
    let post = observations.post.as_ref().expect("the post observation");
    let mut paths: Vec<(String, Option<usize>, Vec<PathStep>)> = post
        .out_of_range
        .iter()
        .map(|witness| {
            (
                witness.field.name.clone(),
                witness.index,
                witness.steps.clone(),
            )
        })
        .collect();
    paths.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    let value = || PathStep::Field("value".to_owned());
    let mut expected = vec![
        ("level".to_owned(), None, vec![PathStep::OptionValue]),
        (
            "reading".to_owned(),
            None,
            vec![value(), PathStep::OptionValue],
        ),
        ("levels".to_owned(), Some(0), Vec::new()),
        ("levels".to_owned(), Some(1), Vec::new()),
        (
            "readings".to_owned(),
            Some(0),
            vec![value(), PathStep::OptionValue],
        ),
        (
            "readings".to_owned(),
            Some(1),
            vec![value(), PathStep::OptionValue],
        ),
        (
            "last".to_owned(),
            None,
            vec![PathStep::OptionValue, value(), PathStep::OptionValue],
        ),
    ];
    expected.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    assert_eq!(paths, expected, "FR-106's own witness paths");
    for (read, result) in reads(&package, &observations) {
        assert!(
            matches!(result, Ok(true)),
            "{read} over the FR-106-witnessed environment completes: {result:?}"
        );
    }
}

/// QSL-1565 (FR-049-AC-2/AC-3): the same environment with one witness
/// removed holds an out-of-range integer no witness names, so every read --
/// of any attribute, the environment being admitted whole at entry --
/// refuses `wrong-value-kind` naming the unwitnessed attribute.
#[trace("TC-136", "FR-049-AC-2", "FR-049-AC-3")]
#[test]
fn an_integer_no_fr106_witness_names_refuses_at_entry() {
    let package = package();
    for (attribute, index) in [("level", None), ("readings", Some(1)), ("last", None)] {
        let mut observations = admit(&package, [10, 11, 12, 13, 14, 15, 16]);
        let post = observations.post.as_mut().expect("the post observation");
        let witnesses: Vec<_> = post
            .out_of_range
            .iter()
            .filter(|witness| !(witness.field.name == attribute && witness.index == index))
            .cloned()
            .collect();
        assert!(witnesses.len() < post.out_of_range.len(), "{attribute}");
        post.environment = std::mem::take(&mut post.environment).with_out_of_range(witnesses);
        for (read, result) in reads(&package, &observations) {
            let Err(CallFailure::Admission(failure)) = &result else {
                panic!("{attribute}: {read} refuses at entry, not {result:?}");
            };
            let record = qsl_eval::value::object_environment_refusal(failure)
                .unwrap_or_else(|| panic!("{attribute}: an object-environment refusal"));
            assert_eq!(record.catalog.cause(), "wrong-value-kind", "{attribute}");
            assert_eq!(
                record.fields.get("field").map(String::as_str),
                Some(attribute),
                "{read}"
            );
        }
    }
}
