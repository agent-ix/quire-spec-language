// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1042 (FR-098 "The object environment", FR-071 `object_environment`,
//! FR-070 reference witness): a replay dereferences a `Reference` over an
//! admitted object environment and refuses each defect in the order FR-098
//! states. One test per row of the TC-1042 tables; every expected result is
//! the table's.

use ix_trace_rs::trace;
use qsl_semantics::model::observation::{
    population_universe_for, AdmissionFailure, AdmissionRecord,
};
use quire_exact::{EffectiveId, ValueType};
use serde_json::{json, Value};

use super::*;
use crate::object_environment::{
    read_object_environment, write_object_environment, EnvironmentLabels, EnvironmentModel,
    object_reference_witness, EnvironmentObject, EnvironmentPopulation, ObjectEnvironmentRef,
    ObjectEnvironmentRefusal, ObjectReferenceError,
};
use qsl_semantics::model::key::DeclarationKey;
use crate::result::{DisagreementCause, Verdict};
use crate::witness::{DecodeRefusal, WitnessBinding, WitnessField, WitnessSlot, WitnessValueType};
use crate::SnapshotValue;

const PACKAGE: &str = "test/graph";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn identity(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

fn origin(identity: &str) -> Value {
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn field(owner: &str, name: &str, type_ref: &str, presence: &str, multiplicity: Value) -> Value {
    let field = format!("{}/{name}", identity(owner));
    json!({
        "identity": field,
        "name": name,
        "typeRef": type_ref,
        "presence": presence,
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": multiplicity,
        "origin": origin(&field),
    })
}

fn object_type(name: &str, supertypes: &[String], fields: Vec<Value>) -> Value {
    let identity = identity(name);
    json!({
        "identity": identity,
        "displayName": identity,
        "kind": {"module": PACKAGE, "name": "object_type"},
        "roles": [],
        "origin": origin(&identity),
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": supertypes,
        "fields": fields,
        "operations": [],
    })
}

/// The `test/graph` domain package: `Node` (`items`, `next`), `Hub`
/// specializing `Node`, `Lone` with its own supertype component, and the
/// closed population `G` over `Node` and `Lone`.
fn domain_document() -> Vec<u8> {
    let (digit, node, lone) = (identity("Digit"), identity("Node"), identity("Lone"));
    let bound = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{digit}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": origin(&digit),
        })
    };
    let one = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
    let construct = |name: &str, meaning: &str| {
        json!({
            "kind": {"module": PACKAGE, "name": name},
            "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {}, "meaning": meaning},
        })
    };
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {
            "identity": identity("spec"),
            "version": "1.0.0",
            "dialect": "spec-bundle",
            "digest": PLACEHOLDER,
        },
        "package": {
            "identity": PACKAGE,
            "version": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "mappingVersions": [],
            "profileVersions": [],
            "lockDigest": PLACEHOLDER,
        },
        "occurrences": [],
        "extensions": [],
        "constructs": [
            construct("object_type", "quire.meaning.model.object-type/v1"),
            construct("population", "quire.meaning.model.population/v1"),
            construct("value_type", "quire.meaning.model.value-type/v1"),
        ],
        "types": [
            {
                "identity": digit,
                "displayName": digit,
                "kind": {"module": PACKAGE, "name": "value_type"},
                "roles": [],
                "origin": origin(&digit),
                "extensions": [],
                "unknownPolicy": "reject",
                "scalar": "integer",
                "constraints": [bound("min", "0"), bound("max", "9")],
            },
            object_type(
                "Node",
                &[],
                vec![
                    field(
                        "Node",
                        "items",
                        &digit,
                        "required",
                        json!({"lower": 0, "upper": 3, "ordered": true, "unique": false}),
                    ),
                    field("Node", "next", &node, "optional", one.clone()),
                ],
            ),
            object_type("Hub", std::slice::from_ref(&node), vec![]),
            object_type("Lone", &[], vec![field("Lone", "n", &digit, "required", one)]),
        ],
        "populations": [{
            "identity": identity("G"),
            "displayName": identity("G"),
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [node, lone],
            "extent": "closed",
            "origin": origin(&identity("G")),
        }],
    })
    .to_string()
    .into_bytes()
}

fn sha256_jcs(bytes: &[u8]) -> [u8; 32] {
    PackageDocument::parse(bytes, qsl_foundation::IntakeLimits::default())
        .expect("the document parses")
        .jcs_digest()
}

/// A snapshot document of the environment, with its `sha256-jcs` digest.
struct Document {
    bytes: Vec<u8>,
    digest: [u8; 32],
}

/// How a variant of S1 differs from it.
#[derive(Default)]
struct Variant {
    observation: Option<&'static str>,
    anchor: Option<&'static str>,
    model_digest: Option<[u8; 32]>,
    duplicate_a: bool,
    a_type: Option<&'static str>,
    a_items: Option<Value>,
    a_next: Option<Value>,
    incomplete: bool,
}

struct Graph {
    unit: String,
    compiled: ComposedUnit,
    package: Vec<u8>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
}

