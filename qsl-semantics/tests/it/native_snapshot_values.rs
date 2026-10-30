// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106's String, UUID and Timestamp native payload objects in snapshots and
//! invocations (TC-1029, TC-1036): admission of the canonical forms, each
//! refusal in the native payload order with its code, cause, member, nested
//! path and kernel class, declared-type fit, frame comparison by payload and
//! value counting. Every expected result is the spec table's; assertions
//! compare typed code, cause, path, member and class, never rendered text.

use std::collections::BTreeMap;
use std::fmt::Debug;

use ix_trace_rs::trace;
use qsl_forms::{build_unit, StateClauseKind};
use qsl_foundation::{Code, SourceIdentity};
use qsl_semantics::check::{CheckedGraph, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::observation::{
    admit_current_snapshot, admit_frame_invocation, admit_observations, population_universe_for,
    AdmissionFailure, AdmittedObservations, ClauseFacts, ClauseSelection, ClauseSelectionInput,
    DocumentRef, FrameChange, FrameFacts, FrameVerdict, NativeKind, ObservationLimits,
    OperationFacts, PathStep, PostStateRange, Provisions, RawNativeKind, SelectedObject,
    SnapshotValue,
};
use quire_exact::{ConstructionCause, FieldValue, Identifier, Value as Datum};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::FieldRef;
use quire_semantic_value::object_closure::ObjectClosure;
use serde_json::{json, Value};

#[path = "../support/native_values.rs"]
mod fixture;

use fixture::id;

/// The fixture object `Item` snapshots hold (TC-1038 calls it `child`).
const OBJECT: &str = "child";

const UUID_TEXT: &str = "00112233-4455-6677-8899-aabbccddeeff";
const OTHER_UUID: &str = "ffeeddcc-bbaa-9988-7766-554433221100";

fn uuid(text: &str) -> Value {
    json!({"kind": "uuid", "text": text})
}

fn timestamp(nanoseconds: &str) -> Value {
    json!({"kind": "timestamp", "nanoseconds": nanoseconds})
}

fn string(text: &str) -> Value {
    json!({"kind": "string", "text": text})
}

fn text(text: &str) -> Value {
    json!({"kind": "text", "text": text})
}

fn integer(spelling: &str) -> Value {
    json!({"integer": spelling})
}

/// A document's reference: its identity and the digest of its bytes.
fn document_ref(label: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "1".to_owned(),
    }
}

fn identity_json(reference: &DocumentRef) -> Value {
    json!({
        "authority": reference.authority, "identity": reference.identity,
        "revision_namespace": reference.revision_namespace, "revision": reference.revision,
    })
}

/// The fixture's package selected as model `Config`, checked.
struct Replay {
    declarations: PackageDeclarations,
    graph: CheckedGraph,
    packages: qsl_semantics::model::intake::PackageInput,
    model: Value,
    /// The field values of `Item.i1` in the base snapshot.
    base: Vec<(String, Value)>,
    /// Whether the package is the collection fixture (TC-1171), whose
    /// snapshots also hold `z1` of `items` and `a1` of `others`.
    collections: bool,
}

/// The collection fixture (TC-1171): `Item` also holding `ids`, `counts`,
/// `order`, `stamps` and `peers`.
fn collection_replay() -> Replay {
    replay_of(fixture::collection_document(), collection_fields())
}

fn replay() -> Replay {
    replay_of(fixture::document(), base_fields())
}

/// The Text fixture (TC-1038): `Item` also holding `title`, `subtitle`,
/// `code` and `aliases`.
fn text_replay() -> Replay {
    replay_of(fixture::text_document(), text_fields())
}

/// The Quantity fixture (FR-106-AC-59): `Item` also holding `reach` of the
/// unit `ms`, which its source unit declares.
fn quantity_replay() -> Replay {
    replay_with(
        fixture::quantity_document(),
        quantity_fields(),
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n",
    )
}

fn replay_of(document: Value, base: Vec<(String, Value)>) -> Replay {
    replay_with(document, base, "")
}

/// [`replay_of`] with `declarations` added to the source unit.
fn replay_with(document: Value, base: Vec<(String, Value)>, declarations: &str) -> Replay {
    let bytes = serde_json::to_vec(&document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]).unwrap();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {package:?};\n\
         {declarations}\
         function keep using v(): Boolean pure {{ true }}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "native-values", "fixture", "fixture:1"),
        "native-values.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let assemble = || {
        let unit = build_unit(&parsed).expect("S2 builds the source unit");
        let selected = admit_unit(
            &unit.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("I1 admits and normalizes the fixture package");
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            selected,
            Vec::new(),
        )
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    };
    let declarations = assemble();
    let graph = assemble()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let model = json!({
        "identity": fixture::PACKAGE, "version": "1",
    });
    let collections = base.iter().any(|(name, _)| name == "peers");
    Replay {
        declarations,
        graph,
        packages,
        model,
        base,
        collections,
    }
}

/// The field values of `Item.i1` in document order. `put` replaces one by
/// name.
fn base_fields() -> Vec<(String, Value)> {
    [
        ("item_id", integer("1")),
        ("token", uuid(UUID_TEXT)),
        ("stamp", timestamp("0")),
        ("label", string("ready")),
        ("prior", json!({"absent": {}})),
        ("seen", json!({"sequence": []})),
        ("tags", json!({"sequence": []})),
        ("versionNumber", integer("1")),
        ("parent", json!({"absent": {}})),
        ("note", json!({"absent": {}})),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value))
    .collect()
}

/// TC-1038's base `Item.i1`: [`base_fields`] and then `title`, `subtitle`,
/// `code` and `aliases`.
fn text_fields() -> Vec<(String, Value)> {
    let mut fields = base_fields();
    fields.extend(
        [
            ("title", text("Ready")),
            ("subtitle", json!({"absent": {}})),
            ("code", text("abc")),
            ("address", address("Main")),
            ("aliases", json!({"sequence": []})),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value)),
    );
    fields
}

/// `{"record": {"type": Address, "fields": {"street": <Text>}}}`.
fn address(street: &str) -> Value {
    json!({"record": {"type": fixture::id("Address"), "fields": {"street": text(street)}}})
}

fn with(mut fields: Vec<(String, Value)>, name: &str, value: Value) -> Vec<(String, Value)> {
    let slot = fields
        .iter_mut()
        .find(|(held, _)| held == name)
        .expect("a declared field");
    slot.1 = value;
    fields
}

/// `fields` as the JSON text of an object, members in the order given.
fn fields_text(fields: &[(String, Value)]) -> String {
    let members: Vec<String> = fields
        .iter()
        .map(|(name, value)| format!("{}:{value}", json!(name)))
        .collect();
    format!("{{{}}}", members.join(","))
}

/// The bytes of a snapshot document holding `Item.i1` with `fields`, the
/// fields written in the order given (`observation.values` counts them in
/// document order).
fn snapshot_text(
    replay: &Replay,
    observation: &str,
    label: &str,
    fields: &[(String, Value)],
) -> String {
    // The collection fixture's snapshot lists `a1` of `others` and `z1` of
    // `items` before `child`, so the last value in walk order is the last
    // field of `child`.
    let populations = if replay.collections {
        let empty = fields_text(&collection_fields());
        json!([
            {
                "population": id("others"), "complete": true,
                "objects": [{"key": "a1", "type": id("Item"), "fields": "EMPTY"}],
            },
            {
                "population": id("items"), "complete": true,
                "objects": [
                    {"key": "z1", "type": id("Item"), "fields": "EMPTY"},
                    {"key": OBJECT, "type": id("Item"), "fields": "FIELDS"},
                ],
            },
        ])
        .to_string()
        .replace("\"EMPTY\"", &empty)
        .parse::<Value>()
        .expect("the populations are JSON")
    } else {
        json!([{
            "population": id("items"), "complete": true,
            "objects": [{"key": OBJECT, "type": id("Item"), "fields": "FIELDS"}],
        }])
    };
    let document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": "test", "identity": label,
            "revision_namespace": "fixture", "revision": "1",
        },
        "observation": observation,
        "model": replay.model,
        "populations": populations,
    });
    document
        .to_string()
        .replace("\"FIELDS\"", &fields_text(fields))
}

/// A snapshot of `fields`, its reference and bytes.
fn snapshot(
    replay: &Replay,
    observation: &str,
    label: &str,
    fields: &[(String, Value)],
) -> (DocumentRef, Vec<u8>) {
    let bytes = snapshot_text(replay, observation, label, fields).into_bytes();
    (document_ref(label), bytes)
}

/// A refusal as its typed parts.
#[derive(Debug, PartialEq)]
struct Refusal {
    code: Code,
    cause: String,
    field: Option<String>,
    path: Vec<PathStep>,
    member: Option<String>,
    kernel: Option<ConstructionCause>,
}

/// The refusal a defect at `field` (a field, parameter or `result`) gives:
/// `invalid_runtime_input` with `cause`, at `path`, naming `member`, over the
/// kernel class `kernel`.
fn expect(
    cause: &str,
    field: &str,
    path: &[PathStep],
    member: Option<&str>,
    kernel: Option<ConstructionCause>,
) -> Refusal {
    Refusal {
        code: Code::InvalidRuntimeInput,
        cause: cause.to_owned(),
        field: Some(field.to_owned()),
        path: path.to_vec(),
        member: member.map(str::to_owned),
        kernel,
    }
}

fn refusal_of<T: Debug>(outcome: Result<T, AdmissionFailure>) -> Refusal {
    match outcome {
        Err(AdmissionFailure::Refused(record)) => Refusal {
            code: record.code,
            cause: record.cause.to_owned(),
            field: record.fields.get("field").cloned(),
            path: record.path,
            member: record.fields.get("member").cloned(),
            kernel: record.kernel,
        },
        other => panic!("a refusal, not {other:?}"),
    }
}

const NONCANONICAL_UUID: Option<ConstructionCause> = Some(ConstructionCause::UuidNoncanonical);
const NONCANONICAL_TIMESTAMP: Option<ConstructionCause> =
    Some(ConstructionCause::TimestampNoncanonical);
const OUT_OF_DOMAIN: Option<ConstructionCause> = Some(ConstructionCause::TimestampOutOfDomain);

/// How an invocation of an `Item` operation is selected.
#[derive(Clone, Copy)]
enum Selected {
    Postcondition,
    Precondition,
}

impl Replay {
    fn item(&self) -> quire_exact::EffectiveId {
        self.declarations
            .types
            .object_types()
            .find(|declared| declared.name() == "Config::Item")
            .expect("Item is declared")
            .key()
    }

    fn operation(&self, name: &str) -> OperationFacts {
        let item = self.item();
        let declaration = self
            .declarations
            .operations
            .declared_by(item)
            .iter()
            .find(|operation| operation.name() == name)
            .expect("the operation is declared")
            .clone();
        OperationFacts {
            declaring: item,
            declaration,
        }
    }

    /// `fields` as the current snapshot's `Item.i1`, admitted.
    fn admit(
        &self,
        fields: &[(String, Value)],
        limits: ObservationLimits,
    ) -> Result<ObjectClosure, AdmissionFailure> {
        let (reference, bytes) = snapshot(self, "current", "current", fields);
        let provision = BTreeMap::from([(reference.clone(), bytes)]);
        admit_current_snapshot(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &provision,
            &reference,
            limits,
        )
        .map(|(environment, _)| environment.objects().clone())
    }

