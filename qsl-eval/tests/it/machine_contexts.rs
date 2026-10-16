// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1045: a snapshot carries the state-machine context an invocation's
//! invariant, precondition and postcondition are evaluated against (FR-106
//! `contexts`, the `Context` selection, checks 5, 6.10 to 6.12, 7, 8, 9 and
//! 11.6). Every expected result is the spec table's.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use qsl_foundation::diagnostic::Code;
use qsl_package::CheckedPackage;
use qsl_semantics::model::observation::{
    admit_current_snapshot, AdmissionFailure, AdmissionRecord, AdmittedObservations, AnchorKind,
    ClauseSelectionInput, ContextSide, DocumentRef, ObservationLimits, PostStateRange,
    SelectedAnchor, SelectedObject, SelfKind, SnapshotValue,
};

use crate::tc235_business_meaning_runtime::{
    lifecycle_document_ref, lifecycle_document_with, lifecycle_identity, lifecycle_selection,
    lifecycle_selection_with, model_package, truth,
};
use crate::tc235_k2::{declaration, field, id, operation, origin, type_named_mut};

const NATIVE: &str = "ix://quire/native";

/// The `Orders` fixture of TC-1040 with TC-1045's additions: `ExpressLifecycle`
/// (`retries: Int[0, 5]`, `expedite()`), `Billing`, `Owned` (`claim()`), the
/// object type `Pump` in the populations `Q` and `Spare`, and the clauses.
struct Orders {
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    model: Json,
}

/// The documents offered to one admission.
#[derive(Clone, Default)]
struct Docs {
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Docs {
    fn snapshot(&mut self, document: &Json) -> DocumentRef {
        let label = document["identity"]["identity"]
            .as_str()
            .unwrap()
            .to_owned();
        let bytes = serde_json::to_vec(document).unwrap();
        let reference = lifecycle_document_ref(&label, &bytes);
        self.snapshots.insert(reference.digest, bytes);
        reference
    }

    fn invocation(&mut self, document: &Json) -> DocumentRef {
        let label = document["identity"]["identity"]
            .as_str()
            .unwrap()
            .to_owned();
        let bytes = serde_json::to_vec(document).unwrap();
        let reference = lifecycle_document_ref(&label, &bytes);
        self.invocations.insert(reference.digest, bytes);
        reference
    }
}

fn labelled(label: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "tc235".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    }
}

fn boolean(value: bool) -> Json {
    json!({"boolean": value})
}

/// An `OrderLifecycle` context entry.
fn lifecycle(placed: bool, cancelled: bool) -> Json {
    json!({
        "machine": id("OrderLifecycle"),
        "fields": {"placed": boolean(placed), "cancelled": boolean(cancelled)},
    })
}

fn express(placed: bool, cancelled: bool, retries: i64) -> Json {
    json!({
        "machine": id("ExpressLifecycle"),
        "fields": {
            "placed": boolean(placed), "cancelled": boolean(cancelled),
            "retries": {"integer": retries.to_string()},
        },
    })
}

fn billing(paid: bool) -> Json {
    json!({"machine": id("Billing"), "fields": {"paid": boolean(paid)}})
}

fn owned(owner: Json, label: Json) -> Json {
    json!({
        "machine": id("Owned"),
        "fields": {
            "owner": owner,
            "label": label,
            "token": {"kind": "uuid", "text": "00112233-4455-6677-8899-aabbccddeeff"},
            "stamp": {"kind": "timestamp", "nanoseconds": "0"},
            "flags": {"record": {"type": id("Flags"), "fields": {"placed": boolean(true)}}},
        },
    })
}

fn pump_reference(key: &str) -> Json {
    json!({"reference": {"population": id("Q"), "key": key}})
}

fn string_value(text: &str) -> Json {
    json!({"kind": "string", "text": text})
}

fn pump_object(key: &str) -> Json {
    json!({"key": key, "type": id("Pump"), "fields": {"pump_id": {"integer": "1"}}})
}

fn population(name: &str, complete: bool, objects: Vec<Json>) -> Json {
    json!({"population": id(name), "complete": complete, "objects": objects})
}

fn flags_parameter() -> Json {
    json!({"flags": {"record": {"type": id("Flags"), "fields": {"placed": boolean(true)}}}})
}

