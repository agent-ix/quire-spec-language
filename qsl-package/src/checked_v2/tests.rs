// SPDX-License-Identifier: AGPL-3.0-or-later
//! Behavioral tests for the I2 `quire.checked-package/v2` byte reader
//! (ADR-011 §4, QSpec FR-322). Test bytes are built inline, following
//! `tests/library_resolution.rs`'s own approach: no QSpec fixtures are
//! vendored into this repo. Fixtures build a complete, minimal
//! `quire.checked-package/v2` wire (one semantic-graph node, one locked
//! source, an edition and a diagnostics catalog) because IR's v2 reader
//! validates the whole I04 contract unconditionally -- there is no narrower
//! "envelope and `package_id` only" admission path (this module's own doc).

use ix_trace_rs::trace;
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageRefusalCause,
    CheckedPackageRefusalCode, CHECKED_PACKAGE_V2, PACKAGE_DOMAIN_V2,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{read_v2, Read, V2ReadLimits, V2ReadRefusal};
use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{
    CatalogCode, CatalogCoded, Category, Code, LimitExceeded, LimitKind, Locus,
};
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::imports::ImportedNames;
use qsl_semantics::check::CheckCause;
use qsl_semantics::library::{
    ImportView, LibraryName, LibraryRefusal, PackageId, PackageNodeKey, PinMismatch, PinnedRequest,
    RefusalClass, Selection,
};
use quire_exact::{Origin, Role};

const NODE_DOMAIN: &str = "quire.checked-semantic-node/v1";
const SOURCE_DOMAIN: &str = "quire.source.bytes/v1";

fn hex(label: &str) -> String {
    Sha256::digest(label.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn node_ref(label: &str) -> Value {
    json!({"digest": hex(label), "domain": NODE_DOMAIN})
}

/// A QSpec `DefinitionRef`: exactly `{authority, identity}`.
fn artifact_ref(label: &str) -> Value {
    json!({"authority": "pkg", "identity": label})
}

/// A QSpec `RawSourceRef`: authority, identity and the digest of its bytes.
fn source_ref(label: &str) -> Value {
    json!({
        "authority": "pkg",
        "identity": label,
        "digest_domain": SOURCE_DOMAIN,
        "digest": hex(label),
    })
}

fn selection(role: &str, label: &str) -> Value {
    json!({"role": role, "definition": artifact_ref(label)})
}

/// A FR-322 `DependencySelection`: library `identity`, whose
/// `package_id` digest is `label`'s hash.
fn dependency_selection(identity: &str, label: &str) -> Value {
    json!({
        "identity": identity,
        "package_id": {"domain": PACKAGE_DOMAIN_V2, "algorithm": "sha256", "digest": hex(label)},
    })
}

/// Fields shared by a `semantic_graph` node and its identity-projection
/// counterpart: `quire_contract_model::CheckedNodeProjectionV2::from`'s own
/// definition of the projection is exactly a `CheckedSemanticNodeV2` with
/// `occurrences` dropped, so IR's own reader requires the two to describe
/// literally the same node (`identity_preimage.identity_projection` must
/// equal `semantic_graph.nodes` mapped through that `From` impl). A
/// self-typed scalar with a trivial boolean literal body needs no other
/// node's identity and no frame/nominal validation. `export` is `R`'s own
/// `declaration`, paired with a `declaration`-role occurrence
/// (`validate_declaration`'s `DeclarationOccurrenceRule`).
fn node_fields(label: &str, export: &str) -> Value {
    let reference = node_ref(label);
    json!({
        "node_id": reference,
        "schema_version": "quire.checked-semantic-graph/v2",
        "node_tag": "scalar_type",
        "semantic_form": "boolean",
        "semantic_type": reference,
        "dependencies": [],
        "declaration": {"qualified_name": [export]},
        "body": {
            "term": "literal",
            "type": reference,
            "value_kind": "boolean",
            "value": true,
        },
    })
}

fn projection_node(label: &str, export: &str) -> Value {
    node_fields(label, export)
}

fn graph_node(label: &str, export: &str) -> Value {
    let mut node = node_fields(label, export);
    node["occurrences"] = json!([{"role": "declaration", "ordinal": 0}]);
    node
}

/// A structurally valid `quire.checked-package-id/v2` identity preimage
/// declaring one export, `R`, with `dependency_selections` supplied by the
/// caller (empty for the ordinary admitted case; findings test a non-empty
/// one).
fn identity_preimage(dependency_selections: Vec<Value>) -> Value {
    json!({
        "definition_selections": [],
        "dependency_selections": dependency_selections,
        "edition": selection("edition", "edition-def"),
        "identity_projection": [projection_node("pkg::R", "R")],
        "model_selections": [],
        "profile_selections": [],
        "required_features": [],
        "version": "quire.checked-package-id/v2",
    })
}

fn lock(dependency_selections: Vec<Value>) -> Value {
    json!({
        "sources": [source_ref("src")],
        "edition": selection("edition", "edition-def"),
        "definition_selections": [],
        "dependency_selections": dependency_selections,
        "model_selections": [],
        "profile_selections": [],
        "required_features": [],
    })
}

fn diagnostics() -> Value {
    json!({"catalog": artifact_ref("diag-catalog"), "entries": []})
}

fn jcs(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn package_id_digest(preimage: &Value) -> String {
    Sha256::digest(jcs(preimage))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn package_id_member(preimage: &Value) -> Value {
    json!({
        "algorithm": "sha256",
        "digest": package_id_digest(preimage),
        "domain": PACKAGE_DOMAIN_V2,
    })
}

/// A structurally valid `quire.checked-package/v2` envelope wrapping
/// `preimage`, with a correctly recomputed `package_id` and one graph node.
fn valid_envelope(preimage: &Value) -> Value {
    json!({
        "capability_report": [],
        "contract_version": CHECKED_PACKAGE_V2,
        "diagnostics": diagnostics(),
        "identity_preimage": preimage,
        "lock": lock(preimage["dependency_selections"].as_array().unwrap().clone()),
        "package_id": package_id_member(preimage),
        "semantic_graph": {
            "graph_version": "quire.checked-semantic-graph/v2",
            "nodes": [graph_node("pkg::R", "R")],
        },
        "source_map": [{
            "node_id": node_ref("pkg::R"),
            "role": "declaration",
            "ordinal": 0,
            "regions": [{"source": source_ref("src"), "start": 0, "end": 1}],
        }],
    })
}

fn identity(label: &str) -> LibraryName {
    LibraryName::new(label).unwrap()
}

/// A pinned request selecting `pkg` at `package_id`.
fn pin(package_id: PackageId) -> PinnedRequest {
    pin_library("pkg", package_id)
}

fn pin_library(library: &str, package_id: PackageId) -> PinnedRequest {
    qsl_semantics::library::fixtures::single_pin(identity(library), Selection { package_id })
}

/// ADR-011 §4 condition 3's own input: `pkg` pinned at the `package_id`
/// `preimage`'s own JCS bytes recompute to -- exactly what a consumer's
/// library lock or pinned request would record for a dependency it expects
/// these bytes to satisfy.
fn pinned_for(preimage: &Value) -> PinnedRequest {
    pin(PackageId::of_preimage(&jcs(preimage)))
}

fn no_pins() -> PinnedRequest {
    PinnedRequest::default()
}

/// The reader's own artifact byte ceiling at `limit`, reached by `actual`
/// offered bytes: no locus (FR-096).
fn input_bytes(limit: usize, actual: usize) -> LimitExceeded {
    LimitExceeded::new(
        LimitKind::InputBytes,
        u64::try_from(limit).unwrap(),
        u128::try_from(actual).unwrap(),
    )
}

fn read(bytes: &[u8], pinned: &PinnedRequest) -> Read {
    read_v2(
        bytes,
        identity("pkg"),
        V2ReadLimits::default(),
        &CheckedPackageEvidence::new(),
        pinned,
    )
}

/// The `library` refusal a wire read ended in, or a panic naming the
/// outcome it got instead.
fn structural(outcome: Read) -> LibraryRefusal {
    match outcome {
        Read::Refused(V2ReadRefusal::Structural(refusal)) => *refusal,
        other => panic!("expected Refused(Structural(_)), got {other:?}"),
    }
}

/// TC-253 step 1: valid v2 bytes, pinned at their own identity and
/// recomputed `package_id`, are admitted as a `VerifiedPackage`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn accepts_valid_bytes() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    match read(&bytes, &pinned_for(&preimage)) {
        Read::Verified { package, .. } => {
            assert_eq!(package.library(), &identity("pkg"));
            assert_eq!(
                package.package_id(),
                PackageId::of_preimage(&jcs(&preimage))
            );
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// A wire whose projection declares `exports` (label, qualified name), each
/// a self-typed boolean scalar with a `declaration`-role occurrence, in
/// ascending node-id order.
fn envelope_declaring(exports: &[(&str, &str)]) -> (Value, Value) {
    let mut sorted = exports.to_vec();
    sorted.sort_by_key(|(label, _)| hex(label));
    envelope_in_graph_order(&sorted)
}

/// A wire whose graph and projection hold `exports` in exactly the given
/// order (FR-322: the projection keeps the graph's node order).
fn envelope_in_graph_order(exports: &[(&str, &str)]) -> (Value, Value) {
    let mut preimage = identity_preimage(vec![]);
    preimage["identity_projection"] = exports
        .iter()
        .map(|(label, export)| projection_node(label, export))
        .collect();
    let mut envelope = valid_envelope(&preimage);
    envelope["semantic_graph"]["nodes"] = exports
        .iter()
        .map(|(label, export)| graph_node(label, export))
        .collect();
    envelope["source_map"] = exports
        .iter()
        .zip(0_u64..)
        .map(|((label, _), start)| {
            json!({
                "node_id": node_ref(label),
                "role": "declaration",
                "ordinal": 0,
                "regions": [{"source": source_ref("src"), "start": start, "end": start + 1}],
            })
        })
        .collect();
    (preimage, envelope)
}

/// QSpec FR-322-AC-14: the projection keeps the semantic graph's
/// node order, and that order is part of `package_id`. The same two exports
/// in ascending and in descending node-id order are both admitted, under two
/// different `package_id`s, and export the same declarations.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn reversed_node_order_is_admitted_under_a_different_package_id() {
    let mut exports = vec![("pkg::Y", "A"), ("pkg::Z", "B")];
    exports.sort_by_key(|(label, _)| hex(label));
    let admitted = |exports: &[(&str, &str)]| {
        let (preimage, envelope) = envelope_in_graph_order(exports);
        match read(&jcs(&envelope), &pinned_for(&preimage)) {
            Read::Verified { package, .. } => package,
            other => panic!("expected Verified, got {other:?}"),
        }
    };
    let ascending = admitted(&exports);
    exports.reverse();
    let descending = admitted(&exports);
    assert_ne!(ascending.package_id(), descending.package_id());
    let names = |package: qsl_semantics::library::VerifiedPackage| {
        package
            .into_import_view()
            .exports()
            .map(|(name, key)| (name.to_owned(), key.node))
            .collect::<Vec<_>>()
    };
    assert_eq!(names(ascending), names(descending));
}

/// The `ImportView` of the verified wire declaring `exports`.
fn import_view_of(exports: &[(&str, &str)]) -> (PackageId, ImportView) {
    let (preimage, envelope) = envelope_declaring(exports);
    let verified = match read(&jcs(&envelope), &pinned_for(&preimage)) {
        Read::Verified { package, .. } => package,
        other => panic!("expected Verified, got {other:?}"),
    };
    let package_id = verified.package_id();
    (package_id, verified.into_import_view())
}

/// FR-087-AC-4, TC-254 steps 1-2: `VerifiedPackage::into_import_view`
/// carries every exported declaration of a two-export package, each at its
/// own `WireNodeId` under the verified `package_id`, keyed by that id and
/// not by name. The node seeds are chosen so that ascending node-id order
/// (`pkg::Z` before `pkg::Y`) is the reverse of ascending name order (`A`
/// before `B`): a view keyed by name yields `A` first and fails.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn import_view_keys_each_export_by_its_own_wire_node_id() {
    let (package_id, view) = import_view_of(&[("pkg::Y", "A"), ("pkg::Z", "B")]);
    assert_eq!(view.package(), package_id);
    let key =
        |label: &str| PackageNodeKey::new(package_id, WireNodeId::from_hex(&hex(label)).unwrap());
    assert!(key("pkg::Z").node < key("pkg::Y").node);
    let exports: Vec<_> = view.exports().collect();
    assert_eq!(exports, vec![("B", key("pkg::Z")), ("A", key("pkg::Y"))]);
}

/// FR-087-AC-4, TC-254 step 5: the importing package's checker resolves a
/// name by its own index over the view's entries (`check::imports`), to the
/// export's `PackageNodeKey`; a name the package does not export is
/// refused `missing_declaration` / `missing-name`.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn the_checker_resolves_an_imported_name_over_the_view_entries() {
    let (package_id, view) = import_view_of(&[("pkg::Y", "A"), ("pkg::Z", "B")]);
    let names = ImportedNames::of(&view);
    let key =
        |label: &str| PackageNodeKey::new(package_id, WireNodeId::from_hex(&hex(label)).unwrap());
    assert_eq!(names.resolve("A"), Ok(key("pkg::Y")));
    assert_eq!(names.resolve("B"), Ok(key("pkg::Z")));
    let missing = names.resolve("C").unwrap_err();
    assert_eq!(missing, CheckCause::MissingName("C".to_owned()));
    assert_eq!(missing.code(), Code::MissingDeclaration);
    assert_eq!(missing.cause(), Some("missing-name"));
}

/// FR-087-AC-3, TC-253 step 4: an otherwise-admissible package refuses when
/// its identity is absent from the caller's pinned request -- I2's first
/// rule, `MissingImport`, naming the absent identity.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_identity_is_absent_from_the_pinned_lock() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    assert_eq!(
        structural(read(&bytes, &no_pins())),
        LibraryRefusal::MissingImport {
            path: vec![identity("pkg")],
        }
    );
}

