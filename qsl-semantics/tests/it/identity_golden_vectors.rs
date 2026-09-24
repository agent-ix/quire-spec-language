// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-194: golden vectors for the RFC 8785 identity domains this crate
//! mints through `quire-canonical` (ADR-013 §2, ADR-013:113: one RFC 8785
//! implementation produces every RFC 8785 encoding).
//!
//! Each vector is the canonical text itself, written out by hand, and its
//! SHA-256. A vector checks three things: the text hashes to the pinned
//! digest (so the digest is the text's, not the code's); the identity the
//! crate mints over the same preimage equals it; and the `serde_json`
//! `Value` encoder these sites used before QSL-194 emitted exactly the same
//! text, so moving to `quire-canonical` left the digest unchanged.
//!
//! The one digest QSL-194 does change is shown here too: a domain-package
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
    DeclarationKey, EffectiveDeclarationPreimage, EffectiveId, Fact, RULE_QUALIFY,
    SHA256_JCS_DIGEST_DOMAIN,
};
use qsl_semantics::model::normalize::{normalize, NormalizeOutcome, ObjectUniverse};
use qsl_semantics::value::enumeration::{EnumDeclarationPreimage, EnumMemberPreimage};
use qsl_semantics::value::{
    CompoundUnitPreimage, DimensionPreimage, NodeIdentityPreimage, UnitPreimage,
};

const EFFECTIVE_TYPE: &str = r#"{"derivation":[{"inputs":[{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"}],"ordinal":"0","rule":{"identity":"quire.model.normalize.qualify/v1","revision":"1-draft.1"}}],"original":{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"},"owner_effective_type":null,"version":"quire.model.effective-declaration/v1"}"#;
const EFFECTIVE_TYPE_DIGEST: &str =
    "3b79bb92933313c6724ccfa216190afc89e1ba4c0c5a1fa0b7832aa6c98b8203";

const EFFECTIVE_MEMBER: &str = r#"{"derivation":[{"inputs":[{"digest_domain":"sha256-jcs","node":"ix://test/orders/A/x","package":"test/orders"}],"ordinal":"0","rule":{"identity":"quire.model.normalize.qualify/v1","revision":"1-draft.1"}}],"original":{"digest_domain":"sha256-jcs","node":"ix://test/orders/A/x","package":"test/orders"},"owner_effective_type":{"digest":"3b79bb92933313c6724ccfa216190afc89e1ba4c0c5a1fa0b7832aa6c98b8203","domain":"quire.model.effective-declaration/v1"},"version":"quire.model.effective-declaration/v1"}"#;
const EFFECTIVE_MEMBER_DIGEST: &str =
    "750ec6cfe7da00cb205a6fd47a10b7a06b71b746afa4b358e3828bc62b1e002f";

const UNIVERSE: &str = r#"{"model_selection":{"digest":"1111111111111111111111111111111111111111111111111111111111111111","digest_domain":"sha256-jcs","identity":"test/orders","version":"1"},"root_types":[{"digest":"3b79bb92933313c6724ccfa216190afc89e1ba4c0c5a1fa0b7832aa6c98b8203","domain":"quire.model.effective-declaration/v1"}],"version":"quire.model.object-universe/v1"}"#;
const UNIVERSE_DIGEST: &str = "41ebe3815ccf4b9ccdad84c1efa3393c37cff35f3d81778b6e2655d3d48ae48d";

const VIEW: &str = r#"{"declarations":[{"effective_id":{"digest":"3b79bb92933313c6724ccfa216190afc89e1ba4c0c5a1fa0b7832aa6c98b8203","domain":"quire.model.effective-declaration/v1"},"preimage":{"derivation":[{"inputs":[{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"}],"ordinal":"0","rule":{"identity":"quire.model.normalize.qualify/v1","revision":"1-draft.1"}}],"original":{"digest_domain":"sha256-jcs","node":"ix://test/orders/A","package":"test/orders"},"owner_effective_type":null,"version":"quire.model.effective-declaration/v1"}}],"model_selection":{"digest":"1111111111111111111111111111111111111111111111111111111111111111","digest_domain":"sha256-jcs","identity":"test/orders","version":"1"},"rules":{"identity":"quire.model.complete.rules/v1","revision":"1-draft.1"},"version":"quire.model.effective-view/v1"}"#;
const VIEW_DIGEST: &str = "6ec2e0e4656fe1d1c174d2598d2f447f8ef68b3447ed52c3b4ad059301ff66e8";

