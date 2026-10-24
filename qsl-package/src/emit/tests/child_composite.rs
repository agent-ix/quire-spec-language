// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-359-AC-3, FR-359-AC-8 (TC-998 step 5, TC-1001 step 1): the emitted
//! package of a unit passing a child record at an ancestor-typed parameter
//! carries each record as a model-owned `record_value_type` node, reads a
//! field by its identity, and adds no conversion node between the call and
//! its argument. The read-back halves depend on IR's reader accepting a
//! conforming record operand (quire-contract-ir FR-347).

use super::owners::node_where;
use super::*;

#[path = "../../../../qsl-semantics/tests/support/child_composite.rs"]
mod fixture;

/// `unit_body` as model `M` over the FR-359 package, through the spine to the
/// v2 emitter with nothing omitted, and a read of the bytes with the package
/// document as evidence.
fn emit_unit(unit_body: &str) -> (Emission, Value, Read) {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {unit_body}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(digest.as_str(), document.as_slice())
        .expect("a fresh evidence key");
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    );
    (emission, wire, read)
}

const UNIT: &str = "\
function occurredAt using v(e: M::OrderPlaced): Integer pure { e.occurred_at }\n\
function fromExpress using v(e: M::ExpressOrderPlaced): Integer pure { occurredAt(e) }\n\
function fromRush using v(e: M::RushOrderPlaced): Integer pure { occurredAt(e) }\n";

/// FR-359-AC-3: each record value type is one `model`/`record_value_type`
/// node with a `ModelOwner` naming the declaring package and the record's
/// node, none is a `composite_type`, no node has a `supertypes` member, and
/// the call to `occurredAt` takes its argument by `ref` with no conversion
/// between them.
#[trace("FR-359-AC-3", "TC-998")]
#[test]
fn records_are_model_owned_nodes_and_the_call_takes_its_argument_by_ref() {
    let (_, wire, _) = emit_unit(UNIT);
    for name in ["OrderPlaced", "ExpressOrderPlaced", "RushOrderPlaced"] {
        let owned: Vec<&Value> = node_where(&wire, "model", "record_value_type")
            .into_iter()
            .filter(|node| node["owner"]["node"] == fixture::id(name))
            .collect();
        assert_eq!(owned.len(), 1, "{name} is one record_value_type node");
        assert_eq!(
            owned[0]["owner"],
            json!({"kind": "model", "identity": fixture::PACKAGE, "node": fixture::id(name)})
        );
    }
    assert_eq!(
        node_where(&wire, "composite_type", "record").len(),
        0,
        "no record of the package is a composite_type"
    );
    let text = wire.to_string();
    assert!(
        !text.contains("supertypes"),
        "no node has a supertypes member"
    );
    let calls: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.function.call")
        .collect();
    assert_eq!(calls.len(), 2, "fromExpress and fromRush each call once");
    for call in calls {
        let arguments = call["body"]["arguments"].as_array().unwrap();
        assert_eq!(arguments.len(), 2, "the callee and one argument");
        assert_eq!(
            arguments[1]["term"], "reference",
            "the argument is taken by ref, with no node between"
        );
    }
    let projects = nodes(&wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.record.project")
        .count();
    assert_eq!(projects, 1, "occurredAt reads its field once");
}

/// FR-359-AC-3, FR-359-AC-8: the package reads back `Verified` through IR's
/// reader (quire-contract-ir FR-347, a conforming record at an operation
/// argument, and QSpec FR-322's `record_value_type` owner).
#[trace("FR-359-AC-3", "FR-359-AC-8", "TC-998", "TC-1001")]
#[test]
fn a_child_record_argument_reads_back_verified() {
    let (emission, _, read) = emit_unit(UNIT);
    match read {
        Read::Verified { package, .. } => {
            assert_eq!(package.package_id(), emission.package.package_id());
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// FR-359-AC-8 over the operation path: a unit with a precondition and a
/// postcondition over `Ledger.pay(m: Money): Money` and a function taking and
/// returning the same record emits with nothing omitted and reads back
/// `Verified`; the operation's parameter and result are the `Money` node, not
/// a conversion of it.
#[trace("FR-359-AC-8", "TC-1001")]
#[test]
fn an_operation_over_a_record_parameter_emits_and_reads_back() {
    let (emission, wire, read) = emit_unit(
        "function keep using v(m: M::Money): M::Money pure { m }\n\
         function passTaxed using v(m: M::TaxedMoney): M::Money pure { keep(m) }\n\
         pre PayHolds using v on M::Ledger::pay { m.amount_minor > 0 }\n\
         post PayReturns using v on M::Ledger::pay { result.amount_minor = m.amount_minor }",
    );
    assert!(
        node_where(&wire, "model", "record_value_type")
            .iter()
            .any(|node| node["owner"]["node"] == fixture::id("Money")),
        "the operation's Money is the model-owned record node"
    );
    assert!(
        !wire.to_string().contains("convert"),
        "no conversion node between an argument and its parameter"
    );
    match read {
        Read::Verified { package, .. } => {
            assert_eq!(package.package_id(), emission.package.package_id());
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}