    /// `fields` as the current snapshot, admitted with its usage.
    fn values_used(&self, fields: &[(String, Value)]) -> u64 {
        let (reference, bytes) = snapshot(self, "current", "current", fields);
        let provision = BTreeMap::from([(reference.clone(), bytes)]);
        admit_current_snapshot(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &provision,
            &reference,
            ObservationLimits::default(),
        )
        .expect("the document admits")
        .1
        .values
    }

    /// The slot `field` of the admitted `Item.i1`.
    fn slot(&self, closure: &ObjectClosure, field: &str) -> FieldValue {
        let item = self.item();
        let universe = population_universe_for(
            self.graph.model_selections(),
            &self.packages,
            ModelNormalizationLimits::default(),
            item,
        )
        .expect("the Item universe");
        let reference = closure.find(universe, OBJECT).expect("the admitted object");
        closure
            .attribute(
                self.graph.scope().types(),
                reference,
                &FieldRef::new(item, field),
            )
            .cloned()
            .expect("the field slot")
    }

    /// The value `field` was admitted with.
    fn value(&self, closure: &ObjectClosure, field: &str) -> Datum {
        let FieldValue::Present(value) = self.slot(closure, field) else {
            panic!("{field} holds a value");
        };
        value
    }

    /// The invocation of `operation` over the snapshot texts `pre` and `post`,
    /// with the given parameters and result.
    fn invocation_over(
        &self,
        operation: &str,
        pre: &str,
        post: &str,
        parameters: Value,
        result: Value,
    ) -> Invoked {
        let (pre, pre_bytes) = (document_ref("pre"), pre.as_bytes().to_vec());
        let (post, post_bytes) = (document_ref("post"), post.as_bytes().to_vec());
        let call = DocumentRef {
            authority: "test".to_owned(),
            identity: "call".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "1".to_owned(),
        };
        let reference = |document: &DocumentRef| {
            json!({
                "identity": identity_json(document),
            })
        };
        let bytes = serde_json::to_vec(&json!({
            "format": "quire.state.invocation/v1",
            "identity": identity_json(&call),
            "model": self.model,
            "context": id("Item"),
            "operation": operation,
            "self": {"population": id("items"), "key": OBJECT},
            "pre": reference(&pre),
            "post": reference(&post),
            "parameters": parameters,
            "result": result,
            "created": [],
            "deleted": [],
        }))
        .expect("the invocation serializes");
        let call = document_ref("call");
        Invoked {
            snapshots: BTreeMap::from([(pre.clone(), pre_bytes), (post.clone(), post_bytes)]),
            invocations: BTreeMap::from([(call.clone(), bytes)]),
            call,
        }
    }

    /// The same over the field values `pre` and `post`.
    fn invocation(
        &self,
        operation: &str,
        pre: &[(String, Value)],
        post: &[(String, Value)],
        parameters: Value,
        result: Value,
    ) -> Invoked {
        self.invocation_over(
            operation,
            &snapshot_text(self, "pre", "pre", pre),
            &snapshot_text(self, "post", "post", post),
            parameters,
            result,
        )
    }

    /// `operation` invoked with `parameters` and `result` over the base
    /// fields, selected as `selected`.
    fn invoke(
        &self,
        operation: &str,
        selected: Selected,
        parameters: Value,
        result: Value,
        limits: ObservationLimits,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let fields = self.base.clone();
        let invoked = self.invocation(operation, &fields, &fields, parameters, result);
        self.select(
            operation,
            selected,
            &invoked.snapshots,
            &invoked.invocations,
            ClauseSelectionInput::Invocation {
                invocation: invoked.call,
            },
            limits,
        )
    }

    /// The same, with a post snapshot of `post` fields.
    fn invoke_with_post(
        &self,
        operation: &str,
        post: &[(String, Value)],
        limits: ObservationLimits,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let (parameters, result) = default_call(operation);
        let invoked = self.invocation(operation, &self.base, post, parameters, result);
        self.select(
            operation,
            Selected::Postcondition,
            &invoked.snapshots,
            &invoked.invocations,
            ClauseSelectionInput::Invocation {
                invocation: invoked.call,
            },
            limits,
        )
    }

    fn select(
        &self,
        operation: &str,
        selected: Selected,
        snapshots: &qsl_semantics::model::observation::DocumentProvision,
        invocations: &qsl_semantics::model::observation::DocumentProvision,
        input: ClauseSelectionInput,
        limits: ObservationLimits,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let kind = match selected {
            Selected::Postcondition => StateClauseKind::Postcondition,
            Selected::Precondition => StateClauseKind::Precondition,
        };
        admit_observations(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &ClauseFacts {
                identity: quire_exact::NodeKey::from_digest([7; 32]),
                kind,
                context: self.item(),
                operation: Some(self.operation(operation)),
                operations: Default::default(),
                named_populations: Vec::new(),
            },
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots,
                invocations,
            },
            &ClauseSelection {
                name: "Clause".to_owned(),
                input,
            },
            limits,
        )
    }

    /// `operation` selected through `PreCall` over the base pre snapshot.
    fn pre_call(
        &self,
        operation: &str,
        parameters: Vec<(&str, SnapshotValue)>,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let (reference, bytes) = snapshot(self, "pre", "pre", &self.base);
        let snapshots = BTreeMap::from([(reference.clone(), bytes)]);
        let parameters = parameters
            .into_iter()
            .map(|(name, value)| (Identifier::new(name).expect("an identifier"), value))
            .collect();
        self.select(
            operation,
            Selected::Precondition,
            &snapshots,
            &BTreeMap::new(),
            ClauseSelectionInput::PreCall {
                snapshot: reference,
                self_object: Some(SelectedObject {
                    population: id("items"),
                    key: OBJECT.to_owned(),
                }),
                parameters,
            },
            ObservationLimits::default(),
        )
    }

    /// Check 11 over `operation` changing the base fields to `post`.
    fn frame(&self, operation: &str, post: &[(String, Value)]) -> FrameVerdict {
        self.frame_from(operation, &self.base, post)
    }

    /// Check 11 over `operation` changing the field values `pre` to `post`.
    fn frame_from(
        &self,
        operation: &str,
        pre: &[(String, Value)],
        post: &[(String, Value)],
    ) -> FrameVerdict {
        self.frame_over(
            operation,
            &snapshot_text(self, "pre", "pre", pre),
            &snapshot_text(self, "post", "post", post),
        )
    }

    /// Check 11 over `operation` between the snapshot texts `pre` and `post`.
    fn frame_over(&self, operation: &str, pre: &str, post: &str) -> FrameVerdict {
        let (parameters, result) = default_call(operation);
        let invoked = self.invocation_over(operation, pre, post, parameters, result);
        let facts = FrameFacts {
            context: self.item(),
            frame: quire_exact::NodeKey::from_digest([8; 32]),
            operation: self.operation(operation),
        };
        let admitted = admit_frame_invocation(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &facts,
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &invoked.snapshots,
                invocations: &invoked.invocations,
            },
            &invoked.call,
            ObservationLimits::default(),
        )
        .unwrap_or_else(|failure| panic!("the invocation admits: {failure:?}"));
        admitted
            .check_frame(facts.operation.declaration.effect())
            .expect("check 11 evaluates")
    }
}

/// An invocation document, its two snapshots and its reference.
struct Invoked {
    snapshots: qsl_semantics::model::observation::DocumentProvision,
    invocations: qsl_semantics::model::observation::DocumentProvision,
    call: DocumentRef,
}

/// An operation's well-formed parameters and result.
fn default_call(operation: &str) -> (Value, Value) {
    match operation {
        "touch" | "hold" => (
            json!({"id": uuid(UUID_TEXT), "at": timestamp("0")}),
            uuid(OTHER_UUID),
        ),
        "retitle" | "rekeep" => (json!({"name": text("Ready")}), text("Steady")),
        "retag" | "restock" => (retag_parameters(), json!({"set": []})),
        _ => (json!({"name": string("n")}), string("r")),
    }
}

/// Whether two values are one value by the kernel's canonical key order.
fn same(left: &Datum, right: &Datum) -> bool {
    quire_exact::compare_keys(left, right) == Some(std::cmp::Ordering::Equal)
}

/// Whether `value` is the String of exactly `text`.
fn is_string(value: &Datum, text: &str) -> bool {
    matches!(value, Datum::String(held) if held == text)
}

fn writes(change: &FrameChange, field: &str) -> bool {
    matches!(change, FrameChange::FieldWrite { object, field: found, .. }
        if object == OBJECT && found == field)
}

/// The refusal an admitted document gives where `field` holds `value`.
fn refused_field(replay: &Replay, field: &str, value: Value) -> Refusal {
    refusal_of(replay.admit(
        &with(base_fields(), field, value),
        ObservationLimits::default(),
    ))
}

// ---------------------------------------------------------------------------
// TC-1029: UUID and Timestamp
// ---------------------------------------------------------------------------

/// TC-1029 step 1 (FR-106-AC-18): the base snapshot admits `token` as a UUID
/// and `stamp` as a Timestamp of the written payloads; reordering the members
/// of either object gives the same typed values.
#[trace("TC-1029", "FR-106-AC-18")]
#[test]
fn the_base_snapshot_admits_a_uuid_and_a_timestamp() {
    let replay = replay();
    let closure = replay
        .admit(&base_fields(), ObservationLimits::default())
        .unwrap_or_else(|failure| panic!("the snapshot admits: {failure:?}"));
    let Datum::Uuid(token) = replay.value(&closure, "token") else {
        panic!("token is a UUID");
    };
    assert_eq!(
        token.octets(),
        [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff
        ]
    );
    let Datum::Timestamp(stamp) = replay.value(&closure, "stamp") else {
        panic!("stamp is a Timestamp");
    };
    assert_eq!(stamp.nanoseconds(), 0);

    let reordered = with(
        with(
            base_fields(),
            "token",
            json!({"text": UUID_TEXT, "kind": "uuid"}),
        ),
        "stamp",
        json!({"nanoseconds": "0", "kind": "timestamp"}),
    );
    let again = replay
        .admit(&reordered, ObservationLimits::default())
        .expect("the reordered document admits");
    assert!(same(
        &replay.value(&again, "token"),
        &replay.value(&closure, "token")
    ));
    assert!(same(
        &replay.value(&again, "stamp"),
        &replay.value(&closure, "stamp")
    ));
}