const ENUM_DECLARATION: &str = r#"{"members":["Active","Closed"],"ordered":false,"owner":{"authority":"agent-ix","identity":"example-model","kind":"definition"},"qualified_declaration":["Example","Status"],"version":"quire.enum-declaration-node/v1"}"#;
const ENUM_DECLARATION_DIGEST: &str =
    "347ab6eb43f9f5769a9fcc8a62b0142d76401297a1e2f1265987e5557fe7a72d";

const ENUM_MEMBER: &str = r#"{"case":"Active","declaration_node_id":{"digest":"347ab6eb43f9f5769a9fcc8a62b0142d76401297a1e2f1265987e5557fe7a72d","domain":"quire.checked-semantic-node/v1"},"version":"quire.enum-member-node/v1"}"#;
const ENUM_MEMBER_DIGEST: &str = "f831f2a77f4edab6b089d6ab53fcc34f365423de9adffa3e9603088a979cc147";

const DIMENSION: &str = r#"{"owner":{"authority":"agent-ix","identity":"example-model","kind":"definition"},"qualified_declaration":["Example","Length"],"terms":[],"version":"quire.dimension-node/v1"}"#;
/// Equal to `LENGTH` in `check::lowering::model::tests`, pinned there
/// before QSL-194 from the same preimage.
const DIMENSION_DIGEST: &str = "b6cc14ab93b670cb0fc74a80dd18131ef7b06e3eee6a730e5ca092266314e22b";

const UNIT: &str = r#"{"dimension_node_id":{"digest":"b6cc14ab93b670cb0fc74a80dd18131ef7b06e3eee6a730e5ca092266314e22b","domain":"quire.checked-semantic-node/v1"},"offset":{"denominator":"1","numerator":"0"},"owner":{"authority":"agent-ix","identity":"example-model","kind":"definition"},"qualified_declaration":["Example","Metre"],"scale":{"denominator":"1","numerator":"1"},"target_unit_node_id":null,"version":"quire.unit-node/v1"}"#;
const UNIT_DIGEST: &str = "41564151551aa8147f616d61756eba356cbfd046d671cfbbe3e85afdc35f7454";

const COMPOUND_UNIT: &str = r#"{"terms":[{"exponent":"2","unit_node_id":{"digest":"41564151551aa8147f616d61756eba356cbfd046d671cfbbe3e85afdc35f7454","domain":"quire.checked-semantic-node/v1"}}],"version":"quire.value.compound-unit/v1"}"#;
const COMPOUND_UNIT_DIGEST: &str =
    "23d34da2c5d4934dca8d31832eedf391f5a5fd6ced34f45d8b24a88ac7ace3b9";

/// A domain-package document as a producer might write it: whitespace,
/// members out of order, an escaped `é`.
const DOCUMENT_RAW: &str = r#"{ "package": { "version": "1", "identity": "test/golden" }, "types": [], "note": "\u00e9" }"#;
const DOCUMENT: &str =
    r#"{"note":"é","package":{"identity":"test/golden","version":"1"},"types":[]}"#;
const DOCUMENT_DIGEST: &str = "846d75a8d7df96aa9445d96b5b3be3fc9bd0ac96c7bf5ad4c9fd3f12ba55cc2f";

/// A document with integral numbers spelled `1.0` and `1e2`: its RFC 8785
/// text, and the text `serde_json` printed for it before QSL-194.
const FLOAT_DOCUMENT_RAW: &str =
    r#"{"package":{"identity":"test/golden","version":"1"},"n":[1.0,1e2,0.5]}"#;
const FLOAT_DOCUMENT: &str =
    r#"{"n":[1,100,0.5],"package":{"identity":"test/golden","version":"1"}}"#;
const FLOAT_DOCUMENT_DIGEST: &str =
    "b44169de7f79ea5fce5adc150bf9a64235cb999482163b46e2e9e31d0e059a30";
