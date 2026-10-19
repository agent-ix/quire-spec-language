// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 (TC-565, TC-566): a population declared `symmetric` is carried
//! into the checked package, and S3 refuses the forms that observe the
//! identity of its objects, over TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).
//!
//! Not covered here, because S3 has no such form yet: a reference literal
//! naming a key (FR-150-AC-3), a state constraint (FR-158), and a temporal
//! atom's condition, which S3 does not type (`temporal_fairness`).

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::{IntakeLimits, Span};
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, IdentityForm, IdentityNode, IdentityObserving,
};
use qsl_semantics::model::domain_package::DomainPackageRecord;
use qsl_semantics::model::intake::{read_records, PackageDocument};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, attempt_update_modifies_version_number, config_unit_with_body,
    config_version_document, PACKAGE_IDENTITY,
};

const POPULATION: &str = "ix://example/config-version/config_history";
const CONFIG_VERSION: &str = "ix://example/config-version/ConfigVersion";

const PARENT_ORDER: &str = "invariant ParentOrder using v on Config::ConfigVersion at current { \
    present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }\n";
const NO_CYCLE: &str = "invariant NoCycle using v on Config::ConfigVersion at current { \
    not reaches(self, self, parent) }\n";
const VERSION_UNCHANGED: &str = "post VersionUnchanged using v on \
    Config::ConfigVersion::attemptUpdate { self.versionNumber = pre(self.versionNumber) }\n";

/// An order-dependent traversal of the unordered `peers`: the last element
/// visited decides the result.
const LAST_PEER: &str = "fold<Flag>(acc, p in self.peers: p = self, identity: false)";
const FLAG: &str = "type Flag = Boolean;\ntype Tally = Integer;\n";

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