/// TC-1029 step 2 (FR-106-AC-18): Nil, Max and a UUID with version nibble 0,
/// and Timestamps `1`, `-1` and both signed-128 endpoints admit unchanged; a
/// present `prior` and the elements of `seen` admit by the same rule.
#[trace("TC-1029", "FR-106-AC-18")]
#[test]
fn edge_uuids_and_timestamps_admit_unchanged() {
    let replay = replay();
    for text in [
        "00000000-0000-0000-0000-000000000000",
        "ffffffff-ffff-ffff-ffff-ffffffffffff",
        "00112233-4455-0677-8899-aabbccddeeff",
    ] {
        let closure = replay
            .admit(
                &with(base_fields(), "token", uuid(text)),
                ObservationLimits::default(),
            )
            .unwrap_or_else(|failure| panic!("{text} admits: {failure:?}"));
        let Datum::Uuid(token) = replay.value(&closure, "token") else {
            panic!("token is a UUID");
        };
        assert_eq!(token.to_string(), text);
    }
    for (text, expected) in [
        ("1", 1_i128),
        ("-1", -1),
        ("170141183460469231731687303715884105727", i128::MAX),
        ("-170141183460469231731687303715884105728", i128::MIN),
    ] {
        let closure = replay
            .admit(
                &with(base_fields(), "stamp", timestamp(text)),
                ObservationLimits::default(),
            )
            .unwrap_or_else(|failure| panic!("{text} admits: {failure:?}"));
        let Datum::Timestamp(stamp) = replay.value(&closure, "stamp") else {
            panic!("stamp is a Timestamp");
        };
        assert_eq!(stamp.nanoseconds(), expected);
    }
    let fields = with(
        with(base_fields(), "prior", json!({"present": uuid(OTHER_UUID)})),
        "seen",
        json!({"sequence": [timestamp("5"), timestamp("-5")]}),
    );
    let closure = replay
        .admit(&fields, ObservationLimits::default())
        .expect("a present prior and a sequence of Timestamps admit");
    assert!(matches!(
        replay.value(&closure, "prior"),
        Datum::Uuid(prior) if prior.to_string() == OTHER_UUID
    ));
    let Datum::Collection(seen) = replay.value(&closure, "seen") else {
        panic!("seen is a sequence");
    };
    let nanoseconds: Vec<i128> = seen
        .elements()
        .iter()
        .map(|element| match element {
            Datum::Timestamp(stamp) => stamp.nanoseconds(),
            other => panic!("a Timestamp, not {other:?}"),
        })
        .collect();
    assert_eq!(nanoseconds, [5, -5]);
}

/// TC-1029 step 3 (FR-106-AC-18): `touch`'s `id`, `at` and `result` admit as
/// a UUID, a Timestamp and a UUID, selected for a postcondition, through an
/// invocation selected for a precondition (which reads no result) and through
/// `PreCall`.
#[trace("TC-1029", "FR-106-AC-18")]
#[test]
fn an_invocation_admits_uuid_and_timestamp_arguments_and_result() {
    let replay = replay();
    let (parameters, result) = default_call("touch");
    for selected in [Selected::Postcondition, Selected::Precondition] {
        let admitted = replay
            .invoke(
                "touch",
                selected,
                parameters.clone(),
                result.clone(),
                ObservationLimits::default(),
            )
            .unwrap_or_else(|failure| panic!("the invocation admits: {failure:?}"));
        assert!(matches!(admitted.parameters[0].1, Datum::Uuid(_)));
        assert!(matches!(admitted.parameters[1].1, Datum::Timestamp(_)));
        match selected {
            Selected::Postcondition => assert!(matches!(admitted.result, Some(Datum::Uuid(_)))),
            Selected::Precondition => assert!(admitted.result.is_none()),
        }
    }
    let admitted = replay
        .pre_call(
            "touch",
            vec![
                (
                    "id",
                    SnapshotValue::native(
                        RawNativeKind::Known(NativeKind::Uuid),
                        vec![("text".to_owned(), Some(UUID_TEXT.to_owned()))],
                    ),
                ),
                (
                    "at",
                    SnapshotValue::native(
                        RawNativeKind::Known(NativeKind::Timestamp),
                        vec![("nanoseconds".to_owned(), Some("0".to_owned()))],
                    ),
                ),
            ],
        )
        .unwrap_or_else(|failure| panic!("the PreCall admits: {failure:?}"));
    assert!(matches!(admitted.parameters[0].1, Datum::Uuid(_)));
    assert!(matches!(admitted.parameters[1].1, Datum::Timestamp(_)));
    assert!(admitted.result.is_none());
}

/// TC-1029 step 4 (FR-106-AC-19): each noncanonical UUID text refuses
/// `invalid-value` naming `token` and member `text`, kernel class
/// `noncanonical-payload`.
#[trace("TC-1029", "FR-106-AC-19")]
#[test]
fn a_noncanonical_uuid_text_refuses_invalid_value() {
    let replay = replay();
    for text in [
        "00112233-4455-6677-8899-AABBCCDDEEFF",
        "00112233-4455-6677-8899-aabbCCddeeff",
        "00112233445566778899aabbccddeeff",
        "{00112233-4455-6677-8899-aabbccddeeff}",
        "urn:uuid:00112233-4455-6677-8899-aabbccddeeff",
        "0011223-34455-6677-8899-aabbccddeeff",
        "00112233-4455-6677-8899-aabbccddeefg",
        "00112233-4455-6677-8899-aabbccddeef",
        " 00112233-4455-6677-8899-aabbccddeeff",
        "\u{0660}0112233-4455-6677-8899-aabbccddeeff",
    ] {
        assert_eq!(
            refused_field(&replay, "token", uuid(text)),
            expect(
                "invalid-value",
                "token",
                &[],
                Some("text"),
                NONCANONICAL_UUID
            ),
            "{text:?}"
        );
    }
}

/// TC-1029 step 5 (FR-106-AC-19): a noncanonical Timestamp refuses
/// `noncanonical-payload`, a canonical one outside the signed-128 domain
/// refuses `payload-out-of-domain`, and a noncanonical and out-of-domain one
/// refuses `noncanonical-payload`.
#[trace("TC-1029", "FR-106-AC-19")]
#[test]
fn a_noncanonical_or_out_of_domain_timestamp_refuses_invalid_value() {
    let replay = replay();
    for text in [
        "-0",
        "+1",
        "01",
        "-01",
        "",
        " 1",
        "1.0",
        "1e3",
        "2026-10-15T00:00:00Z",
        "0170141183460469231731687303715884105728",
    ] {
        assert_eq!(
            refused_field(&replay, "stamp", timestamp(text)),
            expect(
                "invalid-value",
                "stamp",
                &[],
                Some("nanoseconds"),
                NONCANONICAL_TIMESTAMP
            ),
            "{text:?}"
        );
    }
    for text in [
        "170141183460469231731687303715884105728",
        "-170141183460469231731687303715884105729",
    ] {
        assert_eq!(
            refused_field(&replay, "stamp", timestamp(text)),
            expect(
                "invalid-value",
                "stamp",
                &[],
                Some("nanoseconds"),
                OUT_OF_DOMAIN
            ),
            "{text:?}"
        );
    }
}

/// TC-1029 step 6 (FR-106-AC-20): the envelope defects of a UUID or
/// Timestamp object refuse in the native order: a non-text `kind`, an unknown
/// `kind` with its members unexamined, a missing payload member, an unknown
/// member, then a payload that is not text.
#[trace("TC-1029", "FR-106-AC-20")]
#[test]
fn the_native_envelope_refuses_in_the_listed_order() {
    let replay = replay();
    let wrong = "wrong-value-kind";
    let invalid = "invalid-value";
    let uuid_rows: [(Value, &str, Option<&str>); 14] = [
        (json!({"kind": "uuid"}), "missing-member", Some("text")),
        (
            json!({"kind": "uuid", "nanoseconds": "0"}),
            "missing-member",
            Some("text"),
        ),
        (
            json!({"kind": "uuid", "text": UUID_TEXT, "extra": 1}),
            "unknown-member",
            Some("extra"),
        ),
        (json!({"kind": "uuid", "text": null}), wrong, Some("text")),
        (json!({"kind": "uuid", "text": 5}), wrong, Some("text")),
        (json!({"kind": 1, "text": UUID_TEXT}), wrong, Some("kind")),
        (
            json!({"kind": null, "text": UUID_TEXT}),
            wrong,
            Some("kind"),
        ),
        (json!({"kind": "String"}), invalid, Some("kind")),
        (
            json!({"kind": "UUID", "text": UUID_TEXT}),
            invalid,
            Some("kind"),
        ),
        (
            json!({"kind": "uuidv4", "text": UUID_TEXT}),
            invalid,
            Some("kind"),
        ),
        (
            json!({"kind": "String", "nanoseconds": "0"}),
            invalid,
            Some("kind"),
        ),
        (
            json!({"kind": "uuid", "text": "X", "extra": 1}),
            "unknown-member",
            Some("extra"),
        ),
        (json!({"kind": 1}), wrong, Some("kind")),
        (json!({"kind": {}}), wrong, Some("kind")),
    ];
    for (value, cause, member) in uuid_rows {
        assert_eq!(
            refused_field(&replay, "token", value.clone()),
            expect(cause, "token", &[], member, None),
            "{value}"
        );
    }
    for (value, cause, member) in [
        (
            json!({"kind": "timestamp"}),
            "missing-member",
            Some("nanoseconds"),
        ),
        (
            json!({"kind": "timestamp", "nanoseconds": 0}),
            wrong,
            Some("nanoseconds"),
        ),
    ] {
        assert_eq!(
            refused_field(&replay, "stamp", value.clone()),
            expect(cause, "stamp", &[], member, None),
            "{value}"
        );
    }
}

/// TC-1029 step 7 (FR-106-AC-21): a native object of another kind, a
/// single-tag object, or a native object in an integer or `Reference` slot
/// refuses `wrong-value-kind` at the nested path, even where its payload is
/// also noncanonical; the same in a parameter or result is rooted at the
/// binding.
#[trace("TC-1029", "FR-106-AC-21")]
#[test]
fn a_native_object_in_a_slot_of_another_type_refuses_wrong_value_kind() {
    let replay = replay();
    let wrong =
        |field: &str, path: &[PathStep]| expect("wrong-value-kind", field, path, None, None);
    for (field, value, path) in [
        ("token", timestamp("0"), vec![]),
        ("stamp", uuid(UUID_TEXT), vec![]),
        ("stamp", integer("5"), vec![]),
        ("token", integer("5"), vec![]),
        ("token", json!({"boolean": true}), vec![]),
        ("versionNumber", uuid(UUID_TEXT), vec![]),
        ("parent", uuid(UUID_TEXT), vec![]),
        (
            "parent",
            json!({"present": uuid(UUID_TEXT)}),
            vec![PathStep::OptionValue],
        ),
        (
            "prior",
            json!({"present": timestamp("0")}),
            vec![PathStep::OptionValue],
        ),
        (
            "seen",
            json!({"sequence": [timestamp("0"), uuid(UUID_TEXT)]}),
            vec![PathStep::Index(1)],
        ),
        ("token", timestamp("01"), vec![]),
        ("stamp", uuid("X"), vec![]),
    ] {
        assert_eq!(
            refused_field(&replay, field, value.clone()),
            wrong(field, &path),
            "{field}: {value}"
        );
    }
    let invoke = |parameters: Value, result: Value| {
        refusal_of(replay.invoke(
            "touch",
            Selected::Postcondition,
            parameters,
            result,
            ObservationLimits::default(),
        ))
    };
    let (parameters, result) = default_call("touch");
    let mut at_id = parameters.clone();
    at_id["id"] = timestamp("0");
    assert_eq!(invoke(at_id, result.clone()), wrong("id", &[]));
    let mut at_at = parameters.clone();
    at_at["at"] = uuid(UUID_TEXT);
    assert_eq!(invoke(at_at, result), wrong("at", &[]));
    assert_eq!(invoke(parameters, timestamp("0")), wrong("result", &[]));
}