/// TC-253 step 4, adverse: a pin for a *different* library carrying this
/// package's exact `package_id` does not list this identity.
/// Condition 3 matches on identity, never on the digest alone.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_only_a_different_library_pins_the_same_id() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let other = pin_library("other", PackageId::of_preimage(&jcs(&preimage)));
    assert_eq!(
        structural(read(&bytes, &other)),
        LibraryRefusal::MissingImport {
            path: vec![identity("pkg")],
        }
    );
}

/// FR-087-AC-3, condition 3: the pinned entry for this identity names a
/// different `package_id` -- `StaleDependency{ByteDigestMismatch}` (ruling
/// (e): I2's first rule), carrying both the pinned selection and the one
/// the candidate presented.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_the_pinned_package_id_disagrees() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let stale = PackageId::of_preimage(b"not-this-preimage");
    let refusal = structural(read(&bytes, &pin(stale)));
    assert_eq!(
        refusal,
        LibraryRefusal::StaleDependency {
            path: vec![identity("pkg")],
            pin: Box::new(PinMismatch {
                pinned: Selection { package_id: stale },
                presented: Selection {
                    package_id: PackageId::of_preimage(&jcs(&preimage)),
                },
            }),
        }
    );
    assert_eq!(refusal.class(), RefusalClass::I2Rule(1));
}

/// TC-253 step 8 on the wire path: an unsupported contract version
/// (condition one) and an absent pin (condition three) together still refuse
/// cleanly, with condition one's named cause, and yield no `VerifiedPackage`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_unsupported_version_with_no_pin() {
    let preimage = identity_preimage(vec![]);
    let mut envelope = valid_envelope(&preimage);
    envelope["contract_version"] = json!("quire.checked-package/v1");
    let outcome = read(&jcs(&envelope), &no_pins());
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::UnsupportedVersion { wire, .. })
                if &*wire.actual == "quire.checked-package/v1"
        ),
        "expected Refused(UnsupportedVersion), got {outcome:?}"
    );
}

/// The refusal's stable `Code`.
fn envelope_code(outcome: &Read) -> Option<Code> {
    match outcome {
        Read::Refused(
            refusal @ (V2ReadRefusal::Envelope { .. } | V2ReadRefusal::UnsupportedVersion { .. }),
        ) => Some(refusal.code()),
        _ => None,
    }
}

#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_unknown_contract_version() {
    let preimage = identity_preimage(vec![]);
    let mut envelope = valid_envelope(&preimage);
    envelope["contract_version"] = json!("quire.checked-package/v1");
    let bytes = jcs(&envelope);
    let outcome = read(&bytes, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::UnsupportedVersion { .. })
        ),
        "expected Refused(UnsupportedVersion), got {outcome:?}"
    );
    assert_eq!(envelope_code(&outcome), Some(Code::UnknownWire));
}

#[test]
fn refuses_missing_member_as_malformed_wire() {
    let preimage = identity_preimage(vec![]);
    let mut envelope = valid_envelope(&preimage).as_object().unwrap().clone();
    envelope.remove("diagnostics");
    let bytes = jcs(&Value::Object(envelope));
    let outcome = read(&bytes, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::MalformedWire
        ),
        "expected Refused(Envelope(MalformedWire)), got {outcome:?}"
    );
}

#[test]
fn refuses_non_object_wire_as_malformed_wire() {
    let bytes = jcs(&json!(["not", "an", "object"]));
    let outcome = read(&bytes, &no_pins());
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::MalformedWire
        ),
        "expected Refused(Envelope(MalformedWire)), got {outcome:?}"
    );
}

/// Inject a duplicate/extra top-level member by textual mutation: the
/// `json!` macro's own `Map` cannot represent a duplicate key, so the
/// mutation happens on the serialized bytes, exactly as a hostile wire would
/// present it.
fn inject_top_level_member(bytes: &[u8], member: &str) -> Vec<u8> {
    let text = std::str::from_utf8(bytes).unwrap();
    let insert_at = text.find('{').expect("object") + 1;
    let mut mutated = String::with_capacity(text.len() + member.len());
    mutated.push_str(&text[..insert_at]);
    mutated.push_str(member);
    mutated.push_str(&text[insert_at..]);
    mutated.into_bytes()
}

#[test]
fn refuses_duplicate_top_level_member() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let mutated = inject_top_level_member(
        &bytes,
        &format!("\"contract_version\":\"{CHECKED_PACKAGE_V2}\","),
    );
    // IR's own duplicate-member path is `serde_json`'s error text (the
    // member name plus its line/column), not a bare member name, so only
    // the code is asserted -- not IR's `path` text (N3).
    let outcome = read(&mutated, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::DuplicateMember
        ),
        "expected Refused(Envelope(DuplicateMember)), got {outcome:?}"
    );
}

/// A wire whose bytes are not the RFC 8785 bytes of the document they hold
/// (one space after the opening brace) is refused by IR `noncanonical_wire`,
/// which maps to the native `Code::NoncanonicalWire`, not
/// `Code::InvalidPackage`.
#[trace("TC-429", "FR-096-AC-18")]
#[test]
fn maps_noncanonical_bytes_to_the_native_noncanonical_wire_code() {
    let preimage = identity_preimage(vec![]);
    let bytes = inject_top_level_member(&jcs(&valid_envelope(&preimage)), " ");
    let outcome = read(&bytes, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::NoncanonicalWire
        ),
        "expected Refused(Envelope(NoncanonicalWire)), got {outcome:?}"
    );
    assert_eq!(envelope_code(&outcome), Some(Code::NoncanonicalWire));
}

#[test]
fn refuses_unrecognized_top_level_member() {
    let preimage = identity_preimage(vec![]);
    let mut envelope = valid_envelope(&preimage);
    envelope["extra_member"] = json!(null);
    let bytes = jcs(&envelope);
    let outcome = read(&bytes, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::UnknownMember
        ),
        "expected Refused(Envelope(UnknownMember)), got {outcome:?}"
    );
}

/// A definition reference is exactly `{authority, identity}` and a source
/// reference exactly `{authority, identity, digest_domain, digest}`: a
/// `revision`, `digest_domain` or `digest` on the edition, a definition
/// selection or the diagnostics catalog, or a `revision` on a source row or
/// a region's source, refuses `unknown_member`. There is no reader for the
/// old shapes. The admitted envelope reads.
#[trace("TC-416", "FR-093-AC-17")]
#[test]
fn an_old_reference_shape_refuses_unknown_member() {
    let preimage = identity_preimage(vec![]);
    let envelope = valid_envelope(&preimage);
    assert!(matches!(
        read(&jcs(&envelope), &pinned_for(&preimage)),
        Read::Verified { .. }
    ));
    let revision = json!({"namespace": "semver", "value": "1"});
    let digest = json!(hex("bytes"));
    let domain = json!("quire.definition.bytes/v1");
    let definition_sites: [&[&str]; 3] = [
        &["lock", "edition", "definition"],
        &["lock", "definition_selections", "0"],
        &["diagnostics", "catalog"],
    ];
    let mut mutations: Vec<(String, Value)> = Vec::new();
    for site in definition_sites {
        for (member, value) in [
            ("revision", &revision),
            ("digest_domain", &domain),
            ("digest", &digest),
        ] {
            let mut mutated = envelope.clone();
            mutated["lock"]["definition_selections"] = json!([artifact_ref("rule")]);
            let mut at = &mut mutated;
            for key in site {
                at = match key.parse::<usize>() {
                    Ok(index) => &mut at[index],
                    Err(_) => &mut at[*key],
                };
            }
            at[member] = value.clone();
            mutations.push((format!("{} {member}", site.join(".")), mutated));
        }
    }
    let mut source_row = envelope.clone();
    source_row["lock"]["sources"][0]["revision"] = revision.clone();
    mutations.push(("lock.sources.0 revision".to_owned(), source_row));
    let mut region_source = envelope.clone();
    region_source["source_map"][0]["regions"][0]["source"]["revision"] = revision;
    mutations.push(("region source revision".to_owned(), region_source));
    for (name, mutated) in mutations {
        let outcome = read(&jcs(&mutated), &pinned_for(&preimage));
        assert!(
            matches!(
                &outcome,
                Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                    if refusal.code == CheckedPackageRefusalCode::UnknownMember
            ),
            "{name}: expected Refused(Envelope(UnknownMember)), got {outcome:?}"
        );
    }
}

#[test]
fn refuses_digest_domain_mismatch() {
    let preimage = identity_preimage(vec![]);
    let mut envelope = valid_envelope(&preimage);
    envelope["package_id"]["domain"] = json!("quire.definition.bytes/v1");
    let bytes = jcs(&envelope);
    let outcome = read(&bytes, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::DigestDomainMismatch
        ),
        "expected Refused(Envelope(DigestDomainMismatch)), got {outcome:?}"
    );
}