fn graph() -> Graph {
    let package = domain_document();
    let packages = package_input([package.as_slice()]);
    let digest = qsl_semantics::model::key::hex(&sha256_jcs(&package));
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         model M = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         function fa using v(n: Reference<M::Node>): Boolean pure \
         {{ forall(x in deref(n).items: x < 5) }}\n\
         function fb using v(n: Reference<M::Node>): Boolean pure {{ n = n }}\n\
         function fc using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n\
         function fh using v(n: Reference<M::Hub>): Boolean pure \
         {{ forall(x in deref(n).items: x < 5) }}\n\
         function fe using v(p: Sequence<Reference<M::Node>>[0, 2]): Boolean pure \
         {{ forall(r in p: forall(x in deref(r).items: x < 5)) }}\n\
         function fl using v(n: Reference<M::Lone>): Boolean pure {{ n = n }}\n"
    );
    let compiled = spine(&unit, &packages);
    Graph {
        unit,
        compiled,
        package,
        packages,
    }
}

impl Graph {
    /// The effective type id of the `Reference<T>` that `function` takes.
    fn type_of(&self, function: &str) -> [u8; 32] {
        let callable = self
            .compiled
            .package
            .graph()
            .callable(function)
            .expect("declared");
        let ValueType::Reference(object_type) = &callable.parameters[0].1 else {
            panic!("{function} takes a reference");
        };
        *object_type.as_bytes()
    }

    fn universe_of(&self, object_type: [u8; 32]) -> [u8; 32] {
        let graph = self.compiled.package.graph();
        *population_universe_for(
            graph.model_selections(),
            &self.packages,
            SpineLimits::default().model,
            EffectiveId::from_digest(object_type),
        )
        .expect("the object type has a universe")
        .as_bytes()
    }

    /// `ref(key, type)`: the reference to the object `key` of `object_type`,
    /// in `Lone`'s universe `V` for `Lone`'s type and in `U` otherwise.
    fn reference(&self, key: &str, object_type: [u8; 32]) -> WitnessValue {
        let universe = if object_type == self.type_of("fl") {
            self.universe_of(object_type)
        } else {
            self.universe_of(self.type_of("fa"))
        };
        WitnessValue::Reference {
            universe,
            object_type,
            identity: key.as_bytes().to_vec(),
        }
    }

    fn node(&self, key: &str) -> WitnessValue {
        self.reference(key, self.type_of("fa"))
    }

    /// The environment document S1 or, with `variant`, one of its variants.
    fn document(&self, variant: Variant) -> Document {
        let (node, hub, lone) = (identity("Node"), identity("Hub"), identity("Lone"));
        let population = identity("G");
        let numbers = |values: &[i64]| json!({"sequence": values.iter().map(|value| json!({"integer": value.to_string()})).collect::<Vec<_>>()});
        let object = |key: &str, type_name: &str, items: Value, next: Value| json!({"key": key, "type": type_name, "fields": {"items": items, "next": next}});
        let absent = || json!({"absent": {}});
        let mut objects = vec![
            object(
                "a",
                variant.a_type.unwrap_or(&node),
                variant.a_items.clone().unwrap_or_else(|| numbers(&[3, 7])),
                variant.a_next.clone().unwrap_or_else(absent),
            ),
            object("b", &node, numbers(&[1, 2]), absent()),
            object("h", &hub, numbers(&[9]), absent()),
            json!({"key": "z", "type": lone, "fields": {"n": {"integer": "4"}}}),
        ];
        if variant.duplicate_a {
            objects.push(object("a", &node, numbers(&[1]), absent()));
        }
        let model_digest = variant
            .model_digest
            .unwrap_or_else(|| sha256_jcs(&self.package));
        let mut document = json!({
            "format": "quire.state.snapshot/v1",
            "identity": {
                "authority": "agent-ix",
                "identity": "ix://test/graph/environment",
                "revision_namespace": "git",
                "revision": "1",
            },
            "observation": variant.observation.unwrap_or("current"),
            "model": {
                "identity": PACKAGE,
                "version": "1.0.0",
                "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&model_digest)),
            },
            "populations": [{
                "population": population,
                "complete": !variant.incomplete,
                "objects": objects,
            }],
        });
        if variant.observation.is_none() {
            document["anchor"] = json!({
                "kind": "handler",
                "name": variant.anchor.unwrap_or("run"),
            });
        }
        let bytes = document.to_string().into_bytes();
        let digest = sha256_jcs(&bytes);
        Document { bytes, digest }
    }

    fn s1(&self) -> Document {
        self.document(Variant::default())
    }

    fn parameter(&self, function: &str) -> WireNodeId {
        parameter(&self.compiled, function, 0)
    }

    fn base(&self, function: &str, source: ReplaySource) -> ReplayRequestWire {
        let mut wire = request(
            self.unit.as_bytes(),
            self.compiled.emitted.package_id(),
            name(&[function]),
            source,
        );
        wire.byte_provision.push(entry(&self.package));
        wire
    }

    fn input(&self, function: &str, value: WitnessValue) -> ReplayRequestWire {
        self.base(function, typed_input(self.parameter(function), value))
    }

    fn witness(&self, function: &str, value: &WitnessValue) -> ReplayRequestWire {
        let parameter = self.parameter(function);
        let text = value.to_value_text().unwrap();
        let transcript = format!("<<<assertion|{function}|c|{parameter}={text}>>>");
        self.base(
            function,
            ReplaySource::Witness(Witness::parse(transcript).unwrap()),
        )
    }
}

/// A `sha256-jcs` byte-provision entry for the document `bytes`.
fn entry(bytes: &[u8]) -> (Option<String>, String, Vec<u8>) {
    (
        Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
        DigestRecord::mint(DigestDomain::Sha256Jcs, sha256_jcs(bytes)).hex(),
        bytes.to_vec(),
    )
}

