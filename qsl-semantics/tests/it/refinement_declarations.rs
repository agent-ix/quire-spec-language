// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-135, FR-136, FR-137 (TC-540, TC-541, TC-542): the S3 check of a
//! `refinement` declaration over ADR-020 §8's compare-and-set counter and
//! ring buffer, each model a domain package admitted through I1 and the
//! refinement assembled and checked with the rest of its unit.
//!
//! The counters use unbounded `Integer` fields, which the checks cannot
//! tell from `Int[0, 2]`: what they decide is which names resolve, which
//! types conform and which rows exist.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::Code;
use qsl_semantics::check::{
    AssemblyRefusal, CheckCause, CheckRefusal, CheckedGraph, CheckedRefinement,
    PackageDeclarations, RefinementCause, RowClass, StepClass, StepTarget,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use serde_json::{json, Value};

const INTEGER: &str = "ix://quire/native/Integer";
const BOOLEAN: &str = "ix://quire/native/Boolean";

const SPEC: &str = "example/counter-spec";
const CAS: &str = "example/counter-cas";
const QUEUE: &str = "example/queue-spec";
const RING: &str = "example/ring-buffer";

fn node(package: &str, path: &str) -> String {
    format!("ix://{package}/{path}")
}

fn origin(package: &str, path: &str) -> Value {
    let identity = node(package, path);
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn construct(package: &str, name: &str, semantic_meaning: &str) -> Value {
    json!({
        "kind": {"module": package, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "construct": {
            "identity": "none", "shape": "record", "members": {},
            "meaning": semantic_meaning,
        },
    })
}

fn declaration(package: &str, path: &str, kind: &str, members: Value) -> Value {
    let mut value = json!({
        "identity": node(package, path), "displayName": node(package, path),
        "kind": {"module": package, "name": kind},
        "roles": [], "origin": origin(package, path),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    value
        .as_object_mut()
        .expect("an object")
        .extend(members.as_object().expect("an object").clone());
    value
}

fn field(package: &str, owner: &str, name: &str, type_ref: &str) -> Value {
    let path = format!("{owner}/{name}");
    json!({
        "identity": node(package, &path), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(package, &path),
    })
}

fn parameter(package: &str, owner: &str, operation: &str, name: &str, type_ref: &str) -> Value {
    let path = format!("{owner}/{operation}/{name}");
    json!({
        "identity": node(package, &path), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(package, &path),
    })
}

/// The operation `name` of `owner`, taking `parameters` (name, type) and
/// returning `returns`, whose frame modifies `modifies`.
fn operation(
    package: &str,
    owner: &str,
    name: &str,
    parameters: &[(&str, &str)],
    returns: Option<&str>,
    modifies: &[&str],
) -> Value {
    let path = format!("{owner}/{name}");
    let mut value = json!({
        "identity": node(package, &path), "name": name,
        "params": parameters
            .iter()
            .map(|(parameter_name, type_ref)| parameter(package, owner, name, parameter_name, type_ref))
            .collect::<Vec<_>>(),
        "pre": [], "post": [],
        "origin": {"source": {
            "sourceIdentity": node(package, "spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1,
        }},
        "frame": {
            "modifies": modifies
                .iter()
                .map(|field| node(package, &format!("{owner}/{field}")))
                .collect::<Vec<_>>(),
            "creates": [], "deletes": [],
        },
    });
    if let Some(type_ref) = returns {
        value["returns"] = json!({
            "typeRef": type_ref,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        });
    }
    value
}

fn object_type(
    package: &str,
    name: &str,
    fields: &[(&str, &str)],
    operations: Vec<Value>,
) -> Value {
    object_type_under(package, name, &[], fields, operations)
}

/// An object type declaring `supertypes` (names in the same package).
fn object_type_under(
    package: &str,
    name: &str,
    supertypes: &[&str],
    fields: &[(&str, &str)],
    operations: Vec<Value>,
) -> Value {
    declaration(
        package,
        name,
        "object_type",
        json!({
            "supertypes": supertypes.iter().map(|ty| node(package, ty)).collect::<Vec<_>>(),
            "fields": fields
                .iter()
                .map(|(field_name, type_ref)| field(package, name, field_name, type_ref))
                .collect::<Vec<_>>(),
            "operations": operations,
            "relationships": [],
        }),
    )
}

fn population(package: &str, name: &str, member: &str) -> Value {
    json!({
        "identity": node(package, name), "displayName": node(package, name),
        "kind": {"module": package, "name": "population"},
        "members": [node(package, member)], "extent": "closed",
        "origin": origin(package, name),
    })
}

fn document(package: &str, types: Vec<Value>, populations: Vec<Value>) -> Vec<u8> {
    document_with(package, Vec::new(), types, populations)
}

fn document_with(
    package: &str,
    extra_constructs: Vec<Value>,
    types: Vec<Value>,
    populations: Vec<Value>,
) -> Vec<u8> {
    const DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let constructs: Vec<Value> = [
        construct(package, "object_type", meaning::OBJECT_TYPE),
        construct(package, "population", meaning::POPULATION),
    ]
    .into_iter()
    .chain(extra_constructs)
    .collect();
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {
            "identity": node(package, "spec"), "version": "1.0.0",
            "dialect": "spec-bundle", "digest": DIGEST,
        },
        "package": {
            "identity": package, "version": "1.0.0", "manifestDigest": DIGEST,
            "mappingVersions": [], "profileVersions": [], "lockDigest": DIGEST,
        },
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": types,
        "populations": populations,
    })
    .to_string()
    .into_bytes()
}

const SUB: &str = "example/sub-counter";
const NARROW: &str = "example/narrow-counter";
const WIDE: &str = "example/wide-counter";

/// `Counter { value }` with `peek()`, its subtype `SubCounter { extra }` with
/// `bump()`, and the population `counters` of counters.
fn sub_counter() -> Vec<u8> {
    document(
        SUB,
        vec![
            object_type(
                SUB,
                "Counter",
                &[("value", INTEGER)],
                vec![operation(SUB, "Counter", "peek", &[], None, &[])],
            ),
            object_type_under(
                SUB,
                "SubCounter",
                &["Counter"],
                &[("extra", INTEGER)],
                vec![operation(SUB, "SubCounter", "bump", &[], None, &["extra"])],
            ),
        ],
        vec![population(SUB, "counters", "Counter")],
    )
}

/// `Counter { value: Int[0, upper] }` and the population `counters`.
fn bounded_counter(package: &str, upper: u32) -> Vec<u8> {
    let constraints: Vec<Value> = [("min", "0".to_owned()), ("max", upper.to_string())]
        .iter()
        .map(|(keyword, value)| {
            let path = format!("Bound/constraints/{keyword}");
            json!({
                "identity": node(package, &path), "keyword": keyword,
                "operands": {"value": value},
                "appliesTo": INTEGER,
                "diagnosticCode": format!("bound.{keyword}"),
                "origin": origin(package, &path),
            })
        })
        .collect();
    let bound = declaration(
        package,
        "Bound",
        "value_type",
        json!({
            "scalar": "integer",
            "constraints": constraints,
        }),
    );
    document_with(
        package,
        vec![construct(package, "value_type", meaning::VALUE_TYPE)],
        vec![
            bound,
            object_type(
                package,
                "Counter",
                &[("value", &node(package, "Bound"))],
                Vec::new(),
            ),
        ],
        vec![population(package, "counters", "Counter")],
    )
}

/// `Counter { value }` with `inc()` and the population `counters`.
fn counter_spec() -> Vec<u8> {
    document(
        SPEC,
        vec![object_type(
            SPEC,
            "Counter",
            &[("value", INTEGER)],
            vec![operation(SPEC, "Counter", "inc", &[], None, &["value"])],
        )],
        vec![population(SPEC, "counters", "Counter")],
    )
}

/// The compare-and-set counter: two workers that read `value` into a scratch
/// field and write it back, and `peek`.
fn counter_cas() -> Vec<u8> {
    let step = |name: &str, modifies: &[&str]| operation(CAS, "Counter", name, &[], None, modifies);
    document(
        CAS,
        vec![object_type(
            CAS,
            "Counter",
            &[
                ("value", INTEGER),
                ("tmpA", INTEGER),
                ("busyA", BOOLEAN),
                ("tmpB", INTEGER),
                ("busyB", BOOLEAN),
            ],
            vec![
                step("beginA", &["tmpA", "busyA"]),
                step("commitA", &["value", "busyA"]),
                step("retryA", &["busyA"]),
                step("beginB", &["tmpB", "busyB"]),
                step("commitB", &["value", "busyB"]),
                step("retryB", &["busyB"]),
                step("peek", &[]),
            ],
        )],
        vec![population(CAS, "counters", "Counter")],
    )
}

/// `Queue { size }` with `enq(x)`, `deq()` and a Boolean-valued `deqFlag()`,
/// and `Handle { queue }`, a reference to a queue.
fn queue_spec() -> Vec<u8> {
    document(
        QUEUE,
        vec![
            object_type(
                QUEUE,
                "Queue",
                &[("size", INTEGER)],
                vec![
                    operation(QUEUE, "Queue", "enq", &[("x", INTEGER)], None, &["size"]),
                    operation(QUEUE, "Queue", "deq", &[], Some(INTEGER), &["size"]),
                    operation(QUEUE, "Queue", "deqFlag", &[], Some(BOOLEAN), &["size"]),
                ],
            ),
            object_type(
                QUEUE,
                "Handle",
                &[("queue", &node(QUEUE, "Queue"))],
                Vec::new(),
            ),
        ],
        vec![
            population(QUEUE, "queues", "Queue"),
            population(QUEUE, "handles", "Handle"),
        ],
    )
}

/// `Ring { head, size }` and `Slot { ring, next, index, value }`, whose
/// `put(x)` and `take()` are the concrete operations.
fn ring_buffer() -> Vec<u8> {
    document(
        RING,
        vec![
            object_type(
                RING,
                "Ring",
                &[("head", INTEGER), ("size", INTEGER)],
                Vec::new(),
            ),
            object_type(
                RING,
                "Slot",
                &[
                    ("ring", &node(RING, "Ring")),
                    ("next", &node(RING, "Slot")),
                    ("index", INTEGER),
                    ("value", INTEGER),
                ],
                vec![
                    operation(RING, "Slot", "put", &[("x", INTEGER)], None, &["value"]),
                    operation(RING, "Slot", "take", &[], Some(INTEGER), &["value"]),
                ],
            ),
        ],
        vec![
            population(RING, "rings", "Ring"),
            population(RING, "slots", "Slot"),
        ],
    )
}

/// The unit selecting each of `models` (alias, package identity, document)
/// and holding `body`, with the header `profiles`.
fn unit_text(
    profiles: &str,
    models: &[(&str, &str, &[u8])],
    body: &str,
) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let mut packages = BTreeMap::new();
    let mut selections = String::new();
    for (alias, identity, bytes) in models {
        let single = package_input([*bytes]);
        let [(digest, _)] = single.iter().collect::<Vec<_>>()[..] else {
            panic!("one supplied document");
        };
        selections.push_str(&format!(
            "model {alias} = {identity:?} version \"1.0.0\" digest \"sha256-jcs:{}\";\n",
            qsl_semantics::model::key::hex(digest)
        ));
        packages.extend(single);
    }
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         {profiles}{selections}{body}\n"
    );
    (text, packages)
}

fn assemble(
    profiles: &str,
    models: &[(&str, &str, &[u8])],
    body: &str,
) -> (String, Result<PackageDeclarations, AssemblyRefusal>) {
    let (text, packages) = unit_text(profiles, models, body);
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-540", "fixture", "fixture:1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), [], "the unit is admissible: {text}");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 refused: {refusal:?}"));
    let declarations =
        PackageDeclarations::assemble(parsed.source().reference().clone(), built, selected, vec![]);
    (text, declarations)
}