// No `#[trace]` tag: TC-253 step 3 requires a named digest-mismatch cause,
// which this outcome does not carry (IR-238 item 3, M3). Step 3 is backed at
// the `library` level instead, by `library::binding_tests`'s
// `refuses_a_package_id_that_does_not_recompute` (`PackageIdMismatch`).
#[test]
fn refuses_package_id_that_does_not_recompute() {
    let preimage = identity_preimage(vec![]);
    let mut envelope = valid_envelope(&preimage);
    envelope["package_id"]["digest"] = json!(hex("not-the-preimage"));
    let bytes = jcs(&envelope);
    // IR's own v2 reader recomputes and compares `package_id` before this
    // reader ever sees an admitted package (finding #6): a mismatch never
    // reaches `library::verify_package`'s own `PackageIdMismatch`, so this
    // asserts IR's refusal rather than a `Structural` one. IR's
    // `StaleDependency` covers a package that does not match the content
    // identity it names, which includes a `package_id` that does not
    // recompute.
    let outcome = read(&bytes, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.code == CheckedPackageRefusalCode::StaleDependency
        ),
        "expected Refused(Envelope(StaleDependency)), got {outcome:?}"
    );
}

#[test]
fn refuses_an_ambiguous_declaration_at_ir_intake() {
    // Two nodes declaring the same name. IR's v2 reader refuses the
    // ambiguity itself (`ambiguous_declaration` / `ambiguous-name`, IR #185)
    // before this reader's `library::project_declarations` sees an admitted
    // package, so this asserts IR's refusal and its FR-010 code.
    let mut preimage = identity_preimage(vec![]);
    preimage["identity_projection"] = json!([
        projection_node("pkg::R", "R"),
        projection_node("pkg::S", "R"),
    ]);
    let mut envelope = valid_envelope(&preimage);
    envelope["semantic_graph"]["nodes"] =
        json!([graph_node("pkg::R", "R"), graph_node("pkg::S", "R"),]);
    envelope["source_map"] = json!([
        {
            "node_id": node_ref("pkg::R"),
            "role": "declaration",
            "ordinal": 0,
            "regions": [{"source": source_ref("src"), "start": 0, "end": 1}],
        },
        {
            "node_id": node_ref("pkg::S"),
            "role": "declaration",
            "ordinal": 0,
            "regions": [{"source": source_ref("src"), "start": 1, "end": 2}],
        },
    ]);
    let bytes = jcs(&envelope);
    let outcome = read(&bytes, &pinned_for(&preimage));
    let Read::Refused(
        ref refusal @ V2ReadRefusal::Envelope {
            refusal: ref envelope,
            ..
        },
    ) = outcome
    else {
        panic!("expected Refused(Envelope(AmbiguousDeclaration)), got {outcome:?}");
    };
    assert_eq!(
        envelope.code,
        CheckedPackageRefusalCode::AmbiguousDeclaration
    );
    assert_eq!(refusal.code(), Code::AmbiguousDeclaration);
}

/// FR-322 `dependency_selections`: a lock and identity preimage
/// carrying two well-shaped `DependencySelection` entries is read as far as
/// binding them. With no dependency package supplied for them, IR's reader
/// refuses `missing_import` at the first entry (QSpec FR-322-AC-36, IR-289);
/// a package with a supplied closure is admitted in `emit::tests`. An entry
/// in the retired `Selection` shape is a wire-shape refusal located under
/// `/identity_preimage/dependency_selections/0`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn dependency_selections_reach_binding_and_refuse_unsupplied() {
    let dependencies = vec![
        dependency_selection("geometry", "geometry-pkg"),
        dependency_selection("units", "units-pkg"),
    ];
    let preimage = identity_preimage(dependencies);
    assert_ne!(
        PackageId::of_preimage(&jcs(&preimage)),
        PackageId::of_preimage(&jcs(&identity_preimage(vec![])))
    );
    let bytes = jcs(&valid_envelope(&preimage));
    match read(&bytes, &pinned_for(&preimage)) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            let V2ReadRefusal::Envelope { refusal: ir, .. } = &refusal else {
                unreachable!("matched above");
            };
            assert_eq!(ir.code, CheckedPackageRefusalCode::MissingImport);
            let Some(Locus::Artifact { pointer, .. }) = refusal.locus() else {
                panic!("an unsupplied entry is located, got {refusal:?}");
            };
            assert_eq!(pointer.as_str(), "/lock/dependency_selections/0");
        }
        other => panic!("expected missing_import, got {other:?}"),
    }

    let preimage = identity_preimage(vec![selection("profile", "dep-def")]);
    let bytes = jcs(&valid_envelope(&preimage));
    match read(&bytes, &pinned_for(&preimage)) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            let V2ReadRefusal::Envelope { refusal: ir, .. } = &refusal else {
                unreachable!("matched above");
            };
            assert_eq!(ir.code, CheckedPackageRefusalCode::MalformedWire);
            let Some(Locus::Artifact { pointer, .. }) = refusal.locus() else {
                panic!("a shape refusal is located, got {refusal:?}");
            };
            assert!(
                pointer
                    .as_str()
                    .starts_with("/identity_preimage/dependency_selections/0"),
                "{pointer:?}"
            );
        }
        other => panic!("expected a wire-shape refusal, got {other:?}"),
    }
}

#[test]
fn incomplete_when_bytes_exceed_the_ceiling() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let limits = V2ReadLimits {
        artifact_bytes: bytes.len() - 1,
        ..V2ReadLimits::default()
    };
    let outcome = read_v2(
        &bytes,
        identity("pkg"),
        limits,
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    );
    assert_eq!(
        outcome,
        Read::Limit(input_bytes(bytes.len() - 1, bytes.len()))
    );
}

/// A valid wire of at least `min_bytes`, made large by one export whose
/// qualified name is long (it appears in both the identity projection and
/// the semantic graph).
fn wire_of_at_least(min_bytes: usize) -> (Value, Vec<u8>) {
    let export = format!("R{}", "r".repeat(min_bytes / 2));
    let (preimage, envelope) = envelope_declaring(&[("pkg::R", &export)]);
    let bytes = jcs(&envelope);
    assert!(bytes.len() >= min_bytes);
    (preimage, bytes)
}

/// A caller-raised `artifact_bytes` is enforced as given by this
/// reader *and* by IR, which receives it as its own `bytes` ceiling. A valid
/// wire past the 16 MiB default is refused at the byte-length gate by
/// default, and verified under a ceiling raised to its length -- not
/// stopped by IR at a fixed byte ceiling of its own.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn a_caller_raised_artifact_bytes_ceiling_admits_a_valid_wire_past_the_default() {
    let default_bytes = V2ReadLimits::default().artifact_bytes;
    let (preimage, bytes) = wire_of_at_least(default_bytes + 1);
    let read_under = |limits: V2ReadLimits| {
        read_v2(
            &bytes,
            identity("pkg"),
            limits,
            &CheckedPackageEvidence::new(),
            &pinned_for(&preimage),
        )
    };

    assert_eq!(
        read_under(V2ReadLimits::default()),
        Read::Limit(input_bytes(default_bytes, bytes.len())),
        "the default ceiling must refuse an oversized read at the byte-length gate"
    );

    let raised = V2ReadLimits {
        artifact_bytes: bytes.len(),
        ..V2ReadLimits::default()
    };
    match read_under(raised) {
        Read::Verified {
            effective_limits, ..
        } => assert_eq!(effective_limits, raised),
        other => panic!("expected Verified under the raised ceiling, got {other:?}"),
    }
}

/// IR's own ceilings are the caller's too. A wire with more
/// semantic-graph nodes than IR's 10,000-node default is `Incomplete` at
/// IR's `nodes` meter under the default `nodes`, naming that bound, and
/// verified once the caller raises `nodes`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn a_caller_raised_ir_node_ceiling_admits_past_the_ir_default() {
    let default_nodes = V2ReadLimits::default().nodes;
    let count = usize::try_from(default_nodes).unwrap() + 1;
    let labels: Vec<(String, String)> = (0..count)
        .map(|index| (format!("pkg::N{index}"), format!("N{index}")))
        .collect();
    let exports: Vec<(&str, &str)> = labels
        .iter()
        .map(|(label, export)| (label.as_str(), export.as_str()))
        .collect();
    let (preimage, envelope) = envelope_declaring(&exports);
    let bytes = jcs(&envelope);
    let read_under = |limits: V2ReadLimits| {
        read_v2(
            &bytes,
            identity("pkg"),
            limits,
            &CheckedPackageEvidence::new(),
            &pinned_for(&preimage),
        )
    };

    // So many nodes exceed the 16 MiB byte default too; admit the bytes so
    // only IR's node ceiling is in play.
    let base = V2ReadLimits {
        artifact_bytes: bytes.len(),
        ..V2ReadLimits::default()
    };
    match read_under(base) {
        Read::Limit(exceeded) => {
            assert_eq!(exceeded.kind(), LimitKind::NodeCount);
            assert_eq!(exceeded.configured_bound(), default_nodes);
        }
        other => panic!("expected Incomplete(Limit(Nodes)) at IR's default, got {other:?}"),
    }

    let raised = V2ReadLimits {
        nodes: default_nodes + 1,
        ..base
    };
    match read_under(raised) {
        Read::Verified {
            effective_limits, ..
        } => assert_eq!(effective_limits, raised),
        other => panic!("expected Verified under the raised node ceiling, got {other:?}"),
    }
}

/// A `depth` above IR's fixed maximum is charged at that maximum, so a
/// verified read records `depth` as the maximum, not the requested value.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn a_verified_read_records_depth_as_the_charged_maximum() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let requested = V2ReadLimits {
        depth: usize::MAX,
        ..V2ReadLimits::default()
    };
    match read_v2(
        &bytes,
        identity("pkg"),
        requested,
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    ) {
        Read::Verified {
            effective_limits, ..
        } => assert_eq!(
            effective_limits,
            V2ReadLimits {
                depth: maximum_depth(),
                ..requested
            }
        ),
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// IR's fixed depth maximum, in [`V2ReadLimits::depth`]'s type.
fn maximum_depth() -> usize {
    usize::try_from(CheckedPackageReadLimits::MAXIMUM_DEPTH).unwrap()
}

/// Reaching a *caller-raised* ceiling (not just the default)
/// still refuses, naming the limit kind ([`LimitKind::InputBytes`]) and
/// the caller's own configured bound.
#[test]
fn reaching_a_caller_raised_artifact_bytes_ceiling_refuses_naming_the_kind_and_bound() {
    let raised = V2ReadLimits {
        artifact_bytes: V2ReadLimits::default().artifact_bytes + 1_000,
        ..V2ReadLimits::default()
    };
    let oversized = vec![b'x'; raised.artifact_bytes + 1];
    let outcome = read_v2(
        &oversized,
        identity("pkg"),
        raised,
        &CheckedPackageEvidence::new(),
        &no_pins(),
    );
    assert_eq!(
        outcome,
        Read::Limit(input_bytes(raised.artifact_bytes, oversized.len())),
        "must name the raised ceiling actually in force, not the original default"
    );
}

#[test]
fn incomplete_when_a_depth_ceiling_is_reached() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let limits = V2ReadLimits {
        depth: 1,
        ..V2ReadLimits::default()
    };
    match read_v2(
        &bytes,
        identity("pkg"),
        limits,
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    ) {
        Read::Limit(exceeded) if exceeded.kind() == LimitKind::NestingDepth => {}
        other => panic!("expected Incomplete(Limit(Depth)), got {other:?}"),
    }
}