/// `wire` naming `document` as its `object_environment`, with the document in
/// the byte provision when `provided`.
fn environment(
    mut wire: ReplayRequestWire,
    document: &Document,
    provided: bool,
) -> ReplayRequestWire {
    if provided {
        wire.byte_provision.push(entry(&document.bytes));
    }
    wire.object_environment = Some(
        ObjectEnvironmentRef::new(
            EnvironmentLabels {
                authority: "agent-ix".to_owned(),
                identity: "ix://test/graph/environment".to_owned(),
                revision_namespace: "git".to_owned(),
                revision: "1".to_owned(),
            },
            DigestRecord::mint(DigestDomain::Sha256Jcs, document.digest),
        )
        .expect("a sha256-jcs digest"),
    );
    wire
}

fn run(wire: ReplayRequestWire) -> Result<ReplayResult, ReplayRefusal> {
    replay(wire, ReplayLimits::default())
}

/// The call completed `false`: the replay reproduces the violation.
fn assert_completes_false(result: Result<ReplayResult, ReplayRefusal>, witness: bool) {
    let result = result.expect("the replay runs");
    match (&result, witness) {
        (ReplayResult::Input(arm), false) => {
            assert_eq!(arm.settlement(), InputSettlement::ReproducedWithoutWitness);
            assert_eq!(arm.value(), Some(EvaluatedValue::Boolean(false)));
        }
        (ReplayResult::Witness(arm), true) => {
            assert_eq!(
                arm.settlement(),
                WitnessSettlement::ReproducedWithEvaluatedWitness
            );
            assert_eq!(arm.value(), Some(EvaluatedValue::Boolean(false)));
        }
        _ => panic!("the wrong arm settled: {result:?}"),
    }
    assert_eq!(result.category(), Category::Violation);
}

/// The call completed `true`: the replay's verdict is `success`, the proved
/// one `violation`, and the replay settles `inconclusive` for the difference.
fn assert_completes_true(result: Result<ReplayResult, ReplayRefusal>) {
    let result = result.expect("the replay runs");
    let ReplayResult::Input(arm) = &result else {
        panic!("an Input-sourced request settles on the Input arm: {result:?}");
    };
    assert_eq!(arm.settlement(), InputSettlement::Inconclusive);
    assert_eq!(arm.value(), Some(EvaluatedValue::Boolean(true)));
    assert_eq!(result.category(), Category::Success);
    assert_eq!(
        result.disagreement(),
        Some(&DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Success),
        })
    );
}

fn assert_no_value(result: Result<ReplayResult, ReplayRefusal>) {
    let result = result.expect("the replay runs");
    assert_eq!(result.category(), Category::Inconclusive);
    assert!(matches!(
        result.disagreement(),
        Some(DisagreementCause::NoValue { .. })
    ));
}

fn wrong_kind(result: Result<ReplayResult, ReplayRefusal>, what: &str) {
    assert_eq!(
        result.as_ref().err().and_then(ReplayRefusal::input_refusal),
        Some(&InputRefusal::WrongValueKind { parameter: 0 }),
        "{what}: expected WrongValueKind at position 0, got {result:?}"
    );
}

fn environment_failure(result: Result<ReplayResult, ReplayRefusal>) -> ObjectEnvironmentFailure {
    match result {
        Err(ReplayRefusal::ObjectEnvironment(failure)) => failure,
        other => panic!("expected an object environment refusal, got {other:?}"),
    }
}

/// FR-106's own record, refused or incomplete, with the catalog code and
/// cause.
fn admission(
    failure: &ObjectEnvironmentFailure,
    incomplete: bool,
    code: Code,
    cause: &str,
) -> AdmissionRecord {
    let ObjectEnvironmentFailure::Admission(
        AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
    ) = failure
    else {
        panic!("expected FR-106's record, got {failure:?}");
    };
    assert_eq!(failure.is_incomplete(), incomplete);
    assert_eq!((failure.code(), failure.cause()), (code, Some(cause)));
    assert_eq!(record.code, code);
    assert_eq!(record.cause, cause);
    record.clone()
}

/// A reference argument FR-364's `resolve` did not find present.
fn assert_unresolved(
    failure: &ObjectEnvironmentFailure,
    position: usize,
    key: &str,
    resolution: Unresolved,
) {
    let ObjectEnvironmentFailure::Reference {
        position: named,
        reference,
        resolution: found,
    } = failure
    else {
        panic!("expected a resolution refusal, got {failure:?}");
    };
    assert_eq!(
        (*named, reference.object().as_str(), *found),
        (position, key, resolution)
    );
    match resolution {
        Unresolved::Dangling => assert_eq!(
            (failure.code(), failure.cause()),
            (
                Code::DanglingReference,
                Some("absent-target-in-complete-population")
            )
        ),
        Unresolved::Undecidable => {
            assert_eq!(
                (failure.code(), failure.cause()),
                (Code::IncompletePopulation, Some("incomplete-scope"))
            );
            assert!(failure.is_incomplete());
        }
    }
}

fn labels() -> EnvironmentLabels {
    EnvironmentLabels {
        authority: "agent-ix".to_owned(),
        identity: "ix://test/graph/environment".to_owned(),
        revision_namespace: "git".to_owned(),
        revision: "1".to_owned(),
    }
}

fn member(digest: &str) -> Value {
    json!({
        "identity": {
            "authority": "agent-ix",
            "identity": "ix://test/graph/environment",
            "revision_namespace": "git",
            "revision": "1",
        },
        "digest": digest,
    })
}

