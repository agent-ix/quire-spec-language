// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-363-AC-6 (TC-1032): a checked `Navigate` node lowers to one
//! `quire.op.model.navigate` application naming the relationship's model node
//! and the end's role, over the receiver, with no `deref` and no field
//! projection; IR's v2 reader admits the emitted package.

use super::*;

fn multiplicity(lower: u64, upper: u64) -> Value {
    json!({"lower": lower, "upper": upper, "ordered": false, "unique": true})
}

/// A navigation relationship `owner/role` of fixture N: declared by the source
/// type, `source-to-target` unless `direction` says otherwise.
fn relationship(
    id: &str,
    direction: &str,
    source: (&str, &str, Value),
    target: (&str, Option<&str>, Value),
) -> Value {
    let mut target_end = json!({
        "type": format!("ix://acme/orders/{}", target.0),
        "multiplicity": target.2,
    });
    if let Some(role) = target.1 {
        target_end["role"] = json!(role);
    }
    json!({
        "identity": format!("ix://acme/orders/relationship/{id}"),
        "category": "structural",
        "composite": false,
        "direction": direction,
        "origin": {"source": {
            "sourceIdentity": "ix://acme/orders/spec",
            "path": "spec.qspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "sourceEnd": {
            "type": format!("ix://acme/orders/{}", source.0),
            "role": source.1,
            "multiplicity": source.2,
        },
        "targetEnd": target_end,
    })
}

/// The `acme/orders` document with the object types `Fulfilment`, `Order` and
/// `Line` and fixture N's relationships `Fulfilment/order` and `Order/lines`.
fn navigation_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(SPINE_MODEL_DOCUMENT).unwrap();
    let template = document["types"][0].clone();
    let object_type = |name: &str, relationships: Vec<Value>| {
        let mut declared = template.clone();
        declared["identity"] = json!(format!("ix://acme/orders/{name}"));
        declared["displayName"] = json!(name);
        declared["supertypes"] = json!([]);
        declared["fields"] = json!([]);
        declared["operations"] = json!([]);
        declared["relationships"] = json!(relationships);
        declared
    };
    document["types"] = json!([
        object_type(
            "Fulfilment",
            vec![relationship(
                "Fulfilment-order",
                "source-to-target",
                (
                    "Fulfilment",
                    "order",
                    json!({"lower": 0, "ordered": false, "unique": true})
                ),
                ("Order", None, multiplicity(0, 1)),
            )]
        ),
        object_type(
            "Order",
            vec![relationship(
                "Order-lines",
                "bidirectional",
                ("Order", "lines", multiplicity(1, 1)),
                ("Line", Some("owner"), multiplicity(0, 3)),
            )]
        ),
        object_type("Line", vec![]),
    ]);
    serde_json::to_vec(&document).unwrap()
}

const FUNCTIONS: &str = "\
function f1 using v(r: Reference<M::Fulfilment>): Option<Reference<M::Order>> pure { r.order }
function f2 using v(o: Reference<M::Order>): Set<Reference<M::Line>>[0, 3] pure { o.lines }
function f3 using v(l: Reference<M::Line>): Reference<M::Order> pure { l.owner }
";

/// The applications of `identity` in `wire`.
fn applications<'w>(wire: &'w Value, identity: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == identity)
        .collect()
}

/// FR-363-AC-6, TC-1032: `f1`, `f2` and `f3` each lower to one `query`
/// application of `quire.op.model.navigate` with no laws, mode or leaves, a
/// `relationship_end` member naming the relationship's model node and the
/// near end's role, and the one reference argument; the `result_type` is the
/// function's declared result type; no `deref` or `record.project` node is
/// emitted; and IR's v2 reader admits the package.
#[trace("TC-1032", "FR-363-AC-6")]
#[test]
fn a_navigation_lowers_to_one_navigate_application_over_the_reference() {
    let document = navigation_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{digest}\";\n\
         {FUNCTIONS}"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the three navigations check");
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);

    let navigations = applications(&wire, "quire.op.model.navigate");
    assert_eq!(navigations.len(), 3, "one application per navigation");
    assert_eq!(
        applications(&wire, "quire.op.model.deref"),
        Vec::<&Value>::new()
    );
    assert_eq!(
        applications(&wire, "quire.op.record.project"),
        Vec::<&Value>::new()
    );

    let relation = |owner: &str| {
        let found: Vec<&Value> = nodes(&wire)
            .iter()
            .filter(|node| {
                node["node_tag"] == "relation"
                    && node["owner"]["node"] == format!("ix://acme/orders/relationship/{owner}")
            })
            .collect();
        let [relation] = found.as_slice() else {
            panic!("one relation node for {owner}");
        };
        relation["node_id"]["digest"].clone()
    };
    let result_form = |navigation: &Value| {
        let wanted = &navigation["body"]["result_type"]["digest"];
        nodes(&wire)
            .iter()
            .find(|node| &node["node_id"]["digest"] == wanted)
            .map(|node| node["semantic_form"].clone())
            .expect("the result type is a node of the package")
    };
    let expected = [
        ("order", "Fulfilment-order", "option"),
        ("lines", "Order-lines", "collection_bounds"),
        ("owner", "Order-lines", "reference"),
    ];
    for (role, owner, form) in expected {
        let found: Vec<&&Value> = navigations
            .iter()
            .filter(|node| node["body"]["operation"]["member"]["name"] == role)
            .collect();
        let [navigation] = found.as_slice() else {
            panic!("one navigation of {role}");
        };
        let body = &navigation["body"];
        let operation = &body["operation"];
        assert_eq!(body["operator"], "query", "{role}");
        assert_eq!(operation["laws"], json!([]), "{role}");
        assert_eq!(operation["leaves"], json!([]), "{role}");
        assert_eq!(operation["mode"], Value::Null, "{role}");
        assert_eq!(operation["member"]["kind"], "relationship_end", "{role}");
        assert_eq!(
            operation["member"]["declaration"]["digest"],
            relation(owner),
            "{role}: the relationship's model node"
        );
        let arguments = body["arguments"].as_array().unwrap();
        assert_eq!(arguments.len(), 1, "{role}: the receiver only");
        assert_eq!(arguments[0]["term"], "reference", "{role}");
        assert_eq!(result_form(navigation), form, "{role}: result_type");
    }

    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(digest.as_str(), &document[..])
        .expect("a fresh evidence key");
    assert!(
        matches!(read_with(&emission, &evidence), Read::Verified { .. }),
        "IR's v2 reader admits the package"
    );
}