#[test]
fn exact_selected_limits_admit_the_boundary() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let limits = V2ReadLimits {
        artifact_bytes: bytes.len(),
        ..V2ReadLimits::default()
    };
    let outcome = read_v2(
        &bytes,
        identity("pkg"),
        limits,
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    );
    assert!(matches!(outcome, Read::Verified { .. }));
}

#[test]
fn exact_depth_ceiling_admits_the_boundary() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));

    // Measure the envelope's actual IR-reported depth with the engine under
    // test itself, rather than a second, drifting reimplementation of IR's
    // own depth count (L5): an unreachably low ceiling forces
    // `Incomplete(Limit(Depth))`, whose `consumed` is IR's own count.
    let actual_depth = match read_v2(
        &bytes,
        identity("pkg"),
        V2ReadLimits {
            depth: 0,
            ..V2ReadLimits::default()
        },
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    ) {
        Read::Limit(exceeded) if exceeded.kind() == LimitKind::NestingDepth => exceeded.actual(),
        other => panic!("expected Incomplete(Limit(Depth)) at depth 0, got {other:?}"),
    };

    let admits = V2ReadLimits {
        depth: actual_depth as usize,
        ..V2ReadLimits::default()
    };
    let outcome = read_v2(
        &bytes,
        identity("pkg"),
        admits,
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    );
    assert!(
        matches!(outcome, Read::Verified { .. }),
        "expected Verified at the exact depth boundary, got {outcome:?}"
    );

    let refuses = V2ReadLimits {
        depth: (actual_depth - 1) as usize,
        ..V2ReadLimits::default()
    };
    match read_v2(
        &bytes,
        identity("pkg"),
        refuses,
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    ) {
        Read::Limit(exceeded) if exceeded.kind() == LimitKind::NestingDepth => {}
        other => panic!("expected Incomplete(Limit(Depth)) one below the boundary, got {other:?}"),
    }
}

/// Wraps `leaf` in `wraps` nested one-element JSON arrays, e.g.
/// `nested_array(2, json!(1))` is `[[1]]`.
fn nested_array(wraps: usize, leaf: Value) -> Value {
    let mut value = leaf;
    for _ in 0..wraps {
        value = Value::Array(vec![value]);
    }
    value
}

/// The scalar `1` inside `wraps` nested arrays, written as bytes so no deep
/// value is built or encoded on the test's stack: depth `wraps + 1` in IR's
/// units.
fn nested_array_bytes(wraps: usize) -> Vec<u8> {
    format!("{}1{}", "[".repeat(wraps), "]".repeat(wraps)).into_bytes()
}

/// The read of `bytes` under `limits` as a depth incompleteness: its
/// configured bound and measured depth.
fn depth_incompleteness(bytes: &[u8], limits: V2ReadLimits) -> (u64, u128) {
    match read_v2(
        bytes,
        identity("pkg"),
        limits,
        &CheckedPackageEvidence::new(),
        &no_pins(),
    ) {
        Read::Limit(exceeded) if exceeded.kind() == LimitKind::NestingDepth => {
            (exceeded.configured_bound(), exceeded.actual())
        }
        other => panic!("expected Incomplete(Limit(Depth)), got {other:?}"),
    }
}

#[trace("TC-088", "NFR-007-M-5")]
#[test]
fn depth_far_past_the_default_limit_is_incomplete_not_malformed_wire() {
    // A wire nested well past the default limit is a depth incompleteness
    // naming that limit and the measured depth, not a malformed-wire
    // refusal. Bare bytes, not a valid envelope: the depth check runs
    // before schema decode, on any well-formed JSON document.
    let limits = V2ReadLimits::default();
    assert_eq!(
        depth_incompleteness(&nested_array_bytes(200), limits),
        (u64::try_from(limits.depth).unwrap(), 201)
    );
}

#[trace("TC-088", "NFR-007-M-5")]
#[test]
fn depth_raised_past_the_default_admits_a_wire_deeper_than_the_default() {
    // A raised depth is charged as given: the same 201-deep wire that is
    // incomplete at the default passes the depth check at 300 and reaches
    // envelope decode, which refuses the root array as no envelope. That
    // refusal is at a value (the root pointer); a syntax refusal before the
    // depth check carries no pointer.
    let outcome = read_v2(
        &nested_array_bytes(200),
        identity("pkg"),
        V2ReadLimits {
            depth: 300,
            ..V2ReadLimits::default()
        },
        &CheckedPackageEvidence::new(),
        &no_pins(),
    );
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. })
                if refusal.path.as_ref().is_some_and(|path| path.as_str().is_empty())
        ),
        "expected the depth check to pass and the root value to be refused, got {outcome:?}"
    );
}

#[trace("TC-088", "NFR-007-M-5")]
#[test]
fn depth_past_the_charged_maximum_is_incomplete_naming_the_maximum() {
    // A caller depth above IR's fixed maximum is charged at the maximum: a
    // wire one level past it is incomplete, naming the maximum, not the
    // caller's requested limit.
    let limits = V2ReadLimits {
        depth: usize::MAX,
        ..V2ReadLimits::default()
    };
    let maximum = CheckedPackageReadLimits::MAXIMUM_DEPTH;
    assert_eq!(
        depth_incompleteness(&nested_array_bytes(maximum_depth()), limits),
        (maximum, u128::from(maximum) + 1)
    );
}

#[test]
fn depth_boundary_is_fail_closed_for_both_kinds_of_deepest_path() {
    // Decided fail-closed: `V2ReadLimits::depth` is passed to
    // IR unchanged, in entered-container units. IR's own depth count
    // counts a scalar leaf as one further unit beyond the containers
    // entered to reach it, but counts an empty container as exactly the
    // containers entered including itself -- so at one shared limit the two
    // kinds of deepest path disagree by one (module doc, "Ceilings").
    // Bare bytes, not a valid envelope: the depth check runs before schema
    // decode, on any well-formed JSON document.
    let limits = V2ReadLimits {
        depth: 3,
        ..V2ReadLimits::default()
    };
    let is_depth_incomplete = |bytes: &[u8]| {
        matches!(
            read_v2(
                bytes,
                identity("pkg"),
                limits,
                &CheckedPackageEvidence::new(),
                &no_pins()
            ),
            Read::Limit(exceeded) if exceeded.kind() == LimitKind::NestingDepth
        )
    };

    // Three entered containers, terminal empty container: exactly at the
    // limit, not refused for depth (fail-closed does not over-refuse).
    let empty_terminated_at_limit = jcs(&nested_array(2, json!([])));
    assert!(
        !is_depth_incomplete(&empty_terminated_at_limit),
        "an empty-container-terminated path exactly at the depth boundary must not be Incomplete(Limit(Depth))"
    );

    // Three entered containers, terminal scalar: one IR unit past the same
    // limit -- refused fail-closed, the accepted cost of removing `+ 1`.
    let scalar_terminated_at_limit = jcs(&nested_array(3, json!(1)));
    assert!(
        is_depth_incomplete(&scalar_terminated_at_limit),
        "a scalar-terminated path exactly at the depth boundary must be Incomplete(Limit(Depth))"
    );

    // Four entered containers, terminal empty container: one container past
    // the limit -- refused fail-closed. Under the old `+ 1` conversion this
    // was wrongly admitted (the M2 over-admission finding).
    let empty_terminated_past_limit = jcs(&nested_array(3, json!([])));
    assert!(
        is_depth_incomplete(&empty_terminated_past_limit),
        "an empty-container-terminated path one past the depth boundary must be Incomplete(Limit(Depth))"
    );
}

/// The occurrence key of `label`'s `declaration` occurrence.
fn declaration_key(label: &str) -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_hex(&hex(label)).unwrap(),
        Origin::new(Role::new("declaration"), 0),
    )
}