const FLOAT_DOCUMENT_BEFORE_QSL_194: &str =
    r#"{"n":[1.0,100.0,0.5],"package":{"identity":"test/golden","version":"1"}}"#;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `text` hashes to `digest`, the crate minted `actual` over the same
/// preimage, and the pre-QSL-194 `serde_json` encoder emitted `text` too.
fn assert_golden(text: &str, digest: &str, actual: &str) {
    assert_eq!(sha256_hex(text.as_bytes()), digest, "the vector's own text");
    assert_eq!(actual, digest, "the identity minted over {text}");
    let parsed: Value = serde_json::from_str(text).expect("a vector is JSON");
    assert_eq!(
        serde_json::to_string(&parsed).expect("a Value serializes"),
        text,
        "the pre-QSL-194 encoder emitted these bytes, so the digest is unchanged"
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
        digest: [0x11; 32],
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
        }],
    }
}

/// `quire.model.effective-declaration/v1`, for an effective type (`null`
/// owner) and an effective member (an `{domain, digest}` owner), with the
/// encoder's byte count equal to the text's length.
#[trace("TC-195")]
#[test]
fn effective_declaration_identities_match_their_golden_vectors() {
    let effective_type = qualified("ix://test/orders/A", None);
    let type_id = effective_type.identity();
    assert_golden(EFFECTIVE_TYPE, EFFECTIVE_TYPE_DIGEST, &type_id.to_string());
    assert_eq!(
        effective_type.canonical_len(),
        u64::try_from(EFFECTIVE_TYPE.len()).unwrap()
    );

    let member = qualified("ix://test/orders/A/x", Some(type_id));
    assert_golden(
        EFFECTIVE_MEMBER,
        EFFECTIVE_MEMBER_DIGEST,
        &member.identity().to_string(),
    );
    assert_eq!(
        member.canonical_len(),
        u64::try_from(EFFECTIVE_MEMBER.len()).unwrap()
    );
}

/// `quire.model.object-universe/v1`.
#[trace("TC-195", "FR-150-AC-6")]
#[test]
fn object_universe_identity_matches_its_golden_vector() {
    let universe = ObjectUniverse {
        model_selection: selection(),
        root_types: vec![qualified("ix://test/orders/A", None).identity()],
    };
    assert_golden(UNIVERSE, UNIVERSE_DIGEST, &universe.identity().to_string());
    assert_eq!(
        universe.canonical_len(),
        u64::try_from(UNIVERSE.len()).unwrap()
    );
}

/// `quire.model.effective-view/v1`, over the view `normalize` builds for a
/// one-type domain package.
#[trace("TC-195")]
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
    assert_golden(VIEW, VIEW_DIGEST, &view.identity().to_string());
    assert_eq!(view.canonical_len(), u64::try_from(VIEW.len()).unwrap());
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
        ENUM_DECLARATION_DIGEST,
        &hex(&declaration.digest().unwrap()),
    );
    let member = EnumMemberPreimage::from_json(serde_json::from_str(ENUM_MEMBER).unwrap()).unwrap();
    assert_golden(
        ENUM_MEMBER,
        ENUM_MEMBER_DIGEST,
        &hex(&member.digest().unwrap()),
    );
}

/// The dimension and unit node preimages, and the compound-unit
/// `quire.value.compound-unit/v1` identity over that unit.
#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn dimension_unit_and_compound_unit_digests_match_their_golden_vectors() {
    let dimension = DimensionPreimage::from_json(serde_json::from_str(DIMENSION).unwrap()).unwrap();
    assert_golden(
        DIMENSION,
        DIMENSION_DIGEST,
        &hex(&dimension.digest().unwrap()),
    );
    let unit = UnitPreimage::from_json(serde_json::from_str(UNIT).unwrap()).unwrap();
    assert_golden(UNIT, UNIT_DIGEST, &hex(&unit.digest().unwrap()));
    let compound =
        CompoundUnitPreimage::from_json(serde_json::from_str(COMPOUND_UNIT).unwrap()).unwrap();
    let id = compound.id();
    assert_eq!(id.domain(), quire_exact::UnitDomain::Compound);
    assert_golden(COMPOUND_UNIT, COMPOUND_UNIT_DIGEST, &hex(id.as_bytes()));
}