/// TC-1029 step 8 (FR-106-AC-22): changing `token` and `stamp` between two
/// admitted payloads is a change the frame covers only with a grant; the same
/// payload with its members reordered is no change.
#[trace("TC-1029", "FR-106-AC-22")]
#[test]
fn the_frame_compares_uuid_and_timestamp_by_payload() {
    let replay = replay();
    let changed_token = with(base_fields(), "token", uuid(OTHER_UUID));
    let changed_stamp = with(base_fields(), "stamp", timestamp("1"));
    assert_eq!(replay.frame("touch", &changed_token), FrameVerdict::Holds);
    assert_eq!(replay.frame("touch", &changed_stamp), FrameVerdict::Holds);
    for (post, field) in [(&changed_token, "token"), (&changed_stamp, "stamp")] {
        let FrameVerdict::Violation(witness) = replay.frame("hold", post) else {
            panic!("{field}: a change without the grant violates the frame");
        };
        assert_eq!(witness.code, Code::FrameViolation);
        assert_eq!(witness.cause, "unauthorized-change");
        assert!(writes(&witness.change, field), "{:?}", witness.change);
    }
    let reordered = with(
        with(
            base_fields(),
            "token",
            json!({"text": UUID_TEXT, "kind": "uuid"}),
        ),
        "stamp",
        json!({"nanoseconds": "0", "kind": "timestamp"}),
    );
    assert_eq!(replay.frame("hold", &reordered), FrameVerdict::Holds);
}

/// TC-1029 step 9 (FR-106-AC-22): a post snapshot holding a noncanonical UUID
/// or an out-of-domain Timestamp still refuses `invalid-value` under
/// `PostStateRange::Witness`, with the step 4 and 5 classes.
#[trace("TC-1029", "FR-106-AC-22")]
#[test]
fn a_witness_post_snapshot_still_refuses_a_noncanonical_payload() {
    let replay = replay();
    let witness = ObservationLimits {
        post_state: PostStateRange::Witness,
        ..ObservationLimits::default()
    };
    for (field, value, member, kernel) in [
        (
            "token",
            uuid("00112233-4455-6677-8899-AABBCCDDEEFF"),
            "text",
            NONCANONICAL_UUID,
        ),
        (
            "stamp",
            timestamp("170141183460469231731687303715884105728"),
            "nanoseconds",
            OUT_OF_DOMAIN,
        ),
    ] {
        let post = with(base_fields(), field, value);
        assert_eq!(
            refusal_of(replay.invoke_with_post("touch", &post, witness)),
            expect("invalid-value", field, &[], Some(member), kernel),
            "{field}"
        );
    }
}

/// TC-1029 step 10 (FR-106-AC-18, FR-106-AC-22): each native object counts as
/// one value. The base snapshot holds ten value forms, one per field; the
/// exact bound admits and one less refuses `node-count-exceeded` naming
/// `observation.values`.
#[trace("TC-1029", "FR-106-AC-22")]
#[test]
fn each_native_object_counts_as_one_value() {
    let replay = replay();
    assert_eq!(replay.values_used(&base_fields()), 10);
    let exact = ObservationLimits::default().with_values_per_document(10);
    replay
        .admit(&base_fields(), exact)
        .expect("the exact count admits");
    let below = ObservationLimits::default().with_values_per_document(9);
    assert_values_limit(replay.admit(&base_fields(), below), 9);
}

fn assert_values_limit<T: Debug>(outcome: Result<T, AdmissionFailure>, bound: u64) {
    let Err(AdmissionFailure::Refused(record)) = outcome else {
        panic!("a refusal, not {outcome:?}");
    };
    assert_eq!(record.code, Code::StageLimitExceeded);
    assert_eq!(record.cause, "node-count-exceeded");
    let field = |name: &str| record.fields.get(name).map(String::as_str);
    assert_eq!(field("setting"), Some("observation.values"));
    assert_eq!(field("bound"), Some(bound.to_string().as_str()));
    assert_eq!(field("actual"), Some((bound + 1).to_string().as_str()));
}

// ---------------------------------------------------------------------------
// TC-1036: String
// ---------------------------------------------------------------------------

/// TC-1036 step 1 (FR-106-AC-28): the base snapshot admits `label` as the
/// String `ready`; reordering the members gives the same value.
#[trace("TC-1036", "FR-106-AC-28")]
#[test]
fn the_base_snapshot_admits_a_string() {
    let replay = replay();
    let closure = replay
        .admit(&base_fields(), ObservationLimits::default())
        .unwrap_or_else(|failure| panic!("the snapshot admits: {failure:?}"));
    assert!(is_string(&replay.value(&closure, "label"), "ready"));
    let again = replay
        .admit(
            &with(
                base_fields(),
                "label",
                json!({"text": "ready", "kind": "string"}),
            ),
            ObservationLimits::default(),
        )
        .expect("the reordered document admits");
    assert!(same(
        &replay.value(&again, "label"),
        &replay.value(&closure, "label")
    ));
}

/// TC-1036 step 2 (FR-106-AC-28): any Unicode scalar sequence admits
/// unchanged, composed and decomposed spellings are two values, and an
/// escaped surrogate pair is the literal character; a present `note` and the
/// elements of `tags` admit by the same rule.
#[trace("TC-1036", "FR-106-AC-28")]
#[test]
fn any_scalar_sequence_admits_as_a_string_unchanged() {
    let replay = replay();
    let long = "a".repeat(65_536);
    let texts = [
        "",
        "  ready  ",
        "READY",
        "\u{e9}",
        "e\u{301}",
        UUID_TEXT,
        "\u{0}",
        "a\u{1f}b",
        "\u{fffe}",
        "a;b>>>c",
        long.as_str(),
        "\u{1f600}",
    ];
    let mut seen = Vec::new();
    for text in texts {
        let closure = replay
            .admit(
                &with(base_fields(), "label", string(text)),
                ObservationLimits::default(),
            )
            .unwrap_or_else(|failure| panic!("{text:?} admits: {failure:?}"));
        let value = replay.value(&closure, "label");
        assert!(is_string(&value, text), "{text:?}");
        seen.push(value);
    }
    assert!(
        !same(&seen[3], &seen[4]),
        "U+00E9 and U+0065 U+0301 are two values"
    );
    // The escaped surrogate pair is written as the escape in the document.
    let (reference, bytes) = {
        let text = snapshot_text(&replay, "current", "current", &base_fields())
            .replace("\"ready\"", "\"\\ud83d\\ude00\"");
        let bytes = text.into_bytes();
        (document_ref("current"), bytes)
    };
    let provision = BTreeMap::from([(reference.clone(), bytes)]);
    let (environment, _) = admit_current_snapshot(
        replay.graph.model_selections(),
        replay.graph.scope().types(),
        &replay.packages,
        ModelNormalizationLimits::default(),
        &provision,
        &reference,
        ObservationLimits::default(),
    )
    .expect("the escaped pair admits");
    assert!(is_string(
        &replay.value(environment.objects(), "label"),
        "\u{1f600}"
    ));

    let fields = with(
        with(base_fields(), "note", json!({"present": string("n")})),
        "tags",
        json!({"sequence": [string("a"), string("")]}),
    );
    let closure = replay
        .admit(&fields, ObservationLimits::default())
        .expect("a present note and a sequence of Strings admit");
    assert!(is_string(&replay.value(&closure, "note"), "n"));
    let Datum::Collection(tags) = replay.value(&closure, "tags") else {
        panic!("tags is a sequence");
    };
    let [first, second] = tags.elements() else {
        panic!("two tags");
    };
    assert!(is_string(first, "a") && is_string(second, ""));
}

/// TC-1036 step 3 (FR-106-AC-28): `rename`'s `name` and `result` admit as
/// Strings, selected for a postcondition, through a precondition invocation
/// (no result) and through `PreCall`.
#[trace("TC-1036", "FR-106-AC-28")]
#[test]
fn an_invocation_admits_string_arguments_and_result() {
    let replay = replay();
    let (parameters, result) = default_call("rename");
    for selected in [Selected::Postcondition, Selected::Precondition] {
        let admitted = replay
            .invoke(
                "rename",
                selected,
                parameters.clone(),
                result.clone(),
                ObservationLimits::default(),
            )
            .unwrap_or_else(|failure| panic!("the invocation admits: {failure:?}"));
        assert!(is_string(&admitted.parameters[0].1, "n"));
        match selected {
            Selected::Postcondition => {
                assert!(admitted.result.as_ref().is_some_and(|r| is_string(r, "r")));
            }
            Selected::Precondition => assert!(admitted.result.is_none()),
        }
    }
    let admitted = replay
        .pre_call(
            "rename",
            vec![(
                "name",
                SnapshotValue::native(
                    RawNativeKind::Known(NativeKind::String),
                    vec![("text".to_owned(), Some("n".to_owned()))],
                ),
            )],
        )
        .unwrap_or_else(|failure| panic!("the PreCall admits: {failure:?}"));
    assert!(is_string(&admitted.parameters[0].1, "n"));
    assert!(admitted.result.is_none());
}

/// TC-1036 step 4 (FR-106-AC-29): the object defects of a String refuse in
/// the native order, naming `label` and the member; no `text` makes a String
/// refuse `invalid-value`.
#[trace("TC-1036", "FR-106-AC-29")]
#[test]
fn a_defective_string_object_refuses_in_the_native_order() {
    let replay = replay();
    let wrong = "wrong-value-kind";
    let invalid = "invalid-value";
    let missing = "missing-member";
    let unknown = "unknown-member";
    let rows: [(Value, &str, &str); 19] = [
        (json!({"kind": 1, "text": "x"}), wrong, "kind"),
        (json!({"kind": null, "text": "x"}), wrong, "kind"),
        (json!({"kind": {}, "text": "x"}), wrong, "kind"),
        (json!({"kind": 1}), wrong, "kind"),
        (json!({"kind": "String", "text": "x"}), invalid, "kind"),
        (json!({"kind": "STRING", "text": "x"}), invalid, "kind"),
        (json!({"kind": "Text", "text": "x"}), invalid, "kind"),
        (json!({"kind": "blob", "value": "x"}), invalid, "kind"),
        (json!({"kind": "blob", "text": "x"}), invalid, "kind"),
        (json!({"kind": "string"}), missing, "text"),
        (
            json!({"kind": "string", "nanoseconds": "0"}),
            missing,
            "text",
        ),
        (
            json!({"kind": "string", "text": "x", "extra": 1}),
            unknown,
            "extra",
        ),
        (
            json!({"kind": "string", "text": "x", "nanoseconds": "0"}),
            unknown,
            "nanoseconds",
        ),
        (json!({"kind": "string", "text": 5}), wrong, "text"),
        (json!({"kind": "string", "text": null}), wrong, "text"),
        (json!({"kind": "string", "text": true}), wrong, "text"),
        (json!({"kind": "string", "text": ["x"]}), wrong, "text"),
        (json!({"kind": "string", "text": {}}), wrong, "text"),
        (
            json!({"kind": "string", "extra": 1, "text": 5}),
            unknown,
            "extra",
        ),
    ];
    for (value, cause, member) in rows {
        assert_eq!(
            refused_field(&replay, "label", value.clone()),
            expect(cause, "label", &[], Some(member), None),
            "{value}"
        );
    }
}