/// ADR-013 O-12, C-14: the verified read carries the wire's `source_map` as
/// the package source map. Each occurrence key maps to exactly its wire
/// regions, in wire order, under the wire's `RawSourceRef`; each node's
/// occurrences are exactly its own entries.
#[trace("TC-421", "FR-095-AC-3")]
#[test]
fn a_verified_read_carries_the_wire_source_map() {
    let (preimage, mut envelope) = envelope_declaring(&[("pkg::A", "A"), ("pkg::B", "B")]);
    let second = envelope["source_map"][1]["regions"][0].clone();
    envelope["source_map"][1]["regions"] = json!([
        {"source": source_ref("src"), "start": 7, "end": 9},
        second,
    ]);
    let source_map = match read(&jcs(&envelope), &pinned_for(&preimage)) {
        Read::Verified { source_map, .. } => source_map,
        other => panic!("expected Verified, got {other:?}"),
    };
    let entries = envelope["source_map"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        let node = WireNodeId::from_hex(entry["node_id"]["digest"].as_str().unwrap()).unwrap();
        let key = OccurrenceKey::new(node, Origin::new(Role::new("declaration"), 0));
        let regions = source_map
            .regions(&key)
            .unwrap_or_else(|| panic!("no regions for {key}"));
        let spans: Vec<(u64, u64)> = regions.iter().map(|r| (r.start(), r.end())).collect();
        let expected: Vec<(u64, u64)> = entry["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["start"].as_u64().unwrap(), r["end"].as_u64().unwrap()))
            .collect();
        assert_eq!(spans, expected);
        for region in regions {
            let source = region.source();
            assert_eq!((source.authority(), source.identity()), ("pkg", "src"));
            assert_eq!(source.digest().hex(), hex("src"));
        }
        assert_eq!(
            source_map
                .occurrences(node)
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>(),
            [key]
        );
    }
    assert_eq!(source_map.regions(&declaration_key("pkg::C")), None);
}

/// ADR-013 O-07: a source-map entry naming a node the graph does not hold
/// refuses at the read, as `invalid_source_map`.
#[trace("TC-421", "FR-095-AC-4")]
#[test]
fn a_source_map_entry_naming_an_unknown_node_refuses() {
    let (preimage, mut envelope) = envelope_declaring(&[("pkg::A", "A")]);
    envelope["source_map"][0]["node_id"] = node_ref("pkg::unknown");
    match read(&jcs(&envelope), &pinned_for(&preimage)) {
        Read::Refused(refusal) => {
            assert!(
                matches!(
                    &refusal,
                    V2ReadRefusal::Envelope { refusal: ir, .. }
                        if ir.code == CheckedPackageRefusalCode::InvalidSourceMap
                ),
                "{refusal:?}"
            );
            assert_eq!(refusal.code(), Code::InvalidSourceMap);
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// A source-map entry that IR admitted but this crate's provenance types
/// refuse -- here a reversed region and a non-hex node id, which IR's
/// reader never admits -- is `invalid_source_map`, carrying the typed
/// defect, never a panic or a silently dropped entry.
#[trace("TC-421", "FR-095-AC-3")]
#[test]
fn a_source_map_the_provenance_types_refuse_is_invalid_source_map() {
    let entry = |digest: &str, start: u64, end: u64| {
        serde_json::from_value::<quire_contract_model::CheckedSourceMapEntry>(json!({
            "node_id": {"digest": digest, "domain": NODE_DOMAIN},
            "role": "expression",
            "ordinal": 0,
            "regions": [{"source": source_ref("src"), "start": start, "end": end}],
        }))
        .unwrap()
    };
    let reversed = super::package_source_map(&[entry(&hex("pkg::A"), 9, 3)]).unwrap_err();
    assert_eq!(
        reversed,
        super::SourceMapDefect::Provenance(
            qsl_foundation::source::provenance::InvalidProvenance::ReversedRegion {
                start: 9,
                end: 3
            }
        )
    );
    let upper = hex("pkg::A").to_uppercase();
    let node = super::package_source_map(&[entry(&upper, 0, 1)]).unwrap_err();
    assert_eq!(node, super::SourceMapDefect::NodeId(upper.into()));
    for defect in [reversed, node] {
        assert_eq!(V2ReadRefusal::from(defect).code(), Code::InvalidSourceMap);
    }
}

/// ADR-013 O-07: every IR occurrence role maps to its FR-322 spelling, the
/// same string IR itself serializes the role as. The list names all six
/// FR-322 roles, so a swapped pair fails.
#[trace("TC-421", "FR-095-AC-3")]
#[test]
fn every_occurrence_role_is_spelled_as_ir_serializes_it() {
    use quire_contract_model::CheckedOccurrenceRole as R;
    for (role, spelling) in [
        (R::Declaration, "declaration"),
        (R::Type, "type"),
        (R::Expression, "expression"),
        (R::Anchor, "anchor"),
        (R::Claim, "claim"),
        (R::Generated, "generated"),
    ] {
        assert_eq!(super::role_spelling(&role), spelling);
        assert_eq!(super::occurrence_role(spelling), Some(role.clone()));
        assert_eq!(serde_json::to_value(&role).unwrap(), json!(spelling));
    }
    assert_eq!(super::occurrence_role("reference"), None);
}

/// The published QSpec checked-package-v2 fixture directory under
/// `$QSPEC_DIR`, and its `positive-*.json` fixtures in name order; `None`
/// when `QSPEC_DIR` is unset.
fn qspec_v2_fixtures() -> Option<(std::path::PathBuf, Vec<std::path::PathBuf>)> {
    let qspec = std::env::var_os("QSPEC_DIR")?;
    let directory = std::path::Path::new(&qspec).join("proposals/checked-package-v2/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("positive-") && name.ends_with(".json"))
        })
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "{} holds no positive fixture",
        directory.display()
    );
    Some((directory, paths))
}

fn read_fixture(path: &std::path::Path) -> Value {
    let bytes =
        std::fs::read(path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()))
}

/// QSL's whole I2 read of the fixture `envelope`, pinned at the
/// `package_id` its own identity preimage recomputes to. Evidence treats
/// the fixture's required features as supported. The published fixture is
/// pretty-printed; the wire is its canonical form.
fn read_fixture_wire(envelope: &Value) -> (PackageId, Read) {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in envelope["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let package_id = PackageId::of_preimage(&jcs(&envelope["identity_preimage"]));
    let outcome = read_v2(
        &jcs(envelope),
        identity("pkg"),
        V2ReadLimits::default(),
        &evidence,
        &pin(package_id),
    );
    (package_id, outcome)
}

/// ADR-013 C-14 over QSpec's published positive `quire.checked-package/v2`
/// fixtures, read at run time from
/// `$QSPEC_DIR/proposals/checked-package-v2/fixtures/positive-*.json`: QSL's
/// whole I2 read (`read_checked_package_v2`) admits each fixture, and every
/// one of its `source_map` entries looks up in the verified read's package
/// source map, by its occurrence key, to exactly its wire regions, while
/// each node's occurrences number exactly its entries.
///
/// Skipped (and passing) when `QSPEC_DIR` is unset; `make conformance`
/// requires it. Nothing of QSpec is copied into this repository.
#[trace("TC-421", "FR-095-AC-3")]
#[test]
fn conformance_c14_source_map_lookup_over_qspec_positive_fixtures() {
    let Some((_, paths)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let mut looked_up = 0_usize;
    for path in &paths {
        let envelope = read_fixture(path);
        let source_map = match read_fixture_wire(&envelope).1 {
            Read::Verified { source_map, .. } => source_map,
            other => panic!("{}: expected Verified, got {other:?}", path.display()),
        };
        let entries = envelope["source_map"].as_array().unwrap();
        let mut per_node = std::collections::BTreeMap::<WireNodeId, usize>::new();
        for entry in entries {
            let node = WireNodeId::from_hex(entry["node_id"]["digest"].as_str().unwrap()).unwrap();
            let key = OccurrenceKey::new(
                node,
                Origin::new(
                    Role::new(entry["role"].as_str().unwrap()),
                    entry["ordinal"].as_u64().unwrap(),
                ),
            );
            let regions = source_map
                .regions(&key)
                .unwrap_or_else(|| panic!("{}: no regions for {key}", path.display()));
            let wire = entry["regions"].as_array().unwrap();
            assert_eq!(regions.len(), wire.len(), "{}: {key}", path.display());
            for (region, wire) in regions.iter().zip(wire) {
                assert_eq!(region.start(), wire["start"].as_u64().unwrap());
                assert_eq!(region.end(), wire["end"].as_u64().unwrap());
                let source = region.source();
                assert_eq!(source.authority(), wire["source"]["authority"]);
                assert_eq!(source.identity(), wire["source"]["identity"]);
                assert_eq!(source.digest().hex(), wire["source"]["digest"]);
            }
            *per_node.entry(node).or_default() += 1;
            looked_up += 1;
        }
        for (node, count) in per_node {
            assert_eq!(
                source_map.occurrences(node).count(),
                count,
                "{}",
                path.display()
            );
        }
    }
    println!(
        "conformance: {looked_up} source-map entries over {} positive fixtures",
        paths.len()
    );
}

/// QSL's whole I2 read (`read_checked_package_v2`) admits every
/// published positive `quire.checked-package/v2` fixture under the
/// `package_id` its own identity preimage recomputes to. Three of the five
/// fixtures carry an `identity_projection` in graph order that is not
/// ascending by node id (QSpec FR-322-AC-14 makes that order part of the
/// id). Each published structural mutation of `positive-all-families.json`
/// still refuses with its expected cause.
///
/// Skipped (and passing) when `QSPEC_DIR` is unset; `make conformance`
/// requires it. Nothing of QSpec is copied into this repository.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn conformance_i2_read_over_qspec_checked_package_v2_fixtures() {
    let Some((directory, paths)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let mut unsorted = 0_usize;
    for path in &paths {
        let envelope = read_fixture(path);
        let digests: Vec<&str> = envelope["identity_preimage"]["identity_projection"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["node_id"]["digest"].as_str().unwrap())
            .collect();
        if !digests.is_sorted() {
            unsorted += 1;
        }
        match read_fixture_wire(&envelope) {
            (package_id, Read::Verified { package, .. }) => {
                assert_eq!(package.package_id(), package_id, "{}", path.display());
            }
            (_, other) => panic!("{}: expected Verified, got {other:?}", path.display()),
        }
    }
    assert!(
        unsorted > 0,
        "no positive fixture carries a non-ascending identity_projection"
    );
    println!(
        "conformance: {} positive fixtures through QSL's full I2 read ({unsorted} in non-ascending projection order)",
        paths.len()
    );

    let adverse = read_fixture(&directory.join("adverse.json"));
    let base = read_fixture(&directory.join("positive-all-families.json"));
    let mutations = adverse["structural_mutations"].as_array().unwrap();
    for mutation in mutations {
        let id = mutation["id"].as_str().unwrap();
        let expected = match mutation["outcome"].as_str().unwrap() {
            "refused:unknown_contract_version" => CheckedPackageRefusalCode::UnknownContractVersion,
            "refused:digest_domain_mismatch" => CheckedPackageRefusalCode::DigestDomainMismatch,
            "refused:unsupported_node_tag" => CheckedPackageRefusalCode::UnsupportedNodeTag,
            "refused:invalid_semantic_graph" => CheckedPackageRefusalCode::InvalidSemanticGraph,
            "refused:invalid_package" => CheckedPackageRefusalCode::InvalidPackage,
            other => panic!("{id}: unmapped adverse outcome {other}"),
        };
        let mut candidate = base.clone();
        *candidate
            .pointer_mut(mutation["pointer"].as_str().unwrap())
            .unwrap_or_else(|| panic!("{id}: pointer exists")) = mutation["replacement"].clone();
        let (_, outcome) = read_fixture_wire(&candidate);
        // FR-096: an unknown contract version is `unknown_wire`, located at
        // `/contract_version`; every other IR refusal keeps IR's code.
        let refused_as_expected = match &outcome {
            Read::Refused(refusal @ V2ReadRefusal::UnsupportedVersion { .. }) => {
                expected == CheckedPackageRefusalCode::UnknownContractVersion
                    && refusal
                        .locus()
                        .is_some_and(|locus| matches!(locus, Locus::Artifact { pointer, .. } if pointer.as_str() == "/contract_version"))
            }
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => refusal.code == expected,
            _ => false,
        };
        assert!(
            refused_as_expected,
            "{id}: expected a refusal as {expected:?}, got {outcome:?}"
        );
    }
    println!(
        "conformance: {} adverse mutations refused with their expected causes",
        mutations.len()
    );
    unknown_temporal_profile_is_unknown_profile(&base);
}

/// FR-250's five temporal profiles are the only identities a temporal
/// application's `temporal_profile` law may name. `base`'s clause, its law
/// renamed to `quire.fixture.temporal-profile/v1` and every node key and the
/// package identity recomputed, is refused by IR as `unknown_profile`
/// (cause `unsupported-selection`), which QSL's I2 read reports as
/// [`Code::UnknownProfile`], located at the law's definition.
fn unknown_temporal_profile_is_unknown_profile(base: &Value) {
    let application_key = |node: &Value| {
        package_id_digest(&json!({
            "version": "quire.application-node/v1",
            "node_tag": node["node_tag"],
            "semantic_form": node["semantic_form"],
            "semantic_type": node["semantic_type"],
            "declaration": node.get("declaration").cloned().unwrap_or(Value::Null),
            "recursion": Value::Null,
            "body": node["body"],
        }))
    };
    let nodes_of = |package: &Value| {
        package["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .clone()
    };
    let is_application = |node: &Value| node["body"]["term"] == "application";
    // The recomputation is measured against the published fixture first: every
    // application node's key is its own preimage's digest.
    for node in nodes_of(base).iter().filter(|node| is_application(node)) {
        assert_eq!(
            node["node_id"]["digest"].as_str().unwrap(),
            application_key(node),
            "the fixture's application keys recompute"
        );
    }
    let clause = nodes_of(base)
        .iter()
        .position(|node| {
            node["node_tag"] == "temporal" && node["semantic_form"] == "temporal_clause"
        })
        .expect("the all-families fixture holds a temporal clause");
    let mut package = base.clone();
    package["semantic_graph"]["nodes"][clause]["body"]["operation"]["laws"][0]["definition"]
        ["identity"] = json!("quire.fixture.temporal-profile/v1");
    // Rekey innermost first: each changed key is renamed wherever it is named.
    for _ in 0..=nodes_of(&package).len() {
        let mut changed = false;
        for node in nodes_of(&package)
            .iter()
            .filter(|node| is_application(node))
        {
            let fresh = application_key(node);
            let stale = node["node_id"]["digest"].as_str().unwrap().to_owned();
            if stale != fresh {
                package = serde_json::from_str(
                    &serde_json::to_string(&package)
                        .unwrap()
                        .replace(&stale, &fresh),
                )
                .unwrap();
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    let mut nodes = nodes_of(&package);
    for node in &mut nodes {
        if let Some(dependencies) = node["dependencies"].as_array_mut() {
            dependencies.sort_by(|a, b| a["digest"].as_str().cmp(&b["digest"].as_str()));
        }
    }
    package["semantic_graph"]["nodes"] = Value::Array(nodes.clone());
    package["identity_preimage"]["identity_projection"] = Value::Array(
        nodes
            .into_iter()
            .map(|mut node| {
                node.as_object_mut().unwrap().remove("occurrences");
                node
            })
            .collect(),
    );
    package["package_id"]["digest"] = json!(package_id_digest(&package["identity_preimage"]));
    let (_, outcome) = read_fixture_wire(&package);
    let Read::Refused(refusal @ V2ReadRefusal::Envelope { refusal: ir, .. }) = &outcome else {
        panic!("expected an unknown_profile refusal, got {outcome:?}");
    };
    assert_eq!(ir.code, CheckedPackageRefusalCode::UnknownProfile, "{ir:?}");
    assert_eq!(refusal.code(), Code::UnknownProfile);
    assert!(
        matches!(refusal.locus(), Some(Locus::Artifact { pointer, .. })
            if pointer.as_str()
                == format!("/semantic_graph/nodes/{clause}/body/operation/laws/0/definition")),
        "{refusal:?}"
    );
    println!("conformance: an unknown temporal profile reads as unknown_profile");
}

/// Over QSpec's `dependency-selection-vectors.json` (FR-322-AC-35,
/// QSpec TC-233), read at run time from `$QSPEC_DIR`: the published
/// `DependencySelection` entries, inserted into the all-families fixture's
/// lock and identity preimage, recompute the recorded `package_id` and pass
/// QSL's I2 read up to binding the entries to supplied packages (see the
/// body); each authored entry mutation is refused at the
/// mutated entry (a shape mutation in either member, a digest-domain
/// mutation in both); and each order vector gets its recorded outcome, a
/// refusal located at its last recorded locus (the repeating or misordered
/// entry). Skipped when `QSPEC_DIR` is unset; `make conformance` requires it.
#[trace("TC-253", "TC-416", "FR-087-AC-3", "FR-093-AC-16")]
#[test]
fn conformance_dependency_selection_vectors() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let vectors = read_fixture(&directory.join("../dependency-selection-vectors.json"));
    let base = read_fixture(
        &directory.join(
            vectors["base"]
                .as_str()
                .unwrap()
                .trim_start_matches("fixtures/"),
        ),
    );
    let with = |lock: &Value, preimage: &Value| {
        let mut envelope = base.clone();
        envelope["lock"]["dependency_selections"] = lock.clone();
        envelope["identity_preimage"]["dependency_selections"] = preimage.clone();
        envelope["package_id"]["digest"] =
            json!(PackageId::of_preimage(&jcs(&envelope["identity_preimage"])).hex());
        envelope
    };
    // The vectors' `package_id`s are placeholders no admitted package has,
    // so since IR-289 (QSpec FR-322-AC-36) a read that gets past the entries'
    // shape and order binds the first entry to a supplied package and
    // refuses `missing_import` there. A vector recorded `admitted` is checked
    // to that point: its shape and order are admitted. QSpec STD-107 owns
    // vectors with supplied dependency packages.
    let unsupplied = |id: &str, outcome: &Read| match outcome {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { refusal: ir, .. }) => {
            assert_eq!(
                ir.code,
                CheckedPackageRefusalCode::MissingImport,
                "{id}: {ir:?}"
            );
            assert!(
                matches!(refusal.locus(), Some(Locus::Artifact { pointer, .. })
                    if pointer.as_str() == "/lock/dependency_selections/0"),
                "{id}: {refusal:?}"
            );
        }
        other => panic!("{id}: expected missing_import at the first entry, got {other:?}"),
    };
    let selections = &vectors["dependency_selections"];
    let (package_id, outcome) = read_fixture_wire(&with(selections, selections));
    assert_eq!(package_id.hex(), vectors["package_id"].as_str().unwrap());
    unsupplied("dependency_selections", &outcome);

    let refused_at = |outcome: &Read, bytes: &[u8]| -> (CheckedPackageRefusalCode, String) {
        match outcome {
            Read::Refused(refusal @ V2ReadRefusal::Envelope { refusal: ir, .. }) => {
                let Some(Locus::Artifact { pointer, digest }) = refusal.locus() else {
                    panic!("an entry refusal is located, got {refusal:?}");
                };
                assert_eq!(
                    digest,
                    &DigestRecord::mint(
                        DigestDomain::RawArtifactDigest,
                        Sha256::digest(bytes).into(),
                    )
                );
                (ir.code, pointer.as_str().to_owned())
            }
            other => panic!("expected an envelope refusal, got {other:?}"),
        }
    };
    let mut mutations = 0_usize;
    for mutation in vectors["entry_mutations"].as_array().unwrap() {
        let id = mutation["id"].as_str().unwrap();
        let mut mutated = selections.clone();
        mutated[0] = mutation["entry"].clone();
        let cases: Vec<(Value, &str, CheckedPackageRefusalCode)> =
            match mutation["outcome"].as_str().unwrap() {
                "refused:malformed_wire" => vec![
                    (
                        with(&mutated, selections),
                        "/lock",
                        CheckedPackageRefusalCode::MalformedWire,
                    ),
                    (
                        with(selections, &mutated),
                        "/identity_preimage",
                        CheckedPackageRefusalCode::MalformedWire,
                    ),
                ],
                "refused:digest_domain_mismatch" => vec![(
                    with(&mutated, &mutated),
                    "/lock",
                    CheckedPackageRefusalCode::DigestDomainMismatch,
                )],
                other => panic!("{id}: unmapped entry-mutation outcome {other}"),
            };
        for (envelope, member, expected) in cases {
            let bytes = jcs(&envelope);
            let (code, pointer) = refused_at(&read_fixture_wire(&envelope).1, &bytes);
            assert_eq!(code, expected, "{id} in {member}");
            assert!(
                pointer.starts_with(&format!("{member}/dependency_selections/0")),
                "{id} in {member}: refused at {pointer}"
            );
            mutations += 1;
        }
    }

    let order_vectors = vectors["order_vectors"].as_array().unwrap();
    for vector in order_vectors {
        let id = vector["id"].as_str().unwrap();
        let entries = &vector["dependency_selections"];
        let envelope = with(entries, entries);
        let bytes = jcs(&envelope);
        let (_, outcome) = read_fixture_wire(&envelope);
        let expected = vector["outcome"].as_str().unwrap();
        if expected == "admitted" {
            unsupplied(id, &outcome);
            continue;
        }
        let cause = match expected {
            "refused:invalid_package/conflicting-definition" => {
                quire_contract_model::CheckedPackageRefusalCause::ConflictingDefinition
            }
            "refused:invalid_package/invalid-value" => {
                quire_contract_model::CheckedPackageRefusalCause::InvalidValue
            }
            other => panic!("{id}: unmapped order outcome {other}"),
        };
        let Read::Refused(V2ReadRefusal::Envelope { refusal: ir, .. }) = &outcome else {
            panic!("{id}: expected an envelope refusal, got {outcome:?}");
        };
        assert_eq!(ir.cause, Some(cause), "{id}");
        let (code, pointer) = refused_at(&outcome, &bytes);
        assert_eq!(code, CheckedPackageRefusalCode::InvalidPackage, "{id}");
        let loci = vector["loci"].as_array().unwrap();
        assert_eq!(
            Some(pointer.as_str()),
            loci.last().and_then(Value::as_str),
            "{id}: refused at the repeating or misordered entry"
        );
    }
    println!(
        "conformance: {} dependency-selection entry mutations and {} order vectors",
        mutations,
        order_vectors.len()
    );
}

/// `Locus::Artifact` over `bytes`, computed here from the bytes themselves
/// (FR-201 `raw-artifact-digest`: the SHA-256 of the complete supplied
/// bytes), at `pointer`.
fn artifact_locus(bytes: &[u8], pointer: &str) -> Locus {
    Locus::Artifact {
        digest: DigestRecord::mint(
            DigestDomain::RawArtifactDigest,
            Sha256::digest(bytes).into(),
        ),
        pointer: pointer.parse().unwrap(),
    }
}

/// FR-096-AC-9: IR's refusal of the contract version is
/// `unknown_wire`/`unsupported-wire`, naming the version the bytes carry and
/// the one this reader admits, located at the bytes' `raw-artifact-digest`
/// and `/contract_version`. Bytes that are not JSON refuse with no locus.
#[trace("TC-429", "FR-096-AC-9")]
#[test]
fn an_unknown_contract_version_is_unsupported_wire_at_contract_version() {
    let preimage = identity_preimage(vec![]);
    let mut envelope = valid_envelope(&preimage);
    envelope["contract_version"] = json!("quire.checked-package/v3");
    let bytes = jcs(&envelope);
    let Read::Refused(refusal) = read(&bytes, &pinned_for(&preimage)) else {
        panic!("an unknown contract version must refuse");
    };
    let expected_locus = artifact_locus(&bytes, "/contract_version");
    assert_eq!(refusal.locus(), Some(&expected_locus));
    let record = refusal
        .unsupported_wire_record()
        .expect("an unknown contract version is unsupported-wire");
    assert_eq!(
        record.code(),
        CatalogCode::new("unknown_wire", "unsupported-wire")
    );
    assert_eq!(record.category(), Category::Refusal);
    assert_eq!(record.locus(), Some(&expected_locus));
    assert_eq!(
        record.fields(),
        &BTreeMap::from([
            ("actual", "quire.checked-package/v3".to_owned()),
            ("expected", "quire.checked-package/v2".to_owned()),
        ])
    );

    let not_json = b"not json";
    match read(not_json, &no_pins()) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            assert_eq!(refusal.locus(), None);
            assert_eq!(refusal.unsupported_wire_record(), None);
        }
        other => panic!("bytes that are not JSON must refuse, got {other:?}"),
    }
}

/// FR-096 "The I2 reader locates its refusals in the artifact": an IR
/// refusal at a value is located at the bytes' `raw-artifact-digest` and
/// the pointer of that value -- here a `package_id` digest domain and a
/// graph node's tag. A repeated dependency identity is located at the
/// repeating entry, `/lock/dependency_selections/1` (FR-322).
#[trace("TC-429", "FR-096-AC-9")]
#[test]
fn a_refusal_at_a_value_is_located_at_its_pointer() {
    let preimage = identity_preimage(vec![]);
    let mut domain = valid_envelope(&preimage);
    domain["package_id"]["domain"] = json!(SOURCE_DOMAIN);
    let mut tag = valid_envelope(&preimage);
    tag["semantic_graph"]["nodes"][0]["node_tag"] = json!("nonsense");
    for (envelope, code, pointer) in [
        (
            domain,
            CheckedPackageRefusalCode::DigestDomainMismatch,
            "/package_id/domain",
        ),
        (
            tag,
            CheckedPackageRefusalCode::UnsupportedNodeTag,
            "/semantic_graph/nodes/0/node_tag",
        ),
    ] {
        let bytes = jcs(&envelope);
        match read(&bytes, &pinned_for(&preimage)) {
            Read::Refused(refused @ V2ReadRefusal::Envelope { .. }) => {
                let V2ReadRefusal::Envelope { refusal, .. } = &refused else {
                    unreachable!("matched above");
                };
                assert_eq!(refusal.code, code);
                assert_eq!(refused.locus(), Some(&artifact_locus(&bytes, pointer)));
            }
            other => panic!("expected an envelope refusal at {pointer}, got {other:?}"),
        }
    }

    let preimage = identity_preimage(vec![
        dependency_selection("geometry", "geometry-pkg"),
        dependency_selection("geometry", "geometry-pkg-2"),
    ]);
    let bytes = jcs(&valid_envelope(&preimage));
    match read(&bytes, &pinned_for(&preimage)) {
        Read::Refused(refusal @ V2ReadRefusal::Envelope { .. }) => {
            let V2ReadRefusal::Envelope { refusal: ir, .. } = &refusal else {
                unreachable!("matched above");
            };
            assert_eq!(ir.code, CheckedPackageRefusalCode::InvalidPackage);
            assert_eq!(
                refusal.locus(),
                Some(&artifact_locus(&bytes, "/lock/dependency_selections/1"))
            );
        }
        other => panic!("expected a repeated identity to refuse, got {other:?}"),
    }
}

/// FR-096-AC-10 at catalog revision `1-draft.8`: each of IR's reader limits
/// the reader can reach, lowered below a real fixture, stops the read with
/// its own kind, the configured bound, IR's counter as actual, and
/// `Locus::Artifact` at the bytes' `raw-artifact-digest` and the pointer IR
/// reports. The reader's own artifact byte ceiling reports input bytes with
/// no locus.
#[trace("TC-429", "FR-096-AC-10")]
#[test]
fn each_reader_limit_names_its_kind_bound_actual_and_locus() {
    let defaults = V2ReadLimits::default();
    let preimage = identity_preimage(vec![]);
    let base = valid_envelope(&preimage);
    let (two_preimage, two_nodes) = envelope_declaring(&[("pkg::A", "A"), ("pkg::B", "B")]);
    let mut diagnosed = base.clone();
    diagnosed["diagnostics"]["entries"] = json!([{
        "stage": "type_checking",
        "code": "invalid_syntax",
        "cause_tag": "malformed-json",
        "details": [],
        "loci": [],
    }]);
    let mut edge_preimage = preimage.clone();
    edge_preimage["identity_projection"][0]["dependencies"] = json!([node_ref("pkg::R")]);
    let mut edged = valid_envelope(&edge_preimage);
    edged["semantic_graph"]["nodes"][0]["dependencies"] = json!([node_ref("pkg::R")]);

    let cases = [
        (
            &base,
            &preimage,
            V2ReadLimits {
                depth: 1,
                ..defaults
            },
            LimitKind::NestingDepth,
            1,
            7,
            "/capability_report",
        ),
        (
            &two_nodes,
            &two_preimage,
            V2ReadLimits {
                nodes: 1,
                ..defaults
            },
            LimitKind::NodeCount,
            1,
            2,
            "/semantic_graph/nodes/1",
        ),
        (
            &edged,
            &edge_preimage,
            V2ReadLimits {
                edges: 0,
                ..defaults
            },
            LimitKind::EdgeCount,
            0,
            1,
            "/semantic_graph/nodes/0/dependencies/0",
        ),
        (
            &base,
            &preimage,
            V2ReadLimits {
                occurrences: 0,
                ..defaults
            },
            LimitKind::OccurrenceCount,
            0,
            1,
            "/source_map/0",
        ),
        (
            &diagnosed,
            &preimage,
            V2ReadLimits {
                diagnostics: 0,
                ..defaults
            },
            LimitKind::DiagnosticCount,
            0,
            1,
            "/diagnostics/entries/0",
        ),
        (
            &base,
            &preimage,
            V2ReadLimits {
                work: 0,
                ..defaults
            },
            LimitKind::WorkBudget,
            0,
            1,
            "/semantic_graph/nodes/0/body",
        ),
    ];
    for (envelope, preimage, limits, kind, bound, actual, pointer) in cases {
        let bytes = jcs(envelope);
        let outcome = read_v2(
            &bytes,
            identity("pkg"),
            limits,
            &CheckedPackageEvidence::new(),
            &pinned_for(preimage),
        );
        let expected =
            LimitExceeded::new(kind, bound, actual).at(Some(artifact_locus(&bytes, pointer)));
        assert_eq!(outcome, Read::Limit(expected.clone()), "{kind:?}");
        assert_eq!(
            expected.catalog_code(),
            CatalogCode::new("stage_limit_exceeded", kind.catalog_cause())
        );
    }

    let bytes = jcs(&base);
    let outcome = read_v2(
        &bytes,
        identity("pkg"),
        V2ReadLimits {
            artifact_bytes: bytes.len() - 1,
            ..defaults
        },
        &CheckedPackageEvidence::new(),
        &pinned_for(&preimage),
    );
    let Read::Limit(exceeded) = outcome else {
        panic!("the artifact byte ceiling must stop the read, got {outcome:?}");
    };
    assert_eq!(exceeded.kind(), LimitKind::InputBytes);
    assert_eq!(exceeded.locus(), None);
}

/// The single `state`/`frame` node of a QSpec `checked-package/v2` fixture.
/// Asserts there is exactly one, rather than silently taking the first of
/// several.
fn frame_node_index(package: &Value) -> usize {
    let mut frames = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .enumerate()
        .filter(|(_, node)| node["node_tag"] == "state" && node["semantic_form"] == "frame");
    let (index, _) = frames
        .next()
        .expect("fixture carries exactly one frame node");
    assert!(
        frames.next().is_none(),
        "fixture carries exactly one frame node"
    );
    index
}

/// QSpec's `frame_mutations` spelling of `dependencies`/`creates`/`deletes`
/// (a bare hex digest per entry) as `state`/`frame` wire node-key entries
/// (`{domain, digest}`). `name` is the vector's own name, named in the panic
/// message when an entry is not the plain-digest shape this vector schema
/// version publishes.
fn frame_entries(name: &str, digests: &[Value]) -> Value {
    Value::Array(
        digests
            .iter()
            .map(|digest| {
                let digest = digest.as_str().unwrap_or_else(|| {
                    panic!("{name}: frame_mutations entry is not a plain digest string: {digest}")
                });
                json!({"domain": NODE_DOMAIN, "digest": digest})
            })
            .collect(),
    )
}

/// QSpec's `frame_mutations` spelling of `modifies` (STD-111's 30-vector
/// shape: each entry a `{declaration, kind}` object, `kind: "field"` also
/// carrying `name`) as `state`/`frame` `modifies` wire entries (`{kind,
/// declaration: {domain, digest}, [name]}`, `frame.rs`'s `ModifiesEntry`
/// shape). `name` is the vector's own name, named in the panic message when
/// an entry does not carry this schema version's `declaration`/`kind`
/// members.
fn modifies_entries(name: &str, entries: &[Value]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|entry| {
                let declaration = entry["declaration"].as_str().unwrap_or_else(|| {
                    panic!("{name}: frame_mutations modifies entry has no string declaration: {entry}")
                });
                let kind = entry["kind"].as_str().unwrap_or_else(|| {
                    panic!("{name}: frame_mutations modifies entry has no string kind: {entry}")
                });
                let declaration = json!({"domain": NODE_DOMAIN, "digest": declaration});
                match kind {
                    "field" => {
                        let field_name = entry["name"].as_str().unwrap_or_else(|| {
                            panic!(
                                "{name}: frame_mutations modifies field entry has no string name: {entry}"
                            )
                        });
                        modifies_field(&declaration, field_name)
                    }
                    "relationship" => modifies_relationship(&declaration),
                    other => panic!("{name}: frame_mutations modifies entry has unmapped kind {other}"),
                }
            })
            .collect(),
    )
}

/// `nodes` mapped through `CheckedNodeProjectionV2::from`'s own definition
/// of an identity projection entry: each node minus its `occurrences` (see
/// this module's `node_fields` doc). Shared by [`refresh_frame_identity`]
/// (which rebuilds an existing envelope's projection from its own graph) and
/// [`valid_envelope_over`] (which builds one fresh).
fn projection_of(nodes: &[Value]) -> Vec<Value> {
    nodes
        .iter()
        .cloned()
        .map(|mut node| {
            node.as_object_mut()
                .expect("node object")
                .remove("occurrences");
            node
        })
        .collect()
}

/// Rebuilds `envelope`'s `identity_preimage.identity_projection` from its
/// current `semantic_graph.nodes`, and recomputes the envelope's own
/// `package_id` over the refreshed preimage. Every other `identity_preimage`
/// field is left as the fixture's own, so a frame-only mutation changes no
/// other identity input.
fn refresh_frame_identity(envelope: &mut Value) {
    let projection = projection_of(
        envelope["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes"),
    );
    envelope["identity_preimage"]["identity_projection"] = Value::Array(projection);
    envelope["package_id"]["digest"] =
        json!(PackageId::of_preimage(&jcs(&envelope["identity_preimage"])).hex());
}

/// FR-340: QSL's whole I2 read admits or refuses a `state`/`frame`
/// node's `modifies`/`creates`/`deletes` membership exactly as QSpec's
/// `node-identity-vectors.json` `frame_mutations` record: the closed
/// per-member node-kind eligibility table, the
/// `missing_declaration`/`missing-name` vs
/// `invalid_model_binding`/`malformed-declaration` refusal split, and their
/// precedence over each other and over a canonical-order defect (a
/// meaning-join defect always wins; the lower-keyed of two defective frames
/// is reported over a higher-keyed frame's stronger defect). Each vector
/// replaces `positive-all-families.json`'s own frame node's `dependencies`
/// and body members with its own, adds its `second_frame` node where one is
/// given, and recomputes the envelope's identity the way FR-322's own
/// emitter would. The check runs entirely through the existing I2 dispatch
/// (`read_checked_package_v2` delegates frame-body admission to IR's
/// `frame_defect`, already mapped by `map_refusal_code`); no new admission
/// path is added for it.
///
/// Read at run time from `$QSPEC_DIR`; skipped (and passing) when unset.
/// `make conformance` requires it. Nothing of QSpec is copied into this
/// repository. Like this file's other `QSPEC_DIR`-gated conformance checks,
/// the vectors this covers (including FR-340-AC-1, AC-4 and AC-9: an
/// all-empty frame body, a misordered member array, and the lower-digest
/// frame of two defective ones winning) run only when `make conformance` is
/// part of the gate, not under a plain `cargo test`/`make ci`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn conformance_fr340_frame_mutations_match_qspec_vectors() {
    let Some((directory, _)) = qspec_v2_fixtures() else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let vectors = read_fixture(&directory.join("../node-identity-vectors.json"));
    let mutations = vectors["frame_mutations"]
        .as_array()
        .expect("frame_mutations");
    assert!(
        !mutations.is_empty(),
        "node-identity-vectors.json holds no frame_mutations vector"
    );
    let base = read_fixture(&directory.join("positive-all-families.json"));
    let index = frame_node_index(&base);

    for mutation in mutations {
        let name = mutation["name"].as_str().expect("name");
        let mut candidate = base.clone();
        {
            let node = &mut candidate["semantic_graph"]["nodes"][index];
            node["dependencies"] = frame_entries(
                name,
                mutation["dependencies"].as_array().expect("dependencies"),
            );
            node["body"]["modifies"] =
                modifies_entries(name, mutation["modifies"].as_array().expect("modifies"));
            node["body"]["creates"] =
                frame_entries(name, mutation["creates"].as_array().expect("creates"));
            node["body"]["deletes"] =
                frame_entries(name, mutation["deletes"].as_array().expect("deletes"));
        }
        if let Some(second_frame) = mutation.get("second_frame") {
            candidate["semantic_graph"]["nodes"]
                .as_array_mut()
                .expect("nodes")
                .push(second_frame.clone());
        }
        refresh_frame_identity(&mut candidate);

        let expected_code = match mutation["expected_code"].as_str().expect("expected_code") {
            "missing_declaration" => CheckedPackageRefusalCode::MissingDeclaration,
            "invalid_model_binding" => CheckedPackageRefusalCode::InvalidModelBinding,
            "invalid_semantic_graph" => CheckedPackageRefusalCode::InvalidSemanticGraph,
            other => panic!("{name}: unmapped expected_code {other}"),
        };
        let expected_cause = match mutation["expected_cause"].as_str() {
            Some("missing-name") => Some(CheckedPackageRefusalCause::MissingName),
            Some("malformed-declaration") => Some(CheckedPackageRefusalCause::MalformedDeclaration),
            None => None,
            Some(other) => panic!("{name}: unmapped expected_cause {other}"),
        };
        let expected_locus = mutation["expected_locus_digest"]
            .as_str()
            .expect("expected_locus_digest");

        let (_, outcome) = read_fixture_wire(&candidate);
        match outcome {
            Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => {
                assert_eq!(refusal.code, expected_code, "{name}: code");
                assert_eq!(refusal.cause, expected_cause, "{name}: cause");
                assert_eq!(
                    refusal.locus.as_ref().map(|node| node.digest.as_ref()),
                    Some(expected_locus),
                    "{name}: locus"
                );
            }
            other => panic!("{name}: expected a refusal, got {other:?}"),
        }
    }
    println!(
        "conformance: {} frame-body mutation vectors matched",
        mutations.len()
    );
}

