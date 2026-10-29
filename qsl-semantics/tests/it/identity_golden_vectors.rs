// SPDX-License-Identifier: AGPL-3.0-or-later
//! Golden vectors for the RFC 8785 identity domains this crate
//! mints through `quire-canonical` (ADR-013 §2, ADR-013:113: one RFC 8785
//! implementation produces every RFC 8785 encoding).
//!
//! Each vector is the canonical text itself, written out by hand. Its digest
//! is never stored: it is recomputed from the text with `sha2` directly, a
//! path independent of `quire-canonical`, and a digest one text embeds (an
//! owner, a node reference, the model selection) is spliced in from the text
//! it names. A vector checks three things: the identity the crate mints over
//! the preimage equals the independent SHA-256 of the hand-written text; the
//! texts keep their domains apart (no two vectors share a digest); and the `serde_json`
//! `Value` encoder these sites used before `quire-canonical` emitted exactly the same
//! text, so moving to `quire-canonical` left the digest unchanged.
//!
//! The one digest the move does change is shown here too: a domain-package
//! document carrying a non-integral spelling of an integral number
//! (`1.0`, `1e2`). `serde_json` printed those `1.0`/`100.0`, which RFC 8785
//! spells `1`/`100` (ECMAScript `Number::toString`), so the old digest was
//! not a `sha256-jcs` digest at all.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use serde_json::Value;
use sha2::{Digest, Sha256};

use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
};
use qsl_semantics::model::intake::admit;
use qsl_semantics::model::key::{
    EffectiveDeclarationPreimage, EffectiveId, Fact, RULE_QUALIFY, SHA256_JCS_DIGEST_DOMAIN,
};
use qsl_semantics::model::normalize::{normalize, NormalizeOutcome, ObjectUniverse};
use qsl_semantics::model::normalize::{ModelRefusal, ModelRefusalCause};
use qsl_semantics::value::enumeration::{EnumDeclarationPreimage, EnumMemberPreimage};
use qsl_semantics::value::{
    admit_unit_graph, CompoundUnitPreimage, DimensionPreimage, NodeIdentityPreimage, NodeOwner,
    OwnerSelection, OwnerSubject, UnitPreimage,
};
use quire_checked::key::DeclarationKey;
use quire_exact::NodeKey;
use quire_semantic_value::quantity::{result_unit, QuantityUnit, UnitOperation};

const EFFECTIVE_TYPE: &str = r#"{"derivation":[{"inputs":[{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"}],"ordinal":"0","parent":null,"rule":{"identity":"quire.model.normalize.qualify/v1"}}],"original":{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"},"owner_effective_type":null,"version":"quire.model.effective-declaration/v2"}"#;

const EFFECTIVE_MEMBER_TEMPLATE: &str = r#"{"derivation":[{"inputs":[{"digest_domain":"sha256-jcs","node":"ix://test/orders/A/x","package":"test/orders"}],"ordinal":"0","parent":null,"rule":{"identity":"quire.model.normalize.qualify/v1"}}],"original":{"digest_domain":"sha256-jcs","node":"ix://test/orders/A/x","package":"test/orders"},"owner_effective_type":{"digest":"@TYPE@","domain":"quire.model.effective-declaration/v2"},"version":"quire.model.effective-declaration/v2"}"#;

const UNIVERSE_TEMPLATE: &str = r#"{"model_selection":{"digest":"@SEL@","digest_domain":"sha256-jcs","identity":"test/orders"},"root_types":[{"digest":"@TYPE@","domain":"quire.model.effective-declaration/v2"}],"version":"quire.model.object-universe/v2"}"#;

const VIEW_TEMPLATE: &str = r#"{"declarations":[{"effective_id":{"digest":"@TYPE@","domain":"quire.model.effective-declaration/v2"},"preimage":{"derivation":[{"inputs":[{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"}],"ordinal":"0","parent":null,"rule":{"identity":"quire.model.normalize.qualify/v1"}}],"original":{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"},"owner_effective_type":null,"version":"quire.model.effective-declaration/v2"}}],"model_selection":{"digest":"@SEL@","digest_domain":"sha256-jcs","identity":"test/orders"},"rules":{"identity":"quire.model.complete.rules/v1"},"version":"quire.model.effective-view/v2"}"#;

const ENUM_DECLARATION: &str = r#"{"members":["Active","Closed"],"ordered":false,"owner":{"authority":"agent-ix","identity":"example-model","kind":"definition"},"qualified_declaration":["Example","Status"],"version":"quire.enum-declaration-node/v1"}"#;