/// TC-1036 step 5 (FR-106-AC-30): a String object in a slot of another type,
/// and any other form in the String slot, refuses `wrong-value-kind` at the
/// nested path and reads the text as no other kind; the envelope defects
/// before type fit name their member.
#[trace("TC-1036", "FR-106-AC-30")]
#[test]
fn a_string_object_in_another_slot_refuses_wrong_value_kind() {
    let replay = replay();
    let wrong =
        |field: &str, path: &[PathStep]| expect("wrong-value-kind", field, path, None, None);
    let any = string("x");
    for (field, value, path) in [
        ("token", any.clone(), vec![]),
        ("stamp", any.clone(), vec![]),
        ("versionNumber", any.clone(), vec![]),
        (
            "parent",
            json!({"present": any.clone()}),
            vec![PathStep::OptionValue],
        ),
        (
            "prior",
            json!({"present": any.clone()}),
            vec![PathStep::OptionValue],
        ),
        (
            "seen",
            json!({"sequence": [timestamp("0"), any.clone()]}),
            vec![PathStep::Index(1)],
        ),
        ("label", uuid(UUID_TEXT), vec![]),
        ("label", timestamp("0"), vec![]),
        ("label", integer("5"), vec![]),
        ("label", json!({"boolean": true}), vec![]),
        ("label", json!({"absent": {}}), vec![]),
        ("label", json!("ready"), vec![]),
        ("token", string(UUID_TEXT), vec![]),
    ] {
        assert_eq!(
            refused_field(&replay, field, value.clone()),
            wrong(field, &path),
            "{field}: {value}"
        );
    }
    assert_eq!(
        refused_field(&replay, "token", json!({"kind": "string", "text": 5})),
        expect("wrong-value-kind", "token", &[], Some("text"), None)
    );
    assert_eq!(
        refused_field(
            &replay,
            "versionNumber",
            json!({"kind": "string", "text": "x", "extra": 1})
        ),
        expect("unknown-member", "versionNumber", &[], Some("extra"), None)
    );
    let (parameters, result) = default_call("rename");
    let invoke = |parameters: Value, result: Value| {
        refusal_of(replay.invoke(
            "rename",
            Selected::Postcondition,
            parameters,
            result,
            ObservationLimits::default(),
        ))
    };
    assert_eq!(
        invoke(json!({"name": timestamp("0")}), result),
        wrong("name", &[])
    );
    assert_eq!(invoke(parameters, uuid(UUID_TEXT)), wrong("result", &[]));
}

/// TC-1036 step 6 (FR-106-AC-31): changing `label` is a change only a grant
/// covers, including U+00E9 to U+0065 U+0301 and `ready` to `Ready`; the
/// same text with its members reordered, or written `"\u0072eady"`, is no
/// change.
#[trace("TC-1036", "FR-106-AC-31")]
#[test]
fn the_frame_compares_a_string_by_its_scalar_sequence() {
    let replay = replay();
    let from = |label: &str| with(base_fields(), "label", string(label));
    // The pre snapshot is the base's `ready`: first the changes from it.
    let ready_to_big = from("Ready");
    assert_eq!(replay.frame("rename", &ready_to_big), FrameVerdict::Holds);
    let FrameVerdict::Violation(witness) = replay.frame("keep", &ready_to_big) else {
        panic!("a change without the grant violates the frame");
    };
    assert_eq!(witness.code, Code::FrameViolation);
    assert_eq!(witness.cause, "unauthorized-change");
    assert!(writes(&witness.change, "label"), "{:?}", witness.change);

    // U+00E9 to U+0065 U+0301: pre and post both differ from the base.
    let text = |label: &str, observation: &str| {
        snapshot_text(&replay, observation, observation, &from(label))
    };
    let FrameVerdict::Violation(witness) =
        replay.frame_over("keep", &text("\u{e9}", "pre"), &text("e\u{301}", "post"))
    else {
        panic!("U+00E9 to U+0065 U+0301 is a change");
    };
    assert!(writes(&witness.change, "label"), "{:?}", witness.change);
    assert_eq!(
        replay.frame_over("rename", &text("\u{e9}", "pre"), &text("e\u{301}", "post")),
        FrameVerdict::Holds
    );

    // The same text, members reordered or spelled `"\u0072eady"`: no change.
    let reordered = with(
        base_fields(),
        "label",
        json!({"text": "ready", "kind": "string"}),
    );
    assert_eq!(replay.frame("keep", &reordered), FrameVerdict::Holds);
    let escaped = text("ready", "post").replace("\"ready\"", "\"\\u0072eady\"");
    assert_eq!(
        replay.frame_over("keep", &text("ready", "pre"), &escaped),
        FrameVerdict::Holds
    );
}

/// TC-1036 step 7 (FR-106-AC-31): under `PostStateRange::Witness` a post
/// snapshot whose `label` is a UUID object, or a String object with `text`
/// `5`, still refuses `wrong-value-kind`, the second naming `text`.
#[trace("TC-1036", "FR-106-AC-31")]
#[test]
fn a_witness_post_snapshot_still_refuses_a_wrong_string() {
    let replay = replay();
    let witness = ObservationLimits {
        post_state: PostStateRange::Witness,
        ..ObservationLimits::default()
    };
    for (value, member) in [
        (uuid(UUID_TEXT), None),
        (json!({"kind": "string", "text": 5}), Some("text")),
    ] {
        let post = with(base_fields(), "label", value);
        assert_eq!(
            refusal_of(replay.invoke_with_post("rename", &post, witness)),
            expect("wrong-value-kind", "label", &[], member, None)
        );
    }
}

/// TC-1036 step 8 (FR-106-AC-31): a present `note`, last in walk order,
/// counts its option and its String object; the exact bound admits and one
/// less refuses at the String object inside `note`.
#[trace("TC-1036", "FR-106-AC-31")]
#[test]
fn a_string_object_counts_as_one_value() {
    let replay = replay();
    let fields = with(base_fields(), "note", json!({"present": string("n")}));
    // Ten fields, one value form each, and the String inside `note`.
    assert_eq!(replay.values_used(&fields), 11);
    replay
        .admit(
            &fields,
            ObservationLimits::default().with_values_per_document(11),
        )
        .expect("the exact count admits");
    assert_values_limit(
        replay.admit(
            &fields,
            ObservationLimits::default().with_values_per_document(10),
        ),
        10,
    );
}

/// TC-1038 / FR-106-AC-34, FR-106-AC-35 (the part this fixture reaches): a
/// Text object runs the envelope (missing member, unknown member, non-text
/// payload) and then refuses `wrong-value-kind` in every slot this fixture
/// declares, none of which is a Text slot.
#[trace("TC-1038", "FR-106-AC-34", "FR-106-AC-35")]
#[test]
fn a_text_object_is_a_known_kind_refused_outside_a_text_slot() {
    let replay = replay();
    let text = json!({"kind": "text", "text": "Ready"});
    for (field, value, path) in [
        ("label", text.clone(), vec![]),
        ("token", text.clone(), vec![]),
        ("stamp", text.clone(), vec![]),
        ("versionNumber", text.clone(), vec![]),
        ("parent", text.clone(), vec![]),
        (
            "parent",
            json!({"present": text.clone()}),
            vec![PathStep::OptionValue],
        ),
        (
            "note",
            json!({"present": text.clone()}),
            vec![PathStep::OptionValue],
        ),
        (
            "tags",
            json!({"sequence": [string("a"), text.clone()]}),
            vec![PathStep::Index(1)],
        ),
        ("token", json!({"kind": "text", "text": UUID_TEXT}), vec![]),
    ] {
        assert_eq!(
            refused_field(&replay, field, value.clone()),
            expect("wrong-value-kind", field, &path, None, None),
            "{field}: {value}"
        );
    }
    for (value, cause, member) in [
        (json!({"kind": "text"}), "missing-member", "text"),
        (
            json!({"kind": "text", "nanoseconds": "0"}),
            "missing-member",
            "text",
        ),
        (
            json!({"kind": "text", "text": "Ready", "extra": 1}),
            "unknown-member",
            "extra",
        ),
        (
            json!({"kind": "text", "text": "Ready", "nanoseconds": "0"}),
            "unknown-member",
            "nanoseconds",
        ),
        (
            json!({"kind": "text", "extra": 1, "text": 5}),
            "unknown-member",
            "extra",
        ),
        (
            json!({"kind": "text", "text": 5}),
            "wrong-value-kind",
            "text",
        ),
        (
            json!({"kind": "text", "text": null}),
            "wrong-value-kind",
            "text",
        ),
        (
            json!({"kind": "text", "text": ["x"]}),
            "wrong-value-kind",
            "text",
        ),
    ] {
        for field in ["label", "token", "versionNumber"] {
            assert_eq!(
                refused_field(&replay, field, value.clone()),
                expect(cause, field, &[], Some(member), None),
                "{field}: {value}"
            );
        }
    }
}

/// FR-106-AC-29, FR-106-AC-20: an envelope defect directly in an optional
/// slot (not under `present`) is judged before the slot's fit, and a sound
/// native object there is `wrong-value-kind`.
#[trace("FR-106-AC-29")]
#[test]
fn a_native_object_directly_in_an_optional_slot_is_judged_in_order() {
    let replay = replay();
    assert_eq!(
        refused_field(&replay, "note", json!({"kind": "string"})),
        expect("missing-member", "note", &[], Some("text"), None)
    );
    assert_eq!(
        refused_field(&replay, "note", json!({"kind": "string", "text": 5})),
        expect("wrong-value-kind", "note", &[], Some("text"), None)
    );
    assert_eq!(
        refused_field(&replay, "note", string("n")),
        expect("wrong-value-kind", "note", &[], None, None)
    );
}

// ---------------------------------------------------------------------------
// TC-1038: Text value types, the rows QSL-798 left open
// ---------------------------------------------------------------------------

/// Whether `value` is the Text whose retained sequence is exactly `retained`.
fn is_text(value: &Datum, retained: &str) -> bool {
    matches!(value, Datum::Text(held) if held.retained() == retained)
}

/// TC-1044 step 1 (FR-056-AC-37): a field typed by a Text value type has the
/// value type `Text[min, max; profile]` the value type declares.
#[trace("TC-1044", "FR-056-AC-37")]
#[test]
fn a_field_typed_by_a_text_value_type_is_that_text() {
    use quire_exact::{TextProfile, TextType, ValueType};
    let replay = text_replay();
    let attributes = replay
        .graph
        .scope()
        .types()
        .attributes(replay.item())
        .expect("Item has attributes");
    let declared = |name: &str| {
        attributes
            .iter()
            .find(|attribute| attribute.field().name() == name)
            .map(|attribute| attribute.field().value_type().clone())
            .unwrap_or_else(|| panic!("{name} is declared"))
    };
    let text = |min, max, profile| {
        ValueType::Text(TextType::new(min, max, profile).expect("nonempty bounds"))
    };
    assert_eq!(declared("title"), text(1, 8, TextProfile::Nfc));
    assert_eq!(declared("code"), text(0, 4, TextProfile::BinaryUtf8));
    assert_eq!(
        declared("subtitle"),
        text(0, 4, TextProfile::UnicodeScalars)
    );
    assert_eq!(declared("label"), ValueType::String);
}