impl Orders {
    /// The fixture, with `place` granting `placed` and `cancelled` (the wide
    /// frame) or `placed` alone (the narrow frame).
    fn new(narrow: bool) -> Self {
        let document = lifecycle_document_with("not self.placed", |document| {
            let boolean_type = format!("{NATIVE}/Boolean");
            let types = document["types"].as_array_mut().unwrap();
            types.push(declaration(
                "Flags",
                "money",
                json!({"fields": [field("Flags", "placed", &boolean_type)], "operations": []}),
            ));
            types.push(declaration(
                "Retries",
                "scalar",
                json!({
                    "scalar": "integer",
                    "constraints": [
                        bound("Retries", "min", "0"),
                        bound("Retries", "max", "5"),
                    ],
                }),
            ));
            types.push(declaration(
                "Pump",
                "thing",
                json!({
                    "identityFields": [id("Pump/pump_id")],
                    "fields": [field("Pump", "pump_id", &format!("{NATIVE}/Integer"))],
                    "operations": [operation("Pump", "service")],
                }),
            ));
            types.push(declaration(
                "Lamp",
                "thing",
                json!({
                    "identityFields": [id("Lamp/lamp_id")],
                    "fields": [field("Lamp", "lamp_id", &format!("{NATIVE}/Integer"))],
                    "operations": [],
                }),
            ));
            let mut expedite = operation("ExpressLifecycle", "expedite");
            expedite["frame"]["modifies"] = json!([
                id("OrderLifecycle/placed"),
                id("OrderLifecycle/cancelled"),
                id("ExpressLifecycle/retries"),
            ]);
            types.push(declaration(
                "ExpressLifecycle",
                "lifecycle",
                json!({
                    "supertypes": [id("OrderLifecycle")],
                    "fields": [field("ExpressLifecycle", "retries", &id("Retries"))],
                    "operations": [expedite],
                    "states": [{
                        "identity": id("ExpressLifecycle/Expedited"), "name": "Expedited",
                        "origin": origin("ExpressLifecycle/Expedited"),
                    }],
                    "transitions": [{
                        "identity": id("ExpressLifecycle/transition0"),
                        "from": id("OrderLifecycle/Placed"),
                        "to": id("ExpressLifecycle/Expedited"),
                        "trigger": id("ExpressLifecycle/expedite"),
                        "emits": [], "origin": origin("ExpressLifecycle/transition0"),
                    }],
                }),
            ));
            types.push(declaration(
                "Billing",
                "lifecycle",
                machine(
                    "Billing",
                    "pay",
                    vec![field("Billing", "paid", &boolean_type)],
                    operation("Billing", "pay"),
                ),
            ));
            let mut claim = operation("Owned", "claim");
            claim["frame"]["modifies"] = json!([
                id("Owned/owner"),
                id("Owned/label"),
                id("Owned/token"),
                id("Owned/stamp"),
                id("Owned/flags"),
            ]);
            types.push(declaration(
                "Owned",
                "lifecycle",
                machine(
                    "Owned",
                    "claim",
                    vec![
                        field("Owned", "owner", &id("Pump")),
                        field("Owned", "label", &format!("{NATIVE}/String")),
                        field("Owned", "token", &format!("{NATIVE}/UUID")),
                        field("Owned", "stamp", &format!("{NATIVE}/Timestamp")),
                        field("Owned", "flags", &id("Flags")),
                    ],
                    claim,
                ),
            ));
            let lifecycle = type_named_mut(document, "OrderLifecycle");
            lifecycle["operations"][0]["params"] = json!([{
                "identity": id("OrderLifecycle/place/flags"),
                "origin": origin("OrderLifecycle/place/flags"),
                "name": "flags",
                "typeRef": id("Flags"),
                "presence": "required",
                "nullable": false,
                "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            }]);
            if narrow {
                lifecycle["operations"][0]["frame"]["modifies"] =
                    json!([id("OrderLifecycle/placed")]);
            }
            document["constructs"].as_array_mut().unwrap().push(json!({
                "kind": {"module": crate::tc235_k2::BUSINESS, "name": "scalar"},
                "moduleVersion": "1.0.0",
                "construct": {
                    "identity": "none", "shape": "record", "members": {},
                    "meaning": "quire.meaning.model.value-type/v1",
                },
            }));
            document["populations"] = json!([
                population_declaration("Q", "Pump"),
                population_declaration("Spare", "Lamp"),
            ]);
        });
        let (package, _, packages) = model_package(
            &document,
            "invariant Consistent using v on M::OrderLifecycle at current { not (self.placed and self.cancelled) }\n\
             pre PlaceRequires using v on M::OrderLifecycle::place { not self.placed }\n\
             post FlagsPlaced using v on M::OrderLifecycle::place { flags.placed }\n\
             invariant Bounded using v on M::ExpressLifecycle at current { self.retries >= 0 }\n\
             invariant Owner using v on M::Owned at current { self.label = self.label }\n\
             invariant PumpKept using v on M::Pump at current { true }\n\
             post PumpServiced using v on M::Pump::service { true }",
            CHARGES,
        )
        .expect("the Orders fixture checks");
        let digest = packages.keys().next().expect("the package");
        let model = json!({
            "identity": crate::tc235_k2::PACKAGE,
            "version": "1",
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(digest)),
        });
        Self {
            package,
            packages,
            model,
        }
    }

    /// A snapshot document of `role` named `label`, holding `contexts`, of
    /// the invocation `i1`.
    fn snapshot(&self, label: &str, role: &str, contexts: Vec<Json>) -> Json {
        self.snapshot_of("i1", label, role, contexts)
    }

    /// The same, of the invocation `invocation`.
    fn snapshot_of(&self, invocation: &str, label: &str, role: &str, contexts: Vec<Json>) -> Json {
        json!({
            "format": "quire.state.snapshot/v1",
            "identity": lifecycle_identity(&labelled(label)),
            "observation": role,
            "invocation": lifecycle_identity(&labelled(invocation)),
            "model": self.model,
            "populations": [],
            "contexts": contexts,
        })
    }

    /// The invocation `label` of `operation` on the machine `context`, with
    /// `self` `self_json`, between `pre` and `post`.
    #[allow(clippy::too_many_arguments)]
    fn invocation(
        &self,
        label: &str,
        context: &str,
        operation: &str,
        self_json: Json,
        pre: &DocumentRef,
        post: &DocumentRef,
        parameters: Json,
    ) -> Json {
        let reference = |document: &DocumentRef| {
            json!({
                "identity": lifecycle_identity(document),
                "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&document.digest)),
            })
        };
        json!({
            "format": "quire.state.invocation/v1",
            "identity": lifecycle_identity(&labelled(label)),
            "model": self.model,
            "context": id(context),
            "operation": operation,
            "self": self_json,
            "pre": reference(pre),
            "post": reference(post),
            "parameters": parameters,
            "result": null,
            "created": [],
            "deleted": [],
        })
    }

    /// `i1`, the invocation of `place` with the contexts `pre` and `post`.
    fn place(&self, pre: Vec<Json>, post: Vec<Json>) -> (Docs, DocumentRef) {
        let mut docs = Docs::default();
        let pre = docs.snapshot(&self.snapshot("p1", "pre", pre));
        let post = docs.snapshot(&self.snapshot("q1", "post", post));
        let i1 = docs.invocation(&self.invocation(
            "i1",
            "OrderLifecycle",
            "place",
            Json::Null,
            &pre,
            &post,
            flags_parameter(),
        ));
        (docs, i1)
    }

    fn admit(
        &self,
        docs: &Docs,
        clause: &str,
        input: ClauseSelectionInput,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        lifecycle_selection(
            &self.package,
            &self.packages,
            clause,
            &docs.snapshots,
            &docs.invocations,
            input,
        )
    }

    fn context(
        &self,
        docs: &Docs,
        clause: &str,
        invocation: &DocumentRef,
        side: ContextSide,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        self.admit(
            docs,
            clause,
            ClauseSelectionInput::Context {
                invocation: invocation.clone(),
                side,
            },
        )
    }

    fn verdict(&self, clause: &str, admitted: &AdmittedObservations) -> bool {
        truth(&self.package, clause, admitted)
    }
}

/// `normalize.record` charges of the fixture's normalization.
const CHARGES: usize = 46;

/// A state machine `owner` of one state and one transition on its operation
/// `trigger`, with `fields` and `operation`.
fn machine(owner: &str, trigger: &str, fields: Vec<Json>, operation: Json) -> Json {
    let state = format!("{owner}/Open");
    let transition = format!("{owner}/transition0");
    json!({
        "fields": fields,
        "operations": [operation],
        "states": [{"identity": id(&state), "name": "Open", "origin": origin(&state)}],
        "transitions": [{
            "identity": id(&transition),
            "from": id(&state), "to": id(&state),
            "trigger": id(&format!("{owner}/{trigger}")),
            "emits": [], "origin": origin(&transition),
        }],
    })
}

fn bound(owner: &str, keyword: &str, value: &str) -> Json {
    let identity = id(&format!("{owner}/constraints/{keyword}"));
    json!({
        "identity": identity,
        "keyword": keyword,
        "operands": {"value": value},
        "appliesTo": format!("{NATIVE}/Integer"),
        "diagnosticCode": format!("bound.{keyword}"),
        "origin": origin(owner),
    })
}

fn population_declaration(name: &str, member: &str) -> Json {
    json!({
        "identity": id(name), "displayName": id(name),
        "kind": {"module": crate::tc235_k2::BUSINESS, "name": "population"},
        "members": [id(member)], "extent": "closed",
        "origin": origin(name),
    })
}

/// The refusal an admission gave: its code, cause and fields.
fn refused(outcome: Result<AdmittedObservations, AdmissionFailure>) -> AdmissionRecord {
    match outcome {
        Err(AdmissionFailure::Refused(record)) => record,
        other => panic!("a refusal, not {other:?}"),
    }
}

fn assert_refusal(record: &AdmissionRecord, code: Code, cause: &str, named: &[(&str, &str)]) {
    assert_eq!((record.code, record.cause), (code, cause), "{record:?}");
    for (key, value) in named {
        assert_eq!(
            record.fields.get(key).map(String::as_str),
            Some(*value),
            "{record:?}"
        );
    }
}

/// TC-1045 step 1 (FR-106-AC-41): the invariant `Consistent` on side `pre`
/// over `P1`, and on side `post` over `Q1`, admits one observation holding
/// that side's context, with `i1`'s identity, no parameter and no result, and
/// evaluates `Completed(true)`. Reordering `fields` members and `contexts`
/// entries gives the same verdicts and, for `fields`, the same digest.
#[trace("TC-1045", "FR-106-AC-41")]
#[test]
fn a_context_selection_admits_one_side_of_the_invocation() {
    let orders = Orders::new(false);
    let (docs, i1) = orders.place(vec![lifecycle(false, false)], vec![lifecycle(true, false)]);
    for (side, label) in [(ContextSide::Pre, "p1"), (ContextSide::Post, "q1")] {
        let admitted = orders
            .context(&docs, "Consistent", &i1, side)
            .unwrap_or_else(|failure| panic!("{side:?} admits: {failure:?}"));
        let observation = admitted.current.as_ref().expect("one observation");
        assert_eq!(observation.identity.identity, label);
        assert!(admitted.pre.is_none() && admitted.post.is_none());
        assert_eq!(admitted.self_kind, SelfKind::Context);
        assert_eq!(
            admitted.invocation.as_ref().map(|i| i.identity.as_str()),
            Some("i1")
        );
        assert!(admitted.parameters.is_empty() && admitted.result.is_none());
        assert!(orders.verdict("Consistent", &admitted), "{side:?}");
    }

    // Reordered `fields` members and `contexts` entries: the same verdicts,
    // and for `fields` alone the same digest.
    let reversed = |placed: bool| {
        json!({
            "machine": id("OrderLifecycle"),
            "fields": {"cancelled": boolean(false), "placed": boolean(placed)},
        })
    };
    let mut reordered = Docs::default();
    let pre =
        reordered.snapshot(&orders.snapshot("p1", "pre", vec![reversed(false), billing(true)]));
    let post =
        reordered.snapshot(&orders.snapshot("q1", "post", vec![billing(true), reversed(true)]));
    let i1 = reordered.invocation(&orders.invocation(
        "i1",
        "OrderLifecycle",
        "place",
        Json::Null,
        &pre,
        &post,
        flags_parameter(),
    ));
    for side in [ContextSide::Pre, ContextSide::Post] {
        let admitted = orders.context(&reordered, "Consistent", &i1, side).unwrap();
        assert!(orders.verdict("Consistent", &admitted));
    }
    let digest_of = |document: &Json| {
        let mut docs = Docs::default();
        docs.snapshot(document).digest
    };
    assert_eq!(
        digest_of(&orders.snapshot("p1", "pre", vec![lifecycle(false, false)])),
        digest_of(&orders.snapshot("p1", "pre", vec![reversed(false)])),
        "member order is no part of the digest"
    );

    // A false invariant is a logical violation after admission (AC-42).
    let (docs, i1) = orders.place(vec![lifecycle(false, false)], vec![lifecycle(true, true)]);
    let admitted = orders
        .context(&docs, "Consistent", &i1, ContextSide::Post)
        .unwrap();
    assert!(!orders.verdict("Consistent", &admitted));
}

/// TC-1045 step 1 and 2 (FR-106-AC-42): the post context `{placed: true,
/// cancelled: true}` admits side `post` and `Consistent` evaluates `false`;
/// side `pre` ignores the post side whatever it holds, `parameters`
/// included; side `post` without its snapshot is `Incomplete`.
#[trace("TC-1045", "FR-106-AC-42")]
#[test]
fn side_pre_ignores_the_post_side_and_the_parameters() {
    let orders = Orders::new(false);
    let (docs, i1) = orders.place(vec![lifecycle(false, false)], vec![lifecycle(true, true)]);
    let post = orders
        .context(&docs, "Consistent", &i1, ContextSide::Post)
        .unwrap();
    assert!(!orders.verdict("Consistent", &post));
    let pre = orders
        .context(&docs, "Consistent", &i1, ContextSide::Pre)
        .unwrap();
    assert!(orders.verdict("Consistent", &pre));

    let q1 = docs.invocations.values().next().cloned().map(|bytes| {
        let document: Json = serde_json::from_slice(&bytes).unwrap();
        document["post"].clone()
    });
    let q1_digest = {
        let hex = q1.as_ref().unwrap()["digest"]
            .as_str()
            .unwrap()
            .trim_start_matches("sha256-jcs:");
        let mut digest = [0u8; 32];
        for (at, byte) in digest.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[at * 2..at * 2 + 2], 16).unwrap();
        }
        digest
    };

