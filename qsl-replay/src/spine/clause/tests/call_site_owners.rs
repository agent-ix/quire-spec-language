// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1162 (FR-121-AC-25 to AC-27): `crate::call_site`'s operation selection
//! over an owner that is a process or a state machine, and its refusals of
//! an owner that declares no operations and of an ambiguous operation name.
//!
//! The domain package is the ConfigVersion package plus a process `Flow`
//! (FR-104-AC-10), a state machine `OrderLifecycle` and `ExpressLifecycle`
//! specializing it (TC-1045), and one declaration of each kind that cannot
//! own an operation a frame names.

use super::frame::identifier;
use super::*;
use crate::spine::OperationName;
use crate::{
    call_site, CallSite, CallSiteRefusal, DeclineCode, FrameOperation, OperationSite,
    ProofRefusalCause, QualifiedName, TerminalValue,
};

const PACKAGE: &str = CONFIG_VERSION_PACKAGE_IDENTITY;

fn id(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

fn origin(identity: &str) -> serde_json::Value {
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn construct(
    name: &str,
    meaning: &str,
    shape: &str,
    members: serde_json::Value,
) -> serde_json::Value {
    json!({
        "kind": {"module": PACKAGE, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": PLACEHOLDER_DIGEST,
        "construct": {
            "identity": "none",
            "shape": shape,
            "members": members,
            "meaning": meaning,
        },
    })
}

fn declaration(name: &str, kind: &str, body: serde_json::Value) -> serde_json::Value {
    let identity = id(name);
    let mut node = json!({
        "identity": identity,
        "displayName": identity,
        "kind": {"module": PACKAGE, "name": kind},
        "roles": [],
        "origin": origin(&identity),
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
    });
    for (key, value) in body.as_object().expect("a declaration body is an object") {
        node[key] = value.clone();
    }
    node
}

fn field(owner: &str, name: &str, type_ref: &str, presence: &str) -> serde_json::Value {
    let identity = id(&format!("{owner}/{name}"));
    json!({
        "identity": identity,
        "name": name,
        "typeRef": type_ref,
        "presence": presence,
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&identity),
    })
}

/// A no-parameter operation of `owner` returning `Boolean`, modifying the
/// owner fields `modifies`.
fn operation_of(owner: &str, name: &str, modifies: &[&str]) -> serde_json::Value {
    let identity = id(&format!("{owner}/{name}"));
    let modifies: Vec<String> = modifies
        .iter()
        .map(|field| id(&format!("{owner}/{field}")))
        .collect();
    json!({
        "identity": identity,
        "name": name,
        "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Boolean",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [],
        "post": [],
        "origin": origin(&identity),
        "frame": {"modifies": modifies, "creates": [], "deletes": []},
    })
}

fn state(machine: &str, name: &str) -> serde_json::Value {
    let identity = id(&format!("{machine}/{name}"));
    json!({"identity": identity, "name": name, "origin": origin(&identity)})
}

/// A `from` to `to` transition of `machine`, triggered by its first
/// operation.
fn transition(machine: &str, from: &str, to: &str) -> serde_json::Value {
    let identity = id(&format!("{machine}/advance"));
    json!({
        "identity": identity,
        "from": id(&format!("{machine}/{from}")),
        "to": id(&format!("{machine}/{to}")),
        "trigger": id(&format!("{machine}/{}", if machine == "OrderLifecycle" { "place" } else { "expedite" })),
        "emits": [],
        "origin": origin(&identity),
    })
}

/// The ConfigVersion package plus the declarations described in the module
/// header. `touch_diamond` adds `A`, `B` and `C`, where `C` specializes `A`
/// and `B`, each declaring `touch`, and `C` declares none.
fn owners_document(touch_diamond: bool) -> Vec<u8> {
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&config_version_domain_document()).expect("the document is JSON");
    let boolean = "ix://quire/native/Boolean";
    let integer = "ix://quire/native/Integer";
    let constructs = [
        (
            "process",
            "quire.meaning.model.process/v1",
            "record",
            json!({"identityFields": "required", "steps": "required"}),
        ),
        (
            "state_machine",
            "quire.meaning.model.state-machine/v1",
            "record",
            json!({"states": "required", "transitions": "required"}),
        ),
        (
            "record_value_type",
            "quire.meaning.model.record-value-type/v1",
            "record",
            json!({}),
        ),
        (
            "event_type",
            "quire.meaning.model.event-type/v1",
            "record",
            json!({"occurrenceField": "required"}),
        ),
        (
            "persistence_interface",
            "quire.meaning.model.persistence-interface/v1",
            "record",
            json!({"persists": "required"}),
        ),
        (
            "variant_type",
            "quire.meaning.model.variant-type/v1",
            "enumeration",
            json!({"variants": "required"}),
        ),
    ];
    let declared = envelope["constructs"]
        .as_array_mut()
        .expect("constructs is an array");
    for (name, meaning, shape, members) in constructs {
        declared.push(construct(name, meaning, shape, members));
    }
    let machine = |name: &str, supertypes: Vec<String>, operations: Vec<serde_json::Value>| {
        // A specializing machine inherits `placed`; redeclaring it conflicts.
        let fields = if supertypes.is_empty() {
            vec![field(name, "placed", boolean, "required")]
        } else {
            Vec::new()
        };
        let (from, to) = if name == "OrderLifecycle" {
            ("Draft", "Placed")
        } else {
            ("Rush", "Sent")
        };
        declaration(
            name,
            "state_machine",
            json!({
                "fields": fields,
                "operations": operations,
                "clauses": [],
                "supertypes": supertypes,
                "states": [state(name, from), state(name, to)],
                "transitions": [transition(name, from, to)],
            }),
        )
    };
    let mut types = vec![
        declaration(
            "Flow",
            "process",
            json!({
                "identityFields": [id("Flow/x")],
                "fields": [
                    field("Flow", "x", boolean, "required"),
                    field("Flow", "n", integer, "required"),
                ],
                "operations": [operation_of("Flow", "go", &["n"])],
                "steps": [{
                    "identity": id("Flow/receive"),
                    "name": "receive",
                    "stepKind": "command",
                    "consumes": [],
                    "emits": [],
                    "origin": origin(&id("Flow/receive")),
                }],
            }),
        ),
        machine(
            "OrderLifecycle",
            vec![],
            vec![operation_of("OrderLifecycle", "place", &["placed"])],
        ),
        machine(
            "ExpressLifecycle",
            vec![id("OrderLifecycle")],
            vec![operation_of("ExpressLifecycle", "expedite", &[])],
        ),
        declaration(
            "Holder",
            "record_value_type",
            json!({
                "supertypes": [],
                "operations": [],
                "fields": [field("Holder", "flag", boolean, "required")],
            }),
        ),
        declaration(
            "Happened",
            "event_type",
            json!({
                "supertypes": [],
                "operations": [],
                "occurrenceField": id("Happened/at"),
                "fields": [field("Happened", "at", "ix://quire/native/Timestamp", "required")],
            }),
        ),
        declaration(
            "Color",
            "variant_type",
            json!({"variants": [{
                "identity": id("variant/Color-Red"),
                "name": "Red",
                "origin": {"source": {
                    "sourceIdentity": id("spec"), "path": "spec.qspec",
                    "startLine": 1, "startColumn": 3,
                }},
            }]}),
        ),
        declaration(
            "Store",
            "persistence_interface",
            json!({
                "fields": [],
                "operations": [operation_of("Store", "save", &[])],
                "persists": [id("ConfigVersion")],
            }),
        ),
    ];
    if touch_diamond {
        let object = |name: &str, supertypes: Vec<String>, operations: Vec<serde_json::Value>| {
            declaration(
                name,
                "object_type",
                json!({
                    "supertypes": supertypes,
                    "fields": [],
                    "operations": operations,
                }),
            )
        };
        let touch = |owner: &str| vec![operation_of(owner, "touch", &[])];
        types.push(object("A", vec![], touch("A")));
        types.push(object("B", vec![], touch("B")));
        types.push(object("C", vec![id("A"), id("B")], Vec::new()));
    }
    envelope["types"]
        .as_array_mut()
        .expect("types is an array")
        .extend(types);
    envelope.to_string().into_bytes()
}

/// The ConfigVersion unit over [`owners_document`], with `clauses` inserted
/// after `VersionUnchanged`.
fn owners_unit(document: &[u8], clauses: &str) -> String {
    let (unit, _) = config_version_unit_and_packages_for(document.to_vec());
    insert_clause_after(&unit, "VersionUnchanged", clauses)
}

const FLOW_CLAUSES: &str = "\
pre P using v on Config::Flow::go { self.n < 3 }\n\
post Q using v on Config::Flow::go { self.n = pre(self.n) + 1 }\n\
invariant I using v on Config::Flow at current { self.n >= 0 }";

const LIFECYCLE_CLAUSES: &str = "\
pre PlaceRequires using v on Config::OrderLifecycle::place { not self.placed }\n\
post FlagsPlaced using v on Config::OrderLifecycle::place { self.placed }";

fn selection(object: &str, operation: &str) -> OperationName {
    OperationName {
        model: identifier("Config"),
        object: identifier(object),
        operation: identifier(operation),
    }
}

fn locate(
    unit: &str,
    document: &[u8],
    selection: &OperationName,
) -> Result<CallSite<OperationSite>, Box<CallSiteRefusal>> {
    call_site(
        source(),
        "fixture".to_owned(),
        "call-site-owners.native",
        unit.as_bytes(),
        [document],
        &DependencyInput::default(),
        selection,
    )
}

fn frame_operation(object: &str, operation: &str) -> FrameOperation {
    FrameOperation {
        object: QualifiedName::new(vec![identifier("Config"), identifier(object)])
            .expect("two segments"),
        operation: identifier(operation),
    }
}

/// `refusal` settles `Declined` with cause `InvalidInput` and its own code.
fn assert_declined_with_code(refusal: &CallSiteRefusal, what: &str) {
    assert_eq!(
        TerminalValue::from_call_site_refusal(refusal),
        TerminalValue::Declined {
            cause: ProofRefusalCause::InvalidInput,
            code: DeclineCode::Qsl(refusal.code()),
        },
        "{what}"
    );
}

fn clause_names(site: &OperationSite) -> Vec<&str> {
    site.clauses
        .iter()
        .map(|clause| clause.name.as_str())
        .collect()
}

/// The `semantic_form` of the node `site.anchor` is the operation anchor of
/// (its `context`), in the graph compiled from `unit`.
fn anchor_context_form(unit: &str, document: &[u8], anchor: crate::NodeKey) -> String {
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let compiled = compile_config_version_unit(unit, &packages);
    let graph = compiled.package.graph();
    let node = graph
        .semantic_graph()
        .nodes()
        .find(|node| *node.key() == anchor)
        .expect("the anchor is a node of the compiled graph");
    assert_eq!(node.semantic_form(), "operation_anchor");
    let context = node.semantic_type().expect("an anchor has a context");
    graph
        .semantic_graph()
        .node(context)
        .expect("the context is a node of the compiled graph")
        .semantic_form()
        .to_owned()
}

/// The frame `site` names is the compiled graph's frame of the operation
/// that `owner` declares, at its `generated` occurrence.
fn assert_generated_frame(unit: &str, document: &[u8], site: &OperationSite, owner: &str) {
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let compiled = compile_config_version_unit(unit, &packages);
    let graph = compiled.package.graph();
    let frame = graph
        .semantic_graph()
        .nodes()
        .find(|node| *node.key() == site.frame)
        .expect("the frame is a node of the compiled graph");
    assert_eq!(frame.semantic_form(), "frame");
    let checked = graph
        .operation_frame_by_identity(frame.key())
        .expect("the compiled graph holds an operation frame for the node");
    assert_eq!(
        graph
            .declaring_object(&identifier("Config"), checked.operation().declaring)
            .map(quire_exact::Identifier::as_str),
        Some(owner)
    );
    assert_eq!(
        site.frame_occurrence,
        crate::OccurrenceKey::new(
            site.frame,
            crate::Origin::new(crate::Role::new("generated"), 0)
        )
    );
}

/// FR-121-AC-25: a process owner's operation selects its `operation_anchor`
/// (context the `Flow` `process` node), `frame`, and exactly `P` then `Q`;
/// an operation `Flow` does not declare is `UnknownOperation`.
#[trace("TC-1162", "FR-121-AC-25")]
#[test]
fn call_site_selects_a_process_operation() {
    let document = owners_document(false);
    let unit = owners_unit(&document, FLOW_CLAUSES);
    let site = locate(&unit, &document, &selection("Flow", "go")).expect("go is named by P and Q");
    assert_eq!(site.site.frame_operation, frame_operation("Flow", "go"));
    assert_eq!(clause_names(&site.site), ["P", "Q"]);
    assert_eq!(
        anchor_context_form(&unit, &document, site.site.anchor),
        "process"
    );
    assert_generated_frame(&unit, &document, &site.site, "Flow");

    let refusal =
        locate(&unit, &document, &selection("Flow", "stop")).expect_err("Flow declares no `stop`");
    assert!(
        matches!(*refusal, CallSiteRefusal::UnknownOperation { .. }),
        "{refusal:?}"
    );
    assert_eq!(refusal.code().as_str(), "missing_declaration");
}

/// FR-121-AC-26: a state machine's operation selects its anchor (context the
/// `OrderLifecycle` `state_machine` node), frame and clauses; the
/// specializing `ExpressLifecycle` selects the same sites through its
/// inherited `place`; a unit no clause of which names `place` refuses
/// `UnknownOperation`.
#[trace("TC-1162", "FR-121-AC-26")]
#[test]
fn call_site_selects_a_state_machine_operation_and_its_inherited_one() {
    let document = owners_document(false);
    let unit = owners_unit(&document, LIFECYCLE_CLAUSES);
    let declared = locate(&unit, &document, &selection("OrderLifecycle", "place"))
        .expect("place is named by two clauses");
    assert_eq!(
        declared.site.frame_operation,
        frame_operation("OrderLifecycle", "place")
    );
    assert_eq!(
        clause_names(&declared.site),
        ["PlaceRequires", "FlagsPlaced"]
    );
    assert_eq!(
        anchor_context_form(&unit, &document, declared.site.anchor),
        "state_machine"
    );
    assert_generated_frame(&unit, &document, &declared.site, "OrderLifecycle");

    let inherited = locate(&unit, &document, &selection("ExpressLifecycle", "place"))
        .expect("place is inherited by ExpressLifecycle");
    assert_eq!(
        inherited.site.frame_operation,
        declared.site.frame_operation
    );
    assert_eq!(inherited.site.anchor, declared.site.anchor);
    assert_eq!(inherited.site.frame, declared.site.frame);
    assert_eq!(
        inherited.site.frame_occurrence,
        declared.site.frame_occurrence
    );

    let unnamed = owners_unit(&document, "");
    let refusal = locate(&unnamed, &document, &selection("OrderLifecycle", "place"))
        .expect_err("no clause of the unit names place");
    assert!(
        matches!(*refusal, CallSiteRefusal::UnknownOperation { .. }),
        "{refusal:?}"
    );
}

/// FR-121-AC-27: an owner that is no object type, process or state machine
/// refuses `NotAnOperationOwner`; `C` inheriting `touch` from `A` and `B`
/// refuses `AmbiguousOperation` although no frame exists for it. Each
/// pairs the selection with the package and maps to `Declined`.
#[trace("TC-1162", "FR-121-AC-27")]
#[test]
fn call_site_refuses_a_non_owner_and_an_ambiguous_operation() {
    let document = owners_document(true);
    let unit = owners_unit(
        &document,
        "post TouchA using v on Config::A::touch { true }\n\
         post TouchB using v on Config::B::touch { true }",
    );
    let package = qsl_semantics::library::LibraryName::new("fixture").unwrap();
    let owners = [
        ("Holder", "record value type"),
        ("Happened", "event type"),
        ("Color", "enum (variant type)"),
        ("VersionNumber", "scalar type"),
        ("Store", "persistence interface"),
        ("config_history", "population"),
    ];
    for (owner, kind) in owners {
        let wanted = selection(owner, "save");
        let refusal = locate(&unit, &document, &wanted).expect_err(kind);
        match &*refusal {
            CallSiteRefusal::NotAnOperationOwner {
                selection: refused,
                package: refused_in,
            } => {
                assert_eq!(refused, &wanted, "{kind}");
                assert_eq!(*refused_in, package, "{kind}");
            }
            other => panic!("{kind}: expected NotAnOperationOwner, got {other:?}"),
        }
        assert_eq!(refusal.code().as_str(), "invalid_model_binding", "{kind}");
        assert!(
            refusal
                .to_string()
                .starts_with("invalid_model_binding/malformed-declaration"),
            "{kind}: {refusal}"
        );
        assert_declined_with_code(&refusal, kind);
    }

    let wanted = selection("C", "touch");
    let refusal = locate(&unit, &document, &wanted).expect_err("touch is ambiguous on C");
    match &*refusal {
        CallSiteRefusal::AmbiguousOperation {
            selection: refused,
            package: refused_in,
        } => {
            assert_eq!(refused, &wanted);
            assert_eq!(*refused_in, package);
        }
        other => panic!("expected AmbiguousOperation, got {other:?}"),
    }
    assert_eq!(refusal.code().as_str(), "ambiguous_declaration");
    assert!(
        refusal
            .to_string()
            .starts_with("ambiguous_declaration/ambiguous-name"),
        "{refusal}"
    );
    assert_declined_with_code(&refusal, "ambiguous touch");
}

/// FR-115-AC-6 over a state machine, and FR-116's target resolution: the
/// `Config::OrderLifecycle::place` selection resolves to the state machine
/// (never an object type), and the `frame_operation` `call_site` returns
/// resolves back to the same frame for a process and a state machine alike.
/// A target naming `ExpressLifecycle`, which only inherits `place`, is
/// refused as an object that is not the declaring object.
#[trace("TC-514", "FR-115-AC-6")]
#[test]
fn a_call_site_frame_operation_resolves_as_a_replay_frame_target() {
    use crate::execute::frame::resolve_target;
    use crate::{MalformedOperationReason, ReplayRefusal};

    let document = owners_document(false);
    let unit = owners_unit(&document, &format!("{FLOW_CLAUSES}\n{LIFECYCLE_CLAUSES}"));
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let compiled = compile_config_version_unit(&unit, &packages);
    let graph = compiled.package.graph();
    let package_name = qsl_semantics::library::LibraryName::new("fixture").unwrap();

    for (owner, name) in [("Flow", "go"), ("OrderLifecycle", "place")] {
        let site = locate(&unit, &document, &selection(owner, name))
            .unwrap_or_else(|refusal| panic!("{owner}::{name}: {refusal:?}"));
        let (context, frame) = resolve_target(graph, &site.site.frame_operation, &package_name)
            .unwrap_or_else(|refusal| panic!("{owner}::{name}: {refusal:?}"));
        assert_eq!(frame.frame().clone(), site.site.frame);
        assert_eq!(frame.operation().declaring, context);
        assert_eq!(
            graph
                .declaring_object(&identifier("Config"), context)
                .map(quire_exact::Identifier::as_str),
            Some(owner)
        );
    }

    let inherited = frame_operation("ExpressLifecycle", "place");
    let Err(refusal) = resolve_target(graph, &inherited, &package_name) else {
        panic!("ExpressLifecycle only inherits place");
    };
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::MalformedOperationTarget {
                reason: MalformedOperationReason::OnlyInherited,
                ..
            }
        ),
        "{refusal:?}"
    );
}

/// FR-121: a field selection is for object types only. `Config::Flow.n` and
/// `Config::OrderLifecycle.placed` name fields of a process and a state
/// machine, and refuse `UnknownField`, as they did before processes and state
/// machines became operation owners.
#[trace("TC-516", "FR-121-AC-18")]
#[test]
fn a_field_of_a_process_or_state_machine_is_not_selectable() {
    let document = owners_document(false);
    let unit = owners_unit(&document, FLOW_CLAUSES);
    for (owner, field) in [("Flow", "n"), ("OrderLifecycle", "placed")] {
        let wanted = crate::FieldName {
            model: identifier("Config"),
            object: identifier(owner),
            field: identifier(field),
        };
        let refusal = call_site(
            source(),
            "fixture".to_owned(),
            "call-site-owners.native",
            unit.as_bytes(),
            [document.as_slice()],
            &DependencyInput::default(),
            &wanted,
        )
        .expect_err("fields are selected on object types only");
        assert!(
            matches!(*refusal, CallSiteRefusal::UnknownField { .. }),
            "{owner}.{field}: {refusal:?}"
        );
    }
}