/// TC-1038 step 1 (FR-106-AC-33): the base snapshot admits `title` as the Text
/// of the retained sequence `Ready`; reordering the members of the object and
/// the whitespace of the document gives the same value and digest.
#[trace("TC-1038", "FR-106-AC-33")]
#[test]
fn the_base_snapshot_admits_a_text() {
    let replay = text_replay();
    let closure = replay
        .admit(&replay.base, ObservationLimits::default())
        .unwrap_or_else(|failure| panic!("the snapshot admits: {failure:?}"));
    assert!(is_text(&replay.value(&closure, "title"), "Ready"));
    assert!(is_text(&replay.value(&closure, "code"), "abc"));
    let reordered = with(
        replay.base.clone(),
        "title",
        json!({"text": "Ready", "kind": "text"}),
    );
    let spaced = snapshot_text(&replay, "current", "current", &reordered)
        .replace("\",\"", "\" , \"")
        .replace(":{", ": {");
    let (plain, _) = snapshot(&replay, "current", "current", &replay.base);
    let provision = BTreeMap::from([(plain.clone(), spaced.into_bytes())]);
    let (environment, _) = admit_current_snapshot(
        replay.graph.model_selections(),
        replay.graph.scope().types(),
        &replay.packages,
        ModelNormalizationLimits::default(),
        &provision,
        &plain,
        ObservationLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the respelled document admits: {failure:?}"));
    assert!(is_text(
        &replay.value(environment.objects(), "title"),
        "Ready"
    ));
}

/// TC-1038 step 3 (FR-106-AC-33): an invocation of `retitle` admits `name` and
/// `result` as Text values in both selections, the `PreCall` selection
/// reading no result.
#[trace("TC-1038", "FR-106-AC-33")]
#[test]
fn an_invocation_admits_text_arguments_and_result() {
    let replay = text_replay();
    let (parameters, result) = default_call("retitle");
    for selected in [Selected::Postcondition, Selected::Precondition] {
        let admitted = replay
            .invoke(
                "retitle",
                selected,
                parameters.clone(),
                result.clone(),
                ObservationLimits::default(),
            )
            .unwrap_or_else(|failure| panic!("the invocation admits: {failure:?}"));
        assert!(is_text(&admitted.parameters[0].1, "Ready"));
        match selected {
            Selected::Postcondition => {
                assert!(admitted
                    .result
                    .as_ref()
                    .is_some_and(|r| is_text(r, "Steady")));
            }
            Selected::Precondition => assert!(admitted.result.is_none()),
        }
    }
    let admitted = replay
        .pre_call(
            "retitle",
            vec![(
                "name",
                SnapshotValue::native(
                    RawNativeKind::Known(NativeKind::Text),
                    vec![("text".to_owned(), Some("Ready".to_owned()))],
                ),
            )],
        )
        .unwrap_or_else(|failure| panic!("the PreCall admits: {failure:?}"));
    assert!(is_text(&admitted.parameters[0].1, "Ready"));
    assert!(admitted.result.is_none());
}

/// TC-1038 step 7 (FR-106-AC-37): a changed `title` admits its frame with the
/// grant and violates it without; a post that is the same text under `nfc`
/// (U+00E9 against U+0065 U+0301, reordered members, an escaped `R`) is no
/// change; a `subtitle` change under `unicode-scalars` is a change.
#[trace("TC-1038", "FR-106-AC-37")]
#[test]
fn the_frame_compares_a_text_by_its_retained_sequence() {
    let replay = text_replay();
    let steady = with(replay.base.clone(), "title", text("Steady"));
    assert_eq!(replay.frame("retitle", &steady), FrameVerdict::Holds);
    let FrameVerdict::Violation(witness) = replay.frame("rekeep", &steady) else {
        panic!("a change without the grant violates the frame");
    };
    assert_eq!(witness.code, Code::FrameViolation);
    assert_eq!(witness.cause, "unauthorized-change");
    assert!(writes(&witness.change, "title"), "{:?}", witness.change);

    let composed = with(replay.base.clone(), "title", text("\u{e9}"));
    let decomposed = with(replay.base.clone(), "title", text("e\u{301}"));
    assert_eq!(
        replay.frame_from("rekeep", &composed, &decomposed),
        FrameVerdict::Holds,
        "U+00E9 and U+0065 U+0301 are one value under nfc"
    );
    let reordered = with(
        replay.base.clone(),
        "title",
        json!({"text": "Ready", "kind": "text"}),
    );
    assert_eq!(
        replay.frame_from("rekeep", &replay.base, &reordered),
        FrameVerdict::Holds
    );
    let pre = snapshot_text(&replay, "pre", "pre", &replay.base);
    let escaped = snapshot_text(&replay, "post", "post", &replay.base)
        .replace("\"Ready\"", "\"\\u0052eady\"");
    assert_eq!(
        replay.frame_over("rekeep", &pre, &escaped),
        FrameVerdict::Holds
    );

    let present = |text_value: &str| json!({"present": text(text_value)});
    let before = with(replay.base.clone(), "subtitle", present("\u{e9}"));
    let after = with(replay.base.clone(), "subtitle", present("e\u{301}"));
    let FrameVerdict::Violation(witness) = replay.frame_from("rekeep", &before, &after) else {
        panic!("a unicode-scalars change violates the frame");
    };
    assert_eq!(witness.cause, "unauthorized-change");
    assert!(writes(&witness.change, "subtitle"), "{:?}", witness.change);
}

/// TC-1044 step 4 (FR-106-AC-49): a record `Address { street: Text[1, 8;
/// nfc] }` held by `Item.address` compares its `street` by the retained
/// sequence, so U+00E9 against U+0065 U+0301 is no change under a frame
/// granting nothing, and a different street is a change.
#[trace("TC-1044", "FR-106-AC-49")]
#[test]
fn the_frame_compares_a_record_fields_text_by_its_retained_sequence() {
    let replay = text_replay();
    let composed = with(replay.base.clone(), "address", address("\u{e9}"));
    let decomposed = with(replay.base.clone(), "address", address("e\u{301}"));
    assert_eq!(
        replay.frame_from("rekeep", &composed, &decomposed),
        FrameVerdict::Holds,
        "U+00E9 and U+0065 U+0301 are one street under nfc"
    );
    let other = with(replay.base.clone(), "address", address("Side"));
    let FrameVerdict::Violation(witness) = replay.frame_from("rekeep", &composed, &other) else {
        panic!("a different street violates the frame");
    };
    assert_eq!(witness.cause, "unauthorized-change");
    assert!(writes(&witness.change, "address"), "{:?}", witness.change);
}

/// TC-1038 step 8 (FR-106-AC-36): under `PostStateRange::Witness` a post
/// snapshot holding a String object in `title` refuses `wrong-value-kind`, an
/// empty `title` refuses `invalid-value` naming `text`, and a `text` of `5`
/// refuses `wrong-value-kind` naming `text`.
#[trace("TC-1038", "FR-106-AC-36")]
#[test]
fn a_witness_post_snapshot_still_refuses_a_wrong_text() {
    let replay = text_replay();
    let witness = ObservationLimits {
        post_state: PostStateRange::Witness,
        ..ObservationLimits::default()
    };
    for (value, cause, member) in [
        (string("Ready"), "wrong-value-kind", None),
        (text(""), "invalid-value", Some("text")),
        (
            json!({"kind": "text", "text": 5}),
            "wrong-value-kind",
            Some("text"),
        ),
    ] {
        let post = with(replay.base.clone(), "title", value.clone());
        assert_eq!(
            refusal_of(replay.invoke_with_post("retitle", &post, witness)),
            expect(cause, "title", &[], member, None),
            "{value}"
        );
    }
}

/// TC-1038 step 9 (FR-106-AC-37): a Text element inside `aliases`, the last
/// value in walk order, counts as one value; the exact count admits and one
/// less refuses `node-count-exceeded` naming `observation.values`.
#[trace("TC-1038", "FR-106-AC-37")]
#[test]
fn a_text_object_counts_as_one_value() {
    let replay = text_replay();
    let fields = with(
        replay.base.clone(),
        "aliases",
        json!({"sequence": [text("x")]}),
    );
    assert_eq!(replay.values_used(&fields), 17);
    replay
        .admit(
            &fields,
            ObservationLimits::default().with_values_per_document(17),
        )
        .expect("the exact count admits");
    assert_values_limit(
        replay.admit(
            &fields,
            ObservationLimits::default().with_values_per_document(16),
        ),
        16,
    );
}

/// TC-1044 step 4 (FR-106-AC-40): the frame check decides each declared type
/// by its own equality, over an invocation granting nothing. A change
/// refuses `frame_violation`/`unauthorized-change` naming the object and the
/// field; an equal value admits.
#[trace("TC-1044", "FR-106-AC-40")]
#[test]
fn the_frame_decides_each_declared_type_by_its_own_equality() {
    let replay = text_replay();
    let pre = with(
        replay.base.clone(),
        "aliases",
        json!({"sequence": [text("a"), text("b")]}),
    );
    let present = |value: &str| json!({"present": text(value)});
    let changed = |pre: &[(String, Value)], field: &str, value: Value| {
        replay.frame_from("rekeep", pre, &with(pre.to_vec(), field, value))
    };
    let assert_violation = |verdict: FrameVerdict, field: &str| {
        let FrameVerdict::Violation(witness) = verdict else {
            panic!("{field}: a change outside the frame violates it");
        };
        assert_eq!(witness.code, Code::FrameViolation, "{field}");
        assert_eq!(witness.cause, "unauthorized-change", "{field}");
        assert!(
            writes(&witness.change, field),
            "{field}: {:?}",
            witness.change
        );
    };

    // A String compares as written.
    let label = with(pre.clone(), "label", string("\u{e9}"));
    assert_violation(changed(&label, "label", string("e\u{301}")), "label");
    // A Text compares by its retained sequence.
    assert_eq!(
        changed(
            &with(pre.clone(), "title", text("\u{e9}")),
            "title",
            text("e\u{301}")
        ),
        FrameVerdict::Holds
    );
    assert_violation(changed(&pre, "title", text("Steady")), "title");
    // Inside a present option, under unicode-scalars.
    let subtitle = with(pre.clone(), "subtitle", present("\u{e9}"));
    assert_violation(
        changed(&subtitle, "subtitle", present("e\u{301}")),
        "subtitle",
    );
    // Absent to present is a change; the same present value with its members
    // written in another order is none.
    assert_violation(changed(&pre, "subtitle", present("a")), "subtitle");
    let subtitle = with(pre.clone(), "subtitle", present("a"));
    assert_eq!(
        changed(
            &subtitle,
            "subtitle",
            json!({"present": {"text": "a", "kind": "text"}})
        ),
        FrameVerdict::Holds
    );
    // A sequence reordered is a change; the same elements with only the
    // document's whitespace changed are none.
    assert_violation(
        changed(&pre, "aliases", json!({"sequence": [text("b"), text("a")]})),
        "aliases",
    );
    let pre_text = snapshot_text(&replay, "pre", "pre", &pre);
    let spaced = snapshot_text(&replay, "post", "post", &pre).replace("\",\"", "\" , \"");
    assert_eq!(
        replay.frame_over("rekeep", &pre_text, &spaced),
        FrameVerdict::Holds
    );
    // A UUID changed to another canonical payload is a change.
    assert_violation(changed(&pre, "token", uuid(OTHER_UUID)), "token");
}

// ---------------------------------------------------------------------------
// TC-1171: set, bag and ordered-set values
// ---------------------------------------------------------------------------

/// [`base_fields`] and then the five collection fields of TC-1171, each empty:
/// `ids: Set<UUID>[0, 3]`, `counts: Bag<Digit>[0, 4]`, `order:
/// OrderedSet<Digit>[0, 3]`, `stamps: OrderedSet<Timestamp>[0, 2]` and
/// `peers: Set<Reference<Item>>[0, 2]`.
fn collection_fields() -> Vec<(String, Value)> {
    let mut fields = base_fields();
    fields.extend(
        [
            ("ids", json!({"set": []})),
            ("counts", json!({"bag": []})),
            ("order", json!({"ordered-set": []})),
            ("stamps", json!({"ordered-set": []})),
            ("peers", json!({"set": []})),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value)),
    );
    fields
}