// ---------------------------------------------------------------------------
// Request member and reference witness decode
// ---------------------------------------------------------------------------

/// Row 1: the member constructs without its byte-provision entry, and
/// serializes and reads back equal.
#[trace("TC-1042", "FR-071-AC-12")]
#[test]
fn tc_1042_row_01_the_member_constructs_and_round_trips() {
    let graph = graph();
    let s1 = graph.s1();
    let wire = environment(graph.input("fa", graph.node("a")), &s1, false);
    let request = ReplayRequest::decode(wire, ReplayLimits::default()).expect("constructs");
    let named = request.object_environment().expect("the member");
    assert_eq!(named.digest().as_bytes(), &s1.digest);
    assert_eq!(named.identity(), &labels());
    assert_eq!(&read_object_environment(&named.to_value()).unwrap(), named);
    let again = ReplayRequest::decode(request.to_wire(), ReplayLimits::default()).unwrap();
    assert_eq!(again, request);
}

/// Row 2: a string for the member.
#[trace("TC-1042", "FR-071-AC-12")]
#[test]
fn tc_1042_row_02_a_string_member_is_the_wrong_value_kind() {
    let refusal = read_object_environment(&json!("S1")).unwrap_err();
    assert_eq!(
        refusal,
        ObjectEnvironmentRefusal::WrongValueKind {
            member: "object_environment".to_owned()
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "wrong-value-kind")
    );
}

/// Row 3: no `digest`.
#[trace("TC-1042", "FR-071-AC-12")]
#[test]
fn tc_1042_row_03_a_missing_digest_is_a_missing_member() {
    let mut value = member("sha256-jcs:00");
    value.as_object_mut().unwrap().remove("digest");
    let refusal = read_object_environment(&value).unwrap_err();
    assert_eq!(
        refusal,
        ObjectEnvironmentRefusal::MissingMember {
            member: "object_environment.digest".to_owned()
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "missing-member")
    );
}

/// Row 4: a third member.
#[trace("TC-1042", "FR-071-AC-12")]
#[test]
fn tc_1042_row_04_a_third_member_is_an_unknown_member() {
    let mut value = member(&format!("sha256-jcs:{}", "a".repeat(64)));
    value["role"] = json!("current");
    let refusal = read_object_environment(&value).unwrap_err();
    assert_eq!(
        refusal,
        ObjectEnvironmentRefusal::UnknownMember {
            member: "object_environment.role".to_owned()
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "unknown-member")
    );
}

/// Row 5: a digest outside `sha256-jcs`, and one of 63 hex digits.
#[trace("TC-1042", "FR-071-AC-12")]
#[test]
fn tc_1042_row_05_a_bad_digest_is_an_invalid_value() {
    for digest in [
        format!("quire.source.bytes/v1:{}", "a".repeat(64)),
        format!("sha256-jcs:{}", "a".repeat(63)),
    ] {
        let refusal = read_object_environment(&member(&digest)).unwrap_err();
        assert_eq!(
            refusal,
            ObjectEnvironmentRefusal::InvalidValue {
                member: "object_environment.digest".to_owned()
            },
            "{digest}"
        );
        assert_eq!(
            (refusal.code(), refusal.cause()),
            (Code::InvalidRuntimeInput, "invalid-value")
        );
    }
}

fn reference_text(universe: &str, object_type: &str, identity: &str) -> String {
    format!(
        "{{\"identity\":\"{identity}\",\"object_type\":\"{object_type}\",\"type\":\"reference\",\"universe\":\"{universe}\"}}"
    )
}

fn decode_entry(text: &str) -> Result<Vec<WitnessValue>, DecodeRefusal> {
    let parameter = WireNodeId::from_digest([9; 32]);
    let transcript = format!("<<<assertion|f|c|{parameter}={text}>>>");
    Witness::parse(transcript)
        .unwrap()
        .decode(&[WitnessBinding {
            parameter,
            value_type: WitnessValueType::Canonical,
        }])
}

/// Row 6: the one reference spelling decodes, the empty identity too.
#[trace("TC-1042", "FR-070-AC-16")]
#[test]
fn tc_1042_row_06_a_reference_entry_decodes() {
    let (universe, object_type) = ("1".repeat(64), "2".repeat(64));
    let decoded = decode_entry(&reference_text(&universe, &object_type, "61")).unwrap();
    assert_eq!(
        decoded,
        vec![WitnessValue::Reference {
            universe: [0x11; 32],
            object_type: [0x22; 32],
            identity: vec![0x61],
        }]
    );
    let decoded = decode_entry(&reference_text(&universe, &object_type, "")).unwrap();
    assert_eq!(
        decoded,
        vec![WitnessValue::Reference {
            universe: [0x11; 32],
            object_type: [0x22; 32],
            identity: Vec::new(),
        }]
    );
}