    // The post snapshot absent, and its bytes not JSON.
    let mut absent = docs.clone();
    absent.snapshots.remove(&q1_digest);
    let admitted = orders
        .context(&absent, "Consistent", &i1, ContextSide::Pre)
        .unwrap();
    assert!(orders.verdict("Consistent", &admitted));
    let mut garbage = docs.clone();
    garbage.snapshots.insert(q1_digest, b"not json".to_vec());
    let admitted = orders
        .context(&garbage, "Consistent", &i1, ContextSide::Pre)
        .unwrap();
    assert!(orders.verdict("Consistent", &admitted));

    // `parameters` ill-formed, and `result`, `created` and `deleted` removed.
    let rebuilt = |edit: &dyn Fn(&mut Json)| {
        let mut docs = docs.clone();
        let (_, bytes) = docs
            .invocations
            .iter()
            .next()
            .map(|(k, v)| (*k, v.clone()))
            .unwrap();
        let mut document: Json = serde_json::from_slice(&bytes).unwrap();
        edit(&mut document);
        docs.invocations.clear();
        let i1 = docs.invocation(&document);
        (docs, i1)
    };
    let (ill_formed, i1_ill) = rebuilt(&|document| document["parameters"] = json!(5));
    let admitted = orders
        .context(&ill_formed, "Consistent", &i1_ill, ContextSide::Pre)
        .unwrap();
    assert!(orders.verdict("Consistent", &admitted));
    let (stripped, i1_stripped) = rebuilt(&|document| {
        let members = document.as_object_mut().unwrap();
        for member in ["result", "created", "deleted"] {
            members.remove(member);
        }
    });
    let admitted = orders
        .context(&stripped, "Consistent", &i1_stripped, ContextSide::Pre)
        .unwrap();
    assert!(orders.verdict("Consistent", &admitted));