/// Admit `raw` under the `sha256-jcs` digest `digest`: intake's check 3
/// passes only when the digest it takes over the parsed document equals it.
fn admit_under(raw: &str, digest: [u8; 32]) -> bool {
    let offered = DomainPackageRef {
        identity: "test/golden".to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let bytes = BTreeMap::from([(digest, raw.as_bytes().to_vec())]);
    admit(&offered, SHA256_JCS_DIGEST_DOMAIN, &bytes).is_ok()
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
        DOCUMENT_DIGEST,
        &sha256_hex(serde_json::to_string(&parsed).unwrap().as_bytes()),
    );
    assert!(admit_under(DOCUMENT_RAW, digest_bytes(DOCUMENT_DIGEST)));
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
    assert_eq!(sha256_hex(FLOAT_DOCUMENT.as_bytes()), FLOAT_DOCUMENT_DIGEST);
    let parsed: Value = serde_json::from_str(FLOAT_DOCUMENT_RAW).unwrap();
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        FLOAT_DOCUMENT_BEFORE_QSL_194,
        "serde_json's spelling, which RFC 8785 does not use"
    );
    assert!(admit_under(
        FLOAT_DOCUMENT_RAW,
        digest_bytes(FLOAT_DOCUMENT_DIGEST)
    ));
    assert!(!admit_under(
        FLOAT_DOCUMENT_RAW,
        digest_bytes(&sha256_hex(FLOAT_DOCUMENT_BEFORE_QSL_194.as_bytes()))
    ));
}

/// The RFC 8785 text of the big-integer documents below, `n` being the IEEE
/// 754 double the literal parses to, and its SHA-256, both from an
/// independent JavaScript RFC 8785 canonicalizer (`JSON.parse`, sorted
/// keys, `JSON.stringify`).
const BIG_INTEGER_DOCUMENT: &str =
    r#"{"n":18446744073709552000,"package":{"identity":"test/golden","version":"1"}}"#;
const BIG_INTEGER_DIGEST: &str = "307e88eff7a1ca7eef749449cce656ffbb2ee67a65a33866cf87fd232cee79e4";
const BEYOND_U64_DIGEST: &str = "25fd8dabded7b948d8cbf764d17f1a662fc9974d8fc80e32cb1c9a25409a7393";
const BELOW_I53_DIGEST: &str = "bb0f6be8d14b570d7cf7a020276afd187d79a0aa03bfb0767c64a9cfad6defe4";

fn big_integer_document(literal: &str) -> String {
    format!(r#"{{"package":{{"identity":"test/golden","version":"1"}},"n":{literal}}}"#)
}

/// QSL-194 M2: an integer outside ±2^53 has no exact double, and RFC 8785
/// canonicalizes the double it parses to. Both spellings of 2^64's
/// neighbourhood (`2^64 - 1`, a `u64`, and `2^64`, beyond it) admit under
/// the digest of `18446744073709552000`; an integer beyond `u64` admits
/// under `1.2345678901234568e+29`'s; one below `-2^53` under
/// `-9007199254740992`'s. None is refused, and none admits under the
/// digest of its own exact digits.
#[trace("TC-145", "FR-056-AC-2")]
#[test]
fn big_integers_canonicalize_to_the_double_rfc_8785_reads() {
    assert_eq!(
        sha256_hex(BIG_INTEGER_DOCUMENT.as_bytes()),
        BIG_INTEGER_DIGEST
    );
    for (literal, digest) in [
        ("18446744073709551615", BIG_INTEGER_DIGEST),
        ("18446744073709551616", BIG_INTEGER_DIGEST),
        ("123456789012345678901234567890", BEYOND_U64_DIGEST),
        ("-9007199254740993", BELOW_I53_DIGEST),
    ] {
        let raw = big_integer_document(literal);
        assert!(admit_under(&raw, digest_bytes(digest)), "{literal}");
        let exact_digits =
            format!(r#"{{"n":{literal},"package":{{"identity":"test/golden","version":"1"}}}}"#);
        assert!(
            !admit_under(&raw, digest_bytes(&sha256_hex(exact_digits.as_bytes()))),
            "{literal} admitted under its exact digits"
        );
    }
}