/// A `model`/`object_type` node (FR-340's `creates`/`deletes`-eligible, and
/// -- as a `modifies` field entry's `declaration` -- `modifies`-eligible,
/// model member kind), typed by `type_ref`, with the minimal valid
/// empty-`aggregate` body real fixtures use.
fn model_node(label: &str, form: &str, qualified_name: &str, type_ref: &Value) -> Value {
    json!({
        "node_id": node_ref(label),
        "schema_version": "quire.checked-semantic-graph/v2",
        "node_tag": "model",
        "semantic_form": form,
        "semantic_type": type_ref,
        "declaration": {"qualified_name": [qualified_name]},
        "dependencies": [],
        "body": {"term": "aggregate", "members": []},
    })
}

fn model_graph_node(label: &str, form: &str, qualified_name: &str, type_ref: &Value) -> Value {
    let mut node = model_node(label, form, qualified_name, type_ref);
    node["occurrences"] = json!([{"role": "declaration", "ordinal": 0}]);
    node
}

/// A `state`/`frame` `modifies` entry naming a `relation`/`relationship`
/// node (STD-111's `{kind: "relationship", declaration}` shape).
fn modifies_relationship(declaration: &Value) -> Value {
    json!({"kind": "relationship", "declaration": declaration})
}

/// A `state`/`frame` `modifies` entry naming a field of `declaration`, a
/// `model`/`object_type` or `model`/`record_value_type` node (STD-111's
/// `{kind: "field", declaration, name}` shape).
fn modifies_field(declaration: &Value, name: &str) -> Value {
    json!({"kind": "field", "declaration": declaration, "name": name})
}