    // Side `post` without its snapshot returns `Incomplete`.
    let Err(AdmissionFailure::Incomplete(record)) =
        orders.context(&absent, "Consistent", &i1, ContextSide::Post)
    else {
        panic!("side post without its snapshot is incomplete");
    };
    assert_eq!(
        (record.code, record.cause),
        (Code::UnavailableObservation, "missing-required-artifact")
    );
}

/// TC-1045 step 3 (FR-106-AC-43): a snapshot with no context of the machine
/// refuses `wrong-context` for the invariant on both sides, for the
/// precondition by `Invocation` and `PreCall`, and for the postcondition; two
/// contexts of the machine (or of it and a specialization) refuse
/// `duplicate-member`; a context of an unrelated machine is ignored.
#[trace("TC-1045", "FR-106-AC-43")]
#[test]
fn a_snapshot_must_hold_exactly_one_context_of_the_machine() {
    let orders = Orders::new(false);
    let named = [("machine", id("OrderLifecycle"))];
    let selections = |orders: &Orders, docs: &Docs, i1: &DocumentRef| {
        let pre_call = {
            let p1 = docs
                .snapshots
                .values()
                .find_map(|bytes| {
                    let document: Json = serde_json::from_slice(bytes).unwrap();
                    (document["observation"] == "pre").then(|| lifecycle_document_ref("p1", bytes))
                })
                .unwrap();
            ClauseSelectionInput::PreCall {
                snapshot: p1,
                self_object: None,
                parameters: BTreeMap::from([(
                    quire_exact::Identifier::new("flags").unwrap(),
                    SnapshotValue::record(
                        id("Flags"),
                        vec![("placed".to_owned(), SnapshotValue::boolean(true))],
                    ),
                )]),
            }
        };
        vec![
            orders.context(docs, "Consistent", i1, ContextSide::Pre),
            orders.context(docs, "Consistent", i1, ContextSide::Post),
            orders.admit(
                docs,
                "PlaceRequires",
                ClauseSelectionInput::Invocation {
                    invocation: i1.clone(),
                },
            ),
            orders.admit(docs, "PlaceRequires", pre_call),
            orders.admit(
                docs,
                "FlagsPlaced",
                ClauseSelectionInput::Invocation {
                    invocation: i1.clone(),
                },
            ),
        ]
    };

    // No `contexts` member in `P1`, then only a `Billing` entry.
    for pre in [vec![], vec![billing(false)]] {
        let (docs, i1) = orders.place(pre, vec![lifecycle(true, false)]);
        for outcome in selections(&orders, &docs, &i1) {
            let record = refused(outcome);
            assert_refusal(
                &record,
                Code::InvalidModelBinding,
                "wrong-context",
                &[("machine", &named[0].1), ("document", "p1")],
            );
        }
    }

    // `Q1` without a context: the post side refuses, naming `Q1`.
    let (docs, i1) = orders.place(vec![lifecycle(false, false)], vec![]);
    let outcomes = selections(&orders, &docs, &i1);
    let [pre_side, post_side, required, by_pre_call, ensured] = &outcomes[..] else {
        unreachable!("five selections");
    };
    assert!(pre_side.is_ok() && required.is_ok() && by_pre_call.is_ok());
    for outcome in [post_side, ensured] {
        let record = refused(outcome.clone());
        assert_refusal(
            &record,
            Code::InvalidModelBinding,
            "wrong-context",
            &[("machine", &named[0].1), ("document", "q1")],
        );
    }

    // Two entries of one machine, then a machine and its specialization.
    let (docs, i1) = orders.place(
        vec![lifecycle(false, false), lifecycle(false, false)],
        vec![lifecycle(true, false)],
    );
    let record = refused(orders.context(&docs, "Consistent", &i1, ContextSide::Pre));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "duplicate-member",
        &[("entry", "1"), ("document", "p1")],
    );
    let (docs, i1) = orders.place(
        vec![lifecycle(false, false), express(false, false, 0)],
        vec![lifecycle(true, false)],
    );
    let record = refused(orders.context(&docs, "Consistent", &i1, ContextSide::Pre));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "duplicate-member",
        &[("entry", "1"), ("document", "p1")],
    );

    // An unrelated machine is validated and ignored.
    let (docs, i1) = orders.place(
        vec![lifecycle(false, false), billing(false)],
        vec![lifecycle(true, false), billing(true)],
    );
    for side in [ContextSide::Pre, ContextSide::Post] {
        let admitted = orders.context(&docs, "Consistent", &i1, side).unwrap();
        assert!(orders.verdict("Consistent", &admitted));
    }
}