const ENUM_MEMBER_TEMPLATE: &str = r#"{"case":"Active","declaration_node_id":{"digest":"@DECL@","domain":"quire.checked-semantic-node/v1"},"version":"quire.enum-member-node/v1"}"#;

const DIMENSION: &str = r#"{"owner":{"authority":"agent-ix","identity":"example-model","kind":"definition"},"qualified_declaration":["Example","Length"],"terms":[],"version":"quire.dimension-node/v1"}"#;

const UNIT_TEMPLATE: &str = r#"{"dimension_node_id":{"digest":"@DIM@","domain":"quire.checked-semantic-node/v1"},"offset":{"denominator":"1","numerator":"0"},"owner":{"authority":"agent-ix","identity":"example-model","kind":"definition"},"qualified_declaration":["Example","Metre"],"scale":{"denominator":"1","numerator":"1"},"target_unit_node_id":null,"version":"quire.unit-node/v1"}"#;

const COMPOUND_UNIT_TEMPLATE: &str = r#"{"terms":[{"exponent":"2","unit_node_id":{"digest":"@UNIT@","domain":"quire.checked-semantic-node/v1"}}],"version":"quire.value.compound-unit/v1"}"#;

/// A domain-package document as a producer might write it: whitespace,
/// members out of order, an escaped `é`.
const DOCUMENT_RAW: &str = r#"{ "package": { "version": "1", "identity": "test/golden" }, "types": [], "note": "\u00e9" }"#;
const DOCUMENT: &str =
    r#"{"note":"é","package":{"identity":"test/golden","version":"1"},"types":[]}"#;

/// A document with integral numbers spelled `1.0` and `1e2`: its RFC 8785
/// text, and the text `serde_json` printed for it before `quire-canonical`.
const FLOAT_DOCUMENT_RAW: &str =
    r#"{"package":{"identity":"test/golden","version":"1"},"n":[1.0,1e2,0.5]}"#;
const FLOAT_DOCUMENT: &str =
    r#"{"n":[1,100,0.5],"package":{"identity":"test/golden","version":"1"}}"#;
const FLOAT_DOCUMENT_BEFORE_QSL_194: &str =
    r#"{"n":[1.0,100.0,0.5],"package":{"identity":"test/golden","version":"1"}}"#;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The SHA-256 of a vector's hand-written text: the expected digest.