/// A `state`/`frame` node whose `dependencies` and body members are given
/// verbatim.
fn frame_node(
    label: &str,
    type_ref: &Value,
    dependencies: Vec<Value>,
    modifies: Vec<Value>,
    creates: Vec<Value>,
    deletes: Vec<Value>,
) -> Value {
    json!({
        "node_id": node_ref(label),
        "schema_version": "quire.checked-semantic-graph/v2",
        "node_tag": "state",
        "semantic_form": "frame",
        "semantic_type": type_ref,
        "dependencies": dependencies,
        // A `state`/`frame` node's own closed occurrence vocabulary is
        // `generated` (`structural.rs`'s `Frame => CheckedOccurrenceRole::
        // Generated`), not `declaration`.
        "occurrences": [{"role": "generated", "ordinal": 0}],
        "body": {
            "term": "frame",
            "modifies": modifies,
            "creates": creates,
            "deletes": deletes,
        },
    })
}

/// A source-map entry for the node whose `node_id.digest` is `digest`'s own
/// occurrence of `role` (a `state`/`frame` node's own closed occurrence
/// vocabulary is `generated`, not `declaration`; every other node kind these
/// fixtures build uses `declaration`). Takes the digest directly (never a
/// label re-hashed through [`node_ref`]) so it names exactly the node a
/// caller already built.
fn source_map_entry_for_digest(digest: &str, role: &str) -> Value {
    json!({
        "node_id": {"domain": NODE_DOMAIN, "digest": digest},
        "role": role,
        "ordinal": 0,
        "regions": [{"source": source_ref("src"), "start": 0, "end": 1}],
    })
}