/// Row 7: seven malformed spellings.
#[trace("TC-1042", "FR-070-AC-17")]
#[test]
fn tc_1042_row_07_a_malformed_reference_entry_is_refused() {
    let (universe, object_type) = ("1".repeat(64), "2".repeat(64));
    let texts = [
        reference_text(&"1".repeat(63), &object_type, "61"),
        reference_text(&universe, &format!("{}A", "2".repeat(63)), "61"),
        reference_text(&universe, &object_type, "616"),
        format!(
            "{{\"type\":\"reference\",\"universe\":\"{universe}\",\"object_type\":\"{object_type}\",\"identity\":\"61\"}}"
        ),
        format!(
            "{{\"object_type\":\"{object_type}\",\"type\":\"reference\",\"universe\":\"{universe}\"}}"
        ),
        format!(
            "{{\"extra\":\"y\",\"identity\":\"61\",\"object_type\":\"{object_type}\",\"type\":\"reference\",\"universe\":\"{universe}\"}}"
        ),
        format!(
            "{{\"identity\":\"61\",\"object_type\":\"{object_type}\",\"type\":\"reference\",\"universe\":1}}"
        ),
    ];
    for text in texts {
        let parameter = WireNodeId::from_digest([9; 32]);
        assert!(
            matches!(
                decode_entry(&text),
                Err(DecodeRefusal::Malformed { parameter: named, .. }) if named == parameter
            ),
            "{text}"
        );
    }
}

// ---------------------------------------------------------------------------
// Replay over the environment
// ---------------------------------------------------------------------------

/// Row 8: `ref(a)` over S1 completes `false`.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn tc_1042_row_08_a_held_reference_dereferences() {
    let graph = graph();
    let wire = environment(graph.input("fa", graph.node("a")), &graph.s1(), true);
    assert_completes_false(run(wire), false);
}

/// Row 9: the same from a `Witness` entry.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn tc_1042_row_09_the_witness_arm_reproduces_with_the_evaluated_witness() {
    let graph = graph();
    let wire = environment(graph.witness("fa", &graph.node("a")), &graph.s1(), true);
    assert_completes_false(run(wire), true);
}

/// Row 10: `ref(b)` completes `true`.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn tc_1042_row_10_a_held_reference_can_complete_true() {
    let graph = graph();
    let wire = environment(graph.input("fa", graph.node("b")), &graph.s1(), true);
    assert_completes_true(run(wire));
}

/// Row 11: a `Hub` reference for a `Node` parameter is held and completes.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn tc_1042_row_11_a_proper_subtype_reference_dereferences() {
    let graph = graph();
    let hub = graph.reference("h", graph.type_of("fh"));
    let wire = environment(graph.input("fa", hub), &graph.s1(), true);
    assert_completes_false(run(wire), false);
}

/// Row 12: the document's anchor is not compared.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn tc_1042_row_12_the_anchor_is_not_compared() {
    let graph = graph();
    let document = graph.document(Variant {
        anchor: Some("other"),
        ..Variant::default()
    });
    let wire = environment(graph.input("fa", graph.node("a")), &document, true);
    assert_completes_false(run(wire), false);
}

/// Row 13: a sequence of references completes `false` after reading `b`,
/// then `a`.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn tc_1042_row_13_a_sequence_of_references_dereferences_in_order() {
    let graph = graph();
    let sequence = WitnessValue::Sequence(vec![graph.node("b"), graph.node("a")]);
    let wire = environment(graph.input("fe", sequence), &graph.s1(), true);
    assert_completes_false(run(wire), false);
}

/// Row 14: no environment, a dereference completes no value.
#[trace("TC-1042", "FR-098-AC-15")]
#[test]
fn tc_1042_row_14_no_environment_completes_no_value() {
    let graph = graph();
    assert_no_value(run(graph.input("fa", graph.node("a"))));
}

/// Row 15: a comparison with itself completes `true` with and without S1.
#[trace("TC-1042", "FR-098-AC-15")]
#[test]
fn tc_1042_row_15_a_reference_compared_with_itself_completes_true() {
    let graph = graph();
    assert_completes_true(run(graph.input("fb", graph.node("a"))));
    let wire = environment(graph.input("fb", graph.node("a")), &graph.s1(), true);
    assert_completes_true(run(wire));
}

/// Row 16: a record entry for the `Reference` parameter is the wrong kind.
#[trace("TC-1042", "FR-098-AC-15")]
#[test]
fn tc_1042_row_16_a_record_for_a_reference_is_the_wrong_kind() {
    let graph = graph();
    let record = WitnessValue::Record {
        declaration: WireNodeId::from_digest(graph.type_of("fa")),
        fields: vec![
            WitnessField {
                name: "items".to_owned(),
                slot: WitnessSlot::Present(WitnessValue::Sequence(Vec::new())),
            },
            WitnessField {
                name: "next".to_owned(),
                slot: WitnessSlot::Absent,
            },
        ],
    };
    let wire = environment(graph.witness("fa", &record), &graph.s1(), true);
    wrong_kind(run(wire), "a record for a Reference parameter");
}

// ---------------------------------------------------------------------------
// Reference conversion, before the document is read
// ---------------------------------------------------------------------------

/// Row 17: a foreign universe is decided before conformance; a same-universe
/// non-subtype is the wrong kind.
#[trace("TC-1042", "FR-098-AC-16")]
#[test]
fn tc_1042_row_17_a_foreign_universe_is_decided_before_conformance() {
    let graph = graph();
    let s1 = graph.s1();
    let lone = graph.reference("z", graph.type_of("fl"));
    let refusal = run(environment(graph.input("fa", lone), &s1, false)).unwrap_err();
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::ForeignReference, Some("foreign-universe"))
    );
    let ReplayRefusal::ForeignReference {
        position,
        required,
        supplied,
    } = refusal
    else {
        panic!("expected a foreign reference, got {refusal:?}");
    };
    assert_eq!(position, 0);
    assert_eq!(required, graph.universe_of(graph.type_of("fa")));
    assert_eq!(supplied, graph.universe_of(graph.type_of("fl")));
    assert_ne!(required, supplied);
    wrong_kind(
        run(environment(graph.input("fh", graph.node("a")), &s1, false)),
        "a Node for a Reference<Hub>",
    );
}