fn digest_of(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

fn effective_type_digest() -> String {
    digest_of(EFFECTIVE_TYPE)
}

fn effective_member() -> String {
    EFFECTIVE_MEMBER_TEMPLATE.replace("@TYPE@", &effective_type_digest())
}

fn selection_digest() -> String {
    hex(&selection().digest)
}

fn universe_text() -> String {
    UNIVERSE_TEMPLATE
        .replace("@SEL@", &selection_digest())
        .replace("@TYPE@", &effective_type_digest())
}

fn view_text() -> String {
    VIEW_TEMPLATE
        .replace("@SEL@", &selection_digest())
        .replace("@TYPE@", &effective_type_digest())
}

fn enum_member() -> String {
    ENUM_MEMBER_TEMPLATE.replace("@DECL@", &digest_of(ENUM_DECLARATION))
}

fn unit() -> String {
    UNIT_TEMPLATE.replace("@DIM@", &digest_of(DIMENSION))
}

fn compound_unit() -> String {
    COMPOUND_UNIT_TEMPLATE.replace("@UNIT@", &digest_of(&unit()))
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The crate minted `actual` over the preimage `text` spells, equal to the
/// independent SHA-256 of `text`, and the earlier `serde_json` encoder emitted `text` too.
fn assert_golden(text: &str, actual: &str) {
    assert_eq!(
        actual,
        digest_of(text),
        "the identity minted over {text} is the independent SHA-256 of the text"
    );
    let parsed: Value = serde_json::from_str(text).expect("a vector is JSON");
    assert_eq!(
        serde_json::to_string(&parsed).expect("a Value serializes"),
        text,
        "the encoder before `quire-canonical` emitted these bytes, so the digest is unchanged"
    );
}

fn key(node: &str) -> DeclarationKey {
    DeclarationKey {
        package: "test/orders".to_owned(),
        node: node.to_owned(),
    }
}

fn selection() -> DomainPackageRef {
    DomainPackageRef {
        identity: "test/orders".to_owned(),
        version: "1".to_owned(),
    }
}

fn qualified(node: &str, owner: Option<EffectiveId>) -> EffectiveDeclarationPreimage {
    EffectiveDeclarationPreimage {
        owner_effective_type: owner,
        original: key(node),
        derivation: vec![Fact {
            ordinal: 0,
            rule: RULE_QUALIFY,
            inputs: vec![key(node)],
            parent: None,
        }],
    }
}

/// `quire.model.effective-declaration/v2`, for an effective type (`null`
/// owner) and an effective member (an `{domain, digest}` owner), with the
/// encoder's byte count equal to the text's length.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6")]
#[test]
fn effective_declaration_identities_match_their_golden_vectors() {
    let effective_type = qualified("ix://test/orders/A", None);
    let type_id = effective_type.identity();
    assert_golden(EFFECTIVE_TYPE, &type_id.to_string());
    assert_eq!(
        effective_type.canonical_len(),
        u64::try_from(EFFECTIVE_TYPE.len()).unwrap()
    );

    let member = qualified("ix://test/orders/A/x", Some(type_id));
    let member_text = effective_member();
    assert_golden(&member_text, &member.identity().to_string());
    assert_eq!(
        member.canonical_len(),
        u64::try_from(member_text.len()).unwrap()
    );
}

/// `quire.model.object-universe/v2`.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6")]
#[test]
fn object_universe_identity_matches_its_golden_vector() {
    let universe = ObjectUniverse {
        model_selection: selection(),
        root_types: vec![qualified("ix://test/orders/A", None).identity()],
    };
    let text = universe_text();
    assert_golden(&text, &universe.identity().to_string());
    assert_eq!(universe.canonical_len(), u64::try_from(text.len()).unwrap());
}

/// `quire.model.effective-view/v2`, over the view `normalize` builds for a
/// one-type domain package.
#[trace("QSpec-TC-195")]
#[test]
fn effective_view_identity_matches_its_golden_vector() {
    let package = DomainPackage::new(
        selection(),
        vec![DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key("ix://test/orders/A"),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        })],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::default())
    else {
        panic!("a one-type domain package normalizes");
    };
    let text = view_text();
    assert_golden(&text, &view.identity().to_string());
    assert_eq!(view.canonical_len(), u64::try_from(text.len()).unwrap());
}

/// The nominal enum declaration and enum member node preimages (FR-092 rule
/// 1): the member's declaration reference is the declaration's own digest.
#[trace("TC-409", "FR-088-AC-11")]
#[test]
fn enum_node_digests_match_their_golden_vectors() {
    let declaration =
        EnumDeclarationPreimage::from_json(serde_json::from_str(ENUM_DECLARATION).unwrap())
            .unwrap();
    assert_golden(
        ENUM_DECLARATION,
        &hex(&declaration
            .digest(qsl_foundation::IdentityLimits::default())
            .unwrap()),
    );
    let member_text = enum_member();
    let member =
        EnumMemberPreimage::from_json(serde_json::from_str(&member_text).unwrap()).unwrap();
    assert_golden(
        &member_text,
        &hex(&member
            .digest(qsl_foundation::IdentityLimits::default())
            .unwrap()),
    );
}

/// The dimension and unit node preimages, and the compound-unit
/// `quire.value.compound-unit/v1` identity over that unit.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-5")]
#[test]
fn dimension_unit_and_compound_unit_digests_match_their_golden_vectors() {
    let dimension = DimensionPreimage::from_json(serde_json::from_str(DIMENSION).unwrap()).unwrap();
    assert_golden(
        DIMENSION,
        &hex(&dimension
            .digest(qsl_foundation::IdentityLimits::default())
            .unwrap()),
    );
    let unit_text = unit();
    let unit = UnitPreimage::from_json(serde_json::from_str(&unit_text).unwrap()).unwrap();
    assert_golden(
        &unit_text,
        &hex(&unit
            .digest(qsl_foundation::IdentityLimits::default())
            .unwrap()),
    );
    let compound_text = compound_unit();
    let compound =
        CompoundUnitPreimage::from_json(serde_json::from_str(&compound_text).unwrap()).unwrap();
    let id = compound
        .id(qsl_foundation::IdentityLimits::default())
        .unwrap();
    assert_eq!(id.domain(), quire_exact::UnitDomain::Compound);
    assert_golden(&compound_text, &hex(id.as_bytes()));
}