/// The parameters of a well-formed `retag` call.
fn retag_parameters() -> Value {
    json!({"ids": {"set": []}, "counts": {"bag": []}, "order": {"ordered-set": []}})
}

/// `{"integer": ..}` for each spelling.
fn integers(spellings: &[&str]) -> Vec<Value> {
    spellings.iter().map(|spelling| integer(spelling)).collect()
}

fn uuid_ending(last: u8) -> Value {
    uuid(&format!("00000000-0000-0000-0000-0000000000{last:02x}"))
}

fn peer(population: &str, key: &str) -> Value {
    json!({"reference": {"population": id(population), "key": key}})
}

/// The elements of an admitted collection, in the order it holds them.
fn elements_of(value: &Datum) -> Vec<Datum> {
    let Datum::Collection(collection) = value else {
        panic!("a collection, not {value:?}");
    };
    collection.elements().to_vec()
}

fn integer_text(value: &Datum) -> String {
    let Datum::Integer(held) = value else {
        panic!("an integer, not {value:?}");
    };
    held.to_string()
}

fn timestamp_of(value: &Datum) -> i128 {
    let Datum::Timestamp(held) = value else {
        panic!("a Timestamp, not {value:?}");
    };
    held.nanoseconds()
}

fn uuid_last_octet(value: &Datum) -> u8 {
    let Datum::Uuid(held) = value else {
        panic!("a UUID, not {value:?}");
    };
    held.octets()[15]
}

fn reference_key(value: &Datum) -> String {
    let Datum::Reference(held) = value else {
        panic!("a reference, not {value:?}");
    };
    held.object().as_str().to_owned()
}

/// The admission record of a refused outcome.
fn record_of<T: Debug>(
    outcome: Result<T, AdmissionFailure>,
) -> qsl_semantics::model::observation::AdmissionRecord {
    match outcome {
        Err(AdmissionFailure::Refused(record)) => record,
        other => panic!("a refusal, not {other:?}"),
    }
}

/// TC-1171 step 1 (FR-106-AC-50): the snapshot admits `ids` as two members,
/// `counts` as the member `1` once and `2` twice, `order` in the order
/// `3, 1`, `stamps` in the order `5, -1` and `peers` written `a1` of `others`
/// before `z1` of `items`; the same document with the `ids`, `counts` and
/// `peers` elements in another order gives an equal typed observation and a
/// different document digest; with `order` reversed it gives an unequal one.
#[trace("TC-1171", "FR-106-AC-50")]
#[test]
fn set_bag_and_ordered_set_fields_admit_with_their_element_order() {
    let replay = collection_replay();
    let fields = |ids: [u8; 2], counts: [&str; 3], order: [&str; 2], peers: [(&str, &str); 2]| {
        let mut fields = collection_fields();
        for (name, value) in [
            ("ids", json!({"set": ids.map(uuid_ending)})),
            ("counts", json!({"bag": integers(&counts)})),
            ("order", json!({"ordered-set": integers(&order)})),
            (
                "stamps",
                json!({"ordered-set": [timestamp("5"), timestamp("-1")]}),
            ),
            (
                "peers",
                json!({"set": peers.map(|(population, key)| peer(population, key))}),
            ),
        ] {
            fields = with(fields, name, value);
        }
        fields
    };
    let written = fields(
        [2, 1],
        ["2", "1", "2"],
        ["3", "1"],
        [("items", "z1"), ("others", "a1")],
    );
    let closure = replay
        .admit(&written, ObservationLimits::default())
        .unwrap_or_else(|failure| panic!("the snapshot admits: {failure:?}"));
    let members = |name: &str| elements_of(&replay.value(&closure, name));
    assert_eq!(members("ids").len(), 2);
    let mut ids: Vec<u8> = members("ids").iter().map(uuid_last_octet).collect();
    ids.sort_unstable();
    assert_eq!(ids, [1, 2]);
    let counts: Vec<String> = members("counts").iter().map(integer_text).collect();
    assert_eq!(counts, ["1", "2", "2"], "member 1 once and member 2 twice");
    let order: Vec<String> = members("order").iter().map(integer_text).collect();
    assert_eq!(order, ["3", "1"]);
    let stamps: Vec<i128> = members("stamps").iter().map(timestamp_of).collect();
    assert_eq!(stamps, [5, -1]);
    let peers: Vec<String> = members("peers").iter().map(reference_key).collect();
    assert_eq!(peers, ["a1", "z1"], "ordered by key");

    let reversed = fields(
        [1, 2],
        ["1", "2", "2"],
        ["3", "1"],
        [("others", "a1"), ("items", "z1")],
    );
    let again = replay
        .admit(&reversed, ObservationLimits::default())
        .expect("the reordered document admits");
    for name in ["ids", "counts", "peers"] {
        assert!(
            same(&replay.value(&again, name), &replay.value(&closure, name)),
            "{name} is one value in either array order"
        );
    }

    let turned = with(
        written.clone(),
        "order",
        json!({"ordered-set": integers(&["1", "3"])}),
    );
    let other = replay
        .admit(&turned, ObservationLimits::default())
        .expect("the reversed ordered set admits");
    assert!(
        !same(
            &replay.value(&other, "order"),
            &replay.value(&closure, "order")
        ),
        "an ordered set keeps its order"
    );
}

/// One defect and the refusal it gives, as a field of the snapshot and as a
/// `retag` parameter: the value, the cause, the nested path and the other
/// members the record names.
struct Defect {
    name: &'static str,
    value: Value,
    code: Code,
    cause: &'static str,
    path: Vec<PathStep>,
    members: Vec<(&'static str, &'static str)>,
}

fn defects() -> Vec<Defect> {
    let invalid = |name, value, path| Defect {
        name,
        value,
        code: Code::InvalidRuntimeInput,
        cause: "invalid-value",
        path,
        members: vec![],
    };
    vec![
        Defect {
            name: "order",
            value: json!({"ordered-set": integers(&["1", "1"])}),
            code: Code::InvalidRuntimeInput,
            cause: "duplicate-member",
            path: vec![PathStep::Index(1)],
            members: vec![("earlier-index", "0")],
        },
        Defect {
            name: "ids",
            value: json!({"set": [uuid_ending(7), uuid_ending(7)]}),
            code: Code::InvalidRuntimeInput,
            cause: "duplicate-member",
            path: vec![PathStep::Index(1)],
            members: vec![("earlier-index", "0")],
        },
        // The bound comes before any element is read: none of the five
        // elements is an integer.
        Defect {
            name: "counts",
            value: json!({"bag": integers(&["x", "x", "x", "x", "x"])}),
            code: Code::CardinalityOutOfBound,
            cause: "above-maximum",
            path: vec![],
            members: vec![("minimum", "0"), ("maximum", "4"), ("count", "5")],
        },
        Defect {
            name: "ids",
            value: json!({"set": [integer("1")]}),
            code: Code::InvalidRuntimeInput,
            cause: "wrong-value-kind",
            path: vec![PathStep::Index(0)],
            members: vec![],
        },
        Defect {
            name: "ids",
            value: json!({"sequence": [uuid_ending(1)]}),
            code: Code::InvalidRuntimeInput,
            cause: "wrong-value-kind",
            path: vec![],
            members: vec![],
        },
        invalid(
            "ids",
            json!({"set": [uuid("00000000-0000-0000-0000-00000000000A")]}),
            vec![PathStep::Index(0)],
        ),
        // Every element is read before a duplicate is decided.
        invalid(
            "counts",
            json!({"bag": [integer("x"), integer("1"), integer("1")]}),
            vec![PathStep::Index(0)],
        ),
    ]
}

/// TC-1171 step 2 (FR-106-AC-51): the refusals of a defective collection, as
/// a field of the snapshot and as a parameter of an invocation, each with its
/// code, cause and path.
#[trace("TC-1171", "FR-106-AC-51")]
#[test]
fn defective_collections_refuse_in_a_field_and_in_a_parameter() {
    let replay = collection_replay();
    let check = |record: &qsl_semantics::model::observation::AdmissionRecord,
                 defect: &Defect,
                 place: &str| {
        assert_eq!(record.code, defect.code, "{place} {}", defect.value);
        assert_eq!(record.cause, defect.cause, "{place} {}", defect.value);
        assert_eq!(record.path, defect.path, "{place} {}", defect.value);
        assert_eq!(
            record.fields.get("field").map(String::as_str),
            Some(defect.name),
            "{place} {}",
            defect.value
        );
        for (member, expected) in &defect.members {
            assert_eq!(
                record.fields.get(*member).map(String::as_str),
                Some(*expected),
                "{place} {member} {}",
                defect.value
            );
        }
    };
    for defect in defects() {
        let field = record_of(replay.admit(
            &with(collection_fields(), defect.name, defect.value.clone()),
            ObservationLimits::default(),
        ));
        check(&field, &defect, "field");
        let mut parameters = retag_parameters();
        parameters[defect.name] = defect.value.clone();
        let parameter = record_of(replay.invoke(
            "retag",
            Selected::Postcondition,
            parameters,
            json!({"set": []}),
            ObservationLimits::default(),
        ));
        check(&parameter, &defect, "parameter");
    }
    // A Timestamp elements's spelling is judged like any Timestamp.
    let stamps = record_of(replay.admit(
        &with(
            collection_fields(),
            "stamps",
            json!({"ordered-set": [timestamp("01")]}),
        ),
        ObservationLimits::default(),
    ));
    assert_eq!(
        (stamps.code, stamps.cause, stamps.path),
        (
            Code::InvalidRuntimeInput,
            "invalid-value",
            vec![PathStep::Index(0)]
        )
    );
}