/// One defect case: its name, the edit, and the refusal it gives.
type Case = (
    &'static str,
    Box<dyn Fn(&mut Json)>,
    Code,
    &'static str,
    Vec<(&'static str, String)>,
);

/// TC-1045 step 4 (FR-106-AC-44): each condition of checks 6.10 to 6.12 has
/// one case that fails only it and returns exactly its code and cause, naming
/// the snapshot and the entry; the object defect is reported before the
/// context defect, and the first entry's before the second's.
#[trace("TC-1045", "FR-106-AC-44")]
#[test]
fn each_context_defect_refuses_with_its_own_cause() {
    let orders = Orders::new(false);
    let runtime = Code::InvalidRuntimeInput;
    let defect = |edit: &dyn Fn(&mut Json)| {
        let mut pre = orders.snapshot("p1", "pre", vec![lifecycle(false, false)]);
        edit(&mut pre);
        let mut docs = Docs::default();
        let pre = docs.snapshot(&pre);
        let post = docs.snapshot(&orders.snapshot("q1", "post", vec![lifecycle(true, false)]));
        let i1 = docs.invocation(&orders.invocation(
            "i1",
            "OrderLifecycle",
            "place",
            Json::Null,
            &pre,
            &post,
            flags_parameter(),
        ));
        refused(orders.context(&docs, "Consistent", &i1, ContextSide::Pre))
    };
    let cases: Vec<Case> = vec![
        (
            "contexts an object",
            Box::new(|document| document["contexts"] = json!({})),
            runtime,
            "wrong-value-kind",
            vec![("member", "contexts".to_owned())],
        ),
        (
            "an entry without machine",
            Box::new(|document| {
                document["contexts"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("machine");
            }),
            runtime,
            "missing-member",
            vec![("member", "machine".to_owned())],
        ),
        (
            "an entry without fields",
            Box::new(|document| {
                document["contexts"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("fields");
            }),
            runtime,
            "missing-member",
            vec![("member", "fields".to_owned())],
        ),
        (
            "an extra member",
            Box::new(|document| document["contexts"][0]["extra"] = json!(1)),
            runtime,
            "unknown-member",
            vec![("member", "extra".to_owned())],
        ),
        (
            "a numeric machine",
            Box::new(|document| document["contexts"][0]["machine"] = json!(7)),
            runtime,
            "wrong-value-kind",
            vec![("member", "machine".to_owned())],
        ),
        (
            "an array fields",
            Box::new(|document| document["contexts"][0]["fields"] = json!([])),
            runtime,
            "wrong-value-kind",
            vec![("member", "fields".to_owned())],
        ),
        (
            "an object type as the machine",
            Box::new(|document| document["contexts"][0]["machine"] = json!(id("Pump"))),
            runtime,
            "wrong-role-mapping",
            vec![("machine", id("Pump"))],
        ),
        (
            "a process as the machine",
            Box::new(|document| document["contexts"][0]["machine"] = json!(id("Fulfilment"))),
            runtime,
            "wrong-role-mapping",
            vec![("machine", id("Fulfilment"))],
        ),
        (
            "an unknown machine",
            Box::new(|document| document["contexts"][0]["machine"] = json!(id("Missing"))),
            runtime,
            "wrong-role-mapping",
            vec![("machine", id("Missing"))],
        ),
        (
            "a field missing",
            Box::new(|document| {
                document["contexts"][0]["fields"]
                    .as_object_mut()
                    .unwrap()
                    .remove("cancelled");
            }),
            runtime,
            "missing-member",
            vec![("field", "cancelled".to_owned())],
        ),
        (
            "a state member",
            Box::new(|document| {
                document["contexts"][0]["fields"]["state"] = json!({"integer": "1"})
            }),
            runtime,
            "unknown-member",
            vec![("field", "state".to_owned())],
        ),
        (
            "a transitions member",
            Box::new(|document| {
                document["contexts"][0]["fields"]["transitions"] = json!({"sequence": []})
            }),
            runtime,
            "unknown-member",
            vec![("field", "transitions".to_owned())],
        ),
        (
            "an integer for a Boolean",
            Box::new(|document| {
                document["contexts"][0]["fields"]["placed"] = json!({"integer": "1"})
            }),
            runtime,
            "wrong-value-kind",
            vec![
                ("machine", id("OrderLifecycle")),
                ("field", "placed".to_owned()),
            ],
        ),
        (
            "an object defect before a context defect",
            Box::new(|document| {
                document["populations"] = json!([population(
                    "Q",
                    true,
                    vec![json!({"key": "p1", "type": id("Order"), "fields": {}})],
                )]);
                document["contexts"][0]["extra"] = json!(1);
            }),
            runtime,
            "wrong-role-mapping",
            vec![("object", "p1".to_owned())],
        ),
        (
            "defects in two entries",
            Box::new(|document| {
                let mut second = lifecycle(false, false);
                second["extra"] = json!(1);
                let mut first = document["contexts"][0].clone();
                first["fields"]["placed"] = json!({"integer": "1"});
                document["contexts"] = json!([first, second]);
            }),
            runtime,
            "wrong-value-kind",
            vec![("entry", "0".to_owned()), ("field", "placed".to_owned())],
        ),
    ];
    for (name, edit, code, cause, named) in &cases {
        let record = defect(edit.as_ref());
        let named: Vec<(&str, &str)> = named.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(
            (record.code, record.cause),
            (*code, *cause),
            "{name}: {record:?}"
        );
        for (key, value) in &named {
            assert_eq!(
                record.fields.get(key).map(String::as_str),
                Some(*value),
                "{name}: {record:?}"
            );
        }
        if !name.contains("object defect") && !name.contains("contexts an object") {
            assert_eq!(
                record.fields.get("document").map(String::as_str),
                Some("p1"),
                "{name}"
            );
        }
    }

    // `contexts` on a snapshot that says `current`, selected by `Current`
    // for an object invariant.
    let mut current = orders.snapshot("c1", "current", vec![lifecycle(false, false)]);
    current["anchor"] = json!({"kind": "handler", "name": "validate"});
    current["populations"] = json!([population("Q", true, vec![pump_object("p1")])]);
    current.as_object_mut().unwrap().remove("invocation");
    let mut docs = Docs::default();
    let c1 = docs.snapshot(&current);
    let record = refused(orders.admit(
        &docs,
        "PumpKept",
        ClauseSelectionInput::Current {
            snapshot: c1,
            anchor: SelectedAnchor {
                kind: AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: Some(SelectedObject {
                population: id("Q"),
                key: "p1".to_owned(),
            }),
        },
    ));
    assert_refusal(&record, runtime, "unknown-member", &[("field", "contexts")]);
}

/// TC-1045 step 5 (FR-106-AC-45): the post-state range witness reaches a
/// context. `Bounded` on `ExpressLifecycle` over `i2` of `expedite`: the post
/// context `retries: -1` under `Witness` is admitted and reported as one
/// `OutOfRange` naming the machine, `retries`, `[0, 5]` and `-1`, and
/// `Bounded` is `Completed(false)`; under `Refuse`, and in a `pre` context,
/// it refuses `invalid-value`.
#[trace("TC-1045", "FR-106-AC-45")]
#[test]
fn the_post_state_range_witness_reaches_a_context() {
    let orders = Orders::new(false);
    let witness = ObservationLimits {
        post_state: PostStateRange::Witness,
        ..ObservationLimits::default()
    };
    let expedite = |pre_retries: i64| {
        let mut docs = Docs::default();
        let pre = docs.snapshot(&orders.snapshot_of(
            "i2",
            "e1",
            "pre",
            vec![express(false, false, pre_retries)],
        ));
        let post =
            docs.snapshot(&orders.snapshot_of("i2", "e2", "post", vec![express(true, false, -1)]));
        let mut invocation = orders.invocation(
            "i2",
            "ExpressLifecycle",
            "expedite",
            Json::Null,
            &pre,
            &post,
            json!({}),
        );
        invocation["operation"] = json!("expedite");
        let i2 = docs.invocation(&invocation);
        (docs, i2)
    };
    let select = |docs: &Docs, i2: &DocumentRef, side, limits| {
        lifecycle_selection_with(
            &orders.package,
            &orders.packages,
            "Bounded",
            &docs.snapshots,
            &docs.invocations,
            ClauseSelectionInput::Context {
                invocation: i2.clone(),
                side,
            },
            limits,
        )
    };

    let (docs, i2) = expedite(0);
    let admitted = select(&docs, &i2, ContextSide::Post, witness).expect("the witness admits");
    let observation = admitted.current.as_ref().unwrap();
    let [breach] = &observation.out_of_range[..] else {
        panic!("one OutOfRange: {:?}", observation.out_of_range);
    };
    assert_eq!(breach.field.name, "retries");
    assert_eq!(breach.observed, quire_exact::Integer::from(-1_i64));
    assert!(breach.range.contains(&quire_exact::Integer::from(0_i64)));
    assert!(breach.range.contains(&quire_exact::Integer::from(5_i64)));
    assert!(!breach.range.contains(&quire_exact::Integer::from(6_i64)));
    let express = orders
        .package
        .graph()
        .scope()
        .types()
        .object_types()
        .find(|declared| declared.name() == "M::ExpressLifecycle")
        .expect("ExpressLifecycle")
        .key();
    assert_eq!(breach.object.object_type(), express);
    assert!(!orders.verdict("Bounded", &admitted));

    let record = refused(select(
        &docs,
        &i2,
        ContextSide::Post,
        ObservationLimits::default(),
    ));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "invalid-value",
        &[("machine", &id("ExpressLifecycle")), ("field", "retries")],
    );
    let (docs, i2) = expedite(-1);
    let record = refused(select(&docs, &i2, ContextSide::Pre, witness));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "invalid-value",
        &[("machine", &id("ExpressLifecycle")), ("field", "retries")],
    );
}

/// TC-1045 step 6 (FR-106-AC-46): the frame check compares contexts. With
/// `place` modifying `placed` and `cancelled` a post context changing both
/// admits; with `place` modifying only `placed`, one that also changes
/// `cancelled` refuses `frame_violation`/`unauthorized-change` naming the
/// machine and the field, for the postcondition and for the invariant on side
/// `post`; side `pre` runs no frame check. A post context equal to the pre
/// context in another member order is no change.
#[trace("TC-1045", "FR-106-AC-46")]
#[test]
fn the_frame_check_compares_contexts() {
    let wide = Orders::new(false);
    let (docs, i1) = wide.place(vec![lifecycle(false, false)], vec![lifecycle(true, true)]);
    assert!(wide
        .context(&docs, "Consistent", &i1, ContextSide::Post)
        .is_ok());

    let narrow = Orders::new(true);
    let (docs, i1) = narrow.place(vec![lifecycle(false, false)], vec![lifecycle(true, true)]);
    let by_invocation = ClauseSelectionInput::Invocation {
        invocation: i1.clone(),
    };
    for outcome in [
        narrow.context(&docs, "Consistent", &i1, ContextSide::Post),
        narrow.admit(&docs, "FlagsPlaced", by_invocation),
    ] {
        let record = refused(outcome);
        assert_refusal(
            &record,
            Code::FrameViolation,
            "unauthorized-change",
            &[("machine", &id("OrderLifecycle")), ("field", "cancelled")],
        );
    }
    assert!(narrow
        .context(&docs, "Consistent", &i1, ContextSide::Pre)
        .is_ok());

    // Equal contexts in another member order are no change.
    let (mut docs, _) = narrow.place(vec![lifecycle(false, false)], vec![lifecycle(false, false)]);
    let reversed = json!({
        "machine": id("OrderLifecycle"),
        "fields": {"cancelled": boolean(false), "placed": boolean(false)},
    });
    let pre = docs.snapshot(&narrow.snapshot("p1", "pre", vec![lifecycle(false, false)]));
    let post = docs.snapshot(&narrow.snapshot("q1", "post", vec![reversed]));
    let i1 = docs.invocation(&narrow.invocation(
        "i1",
        "OrderLifecycle",
        "place",
        Json::Null,
        &pre,
        &post,
        flags_parameter(),
    ));
    assert!(narrow
        .context(&docs, "Consistent", &i1, ContextSide::Post)
        .is_ok());

    // A retype is a change no frame authorizes.
    let (docs, i1) = wide.place(
        vec![lifecycle(false, false)],
        vec![express(false, false, 0)],
    );
    let record = refused(wide.context(&docs, "Consistent", &i1, ContextSide::Post));
    assert_refusal(&record, Code::FrameViolation, "unauthorized-change", &[]);
}

/// TC-1045 step 6 (FR-106-AC-46): `self` must agree with the receiver, and a
/// selection must agree with the clause.
#[trace("TC-1045", "FR-106-AC-46")]
#[test]
fn self_and_the_selection_must_agree_with_the_receiver() {
    let orders = Orders::new(false);
    let (mut docs, i1) = orders.place(vec![lifecycle(false, false)], vec![lifecycle(true, false)]);

    // An object reference for a state-machine operation.
    let pre = docs.snapshot(&orders.snapshot("p1", "pre", vec![lifecycle(false, false)]));
    let post = docs.snapshot(&orders.snapshot("q1", "post", vec![lifecycle(true, false)]));
    let with_self = docs.invocation(&orders.invocation(
        "i1",
        "OrderLifecycle",
        "place",
        json!({"population": id("Q"), "key": "p1"}),
        &pre,
        &post,
        flags_parameter(),
    ));
    let record = refused(orders.context(&docs, "Consistent", &with_self, ContextSide::Pre));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        &[("field", "self")],
    );

    // `null` for an object operation.
    let mut pumps = Docs::default();
    let mut snapshot = |label: &str, role: &str| {
        let mut document = orders.snapshot(label, role, vec![]);
        document["populations"] = json!([population("Q", true, vec![pump_object("p1")])]);
        pumps.snapshot(&document)
    };
    let (pre, post) = (snapshot("p1", "pre"), snapshot("q1", "post"));
    let service = pumps.invocation(&orders.invocation(
        "i1",
        "Pump",
        "service",
        Json::Null,
        &pre,
        &post,
        json!({}),
    ));
    let record = refused(orders.admit(
        &pumps,
        "PumpServiced",
        ClauseSelectionInput::Invocation {
            invocation: service,
        },
    ));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        &[("field", "self")],
    );

    // Selections the clause does not take.
    let p1 = lifecycle_document_ref(
        "p1",
        docs.snapshots
            .values()
            .find(|bytes| {
                serde_json::from_slice::<Json>(bytes).unwrap()["identity"]["identity"] == "p1"
            })
            .unwrap(),
    );
    let wrong = |outcome| {
        assert_refusal(
            &refused(outcome),
            Code::WrongSnapshot,
            "wrong-observation",
            &[],
        );
    };
    wrong(orders.admit(
        &docs,
        "Consistent",
        ClauseSelectionInput::Current {
            snapshot: p1.clone(),
            anchor: SelectedAnchor {
                kind: AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: None,
        },
    ));
    wrong(orders.admit(
        &docs,
        "Consistent",
        ClauseSelectionInput::PreCall {
            snapshot: p1.clone(),
            self_object: None,
            parameters: BTreeMap::new(),
        },
    ));
    wrong(orders.admit(
        &docs,
        "Consistent",
        ClauseSelectionInput::Invocation {
            invocation: i1.clone(),
        },
    ));
    wrong(orders.context(&docs, "PumpKept", &i1, ContextSide::Pre));
    wrong(orders.admit(
        &docs,
        "PlaceRequires",
        ClauseSelectionInput::PreCall {
            snapshot: p1,
            self_object: Some(SelectedObject {
                population: id("Q"),
                key: "p1".to_owned(),
            }),
            parameters: BTreeMap::new(),
        },
    ));
}

/// TC-1045 step 7 (FR-106-AC-47): a reference in a context field closes
/// against the populations of the same snapshot; each context entry and field
/// value counts toward `observation.values`; an environment document with
/// `contexts` is refused.
#[trace("TC-1045", "FR-106-AC-47")]
#[test]
fn a_context_reference_closes_against_the_snapshot() {
    let orders = Orders::new(false);
    let claim = |label: Json, populations: Json, owner: Json| {
        let mut docs = Docs::default();
        let mut pre = orders.snapshot_of("i3", "o1", "pre", vec![owned(owner, label)]);
        pre["populations"] = populations;
        let pre = docs.snapshot(&pre);
        let post = docs.snapshot(&orders.snapshot_of("i3", "o2", "post", vec![]));
        let i3 = docs.invocation(&orders.invocation(
            "i3",
            "Owned",
            "claim",
            Json::Null,
            &pre,
            &post,
            json!({}),
        ));
        (docs, i3)
    };
    let owner = |docs: &Docs, i3: &DocumentRef| orders.context(docs, "Owner", i3, ContextSide::Pre);
    let x = string_value("x");

    // `Q` complete without `p9`.
    let (docs, i3) = claim(
        x.clone(),
        json!([population("Q", true, vec![])]),
        pump_reference("p9"),
    );
    let record = refused(owner(&docs, &i3));
    assert_refusal(
        &record,
        Code::DanglingReference,
        "absent-target-in-complete-population",
        &[
            ("machine", &id("Owned")),
            ("field", "owner"),
            ("population", &id("Q")),
            ("object", "p9"),
        ],
    );

    // `Q` listed incomplete.
    let (docs, i3) = claim(
        x.clone(),
        json!([population("Q", false, vec![])]),
        pump_reference("p9"),
    );
    let Err(AdmissionFailure::Incomplete(record)) = owner(&docs, &i3) else {
        panic!("an incomplete population returns Incomplete");
    };
    assert_eq!(
        (record.code, record.cause),
        (Code::IncompletePopulation, "incomplete-scope")
    );
    assert_eq!(
        record.fields.get("population").map(String::as_str),
        Some(id("Q").as_str())
    );

    // `owner` naming the listed `p1`, with and without an unnamed incomplete
    // population.
    let listed = json!([population("Q", true, vec![pump_object("p1")])]);
    let (docs, i3) = claim(x.clone(), listed, pump_reference("p1"));
    let admitted = owner(&docs, &i3).expect("a listed target admits");
    assert!(orders.verdict("Owner", &admitted));
    let with_spare = json!([
        population("Q", true, vec![pump_object("p1")]),
        population("Spare", false, vec![]),
    ]);
    let (docs, i3) = claim(x, with_spare, pump_reference("p1"));
    assert!(owner(&docs, &i3).is_ok());

    // A String of U+00E9 admits; a `text` of `5` refuses at check 6.12,
    // before any reference is closed.
    let populations = json!([population("Q", true, vec![pump_object("p1")])]);
    let (docs, i3) = claim(
        string_value("\u{e9}"),
        populations.clone(),
        pump_reference("p1"),
    );
    assert!(owner(&docs, &i3).is_ok());
    let (docs, i3) = claim(
        json!({"kind": "string", "text": 5}),
        json!([population("Q", true, vec![])]),
        pump_reference("p9"),
    );
    let record = refused(owner(&docs, &i3));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        &[
            ("machine", &id("Owned")),
            ("field", "label"),
            ("member", "text"),
        ],
    );

    // The value bound: the exact count admits, one less refuses.
    let (docs, i3) = claim(string_value("x"), populations, pump_reference("p1"));
    let exact = owner(&docs, &i3).unwrap().usage.values;
    let at = |bound: u64| {
        lifecycle_selection_with(
            &orders.package,
            &orders.packages,
            "Owner",
            &docs.snapshots,
            &docs.invocations,
            ClauseSelectionInput::Context {
                invocation: i3.clone(),
                side: ContextSide::Pre,
            },
            ObservationLimits::default().with_values_per_document(bound),
        )
    };
    assert!(at(exact).is_ok());
    let record = refused(at(exact - 1));
    assert_eq!(
        (record.code, record.cause),
        (Code::StageLimitExceeded, "node-count-exceeded")
    );

    // An environment document is a current snapshot: `contexts` refuse.
    let mut environment = orders.snapshot("env", "current", vec![lifecycle(false, false)]);
    environment.as_object_mut().unwrap().remove("invocation");
    let mut docs = Docs::default();
    let env = docs.snapshot(&environment);
    let Err(AdmissionFailure::Refused(record)) = admit_current_snapshot(
        orders.package.graph().model_selections(),
        orders.package.graph().scope().types(),
        &orders.packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &docs.snapshots,
        &env,
        ObservationLimits::default(),
    ) else {
        panic!("an environment document with contexts refuses");
    };
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "unknown-member",
        &[("field", "contexts")],
    );
}