/// A refusal with the text of the unit it is located at.
struct Refused {
    refusal: CheckRefusal,
    at: String,
    unit: String,
}

impl Refused {
    fn refinement(&self) -> &RefinementCause {
        let CheckCause::Refinement(refusal) = &self.refusal.cause else {
            panic!("a refinement refusal: {:?}", self.refusal);
        };
        &refusal.cause
    }

    fn code_and_cause(&self) -> (Code, Option<&'static str>) {
        (self.refusal.cause.code(), self.refusal.cause.cause())
    }
}

fn check_with(
    profiles: &str,
    models: &[(&str, &str, &[u8])],
    body: &str,
) -> Result<CheckedGraph, Vec<Refused>> {
    let (text, declarations) = assemble(profiles, models, body);
    let declarations = declarations
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"))
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
                        unit: text.clone(),
                        refusal,
                    }
                })
                .collect()
        })
}

fn refused(models: &[(&str, &str, &[u8])], body: &str) -> Vec<Refused> {
    match check_with("", models, body) {
        Ok(_) => panic!("{body}: expected a refusal"),
        Err(refusals) => refusals,
    }
}

/// The one refusal of `found`.
fn only(found: &[Refused]) -> &Refused {
    let [only] = found else {
        panic!(
            "one refusal: {:?}",
            found.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        );
    };
    only
}