/// Row 18: an object type the model does not have.
#[trace("TC-1042", "FR-098-AC-16")]
#[test]
fn tc_1042_row_18_an_unknown_object_type_is_the_wrong_kind() {
    let graph = graph();
    let unknown = graph.reference("a", [0; 32]);
    let wire = environment(graph.input("fa", unknown), &graph.s1(), false);
    wrong_kind(run(wire), "an object type no model declares");
}

/// Row 19: an empty identity, and one that is not UTF-8.
#[trace("TC-1042", "FR-098-AC-16")]
#[test]
fn tc_1042_row_19_a_bad_identity_is_the_wrong_kind() {
    let graph = graph();
    let s1 = graph.s1();
    for identity in [Vec::new(), vec![0xff]] {
        let WitnessValue::Reference {
            universe,
            object_type,
            ..
        } = graph.node("a")
        else {
            panic!("a reference");
        };
        let value = WitnessValue::Reference {
            universe,
            object_type,
            identity: identity.clone(),
        };
        wrong_kind(
            run(environment(graph.input("fa", value), &s1, false)),
            &format!("identity {identity:?}"),
        );
    }
}

// ---------------------------------------------------------------------------
// Unused environment
// ---------------------------------------------------------------------------

/// Row 20: an environment for a function with no `Reference` parameter.
#[trace("TC-1042", "FR-098-AC-17")]
#[test]
fn tc_1042_row_20_an_unused_environment_is_refused() {
    let graph = graph();
    let wire = environment(
        graph.input("fc", WitnessValue::Integer(7)),
        &graph.s1(),
        false,
    );
    let refusal = run(wire).unwrap_err();
    assert!(matches!(refusal, ReplayRefusal::UnusedObjectEnvironment));
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, Some("unknown-member"))
    );
}

/// Row 21: the check precedes conversion.
#[trace("TC-1042", "FR-098-AC-17")]
#[test]
fn tc_1042_row_21_the_unused_check_precedes_conversion() {
    let graph = graph();
    let wire = environment(
        graph.input("fc", WitnessValue::Boolean(true)),
        &graph.s1(),
        true,
    );
    assert!(matches!(
        run(wire),
        Err(ReplayRefusal::UnusedObjectEnvironment)
    ));
}

// ---------------------------------------------------------------------------
// Environment admission
// ---------------------------------------------------------------------------

fn admit_variant(
    graph: &Graph,
    variant: Variant,
    argument: WitnessValue,
) -> ObjectEnvironmentFailure {
    let document = graph.document(variant);
    environment_failure(run(environment(
        graph.input("fa", argument),
        &document,
        true,
    )))
}

/// Row 22: the document is not in the byte provision (check 1).
#[trace("TC-1042", "FR-098-AC-18")]
#[test]
fn tc_1042_row_22_a_missing_document_is_incomplete() {
    let graph = graph();
    let wire = environment(graph.input("fa", graph.node("a")), &graph.s1(), false);
    let failure = environment_failure(run(wire));
    admission(
        &failure,
        true,
        Code::UnavailableObservation,
        "missing-required-artifact",
    );
}

/// Row 23: a `pre` document (check 3).
#[trace("TC-1042", "FR-098-AC-18")]
#[test]
fn tc_1042_row_23_a_pre_document_is_the_wrong_observation() {
    let graph = graph();
    let failure = admit_variant(
        &graph,
        Variant {
            observation: Some("pre"),
            ..Variant::default()
        },
        graph.node("a"),
    );
    admission(&failure, false, Code::WrongSnapshot, "wrong-observation");
}

/// Row 24: a model that is none of the package's selections (check 4).
#[trace("TC-1042", "FR-098-AC-18")]
#[test]
fn tc_1042_row_24_a_foreign_model_is_refused() {
    let graph = graph();
    let failure = admit_variant(
        &graph,
        Variant {
            model_digest: Some([0xaa; 32]),
            ..Variant::default()
        },
        graph.node("a"),
    );
    admission(
        &failure,
        false,
        Code::InvalidModelBinding,
        "wrong-model-selection",
    );
}

/// Row 25: a duplicate key is decided before a dangling argument (check 6.3).
#[trace("TC-1042", "FR-098-AC-18")]
#[test]
fn tc_1042_row_25_a_duplicate_key_precedes_a_dangling_argument() {
    let graph = graph();
    let failure = admit_variant(
        &graph,
        Variant {
            duplicate_a: true,
            ..Variant::default()
        },
        graph.node("n9"),
    );
    let record = admission(
        &failure,
        false,
        Code::InvalidRuntimeInput,
        "conflicting-identity",
    );
    assert_eq!(record.fields.get("object").map(String::as_str), Some("a"));
}

/// Row 26: an ill-typed field, and one outside its range (check 6.5).
#[trace("TC-1042", "FR-098-AC-18")]
#[test]
fn tc_1042_row_26_an_ill_typed_field_is_refused() {
    let graph = graph();
    let shape = admit_variant(
        &graph,
        Variant {
            a_items: Some(json!({"integer": "3"})),
            ..Variant::default()
        },
        graph.node("a"),
    );
    let record = admission(&shape, false, Code::InvalidRuntimeInput, "wrong-value-kind");
    assert_eq!(record.fields.get("object").map(String::as_str), Some("a"));
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("items")
    );
    let range = admit_variant(
        &graph,
        Variant {
            a_items: Some(json!({"sequence": [{"integer": "12"}]})),
            ..Variant::default()
        },
        graph.node("a"),
    );
    let record = admission(&range, false, Code::InvalidRuntimeInput, "invalid-value");
    assert_eq!(record.fields.get("object").map(String::as_str), Some("a"));
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("items")
    );
}