/// TC-1171 step 3 (FR-106-AC-52): an invocation whose post `ids` or `counts`
/// holds the pre members in another order is no change; a post `counts` that
/// drops a `2`, or a post `order` of `1, 3` against `3, 1`, is a change that
/// refuses `frame_violation`/`unauthorized-change` naming the object and the
/// field; under `Witness` a post `counts` of `1, 10, 2` admits and reports
/// one `OutOfRange` at the supplied index 1; `observation.values` counts the
/// last element of `order` exactly.
#[trace("TC-1171", "FR-106-AC-52")]
#[test]
fn the_frame_and_the_limits_see_collections_by_value() {
    let replay = collection_replay();
    let pre = {
        let mut fields = collection_fields();
        for (name, value) in [
            ("ids", json!({"set": [uuid_ending(1), uuid_ending(2)]})),
            ("counts", json!({"bag": integers(&["1", "2", "2"])})),
            ("order", json!({"ordered-set": integers(&["3", "1"])})),
        ] {
            fields = with(fields, name, value);
        }
        fields
    };
    let frame = |post: Vec<(String, Value)>| replay.frame_from("retag", &pre, &post);
    let reordered = with(
        with(
            pre.clone(),
            "ids",
            json!({"set": [uuid_ending(2), uuid_ending(1)]}),
        ),
        "counts",
        json!({"bag": integers(&["2", "1", "2"])}),
    );
    assert_eq!(frame(reordered), FrameVerdict::Holds);
    for (field, value) in [
        ("counts", json!({"bag": integers(&["1", "2"])})),
        ("order", json!({"ordered-set": integers(&["1", "3"])})),
    ] {
        let FrameVerdict::Violation(witness) = frame(with(pre.clone(), field, value)) else {
            panic!("{field}: a change without a grant violates the frame");
        };
        assert_eq!(witness.code, Code::FrameViolation);
        assert_eq!(witness.cause, "unauthorized-change");
        assert!(
            writes(&witness.change, field),
            "{field}: {:?}",
            witness.change
        );
    }

    // Under `Witness` a post element above the declared range admits exactly
    // and is reported at its supplied array index.
    let witness = ObservationLimits {
        post_state: PostStateRange::Witness,
        ..ObservationLimits::default()
    };
    let post = with(
        collection_fields(),
        "counts",
        json!({"bag": integers(&["1", "10", "2"])}),
    );
    let admitted = replay
        .invoke_with_post("restock", &post, witness)
        .unwrap_or_else(|failure| panic!("the post snapshot admits: {failure:?}"));
    let post = admitted
        .post
        .expect("a postcondition admits a post snapshot");
    let [violation] = &post.out_of_range[..] else {
        panic!("one range violation, not {:?}", post.out_of_range);
    };
    assert_eq!(violation.field.name, "counts");
    assert_eq!(violation.index, Some(1));
    assert_eq!(violation.observed.to_string(), "10");
    assert_eq!(violation.range.upper().to_string(), "9");
    assert!(admitted
        .pre
        .expect("a postcondition admits a pre snapshot")
        .out_of_range
        .is_empty());

    // The last value in walk order is the last element of `order`.
    let walked: Vec<(String, Value)> = pre
        .iter()
        .filter(|(name, _)| name != "order")
        .cloned()
        .chain(pre.iter().filter(|(name, _)| name == "order").cloned())
        .collect();
    let used = replay.values_used(&walked);
    // Every field and every element counts one value: `child` holds its ten
    // base fields, `ids` and its two members, `counts` and its three, `order`
    // and its two, and the empty `stamps` and `peers` (22); `z1` and `a1`
    // hold the ten base fields and five empty collections each (15 + 15).
    assert_eq!(used, 22 + 15 + 15);
    replay
        .admit(
            &walked,
            ObservationLimits::default().with_values_per_document(used),
        )
        .expect("the exact count admits");
    assert_values_limit(
        replay.admit(
            &walked,
            ObservationLimits::default().with_values_per_document(used - 1),
        ),
        used - 1,
    );
}

// ---------------------------------------------------------------------------
// TC-1171: Quantity (FR-106-AC-59)
// ---------------------------------------------------------------------------

/// The field values of the Quantity fixture: the base fields, `reach` one
/// millisecond.
fn quantity_fields() -> Vec<(String, Value)> {
    let mut fields = base_fields();
    fields.push(("reach".to_owned(), ms(ratio("1", "1"))));
    fields
}

/// A Quantity object of `ms` with `magnitude`.
fn ms(magnitude: Value) -> Value {
    json!({"kind": "quantity", "unit": "ms", "magnitude": magnitude})
}

/// A Rational magnitude `numerator` / `denominator`.
fn ratio(numerator: &str, denominator: &str) -> Value {
    json!({"kind": "rational", "numerator": numerator, "denominator": denominator})
}

/// The Decimal magnitude `5` scaled by `1`, that is `1`/`2`.
fn half_as_decimal() -> Value {
    json!({"kind": "decimal", "coefficient": "5", "scale": "1"})
}

/// The refusal of the Quantity fixture where `field` holds `value`.
fn refused_quantity(
    replay: &Replay,
    field: &str,
    value: Value,
) -> qsl_semantics::model::observation::AdmissionRecord {
    record_of(replay.admit(
        &with(quantity_fields(), field, value),
        ObservationLimits::default(),
    ))
}

/// The exact value of an admitted Quantity: its magnitude as `numerator/denominator`.
fn magnitude_of(value: &Datum) -> String {
    let Datum::Quantity(quantity) = value else {
        panic!("a Quantity, not {value:?}");
    };
    quantity.magnitude().to_string()
}

/// Part of FR-106-AC-59 (TC-1171): a Quantity whose magnitude is the Rational
/// `3`/`6` admits as `1`/`2` of its unit, and the Decimal magnitude `5`
/// scaled by `1` is the same value.
#[trace("TC-1171")]
#[test]
fn a_quantity_admits_a_rational_or_a_decimal_magnitude_as_one_value() {
    let replay = quantity_replay();
    let rational = replay
        .admit(
            &with(quantity_fields(), "reach", ms(ratio("3", "6"))),
            ObservationLimits::default(),
        )
        .unwrap_or_else(|failure| panic!("the Rational magnitude admits: {failure:?}"));
    let decimal = replay
        .admit(
            &with(quantity_fields(), "reach", ms(half_as_decimal())),
            ObservationLimits::default(),
        )
        .unwrap_or_else(|failure| panic!("the Decimal magnitude admits: {failure:?}"));
    assert_eq!(magnitude_of(&replay.value(&rational, "reach")), "1/2");
    assert!(same(
        &replay.value(&rational, "reach"),
        &replay.value(&decimal, "reach")
    ));
}

/// Part of FR-106-AC-59 (TC-1171): a post `reach` that is the other spelling
/// of the pre magnitude is no change and holds without a grant, while a post
/// magnitude of `2`/`3` violates the frame with `unauthorized-change` naming
/// `reach`.
#[trace("TC-1171")]
#[test]
fn the_frame_compares_a_quantity_by_its_exact_magnitude() {
    let replay = quantity_replay();
    let pre = with(quantity_fields(), "reach", ms(ratio("3", "6")));
    let same_value = with(quantity_fields(), "reach", ms(half_as_decimal()));
    assert_eq!(
        replay.frame_from("hold", &pre, &same_value),
        FrameVerdict::Holds
    );
    let other = with(quantity_fields(), "reach", ms(ratio("2", "3")));
    let FrameVerdict::Violation(witness) = replay.frame_from("hold", &pre, &other) else {
        panic!("a different magnitude violates the frame");
    };
    assert_eq!(witness.code, Code::FrameViolation);
    assert_eq!(witness.cause, "unauthorized-change");
    assert!(writes(&witness.change, "reach"), "{:?}", witness.change);
}

/// Part of FR-106-AC-59 (TC-1171): a defective Quantity object refuses
/// `invalid_runtime_input` with its cause, naming the defective member:
/// a missing `unit` or `magnitude` `missing-member`, an added `bits`
/// `unknown-member`, a `magnitude` that is a String object, an integer or a
/// JSON string, and a `unit` that is a JSON number or another unit,
/// `wrong-value-kind`.
#[trace("TC-1171")]
#[test]
fn a_defective_quantity_object_refuses_naming_its_member() {
    let replay = quantity_replay();
    let half = ratio("1", "2");
    for (object, cause, member) in [
        (
            json!({"kind": "quantity", "magnitude": half}),
            "missing-member",
            "unit",
        ),
        (
            json!({"kind": "quantity", "unit": "ms"}),
            "missing-member",
            "magnitude",
        ),
        (
            json!({"kind": "quantity", "unit": "ms", "magnitude": half, "bits": "1"}),
            "unknown-member",
            "bits",
        ),
        (ms(string("1")), "wrong-value-kind", "magnitude"),
        (ms(integer("1")), "wrong-value-kind", "magnitude"),
        (ms(json!("1")), "wrong-value-kind", "magnitude"),
        (
            json!({"kind": "quantity", "unit": 1, "magnitude": half}),
            "wrong-value-kind",
            "unit",
        ),
        (
            json!({"kind": "quantity", "unit": "s", "magnitude": half}),
            "wrong-value-kind",
            "unit",
        ),
    ] {
        let record = refused_quantity(&replay, "reach", object.clone());
        assert_eq!(record.code, Code::InvalidRuntimeInput, "{object}");
        assert_eq!(record.cause, cause, "{object}");
        assert_eq!(
            record.fields.get("member").map(String::as_str),
            Some(member),
            "{object}"
        );
        assert_eq!(
            record.fields.get("field").map(String::as_str),
            Some("reach"),
            "{object}"
        );
    }
}

/// Part of FR-106-AC-59 (TC-1171): a defective magnitude refuses
/// `invalid-value` naming the member inside it at the path `magnitude`, a
/// Quantity object in a Rational slot is covered with the Rational slots, and
/// a Decimal magnitude far outside any bound admits.
#[trace("TC-1171")]
#[test]
fn a_defective_magnitude_refuses_at_the_path_magnitude() {
    let replay = quantity_replay();
    for (magnitude, member) in [
        (ratio("1", "0"), "denominator"),
        (
            json!({"kind": "decimal", "coefficient": "5", "scale": "-1"}),
            "scale",
        ),
    ] {
        let record = refused_quantity(&replay, "reach", ms(magnitude.clone()));
        assert_eq!(record.cause, "invalid-value", "{magnitude}");
        assert_eq!(
            record.fields.get("member").map(String::as_str),
            Some(member),
            "{magnitude}"
        );
        assert_eq!(
            record.path,
            [PathStep::Field("magnitude".to_owned())],
            "{magnitude}"
        );
    }
    let huge =
        json!({"kind": "decimal", "coefficient": "123456789012345678901234567890", "scale": "0"});
    replay
        .admit(
            &with(quantity_fields(), "reach", ms(huge)),
            ObservationLimits::default(),
        )
        .expect("a Quantity magnitude has no declared bound");
}