/// The compound unit `metre^2` formed at runtime over the admitted golden
/// dimension and unit carries the golden `quire.value.compound-unit/v1` id
/// byte for byte, whether built from its preimage's terms or by multiplying
/// `metre` by itself. `quire-semantic-value` mints it through
/// `quire-canonical` without `std`, so this pins that build to the golden
/// bytes.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-5")]
#[test]
fn a_runtime_compound_unit_carries_the_golden_compound_unit_id() {
    let dimension = DimensionPreimage::from_json(serde_json::from_str(DIMENSION).unwrap()).unwrap();
    let unit_text = unit();
    let unit = UnitPreimage::from_json(serde_json::from_str(&unit_text).unwrap()).unwrap();
    let metre = NodeKey::from_digest(digest_bytes(&digest_of(&unit_text)));
    let graph = admit_unit_graph(
        [(
            dimension,
            NodeKey::from_digest(digest_bytes(&digest_of(DIMENSION))),
        )],
        [(unit, metre)],
        &OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
            authority: "agent-ix".into(),
            identity: "example-model".into(),
        })]),
        qsl_foundation::IdentityLimits::default(),
    )
    .expect("the golden dimension and unit admit");
    let compound_text = compound_unit();
    let compound =
        CompoundUnitPreimage::from_json(serde_json::from_str(&compound_text).unwrap()).unwrap();
    let built = graph
        .compound_unit(compound.terms())
        .expect("metre is a root unit");
    assert_eq!(
        hex(built.id().unwrap().as_bytes()),
        digest_of(&compound_text)
    );
    let metre = QuantityUnit::Declared(Box::new(graph.unit(metre).unwrap().clone()));
    let squared = result_unit(UnitOperation::Multiply, &metre, &metre).unwrap();
    assert_eq!(squared, QuantityUnit::Compound(built));
    assert_eq!(
        hex(squared.id().unwrap().as_bytes()),
        digest_of(&compound_text)
    );
    assert_eq!(
        squared.id().unwrap().domain(),
        quire_exact::UnitDomain::Compound
    );
}

/// Admit `raw` under the `sha256-jcs` digest `digest`: intake's check 3
/// passes only when the digest it takes over the parsed document equals it.
fn admit_under(raw: &str, digest: [u8; 32]) -> bool {
    refusal_under(raw, digest).is_none()
}

/// The refusal of [`admit_under`], or `None` when `raw` admits.
fn refusal_under(raw: &str, digest: [u8; 32]) -> Option<ModelRefusal> {
    let offered = DomainPackageRef {
        identity: "test/golden".to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let bytes = BTreeMap::from([(digest, raw.as_bytes().to_vec())]);
    admit(
        &offered,
        SHA256_JCS_DIGEST_DOMAIN,
        &bytes,
        qsl_foundation::IntakeLimits::default(),
    )
    .err()
}

/// `raw`, offered under `digest`, refuses `noncanonical_wire` with cause
/// `inexact-integer` at `/n`, never a digest mismatch.
fn assert_inexact_integer_at_n(raw: &str, digest: [u8; 32], what: &str) {
    let refusal = refusal_under(raw, digest).unwrap_or_else(|| panic!("{what} admitted"));
    assert_eq!(
        refusal.code,
        qsl_foundation::diagnostic::Code::NoncanonicalWire,
        "{what}"
    );
    assert!(
        matches!(
            &refusal.cause,
            ModelRefusalCause::NoncanonicalNumber { inexact, document_pointer }
                if inexact.as_str() == "inexact-integer" && document_pointer.as_str() == "/n"
        ),
        "{what}: {:?}",
        refusal.cause
    );
}

fn digest_bytes(hex: &str) -> [u8; 32] {
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * index..2 * index + 2], 16).unwrap();
    }
    digest
}

/// The domain-package `sha256-jcs` digest intake recomputes (FR-154 check
/// 3): taken over the RFC 8785 text of the parsed document, not the raw
/// bytes, so a whitespace/order variant admits under the canonical digest.
#[trace("TC-145", "FR-056-AC-2")]
#[test]
fn domain_package_digest_matches_its_golden_vector() {
    let parsed: Value = serde_json::from_str(DOCUMENT_RAW).unwrap();
    assert_golden(
        DOCUMENT,
        &sha256_hex(serde_json::to_string(&parsed).unwrap().as_bytes()),
    );
    assert!(admit_under(
        DOCUMENT_RAW,
        digest_bytes(&digest_of(DOCUMENT))
    ));
    assert!(!admit_under(
        DOCUMENT_RAW,
        digest_bytes(&sha256_hex(DOCUMENT_RAW.as_bytes()))
    ));
}