/// TC-1045 step 5 (FR-106-AC-45): a context field of a record, String, UUID or
/// Timestamp type follows the value rules of an object field: a payload
/// defect refuses in the payload order at the context path, and a record
/// refuses at its nested path.
#[trace("TC-1045", "FR-106-AC-45")]
#[test]
fn a_context_field_follows_the_value_rules_of_an_object_field() {
    let orders = Orders::new(false);
    let admit = |field: &str, value: Json| {
        let mut docs = Docs::default();
        let mut entry = owned(pump_reference("p1"), string_value("x"));
        entry["fields"][field] = value;
        let mut pre = orders.snapshot_of("i3", "o1", "pre", vec![entry]);
        pre["populations"] = json!([population("Q", true, vec![pump_object("p1")])]);
        let pre = docs.snapshot(&pre);
        let post = docs.snapshot(&orders.snapshot_of("i3", "o2", "post", vec![]));
        let i3 = docs.invocation(&orders.invocation(
            "i3",
            "Owned",
            "claim",
            Json::Null,
            &pre,
            &post,
            json!({}),
        ));
        orders.context(&docs, "Owner", &i3, ContextSide::Pre)
    };
    admit("label", string_value("x")).expect("the base context admits");

    let refusal = |field: &str, value: Json| refused(admit(field, value));
    let record = refusal("label", json!({"kind": "string", "text": 5}));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        &[
            ("machine", &id("Owned")),
            ("field", "label"),
            ("member", "text"),
        ],
    );
    let record = refusal("token", json!({"kind": "uuid", "text": "not-a-uuid"}));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "invalid-value",
        &[("machine", &id("Owned")), ("field", "token")],
    );
    let record = refusal("stamp", json!({"kind": "timestamp", "nanoseconds": "x"}));
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "invalid-value",
        &[("machine", &id("Owned")), ("field", "stamp")],
    );
    let record = refusal(
        "flags",
        json!({"record": {"type": id("Flags"), "fields": {}}}),
    );
    assert_refusal(
        &record,
        Code::InvalidRuntimeInput,
        "missing-member",
        &[("machine", &id("Owned")), ("field", "flags")],
    );
}