/// A structurally valid `quire.checked-package/v2` envelope over `nodes`
/// (each already carrying its own `occurrences`), with a correctly
/// recomputed `identity_projection` and `package_id`. Extends
/// [`valid_envelope`] to more than the fixed single scalar node, for FR-340
/// frame-body tests that need field/object-type members alongside their
/// frame.
fn valid_envelope_over(nodes: Vec<Value>) -> Value {
    let projection = projection_of(&nodes);
    let preimage = json!({
        "definition_selections": [],
        "dependency_selections": [],
        "edition": selection("edition", "edition-def"),
        "identity_projection": projection,
        "model_selections": [],
        "profile_selections": [],
        "required_features": [],
        "version": "quire.checked-package-id/v2",
    });
    json!({
        "capability_report": [],
        "contract_version": CHECKED_PACKAGE_V2,
        "diagnostics": diagnostics(),
        "identity_preimage": preimage.clone(),
        "lock": lock(vec![]),
        "package_id": package_id_member(&preimage),
        "semantic_graph": {
            "graph_version": "quire.checked-semantic-graph/v2",
            "nodes": nodes,
        },
        "source_map": nodes_source_map(&nodes),
    })
}

fn nodes_source_map(nodes: &[Value]) -> Value {
    Value::Array(
        nodes
            .iter()
            .map(|node| {
                let digest = node["node_id"]["digest"].as_str().unwrap();
                let role = node["occurrences"][0]["role"].as_str().unwrap();
                source_map_entry_for_digest(digest, role)
            })
            .collect(),
    )
}

/// A minimal, always-run (no `QSPEC_DIR` needed) FR-340 frame-body package:
/// a self-typed scalar type `T` and a `model`/`object_type` `O`, typed by
/// `T`, and a `state`/`frame` naming `O` as its only `dependencies` entry.
/// `frame_body` supplies the frame's own `modifies`/`creates`/`deletes`.
fn frame_fixture(frame_body: impl FnOnce(&Value) -> (Vec<Value>, Vec<Value>, Vec<Value>)) -> Value {
    let type_ref = node_ref("pkg::T");
    let object_ref = node_ref("pkg::O");
    let (modifies, creates, deletes) = frame_body(&object_ref);
    let type_node = graph_node("pkg::T", "T");
    let object_node = model_graph_node("pkg::O", "object_type", "O", &type_ref);
    let frame = frame_node(
        "pkg::Frame",
        &object_ref,
        vec![object_ref.clone()],
        modifies,
        creates,
        deletes,
    );
    valid_envelope_over(vec![type_node, object_node, frame])
}

fn read_frame_fixture(envelope: &Value) -> Read {
    let (_, outcome) = read_fixture_wire(envelope);
    outcome
}

/// FR-340: a `state`/`frame` node whose `modifies`/`creates`/`deletes`
/// entries are eligible members of their own declared `dependencies` (a
/// `field` entry naming an `object_type` in `modifies`, and the same
/// `object_type` in `creates`) is admitted through QSL's whole I2 read.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn frame_body_membership_against_dependencies_is_admitted() {
    let envelope = frame_fixture(|object| {
        (
            vec![modifies_field(object, "value")],
            vec![object.clone()],
            vec![],
        )
    });
    let outcome = read_frame_fixture(&envelope);
    assert!(
        matches!(outcome, Read::Verified { .. }),
        "a frame whose modifies/creates entries are both declared dependencies of \
         their own eligible kind must be admitted, got {outcome:?}"
    );
}

/// FR-340: an entry naming a node that no dependency of the frame declares
/// -- here, a node absent from the whole graph -- refuses
/// `missing_declaration`/`missing-name`, located at the offending entry, and
/// this crate's own `V2ReadRefusal::code()` maps that to `Code::
/// MissingDeclaration`. The other half of FR-340-AC-13 (a real node elsewhere
/// in the graph that is simply never declared to this frame) is covered by
/// the `QSPEC_DIR`-gated `undeclared-entry` vector in
/// `conformance_fr340_frame_mutations_match_qspec_vectors`, not repeated here.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn frame_entry_outside_dependencies_refuses_as_missing_declaration() {
    let field_digest = hex("pkg::F");
    let type_ref = node_ref("pkg::T");
    let object_ref = node_ref("pkg::O");
    let type_node = graph_node("pkg::T", "T");
    let object_node = model_graph_node("pkg::O", "object_type", "O", &type_ref);
    // `F` is never declared as a dependency of this frame, only named in
    // `modifies`.
    let frame = frame_node(
        "pkg::Frame",
        &object_ref,
        vec![object_ref.clone()],
        vec![modifies_field(&node_ref("pkg::F"), "value")],
        vec![],
        vec![],
    );
    let envelope = valid_envelope_over(vec![type_node, object_node, frame]);
    let outcome = read_frame_fixture(&envelope);
    match &outcome {
        Read::Refused(outer @ V2ReadRefusal::Envelope { refusal, .. }) => {
            assert_eq!(refusal.code, CheckedPackageRefusalCode::MissingDeclaration);
            assert_eq!(refusal.cause, Some(CheckedPackageRefusalCause::MissingName));
            assert_eq!(
                refusal.locus.as_ref().map(|node| node.digest.as_ref()),
                Some(field_digest.as_str())
            );
            assert_eq!(outer.code(), Code::MissingDeclaration);
        }
        other => panic!("expected a missing_declaration refusal, got {other:?}"),
    }
}

/// FR-340: a `modifies` entry claiming the `relationship` kind for a
/// declared dependency whose node kind that kind does not admit (a
/// `model`/`object_type` node, which `relationship` never admits -- only a
/// `relation`/`relationship` node) refuses
/// `invalid_model_binding`/`malformed-declaration`, located at the offending
/// entry, and this crate's own `V2ReadRefusal::code()` maps that to
/// `Code::InvalidModelBinding`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn frame_entry_of_an_ineligible_kind_refuses_as_invalid_model_binding() {
    let object_digest = hex("pkg::O");
    let envelope = frame_fixture(|object| {
        // `object` (`model`/`object_type`) is declared, but a `relationship`
        // kind entry admits only a `relation`/`relationship` node, never
        // `model`/`object_type`.
        (vec![modifies_relationship(object)], vec![], vec![])
    });
    let outcome = read_frame_fixture(&envelope);
    match &outcome {
        Read::Refused(outer @ V2ReadRefusal::Envelope { refusal, .. }) => {
            assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidModelBinding);
            assert_eq!(
                refusal.cause,
                Some(CheckedPackageRefusalCause::MalformedDeclaration)
            );
            assert_eq!(
                refusal.locus.as_ref().map(|node| node.digest.as_ref()),
                Some(object_digest.as_str())
            );
            assert_eq!(outer.code(), Code::InvalidModelBinding);
        }
        other => panic!("expected an invalid_model_binding refusal, got {other:?}"),
    }
}