/// The one changed digest: an integral number spelled `1.0` or `1e2`.
/// Intake now admits the document under the digest of its RFC 8785 text
/// (`1`, `100`), and no longer under the digest of `serde_json`'s
/// non-conformant `1.0`/`100.0` text.
#[trace("TC-145", "FR-056-AC-2")]
#[test]
fn domain_package_digest_spells_numbers_as_rfc_8785_does() {
    let parsed: Value = serde_json::from_str(FLOAT_DOCUMENT_RAW).unwrap();
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        FLOAT_DOCUMENT_BEFORE_QSL_194,
        "serde_json's spelling, which RFC 8785 does not use"
    );
    assert!(admit_under(
        FLOAT_DOCUMENT_RAW,
        digest_bytes(&digest_of(FLOAT_DOCUMENT))
    ));
    assert!(!admit_under(
        FLOAT_DOCUMENT_RAW,
        digest_bytes(&sha256_hex(FLOAT_DOCUMENT_BEFORE_QSL_194.as_bytes()))
    ));
}

/// The RFC 8785 text of the big-integer documents below, `n` being the IEEE
/// 754 double the literal parses to, written by hand from an independent
/// JavaScript RFC 8785 canonicalizer (`JSON.parse`, sorted keys,
/// `JSON.stringify`); its digest is the SHA-256 of that text, recomputed.
const BIG_INTEGER_DOCUMENT: &str =
    r#"{"n":18446744073709552000,"package":{"identity":"test/golden","version":"1"}}"#;
const BEYOND_U64_DOCUMENT: &str =
    r#"{"n":1.2345678901234568e+29,"package":{"identity":"test/golden","version":"1"}}"#;
const BELOW_I53_DOCUMENT: &str =
    r#"{"n":-9007199254740992,"package":{"identity":"test/golden","version":"1"}}"#;

fn big_integer_document(literal: &str) -> String {
    format!(r#"{{"package":{{"identity":"test/golden","version":"1"}},"n":{literal}}}"#)
}

/// An integer outside ±2^53 has no exact double, and RFC 8785
/// canonicalizes the double it parses to, so `2^64 - 1` and `2^64` once
/// both admitted under the digest of `18446744073709552000`, an integer
/// beyond `u64` under `1.2345678901234568e+29`'s and one below `-2^53`
/// under `-9007199254740992`'s. Intake now refuses each document, so none
/// admits under the digest of its double, nor under the digest of its own
/// exact digits.
#[trace("TC-145", "FR-056-AC-13")]
#[test]
fn big_integers_admit_under_neither_their_double_nor_their_digits() {
    for (literal, double_text) in [
        ("18446744073709551615", BIG_INTEGER_DOCUMENT),
        ("18446744073709551616", BIG_INTEGER_DOCUMENT),
        ("123456789012345678901234567890", BEYOND_U64_DOCUMENT),
        ("-9007199254740993", BELOW_I53_DOCUMENT),
    ] {
        let raw = big_integer_document(literal);
        assert_inexact_integer_at_n(&raw, digest_bytes(&digest_of(double_text)), literal);
        let exact_digits =
            format!(r#"{{"n":{literal},"package":{{"identity":"test/golden","version":"1"}}}}"#);
        assert_inexact_integer_at_n(
            &raw,
            digest_bytes(&sha256_hex(exact_digits.as_bytes())),
            &format!("{literal} under its exact digits"),
        );
    }
}

/// Domain separation: every identity vector above digests to a different
/// value, so no two preimage domains can be confused for one another.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6")]
#[test]
fn no_two_identity_vectors_share_a_digest() {
    let texts = [
        EFFECTIVE_TYPE.to_owned(),
        effective_member(),
        universe_text(),
        view_text(),
        ENUM_DECLARATION.to_owned(),
        enum_member(),
        DIMENSION.to_owned(),
        unit(),
        compound_unit(),
        DOCUMENT.to_owned(),
        FLOAT_DOCUMENT.to_owned(),
    ];
    let digests: std::collections::BTreeSet<String> =
        texts.iter().map(|text| digest_of(text)).collect();
    assert_eq!(digests.len(), texts.len());
}