/// Row 27: a reference to an object a complete population does not list
/// (check 8).
#[trace("TC-1042", "FR-098-AC-18")]
#[test]
fn tc_1042_row_27_a_dangling_field_reference_is_refused() {
    let graph = graph();
    let gone = json!({"present": {"reference": {"population": identity("G"), "key": "gone"}}});
    let failure = admit_variant(
        &graph,
        Variant {
            a_next: Some(gone),
            ..Variant::default()
        },
        graph.node("a"),
    );
    let record = admission(
        &failure,
        false,
        Code::DanglingReference,
        "absent-target-in-complete-population",
    );
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("gone")
    );
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(identity("G").as_str())
    );
}

/// Row 28: the population holding `a` is not complete (check 7).
#[trace("TC-1042", "FR-098-AC-18")]
#[test]
fn tc_1042_row_28_an_incomplete_required_population_is_incomplete() {
    let graph = graph();
    let failure = admit_variant(
        &graph,
        Variant {
            incomplete: true,
            ..Variant::default()
        },
        graph.node("a"),
    );
    let record = admission(
        &failure,
        true,
        Code::IncompletePopulation,
        "incomplete-scope",
    );
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(identity("G").as_str())
    );
}

// ---------------------------------------------------------------------------
// Reference resolution
// ---------------------------------------------------------------------------

/// Row 29: a key the complete population does not list.
#[trace("TC-1042", "FR-098-AC-19")]
#[test]
fn tc_1042_row_29_an_unlisted_key_is_dangling() {
    let graph = graph();
    let wire = environment(graph.input("fa", graph.node("n9")), &graph.s1(), true);
    assert_unresolved(
        &environment_failure(run(wire)),
        0,
        "n9",
        Unresolved::Dangling,
    );
}

/// Row 30: the held `a` is not `ref(a, TH)`: a key differing in type is
/// another reference.
#[trace("TC-1042", "FR-098-AC-19")]
#[test]
fn tc_1042_row_30_a_reference_of_another_type_is_dangling() {
    let graph = graph();
    let other_type = graph.reference("a", graph.type_of("fh"));
    let wire = environment(graph.input("fa", other_type), &graph.s1(), true);
    assert_unresolved(
        &environment_failure(run(wire)),
        0,
        "a",
        Unresolved::Dangling,
    );
}

/// Row 31: no held object names a required population, and the covering row
/// is not complete.
#[trace("TC-1042", "FR-098-AC-19")]
#[test]
fn tc_1042_row_31_an_unlisted_key_over_an_incomplete_population_is_undecidable() {
    let graph = graph();
    let document = graph.document(Variant {
        incomplete: true,
        ..Variant::default()
    });
    let wire = environment(graph.input("fa", graph.node("n9")), &document, true);
    assert_unresolved(
        &environment_failure(run(wire)),
        0,
        "n9",
        Unresolved::Undecidable,
    );
}

/// Row 32: the first reference that is not present is named.
#[trace("TC-1042", "FR-098-AC-19")]
#[test]
fn tc_1042_row_32_the_first_missing_reference_is_named() {
    let graph = graph();
    let sequence = WitnessValue::Sequence(vec![graph.node("n9"), graph.node("n8")]);
    let wire = environment(graph.input("fe", sequence), &graph.s1(), true);
    assert_unresolved(
        &environment_failure(run(wire)),
        0,
        "n9",
        Unresolved::Dangling,
    );
}

/// Row 33: a present reference before the missing one is not named.
#[trace("TC-1042", "FR-098-AC-19")]
#[test]
fn tc_1042_row_33_a_present_reference_is_not_named() {
    let graph = graph();
    let sequence = WitnessValue::Sequence(vec![graph.node("a"), graph.node("n9")]);
    let wire = environment(graph.input("fe", sequence), &graph.s1(), true);
    assert_unresolved(
        &environment_failure(run(wire)),
        0,
        "n9",
        Unresolved::Dangling,
    );
}

// ---------------------------------------------------------------------------
// The writer, the labels, and the fault mapping
// ---------------------------------------------------------------------------

/// FR-071: admission selects the document by digest alone, so labels that
/// are not the document's own identity change nothing.
#[test]
fn the_member_labels_are_not_compared_with_the_documents_identity() {
    let graph = graph();
    let s1 = graph.s1();
    let mut wire = graph.input("fa", graph.node("a"));
    wire.byte_provision.push(entry(&s1.bytes));
    wire.object_environment = Some(
        ObjectEnvironmentRef::new(
            EnvironmentLabels {
                authority: "elsewhere".to_owned(),
                identity: "ix://elsewhere/other".to_owned(),
                revision_namespace: "svn".to_owned(),
                revision: "9".to_owned(),
            },
            DigestRecord::mint(DigestDomain::Sha256Jcs, s1.digest),
        )
        .expect("a sha256-jcs digest"),
    );
    assert_completes_false(run(wire), false);
}