/// `document` with `config_history` declared `symmetric`.
fn annotated(document: &[u8]) -> Vec<u8> {
    let mut envelope: serde_json::Value = serde_json::from_slice(document).expect("valid JSON");
    for population in envelope["populations"].as_array_mut().expect("populations") {
        if population["identity"] == POPULATION {
            population["symmetric"] = json!(true);
        }
    }
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// `document` with `ConfigVersion.peers`, an unordered set of `ConfigVersion`.
fn with_peers(document: &[u8]) -> Vec<u8> {
    let mut envelope: serde_json::Value = serde_json::from_slice(document).expect("valid JSON");
    let peers = format!("{CONFIG_VERSION}/peers");
    envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|entry| entry["identity"] == CONFIG_VERSION)
        .expect("ConfigVersion is declared")["fields"]
        .as_array_mut()
        .expect("ConfigVersion declares fields")
        .push(json!({
            "identity": peers,
            "name": "peers",
            "typeRef": CONFIG_VERSION,
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 0, "upper": 5, "ordered": false, "unique": true},
            "origin": {
                "generated": {
                    "generatorIdentity": peers,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [peers],
                }
            },
        }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

fn check(document: &[u8], body: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    admit_and_assemble_with_body(document, body)
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"))
        .with_unwritten_terminals()
        .check(CheckingLimits::default())
}

fn span_of(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// The one refusal of `body` over `document`: its payload, with its region
/// asserted to be exactly the first occurrence of `needle` in the unit.
fn identity_refusal(document: &[u8], body: &str, needle: &str) -> IdentityObserving {
    let refusals = check(document, body).expect_err("the unit refuses");
    let [refusal] = &refusals[..] else {
        panic!("{body}: one refusal, not {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), Code::IllTyped);
    assert_eq!(
        refusal.cause.cause(),
        Some("identity-observing-form"),
        "{body}"
    );
    let regions = admit_and_assemble_with_body(document, body)
        .expect("the unit assembles")
        .regions();
    let (text, _) = config_unit_with_body(document, body);
    let region = regions.refusal_region(refusal).expect("a region");
    let node = span_of(&text, needle);
    assert_eq!(
        (region.start(), region.end()),
        (node.start as u64, node.end as u64),
        "the refusal is at the refused node"
    );
    match &refusal.cause {
        CheckCause::IdentityObservingForm(payload) => payload.as_ref().clone(),
        other => panic!("{body}: an identity-observing-form cause, not {other:?}"),
    }
}

fn causes(refusals: &[CheckRefusal]) -> Vec<(Code, Option<&'static str>)> {
    refusals
        .iter()
        .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
        .collect()
}

/// An annotated population read only by dereference, equality and `reaches`
/// checks, carries `symmetric: true` into its record, and the annotation
/// changes the package selection the unit checks against.
#[trace("TC-565", "FR-150-AC-1")]
#[test]
fn an_annotated_population_checks_and_changes_the_package_selection() {
    let body = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}");
    let plain = check(&document(), &body).expect("the unannotated unit checks");
    let marked = check(&annotated(&document()), &body).expect("the annotated unit checks");
    assert_ne!(
        plain.model_selections(),
        marked.model_selections(),
        "a package differing only by the annotation is a different package"
    );

    for (bytes, symmetric) in [(document(), false), (annotated(&document()), true)] {
        let package = PackageDocument::parse(&bytes, IntakeLimits::default()).expect("parses");
        let records = read_records(PACKAGE_IDENTITY, &package).expect("reads");
        let flags: Vec<bool> = records
            .iter()
            .filter_map(|record| match record {
                DomainPackageRecord::Population(population)
                    if population.key.node == POPULATION =>
                {
                    Some(population.symmetric)
                }
                _ => None,
            })
            .collect();
        assert_eq!(flags, [symmetric]);
    }
}

/// An order-dependent `fold` over a set of references into an annotated
/// population refuses `identity-observing-form` at the `fold`, naming the
/// form and the population; unannotated it keeps today's FR-145 refusal
/// (`operator-ineligible`), so this does not verify the AC's "unannotated
/// checks" half.
#[trace("TC-565")]
#[test]
fn a_fold_over_references_into_an_annotated_population_refuses() {
    let body = format!(
        "{FLAG}invariant Last using v on Config::ConfigVersion at current {{ not {LAST_PEER} }}\n"
    );
    let payload = identity_refusal(&annotated(&with_peers(&document())), &body, LAST_PEER);
    assert_eq!(payload.node, IdentityNode::Fold);
    assert_eq!(payload.form, IdentityForm::OrderSensitiveTraversal);
    assert_eq!(payload.population.node, POPULATION);

    let unannotated = check(&with_peers(&document()), &body).expect_err("refuses as today");
    assert_eq!(
        causes(&unannotated),
        [(Code::IllTyped, Some("operator-ineligible"))]
    );
}

/// `reduce`, and a conversion of a reference, refuse the same way.
#[trace("TC-566", "FR-150-AC-3")]
#[test]
fn a_reduce_and_a_reference_conversion_refuse() {
    let document = annotated(&with_peers(&document()));
    let reduce = "reduce<Peer>(acc, p in self.peers: acc)";
    let body = format!(
        "type Peer = Config::ConfigVersion;\ninvariant Last using v on Config::ConfigVersion at current {{ self = {reduce} }}\n"
    );
    let payload = identity_refusal(&document, &body, reduce);
    assert_eq!(payload.node, IdentityNode::Reduce);

    let convert = "convert<String>(self)";
    let body = format!(
        "invariant Rendered using v on Config::ConfigVersion at current {{ {convert} = \"a\" }}\n"
    );
    let payload = identity_refusal(&document, &body, convert);
    assert_eq!(payload.node, IdentityNode::Convert);
    assert_eq!(payload.form, IdentityForm::ReferenceConversion);

    let body = format!(
        "invariant Rendered using v on Config::ConfigVersion at current {{ \
         let x = {convert} in x = \"a\" }}\n"
    );
    let payload = identity_refusal(&document, &body, convert);
    assert_eq!(payload.node, IdentityNode::Convert);
}

/// The refusal reaches a `terminal when` predicate, and two refused forms of
/// one unit are both reported, in source order.
#[trace("TC-566", "FR-150-AC-3")]
#[test]
fn a_terminal_predicate_refuses_and_two_refusals_report_in_source_order() {
    let document = annotated(&with_peers(&document()));
    let terminal = format!("{FLAG}terminal Done using v on Config when {{ {LAST_PEER} }};\n");
    let refusals = check(&document, &terminal).expect_err("the terminal refuses");
    assert_eq!(
        causes(&refusals),
        [(Code::IllTyped, Some("identity-observing-form"))]
    );

    let both = format!(
        "{FLAG}invariant A using v on Config::ConfigVersion at current {{ not {LAST_PEER} }}\n\
         invariant B using v on Config::ConfigVersion at current {{ convert<String>(self) = \"a\" }}\n"
    );
    let refusals = check(&document, &both).expect_err("both refuse");
    assert_eq!(
        causes(&refusals),
        [
            (Code::IllTyped, Some("identity-observing-form")),
            (Code::IllTyped, Some("identity-observing-form")),
        ]
    );
    let regions = admit_and_assemble_with_body(&document, &both)
        .expect("assembles")
        .regions();
    let starts: Vec<u64> = refusals
        .iter()
        .map(|refusal| regions.refusal_region(refusal).expect("a region").start())
        .collect();
    assert!(starts[0] < starts[1], "source order: {starts:?}");
}

/// Dereference, `reaches`, `size`, `contains` and `count` over an annotated
/// population's references still check. A `fold` over a `Set<Int[0, 3]>`,
/// `forall` and `holds` are not exercised.
#[trace("TC-566")]
#[test]
fn transparent_uses_of_an_annotated_population_check() {
    let document = annotated(&with_peers(&document()));
    let body = format!(
        "{FLAG}{PARENT_ORDER}{NO_CYCLE}\
         invariant Peers using v on Config::ConfigVersion at current {{ \
           size(self.peers) <= 5 and contains(self.peers, self) }}\n\
         invariant Count using v on Config::ConfigVersion at current {{ \
           count<Tally>(p in self.peers: p = self) = 0 }}\n"
    );
    check(&document, &body).expect("transparent forms check");
}

/// `convert<Sequence<..>>` of a set of symmetric references reads the set in
/// identity order, so it refuses as an order-sensitive traversal.
#[trace("TC-566", "FR-150-AC-3")]
#[test]
fn a_set_converted_to_a_sequence_refuses() {
    let document = annotated(&with_peers(&document()));
    let convert = "convert<Sequence<Config::ConfigVersion>[0, 5]>(self.peers)";
    let body = format!(
        "invariant Order using v on Config::ConfigVersion at current {{ size({convert}) = 0 }}\n"
    );
    let payload = identity_refusal(&document, &body, convert);
    assert_eq!(payload.node, IdentityNode::Convert);
    assert_eq!(payload.form, IdentityForm::OrderSensitiveTraversal);
}

/// A reference typed as a supertype of a symmetric population's member type
/// is a reference into that population.
#[trace("TC-566", "FR-150-AC-3")]
#[test]
fn a_supertype_reference_into_a_symmetric_subtype_population_refuses() {
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&annotated(&with_peers(&document()))).expect("valid JSON");
    let sub = "ix://example/config-version/Sub";
    envelope["populations"][0]["members"] = json!([sub]);
    let document = serde_json::to_vec(&envelope).expect("valid JSON");
    let document = crate::model_operations::add_sub_type(document, &[]);
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    envelope["populations"][0]["members"] = json!([sub]);
    let document = serde_json::to_vec(&envelope).expect("valid JSON");
    let body =
        format!("{FLAG}invariant Last using v on Config::Sub at current {{ not {LAST_PEER} }}\n");
    let payload = identity_refusal(&document, &body, LAST_PEER);
    assert_eq!(payload.node, IdentityNode::Fold);
    assert_eq!(payload.population.node, POPULATION);
}

/// An identity conversion of a reference in an equality operand is not a
/// conversion to another type, so it checks.
#[trace("TC-566", "FR-150-AC-4")]
#[test]
fn an_identity_conversion_in_an_equality_operand_checks() {
    let document = annotated(&with_peers(&document()));
    let body = "invariant Same using v on Config::ConfigVersion at current { \
                convert<Config::ConfigVersion>(self) = self }\n";
    check(&document, body).expect("an identity conversion checks");
}

/// A non-Boolean `symmetric` refuses as a malformed declaration, and another
/// extra population member is still refused.
#[trace("TC-565")]
#[test]
fn a_malformed_marker_and_another_extra_member_refuse() {
    for (member, value) in [("symmetric", json!("yes")), ("unknown", json!(true))] {
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&document()).expect("valid JSON");
        envelope["populations"][0][member] = value;
        let bytes = serde_json::to_vec(&envelope).expect("valid JSON");
        let package = PackageDocument::parse(&bytes, IntakeLimits::default()).expect("parses");
        let refusals =
            read_records(PACKAGE_IDENTITY, &package).expect_err(&format!("{member} refuses"));
        assert!(
            format!("{refusals:?}").contains(member),
            "{member}: {refusals:?}"
        );
    }
}