fn counters() -> (Vec<u8>, Vec<u8>) {
    (counter_spec(), counter_cas())
}

fn checked_counter(body: &str) -> CheckedGraph {
    let (spec, cas) = counters();
    check_with("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], body).unwrap_or_else(|refusals| {
        panic!(
            "{body}: {:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    })
}

fn refused_counter(body: &str) -> Vec<Refused> {
    let (spec, cas) = counters();
    refused(&[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], body)
}

/// ADR-020 §8's `CasRefinesCounter`.
fn cas_refines_counter() -> String {
    "refinement CasRefinesCounter using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     step Impl::Counter::commitA -> Spec::Counter::inc(self);\n\
     step Impl::Counter::commitB -> Spec::Counter::inc(self);\n\
     step Impl::Counter::beginA -> stutter;\n\
     step Impl::Counter::beginB -> stutter;\n\
     step Impl::Counter::retryA -> stutter;\n\
     step Impl::Counter::retryB -> stutter;\n\
     step Impl::Counter::peek -> stutter;\n\
     assume fair weak each Impl::Counter::beginA;\n\
     assume fair weak each Impl::Counter::beginB;\n\
     assume fair weak each Impl::Counter::commitA;\n\
     assume fair weak each Impl::Counter::commitB;\n\
     assume fair weak each Impl::Counter::retryA;\n\
     assume fair weak each Impl::Counter::retryB;\n\
     ensure fair weak each Spec::Counter::inc;\n\
     }"
    .to_owned()
}

/// `body` with the one occurrence of `from` replaced by `to`.
fn mutated(body: &str, from: &str, to: &str) -> String {
    assert_eq!(body.matches(from).count(), 1, "{from} occurs once");
    body.replace(from, to)
}

fn only_refinement(graph: &CheckedGraph) -> &CheckedRefinement {
    let [refinement] = graph.refinements() else {
        panic!("one refinement: {:?}", graph.refinements());
    };
    refinement
}

fn spec_key(node_name: &str) -> DeclarationKey {
    DeclarationKey {
        package: SPEC.to_owned(),
        node: node(SPEC, node_name),
    }
}

fn cas_key(node_name: &str) -> DeclarationKey {
    DeclarationKey {
        package: CAS.to_owned(),
        node: node(CAS, node_name),
    }
}

/// FR-135-AC-1 (TC-540 step 1): `CasRefinesCounter` checks to one population
/// row, one object row with the field row `value = self.value` and no hidden
/// field. The dependency-package half needs the S4 closure and is not
/// tested.
#[trace("TC-540")]
#[test]
fn the_counter_refinement_checks_to_one_population_and_one_object_row() {
    let graph = checked_counter(&cas_refines_counter());
    let refinement = only_refinement(&graph);
    assert_eq!(refinement.name(), "CasRefinesCounter");
    assert_eq!(refinement.abstract_model(), "Spec");
    assert_eq!(refinement.concrete_model(), "Impl");

    let [population] = refinement.populations() else {
        panic!("one population row: {:?}", refinement.populations());
    };
    assert_eq!(population.abstract_population, spec_key("counters"));
    assert_eq!(population.concrete_population, cas_key("counters"));

    let [object] = refinement.objects() else {
        panic!("one object row: {:?}", refinement.objects());
    };
    assert_eq!(object.abstract_type, spec_key("Counter"));
    assert_eq!(object.concrete_type, cas_key("Counter"));
    assert!(object.hidden.is_empty(), "{:?}", object.hidden);
    let [field] = &object.fields[..] else {
        panic!("one field row: {:?}", object.fields);
    };
    assert_eq!(field.field, spec_key("Counter/value"));
    assert_eq!(
        field
            .expression
            .parameters()
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["self"]
    );
}

/// FR-135-AC-2 (TC-540 step 2): each malformed row refuses at its span, with
/// no `CheckedRefinement`.
#[trace("TC-540", "FR-135-AC-2")]
#[test]
fn each_malformed_header_and_row_refuses_at_its_span() {
    let base = cas_refines_counter();
    let population_row = "population Spec::counters from Impl::counters;";
    let object_row = "object Spec::Counter from Impl::Counter { value = self.value; }";
    let missing = |class, name: &str| RefinementCause::MissingRow {
        class,
        name: name.to_owned(),
    };

    // An abstract alias that names no model.
    let found = refused_counter(&mutated(&base, "abstract Spec", "abstract Nope"));
    let nope = only(&found);
    assert_eq!(
        nope.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(nope.at, "Nope");

    // The population row removed.
    let found = refused_counter(&mutated(&base, population_row, ""));
    let row = only(&found);
    assert_eq!(
        row.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(
        row.refinement(),
        &missing(RowClass::Population, "Spec::counters")
    );
    assert_eq!(row.at, "CasRefinesCounter");

    // The row written twice: the second row conflicts with the first.
    let found = refused_counter(&mutated(
        &base,
        population_row,
        &format!("{population_row}\n{population_row}"),
    ));
    let twice = only(&found);
    assert_eq!(
        twice.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(twice.at, population_row);
    let RefinementCause::Conflicting { name, first } = twice.refinement() else {
        panic!("a conflict: {:?}", twice.refinement());
    };
    assert_eq!(name, "Spec::counters");
    assert_eq!(&twice.unit[first.start..first.end], population_row);

    // The row reversed.
    let found = refused_counter(&mutated(
        &base,
        population_row,
        "population Impl::counters from Spec::counters;",
    ));
    let reversed = only(&found);
    assert_eq!(
        reversed.code_and_cause(),
        (Code::InvalidModelBinding, Some("malformed-declaration"))
    );
    assert_eq!(
        reversed.at,
        "population Impl::counters from Spec::counters;"
    );

    // The object row removed.
    let found = refused_counter(&mutated(&base, object_row, ""));
    let row = only(&found);
    assert_eq!(
        row.refinement(),
        &missing(RowClass::Object, "Spec::Counter")
    );
    assert_eq!(row.at, "CasRefinesCounter");

    // `value` mapped twice.
    let found = refused_counter(&mutated(
        &base,
        "{ value = self.value; }",
        "{ value = self.value; value = self.tmpA; }",
    ));
    let twice = only(&found);
    assert_eq!(
        twice.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(twice.at, "value = self.tmpA;");
    let RefinementCause::Conflicting { name, first } = twice.refinement() else {
        panic!("a conflict: {:?}", twice.refinement());
    };
    assert_eq!(name, "Spec::Counter.value");
    assert_eq!(&twice.unit[first.start..first.end], "value = self.value;");

    // A header that selects a bounded profile.
    let bounded = "profile finite = \"quire.temporal.event-position.false-extension/v1\";\n";
    let (spec, cas) = counters();
    let found = match check_with(
        bounded,
        &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)],
        &mutated(&base, "using inf", "using finite"),
    ) {
        Ok(_) => panic!("a bounded profile refuses"),
        Err(found) => found,
    };
    let profile = only(&found);
    assert_eq!(
        profile.code_and_cause(),
        (Code::UnknownProfile, Some("wrong-selection-role"))
    );
    assert_eq!(profile.at, "finite");
}

/// FR-135-AC-3 (TC-540 step 3): a field mapped by an expression of the wrong
/// type refuses `ill_typed`/`type-mismatch` at the expression; a reference
/// to a mapped abstract type is lifted, so a reference to its concrete
/// source conforms and a reference to another concrete type does not.
#[trace("TC-540", "FR-135-AC-3")]
#[test]
fn a_field_row_is_typed_against_the_abstract_field_after_reference_lifting() {
    let base = cas_refines_counter();
    let found = refused_counter(&mutated(
        &base,
        "value = self.value;",
        "value = self.busyA;",
    ));
    let mismatch = only(&found);
    assert_eq!(
        mismatch.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(mismatch.at, "self.busyA");

    let ring = |handle: &str| {
        format!(
            "refinement RingIsQueue using inf abstract Q concrete R {{\n\
             population Q::queues from R::rings;\n\
             population Q::handles from R::slots;\n\
             object Q::Queue from R::Ring {{ size = self.size; }}\n\
             object Q::Handle from R::Slot {{ {handle} }}\n\
             step R::Slot::put -> Q::Queue::enq(self.ring, x);\n\
             step R::Slot::take -> Q::Queue::deq(self.ring);\n\
             }}"
        )
    };
    let (queue, ring_buffer) = (queue_spec(), ring_buffer());
    let models = [("Q", QUEUE, &queue[..]), ("R", RING, &ring_buffer[..])];
    let graph = check_with("", &models, &ring("queue = self.ring;")).unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    assert_eq!(only_refinement(&graph).objects().len(), 2);

    let found = refused(&models, &ring("queue = self.next;"));
    let slot = only(&found);
    assert_eq!(
        slot.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(slot.at, "self.next");
}

/// FR-135-AC-4 (TC-540 step 4): with the `value` row removed, the refinement
/// checks with `value` hidden. The node-identity half needs S4 emission and
/// is not tested.
#[trace("TC-540")]
#[test]
fn an_abstract_field_with_no_row_is_hidden() {
    let base = cas_refines_counter();
    let graph = checked_counter(&mutated(&base, "value = self.value;", ""));
    let [object] = only_refinement(&graph).objects() else {
        panic!("one object row");
    };
    assert!(object.fields.is_empty());
    assert_eq!(object.hidden, [spec_key("Counter/value")]);
}

/// FR-136-AC-1 (TC-541 step 1): `CasRefinesCounter` checks seven step rows,
/// `commitA` and `commitB` to `inc(self)` and the others to `stutter`; the
/// ring's `put` row checks as `enq(self.ring, x)` and its `take` row as
/// `deq(self.ring)` binding the result.
#[trace("TC-541", "FR-136-AC-1")]
#[test]
fn every_concrete_operation_has_a_checked_step_row() {
    let graph = checked_counter(&cas_refines_counter());
    let steps = only_refinement(&graph).steps();
    assert_eq!(steps.len(), 7);
    let mut applied = Vec::new();
    let mut stuttered = 0;
    for step in steps {
        let StepClass::Operation(concrete) = &step.class;
        match &step.target {
            StepTarget::Abstract {
                operation,
                receiver,
                arguments,
                binds_result,
            } => {
                assert_eq!(operation, &spec_key("Counter/inc"));
                let [(name, receiver_type)] = receiver.parameters() else {
                    panic!("self alone: {:?}", receiver.parameters());
                };
                assert_eq!(name, "self");
                assert_eq!(receiver.body().value_type(), receiver_type);
                assert!(matches!(receiver_type, ValueType::Reference(_)));
                assert!(arguments.is_empty());
                assert!(!binds_result);
                applied.push(concrete.clone());
            }
            StepTarget::Stutter => stuttered += 1,
            StepTarget::Any => panic!("no row maps to any"),
        }
    }
    applied.sort();
    assert_eq!(
        applied,
        [cas_key("Counter/commitA"), cas_key("Counter/commitB")]
    );
    assert_eq!(stuttered, 5);

    let ring = ring_refinement("enq(self.ring, x)", "deq(self.ring)");
    let (queue, ring_buffer) = (queue_spec(), ring_buffer());
    let graph = check_with(
        "",
        &[("Q", QUEUE, &queue), ("R", RING, &ring_buffer)],
        &ring,
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let [put, take] = only_refinement(&graph).steps() else {
        panic!("two step rows");
    };
    let StepTarget::Abstract {
        arguments: put_arguments,
        binds_result: put_binds,
        ..
    } = &put.target
    else {
        panic!("put applies enq: {:?}", put.target);
    };
    let [Some(put_argument)] = &put_arguments[..] else {
        panic!("one argument: {put_arguments:?}");
    };
    assert_eq!(put_argument.body().value_type(), &ValueType::Integer);
    assert_eq!(
        put_argument
            .parameters()
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["self", "x"]
    );
    assert!(!put_binds);
    let StepTarget::Abstract {
        arguments: take_arguments,
        binds_result: take_binds,
        ..
    } = &take.target
    else {
        panic!("take applies deq: {:?}", take.target);
    };
    assert!(take_arguments.is_empty());
    assert!(take_binds);
}

/// ADR-020 §8's `RingIsQueue` over the two populations and `put` and `take`
/// rows `put` and `take`.
fn ring_refinement(put: &str, take: &str) -> String {
    format!(
        "refinement RingIsQueue using inf abstract Q concrete R {{\n\
         population Q::queues from R::rings;\n\
         population Q::handles from R::slots;\n\
         object Q::Queue from R::Ring {{ size = self.size; }}\n\
         object Q::Handle from R::Slot {{ queue = self.ring; }}\n\
         step R::Slot::put -> Q::Queue::{put};\n\
         step R::Slot::take -> Q::Queue::{take};\n\
         }}"
    )
}

/// FR-136-AC-2 (TC-541 step 2): a concrete operation with no row refuses
/// `missing-name` naming it, two rows refuse `conflicting-binding`, and a row
/// naming no concrete operation refuses `missing-name`.
#[trace("TC-541", "FR-136-AC-2")]
#[test]
fn step_rows_cover_each_concrete_operation_exactly_once() {
    let base = cas_refines_counter();
    let peek = "step Impl::Counter::peek -> stutter;";

    let found = refused_counter(&mutated(&base, peek, ""));
    let missing = only(&found);
    assert_eq!(
        missing.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(
        missing.refinement(),
        &RefinementCause::MissingRow {
            class: RowClass::Step,
            name: "Impl::Counter::peek".to_owned(),
        }
    );
    assert_eq!(missing.at, "CasRefinesCounter");

    let found = refused_counter(&mutated(
        &base,
        peek,
        &format!("{peek}\nstep Impl::Counter::peek -> any;"),
    ));
    let twice = only(&found);
    assert_eq!(
        twice.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(twice.at, "step Impl::Counter::peek -> any;");
    let RefinementCause::Conflicting { name, first } = twice.refinement() else {
        panic!("a conflict: {:?}", twice.refinement());
    };
    assert_eq!(name, "Impl::Counter::peek");
    assert_eq!(&twice.unit[first.start..first.end], peek);

    let found = refused_counter(&mutated(
        &base,
        peek,
        &format!("{peek}\nstep Impl::Counter::nope -> stutter;"),
    ));
    let nope = only(&found);
    assert_eq!(
        nope.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(nope.at, "Impl::Counter::nope");
}

/// FR-136-AC-3 (TC-541 step 3): the arguments of an abstract application
/// are counted and typed against the abstract operation, `_` leaves one
/// open, and the results must conform.
#[trace("TC-541", "FR-136-AC-3")]
#[test]
fn an_abstract_application_checks_its_arguments_and_result() {
    let base = cas_refines_counter();
    let commit = "step Impl::Counter::commitA -> Spec::Counter::inc(self);";
    let found = refused_counter(&mutated(
        &base,
        commit,
        "step Impl::Counter::commitA -> Spec::Counter::inc(self, 1);",
    ));
    let count = only(&found);
    assert_eq!(
        count.code_and_cause(),
        (Code::IllTyped, Some("operator-ineligible"))
    );
    assert_eq!(
        count.at,
        "step Impl::Counter::commitA -> Spec::Counter::inc(self, 1);"
    );

    let (queue, ring_buffer) = (queue_spec(), ring_buffer());
    let models = [("Q", QUEUE, &queue[..]), ("R", RING, &ring_buffer[..])];
    let found = refused(
        &models,
        &ring_refinement("enq(self.ring, self.index = 0)", "deq(self.ring)"),
    );
    let argument = only(&found);
    assert_eq!(
        argument.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(argument.at, "self.index = 0");

    let graph = check_with(
        "",
        &models,
        &ring_refinement("enq(self.ring, _)", "deq(self.ring)"),
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let StepTarget::Abstract { arguments, .. } = &only_refinement(&graph).steps()[0].target else {
        panic!("put applies enq");
    };
    assert!(matches!(&arguments[..], [None]));

    let found = refused(
        &models,
        &ring_refinement("enq(self.ring, x)", "deqFlag(self.ring)"),
    );
    let result = only(&found);
    assert_eq!(
        result.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(
        result.at,
        "step R::Slot::take -> Q::Queue::deqFlag(self.ring);"
    );
}

fn constraints(refinement: &CheckedRefinement, concrete: bool) -> Vec<String> {
    let set = if concrete {
        refinement.concrete_fairness()
    } else {
        refinement.abstract_fairness()
    };
    set.iter()
        .map(|constraint| {
            format!(
                "{:?} {:?} {}",
                constraint.kind(),
                constraint.granularity(),
                constraint.operation().name()
            )
        })
        .collect()
}

/// FR-137-AC-1 (TC-542 step 1): `F_C` is the six `Weak` `Each` constraints in
/// source order and `F_A` is `[Weak Each inc]`; an `ensure` row with no
/// granularity reads `Whole`, and with no `ensure` row `F_A` is empty. The
/// node-identity half needs S4 emission and is not tested.
#[trace("TC-542")]
#[test]
fn the_fairness_rows_form_the_two_sets_in_source_order() {
    let base = cas_refines_counter();
    let graph = checked_counter(&base);
    let refinement = only_refinement(&graph);
    assert_eq!(
        constraints(refinement, true),
        [
            "Weak Each beginA",
            "Weak Each beginB",
            "Weak Each commitA",
            "Weak Each commitB",
            "Weak Each retryA",
            "Weak Each retryB",
        ]
    );
    assert_eq!(constraints(refinement, false), ["Weak Each inc"]);

    let ensure = "ensure fair weak each Spec::Counter::inc;";
    let graph = checked_counter(&mutated(
        &base,
        ensure,
        "ensure fair weak Spec::Counter::inc;",
    ));
    assert_eq!(
        constraints(only_refinement(&graph), false),
        ["Weak Whole inc"]
    );
    let graph = checked_counter(&mutated(&base, ensure, ""));
    assert!(only_refinement(&graph).abstract_fairness().is_empty());
}

/// FR-137-AC-2 (TC-542 step 2): a row naming an operation of the other
/// model, or of neither, refuses `missing-name`, naming the other model when
/// the operation is its; two equal rows check as one constraint.
#[trace("TC-542", "FR-137-AC-2")]
#[test]
fn a_fairness_row_names_an_operation_of_its_own_model() {
    let base = cas_refines_counter();
    let ensure = "ensure fair weak each Spec::Counter::inc;";

    let found = refused_counter(&mutated(
        &base,
        ensure,
        &format!("{ensure}\nassume fair weak Spec::Counter::inc;"),
    ));
    let assumed = only(&found);
    assert_eq!(
        assumed.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(
        assumed.refinement(),
        &RefinementCause::Unresolved {
            name: "Spec::Counter::inc".to_owned(),
            held_by: Some("Spec".to_owned()),
        }
    );

    let found = refused_counter(&mutated(
        &base,
        ensure,
        &format!("{ensure}\nensure fair weak Impl::Counter::peek;"),
    ));
    let ensured = only(&found);
    assert_eq!(
        ensured.refinement(),
        &RefinementCause::Unresolved {
            name: "Impl::Counter::peek".to_owned(),
            held_by: Some("Impl".to_owned()),
        }
    );

    let found = refused_counter(&mutated(
        &base,
        ensure,
        &format!("{ensure}\nassume fair weak Impl::Counter::nope;"),
    ));
    let nope = only(&found);
    assert_eq!(
        nope.refinement(),
        &RefinementCause::Unresolved {
            name: "Impl::Counter::nope".to_owned(),
            held_by: None,
        }
    );

    let repeated = "assume fair weak each Impl::Counter::beginA;";
    let graph = checked_counter(&mutated(
        &base,
        repeated,
        &format!("{repeated}\n{repeated}"),
    ));
    assert_eq!(only_refinement(&graph).concrete_fairness().len(), 6);
}

/// FR-137-AC-4 (TC-542 step 4): a `strong` row checks as `Strong`, with its
/// granularity.
#[trace("TC-542", "FR-137-AC-4")]
#[test]
fn a_strong_row_checks_as_strong() {
    let base = cas_refines_counter();
    let ensure = "ensure fair weak each Spec::Counter::inc;";
    let graph = checked_counter(&mutated(
        &mutated(&base, ensure, "ensure fair strong each Spec::Counter::inc;"),
        "assume fair weak each Impl::Counter::retryA;",
        "assume fair strong Impl::Counter::retryA;",
    ));
    let refinement = only_refinement(&graph);
    assert_eq!(constraints(refinement, false), ["Strong Each inc"]);
    let retry = refinement
        .concrete_fairness()
        .iter()
        .find(|constraint| constraint.operation().name() == "retryA")
        .expect("retryA is assumed");
    assert_eq!(retry.kind(), FairnessKind::Strong);
    assert_eq!(retry.granularity(), FairnessGranularity::Whole);
}

/// A checked `refinement` declaration refuses `not_emitted` at package build
/// until S4 writes it, so a package is never built with an identity equal to
/// the same package without it.
#[trace("TC-540")]
#[test]
fn a_checked_refinement_refuses_not_emitted_at_package_build() {
    let (spec, cas) = counters();
    let (_, declarations) = assemble(
        "",
        &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)],
        &cas_refines_counter(),
    );
    let refusals = declarations
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("a refinement is not emitted yet");
    assert_eq!(
        refusals
            .iter()
            .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
            .collect::<Vec<_>>(),
        [(Code::UnsupportedConstruct, Some("not-yet-implemented"))]
    );
}

fn sub_refinement(source: &str, peek: &str) -> String {
    format!(
        "refinement SubRefines using inf abstract Spec concrete Impl {{\n\
         population Spec::counters from Impl::counters;\n\
         object Spec::Counter from Impl::{source} {{ value = self.value; }}\n\
         step Impl::Counter::peek -> {peek};\n\
         step Impl::SubCounter::bump -> Spec::Counter::inc(self);\n\
         }}"
    )
}

/// ADR-016 SC-2 over a subtype: a step on `SubCounter` supplies `self` where
/// the abstract receiver lifts to a reference to its supertype `Counter`.
#[trace("TC-541")]
#[test]
fn a_subtype_receiver_conforms_to_the_supertype_it_lifts_to() {
    let (spec, sub) = (counter_spec(), sub_counter());
    let models = [("Spec", SPEC, &spec[..]), ("Impl", SUB, &sub[..])];
    let graph =
        check_with("", &models, &sub_refinement("Counter", "stutter")).unwrap_or_else(|refusals| {
            panic!(
                "{:?}",
                refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
            )
        });
    assert_eq!(only_refinement(&graph).steps().len(), 2);
}

/// The other direction does not conform: with the object row mapped from the
/// subtype, a step on the supertype `Counter` supplies a reference the lifted
/// receiver type, a reference to `SubCounter`, does not accept.
#[trace("TC-541")]
#[test]
fn a_supertype_receiver_does_not_conform_to_the_subtype_it_lifts_to() {
    let (spec, sub) = (counter_spec(), sub_counter());
    let models = [("Spec", SPEC, &spec[..]), ("Impl", SUB, &sub[..])];
    let found = refused(
        &models,
        &sub_refinement("SubCounter", "Spec::Counter::inc(self)").replace(
            "step Impl::SubCounter::bump -> Spec::Counter::inc(self);",
            "step Impl::SubCounter::bump -> stutter;",
        ),
    );
    let receiver = only(&found);
    assert_eq!(
        receiver.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(receiver.at, "self");
}

fn range_refinement() -> String {
    "refinement Ranges using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     }"
    .to_owned()
}

/// ADR-016 SC-2 over integer ranges: a field of `Int[0, 3]` defines a field of
/// `Int[0, 10]`, and a field of `Int[0, 10]` is refused for one of
/// `Int[0, 3]` rather than range-checked into it.
#[trace("TC-540")]
#[test]
fn an_integer_range_must_lie_inside_the_range_it_defines() {
    let (narrow, wide) = (bounded_counter(NARROW, 3), bounded_counter(WIDE, 10));
    let inside = [("Spec", WIDE, &wide[..]), ("Impl", NARROW, &narrow[..])];
    check_with("", &inside, &range_refinement()).unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let outside = [("Spec", NARROW, &narrow[..]), ("Impl", WIDE, &wide[..])];
    let found = refused(&outside, &range_refinement());
    let range = only(&found);
    assert_eq!(
        range.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(range.at, "self.value");
}

/// A unit holding a `terminal` declaration and a refinement reports both as
/// not emitted, not only the first.
#[trace("TC-540")]
#[test]
fn a_terminal_and_a_refinement_are_each_refused_not_emitted() {
    let (spec, cas) = counters();
    let body = format!(
        "terminal Halt using v on Spec any;\n{}",
        cas_refines_counter()
    );
    let (_, declarations) = assemble("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], &body);
    let refusals = declarations
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("neither is emitted yet");
    assert_eq!(
        refusals
            .iter()
            .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
            .collect::<Vec<_>>(),
        [(Code::UnsupportedConstruct, Some("not-yet-implemented")); 2]
    );
}

const DIAMOND: &str = "example/diamond-counter";

/// `Base1 { value }` and `Base2 { other }`, each with `op()`, and `Child`
/// specializing both, so `Child::op` names two unrelated declarations.
fn diamond_counter() -> Vec<u8> {
    document(
        DIAMOND,
        vec![
            object_type(
                DIAMOND,
                "Base1",
                &[("value", INTEGER)],
                vec![operation(DIAMOND, "Base1", "op", &[], None, &[])],
            ),
            object_type(
                DIAMOND,
                "Base2",
                &[("other", INTEGER)],
                vec![operation(DIAMOND, "Base2", "op", &[], None, &[])],
            ),
            object_type_under(DIAMOND, "Child", &["Base1", "Base2"], &[], Vec::new()),
        ],
        vec![population(DIAMOND, "counters", "Child")],
    )
}

/// A step row naming an operation that two unrelated ancestors declare is
/// refused at the row's name, not dropped.
#[trace("TC-541")]
#[test]
fn a_step_row_naming_an_ambiguous_operation_refuses_at_its_name() {
    let (spec, diamond) = (counter_spec(), diamond_counter());
    let models = [("Spec", SPEC, &spec[..]), ("Impl", DIAMOND, &diamond[..])];
    let body = "refinement Diamond using inf abstract Spec concrete Impl {\n\
        population Spec::counters from Impl::counters;\n\
        object Spec::Counter from Impl::Child { value = self.value; }\n\
        step Impl::Child::op -> stutter;\n\
        step Impl::Base1::op -> stutter;\n\
        step Impl::Base2::op -> stutter;\n\
        }";
    let found = refused(&models, body);
    let ambiguous = only(&found);
    assert_eq!(
        ambiguous.code_and_cause(),
        (Code::AmbiguousDeclaration, Some("ambiguous-name"))
    );
    assert_eq!(ambiguous.at, "Impl::Child::op");
}