/// The builder's document is the one admission reads: `Node.items` of length
/// 0, 1 and 3 round-trips through the executor, and the deref completes the
/// value `forall(x in items: x < 5)` has over those items.
#[test]
fn a_written_environment_document_round_trips_through_admission() {
    let graph = graph();
    let model = EnvironmentModel {
        identity: PACKAGE.to_owned(),
        version: "1.0.0".to_owned(),
        digest: sha256_jcs(&graph.package),
    };
    for (items, holds) in [(vec![], true), (vec![4], true), (vec![3, 7, 1], false)] {
        let node = EnvironmentObject {
            key: "a".to_owned(),
            type_identity: crate::DeclarationKey {
                package: PACKAGE.to_owned(),
                node: identity("Node"),
            },
            fields: vec![
                (
                    "items".to_owned(),
                    SnapshotValue::sequence(
                        items
                            .iter()
                            .map(|item: &i64| SnapshotValue::integer(item.to_string()))
                            .collect(),
                    ),
                ),
                ("next".to_owned(), SnapshotValue::absent()),
            ],
        };
        let document = write_object_environment(
            labels(),
            &model,
            &[EnvironmentPopulation {
                population: identity("G"),
                complete: true,
                objects: vec![node],
            }],
        )
        .expect("the document is written");
        assert_eq!(
            sha256_jcs(document.bytes()),
            *document.member().digest().as_bytes()
        );
        let mut wire = graph.input("fa", graph.node("a"));
        wire.byte_provision.push(document.provision_entry());
        wire.object_environment = Some(document.member().clone());
        if holds {
            assert_completes_true(run(wire));
        } else {
            assert_completes_false(run(wire), false);
        }
    }
}

/// A fault in admission or in the bundle stays a tool failure
/// (ADR-013 C-09); a refusal stays the environment's.
#[test]
fn an_admission_fault_stays_a_fault() {
    use crate::execute::object_environment::environment_refusal;
    use crate::proof_result::TerminalValue;
    let fault = AdmissionFailure::Fault(qsl_foundation::diagnostic::InternalFault::new(
        "test",
        "broken-invariant",
    ));
    let refusal = environment_refusal(fault);
    assert!(matches!(
        refusal,
        ReplayRefusal::Admission(AdmissionFailure::Fault(_))
    ));
    assert!(matches!(
        TerminalValue::from_replay_refusal(&refusal),
        TerminalValue::Failed
    ));
    let refused = environment_refusal(AdmissionFailure::Refused(AdmissionRecord {
        code: Code::WrongSnapshot,
        cause: "wrong-observation",
        fields: BTreeMap::new(),
        path: Vec::new(),
        kernel: None,
    }));
    assert!(matches!(refused, ReplayRefusal::ObjectEnvironment(_)));
}

/// FR-106 check 6.1: an object whose type is not an object type of the
/// selected model (here a type of another package) refuses
/// `invalid_runtime_input`/`wrong-role-mapping`, naming the object.
#[test]
fn an_object_of_a_type_the_model_does_not_declare_is_refused() {
    let graph = graph();
    let failure = admit_variant(
        &graph,
        Variant {
            a_type: Some("ix://other/package/Node"),
            ..Variant::default()
        },
        graph.node("a"),
    );
    let record = admission(
        &failure,
        false,
        Code::InvalidRuntimeInput,
        "wrong-role-mapping",
    );
    assert_eq!(record.fields.get("object").map(String::as_str), Some("a"));
}

fn node_key() -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: identity("Node"),
    }
}

/// FR-098, FR-070: the builder's witness for `Node`/`a` equals the one replay
/// resolves for a `Reference<Node>` parameter, and replayed over the document
/// `write_object_environment` wrote it dereferences `a` (`items` [3, 7, 1],
/// where 7 is not below 5) and completes `false`.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn the_built_reference_witness_resolves_the_written_object() {
    let graph = graph();
    let witness = object_reference_witness(
        graph.compiled.package.graph().model_selections(),
        &graph.packages,
        SpineLimits::default().model,
        &node_key(),
        "a",
    )
    .expect("Node is declared");
    assert_eq!(witness, graph.node("a"));

    let items = SnapshotValue::sequence(
        ["3", "7", "1"]
            .into_iter()
            .map(SnapshotValue::integer)
            .collect(),
    );
    let object = EnvironmentObject {
        key: "a".to_owned(),
        type_identity: node_key(),
        fields: vec![
            ("items".to_owned(), items),
            ("next".to_owned(), SnapshotValue::absent()),
        ],
    };
    let document = write_object_environment(
        labels(),
        &EnvironmentModel {
            identity: PACKAGE.to_owned(),
            version: "1.0.0".to_owned(),
            digest: sha256_jcs(&graph.package),
        },
        &[EnvironmentPopulation {
            population: identity("G"),
            complete: true,
            objects: vec![object],
        }],
    )
    .expect("the document has a canonical form");
    let mut wire = graph.input("fa", witness);
    wire.byte_provision.push(document.provision_entry());
    wire.object_environment = Some(document.member().clone());
    assert_completes_false(run(wire), false);
}

/// A type no selected model declares is refused by name.
#[trace("TC-1042", "FR-098-AC-14")]
#[test]
fn a_type_no_selected_model_declares_is_refused() {
    let graph = graph();
    let missing = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: identity("Missing"),
    };
    let refusal = object_reference_witness(
        graph.compiled.package.graph().model_selections(),
        &graph.packages,
        SpineLimits::default().model,
        &missing,
        "a",
    );
    assert!(matches!(
        refusal,
        Err(ObjectReferenceError::UndeclaredType(key)) if key == missing
    ));
}
