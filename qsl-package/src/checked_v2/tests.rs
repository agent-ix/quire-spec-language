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
use quire_contract_ir::{
    CheckedArtifactLocator, CheckedPackageEvidence, CheckedPackageLimit, CheckedPackageRefusalCode,
    CHECKED_PACKAGE_V2, PACKAGE_DOMAIN_V2,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    read_checked_package_v2, V2ReadIncomplete, V2ReadLimits, V2ReadOutcome, V2ReadRefusal,
};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::imports::ImportedNames;
use qsl_semantics::check::CheckCause;
use qsl_semantics::library::{
    ImportView, LibraryName, LibraryRefusal, PackageId, PackageNodeKey, PinMismatch, PinnedRequest,
    PreimageDefect, RefusalClass, Selection, StaleCause, StalePin,
};

const NODE_DOMAIN: &str = "quire.checked-semantic-node/v1";
const SOURCE_DOMAIN: &str = "quire.source.bytes/v1";
const DEFINITION_DOMAIN: &str = "quire.definition.bytes/v1";

fn hex(label: &str) -> String {
    Sha256::digest(label.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn node_ref(label: &str) -> Value {
    json!({"digest": hex(label), "domain": NODE_DOMAIN})
}

fn artifact_ref(label: &str) -> Value {
    json!({
        "authority": "pkg",
        "identity": label,
        "revision": {"namespace": "semver", "value": "1"},
        "digest_domain": DEFINITION_DOMAIN,
        "digest": hex(label),
    })
}

fn source_ref(label: &str) -> Value {
    json!({
        "authority": "pkg",
        "identity": label,
        "revision": {"namespace": "semver", "value": "1"},
        "digest_domain": SOURCE_DOMAIN,
        "digest": hex(label),
    })
}

fn selection(role: &str, label: &str) -> Value {
    json!({"role": role, "definition": artifact_ref(label)})
}

/// Fields shared by a `semantic_graph` node and its identity-projection
/// counterpart: `quire_contract_ir::CheckedNodeProjectionV2::from`'s own
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

fn locator(domain: &str, label: &str) -> CheckedArtifactLocator {
    CheckedArtifactLocator {
        authority: "pkg".into(),
        identity: label.into(),
        revision_namespace: "semver".into(),
        revision_value: "1".into(),
        domain: domain.into(),
    }
}

/// Evidence proving `src`, `edition-def` and `diag-catalog` are current, plus
/// (when `dependency` is set) the dependency selection's own definition.
fn evidence(dependency: Option<&str>) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    evidence.insert_artifact_digest(locator(SOURCE_DOMAIN, "src"), hex("src"));
    evidence.insert_artifact_digest(
        locator(DEFINITION_DOMAIN, "edition-def"),
        hex("edition-def"),
    );
    evidence.insert_artifact_digest(
        locator(DEFINITION_DOMAIN, "diag-catalog"),
        hex("diag-catalog"),
    );
    if let Some(label) = dependency {
        evidence.insert_artifact_digest(locator(DEFINITION_DOMAIN, label), hex(label));
    }
    evidence
}

fn identity(label: &str) -> LibraryName {
    LibraryName::new(vec![label.to_owned()]).unwrap()
}

/// A pinned request selecting `pkg` at `version` and `package_id`.
fn pin(version: &str, package_id: PackageId) -> PinnedRequest {
    pin_library("pkg", version, package_id)
}

fn pin_library(library: &str, version: &str, package_id: PackageId) -> PinnedRequest {
    qsl_semantics::library::fixtures::single_pin(
        identity(library),
        Selection {
            version: version.to_owned(),
            package_id,
        },
    )
}

/// ADR-011 §4 condition 3's own input: `pkg`/`"1"` pinned at the `package_id`
/// `preimage`'s own JCS bytes recompute to -- exactly what a consumer's
/// library lock or pinned request would record for a dependency it expects
/// these bytes to satisfy.
fn pinned_for(preimage: &Value) -> PinnedRequest {
    pin("1", PackageId::of_preimage(&jcs(preimage)))
}

fn no_pins() -> PinnedRequest {
    PinnedRequest::default()
}

fn read(bytes: &[u8], pinned: &PinnedRequest) -> V2ReadOutcome {
    read_checked_package_v2(
        bytes,
        identity("pkg"),
        "1".to_owned(),
        V2ReadLimits::default(),
        &evidence(None),
        pinned,
    )
}

/// The `library` refusal a wire read ended in, or a panic naming the
/// outcome it got instead.
fn structural(outcome: V2ReadOutcome) -> LibraryRefusal {
    match outcome {
        V2ReadOutcome::Refused(V2ReadRefusal::Structural(refusal)) => refusal,
        other => panic!("expected Refused(Structural(_)), got {other:?}"),
    }
}

/// TC-253 step 1: valid v2 bytes, pinned at their own identity, version and
/// recomputed `package_id`, are admitted as a `VerifiedPackage`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn accepts_valid_bytes() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    match read(&bytes, &pinned_for(&preimage)) {
        V2ReadOutcome::Verified { package, .. } => {
            assert_eq!(package.library(), &identity("pkg"));
            assert_eq!(package.version(), "1");
            assert_eq!(
                package.package_id(),
                PackageId::of_preimage(&jcs(&preimage))
            );
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// A wire whose projection declares `exports` (label, qualified name), each
/// a self-typed boolean scalar with a `declaration`-role occurrence.
fn envelope_declaring(exports: &[(&str, &str)]) -> (Value, Value) {
    // IR requires the projection in ascending node-id order.
    let mut sorted = exports.to_vec();
    sorted.sort_by_key(|(label, _)| hex(label));
    let mut preimage = identity_preimage(vec![]);
    preimage["identity_projection"] = sorted
        .iter()
        .map(|(label, export)| projection_node(label, export))
        .collect();
    let mut envelope = valid_envelope(&preimage);
    envelope["semantic_graph"]["nodes"] = sorted
        .iter()
        .map(|(label, export)| graph_node(label, export))
        .collect();
    envelope["source_map"] = sorted
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

/// The `ImportView` of the verified wire declaring `exports`.
fn import_view_of(exports: &[(&str, &str)]) -> (PackageId, ImportView) {
    let (preimage, envelope) = envelope_declaring(exports);
    let verified = match read(&jcs(&envelope), &pinned_for(&preimage)) {
        V2ReadOutcome::Verified { package, .. } => package,
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
/// package's exact `package_id` and version does not list this identity.
/// Condition 3 matches on identity, never on the digest alone.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_only_a_different_library_pins_the_same_id() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let other = pin_library("other", "1", PackageId::of_preimage(&jcs(&preimage)));
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
    let refusal = structural(read(&bytes, &pin("1", stale)));
    assert_eq!(
        refusal,
        LibraryRefusal::StaleDependency {
            path: vec![identity("pkg")],
            pin: StalePin::Pinned(Box::new(PinMismatch {
                pinned: Selection {
                    version: "1".to_owned(),
                    package_id: stale,
                },
                presented: Selection {
                    version: "1".to_owned(),
                    package_id: PackageId::of_preimage(&jcs(&preimage)),
                },
            })),
            cause: StaleCause::ByteDigestMismatch,
        }
    );
    assert_eq!(refusal.class(), RefusalClass::I2Rule(1));
}

/// FR-087-AC-3 and AC-12, condition 3 (FR-087 ruling (e)): the pinned
/// entry names this identity and this `package_id` at another version --
/// `StaleDependency{RevisionMismatch}`, classified to condition 3.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_the_pinned_version_disagrees() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let id = PackageId::of_preimage(&jcs(&preimage));
    let refusal = structural(read(&bytes, &pin("2", id)));
    assert_eq!(
        refusal,
        LibraryRefusal::StaleDependency {
            path: vec![identity("pkg")],
            pin: StalePin::Pinned(Box::new(PinMismatch {
                pinned: Selection {
                    version: "2".to_owned(),
                    package_id: id,
                },
                presented: Selection {
                    version: "1".to_owned(),
                    package_id: id,
                },
            })),
            cause: StaleCause::RevisionMismatch,
        }
    );
    assert_eq!(refusal.class(), RefusalClass::BindingCondition(3));
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
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
                if refusal.code == CheckedPackageRefusalCode::UnknownContractVersion
        ),
        "expected Refused(Envelope(UnknownContractVersion)), got {outcome:?}"
    );
}

/// The refusal's stable `Code`; asserted rather than IR's own `path` text
/// (which embeds `serde_json`'s line/column and is not a stable contract).
fn envelope_code(outcome: &V2ReadOutcome) -> Option<Code> {
    match outcome {
        V2ReadOutcome::Refused(refusal @ V2ReadRefusal::Envelope(_)) => Some(refusal.code()),
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
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
                if refusal.code == CheckedPackageRefusalCode::UnknownContractVersion
        ),
        "expected Refused(Envelope(UnknownContractVersion)), got {outcome:?}"
    );
    assert_eq!(envelope_code(&outcome), Some(Code::InvalidPackage));
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
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
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
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
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
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
                if refusal.code == CheckedPackageRefusalCode::DuplicateMember
        ),
        "expected Refused(Envelope(DuplicateMember)), got {outcome:?}"
    );
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
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
                if refusal.code == CheckedPackageRefusalCode::UnknownMember
        ),
        "expected Refused(Envelope(UnknownMember)), got {outcome:?}"
    );
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
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
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
    // asserts IR's refusal rather than a `Structural` one. IR classifies
    // this mismatch as `StaleDependency`, which is the wrong code for a
    // condition-2 recompute failure (filed with IR as a ticket); this test
    // asserts IR's actual current behavior, not the code IR should emit.
    let outcome = read(&bytes, &pinned_for(&preimage));
    assert!(
        matches!(
            &outcome,
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
                if refusal.code == CheckedPackageRefusalCode::StaleDependency
        ),
        "expected Refused(Envelope(StaleDependency)), got {outcome:?}"
    );
}

#[test]
fn refuses_malformed_identity_preimage_structurally() {
    // Two nodes declaring the same name: IR's own reader never inspects
    // *what* `identity_projection` declares, only that it exactly mirrors
    // `semantic_graph.nodes` (finding above) -- so both must carry the
    // second node too, and the ambiguity is genuinely this layer's own,
    // caught by `library::project_declarations`, not IR's.
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
    match read(&bytes, &pinned_for(&preimage)) {
        V2ReadOutcome::Refused(V2ReadRefusal::Structural(LibraryRefusal::InvalidPreimage {
            library,
            defect: PreimageDefect::AmbiguousDeclaration { name, .. },
        })) => {
            assert_eq!(library, identity("pkg"));
            assert_eq!(name, "R");
        }
        other => panic!("expected Refused(Structural(InvalidPreimage)), got {other:?}"),
    }
}

#[test]
fn refuses_dependency_selections_present() {
    let dependency = vec![selection("dependency", "dep-def")];
    let preimage = identity_preimage(dependency);
    let bytes = jcs(&valid_envelope(&preimage));
    let outcome = read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        V2ReadLimits::default(),
        &evidence(Some("dep-def")),
        &pinned_for(&preimage),
    );
    assert_eq!(
        outcome,
        V2ReadOutcome::Refused(V2ReadRefusal::UnsupportedDependencySelections)
    );
}

#[test]
fn incomplete_when_bytes_exceed_the_ceiling() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));
    let limits = V2ReadLimits {
        artifact_bytes: bytes.len() - 1,
        ..V2ReadLimits::default()
    };
    let outcome = read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        limits,
        &evidence(None),
        &pinned_for(&preimage),
    );
    assert_eq!(
        outcome,
        V2ReadOutcome::Incomplete(V2ReadIncomplete::Bytes {
            limit: bytes.len() - 1,
            actual: bytes.len(),
        })
    );
}

/// QSL-199: `V2ReadLimits::bounded()` no longer clamps a caller-supplied
/// `artifact_bytes` down to [`V2ReadLimits::default`] (ADR-011 §7.3; NFR-001
/// "an implementation ceiling is not a domain bound"). The default ceiling
/// (16 MiB) makes a genuinely oversized *valid* wire impractical to build in
/// a unit test, so this exercises the byte-length gate directly with
/// deliberately non-JSON filler bytes: `read_checked_package_v2` checks
/// `bytes.len() > limits.artifact_bytes` before ever parsing, so the
/// distinction this test names -- refused at the byte-length gate under the
/// default, past that gate under a caller-raised ceiling -- holds regardless
/// of the bytes' own validity. Once past the gate, the reader correctly
/// refuses the filler as malformed JSON: a defect in the input, never
/// conflated with a resource ceiling ([`V2ReadRefusal`]'s own doc).
#[test]
fn a_caller_raised_artifact_bytes_ceiling_admits_past_the_byte_length_gate() {
    let oversized = vec![b'x'; V2ReadLimits::default().artifact_bytes + 1];

    let default_outcome = read_checked_package_v2(
        &oversized,
        identity("pkg"),
        "1".to_owned(),
        V2ReadLimits::default(),
        &evidence(None),
        &no_pins(),
    );
    assert_eq!(
        default_outcome,
        V2ReadOutcome::Incomplete(V2ReadIncomplete::Bytes {
            limit: V2ReadLimits::default().artifact_bytes,
            actual: oversized.len(),
        }),
        "the default ceiling must refuse an oversized read at the byte-length gate"
    );

    let raised = V2ReadLimits {
        artifact_bytes: oversized.len(),
        ..V2ReadLimits::default()
    };
    let raised_outcome = read_checked_package_v2(
        &oversized,
        identity("pkg"),
        "1".to_owned(),
        raised,
        &evidence(None),
        &no_pins(),
    );
    assert!(
        !matches!(
            raised_outcome,
            V2ReadOutcome::Incomplete(V2ReadIncomplete::Bytes { .. })
        ),
        "a caller-raised artifact_bytes ceiling must not refuse at the byte-length gate, got {raised_outcome:?}"
    );
}

/// QSL-199 AC-3: reaching a *caller-raised* ceiling (not just the default)
/// still refuses, naming the limit kind ([`V2ReadIncomplete::Bytes`]) and
/// the caller's own configured bound.
#[test]
fn reaching_a_caller_raised_artifact_bytes_ceiling_refuses_naming_the_kind_and_bound() {
    let raised = V2ReadLimits {
        artifact_bytes: V2ReadLimits::default().artifact_bytes + 1_000,
        ..V2ReadLimits::default()
    };
    let oversized = vec![b'x'; raised.artifact_bytes + 1];
    let outcome = read_checked_package_v2(
        &oversized,
        identity("pkg"),
        "1".to_owned(),
        raised,
        &evidence(None),
        &no_pins(),
    );
    assert_eq!(
        outcome,
        V2ReadOutcome::Incomplete(V2ReadIncomplete::Bytes {
            limit: raised.artifact_bytes,
            actual: oversized.len(),
        }),
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
    match read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        limits,
        &evidence(None),
        &pinned_for(&preimage),
    ) {
        V2ReadOutcome::Incomplete(V2ReadIncomplete::Limit {
            kind: CheckedPackageLimit::Depth,
            ..
        }) => {}
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
    let outcome = read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        limits,
        &evidence(None),
        &pinned_for(&preimage),
    );
    assert!(matches!(outcome, V2ReadOutcome::Verified { .. }));
}

#[test]
fn exact_depth_ceiling_admits_the_boundary() {
    let preimage = identity_preimage(vec![]);
    let bytes = jcs(&valid_envelope(&preimage));

    // Measure the envelope's actual IR-reported depth with the engine under
    // test itself, rather than a second, drifting reimplementation of IR's
    // own `json_depth` (L5): an unreachably low ceiling forces
    // `Incomplete(Limit(Depth))`, whose `consumed` is IR's own count.
    let actual_depth = match read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        V2ReadLimits {
            depth: 0,
            ..V2ReadLimits::default()
        },
        &evidence(None),
        &pinned_for(&preimage),
    ) {
        V2ReadOutcome::Incomplete(V2ReadIncomplete::Limit {
            kind: CheckedPackageLimit::Depth,
            consumed,
            ..
        }) => consumed,
        other => panic!("expected Incomplete(Limit(Depth)) at depth 0, got {other:?}"),
    };

    let admits = V2ReadLimits {
        depth: actual_depth as usize,
        ..V2ReadLimits::default()
    };
    let outcome = read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        admits,
        &evidence(None),
        &pinned_for(&preimage),
    );
    assert!(
        matches!(outcome, V2ReadOutcome::Verified { .. }),
        "expected Verified at the exact depth boundary, got {outcome:?}"
    );

    let refuses = V2ReadLimits {
        depth: (actual_depth - 1) as usize,
        ..V2ReadLimits::default()
    };
    match read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        refuses,
        &evidence(None),
        &pinned_for(&preimage),
    ) {
        V2ReadOutcome::Incomplete(V2ReadIncomplete::Limit {
            kind: CheckedPackageLimit::Depth,
            ..
        }) => {}
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

#[test]
fn depth_far_past_the_default_limit_is_refused_as_malformed_wire_not_incomplete() {
    // IR-238 item 1 (M1): IR's `strict_json_value` parses with
    // `serde_json::Deserializer` and never disables its recursion limit, so
    // serde_json's own fixed 128-container recursion cap fires before IR's
    // own `json_depth` resource meter ever runs. Pins the module doc's
    // "Ceilings" section: a wire this far past the default limit is
    // refused, not reported `Incomplete`, however `V2ReadLimits::depth` is
    // configured. Bare bytes, not a valid envelope: the depth check runs
    // before schema decode, on any well-formed JSON document.
    let bytes = jcs(&nested_array(200, json!(1)));
    let outcome = read_checked_package_v2(
        &bytes,
        identity("pkg"),
        "1".to_owned(),
        V2ReadLimits::default(),
        &evidence(None),
        &no_pins(),
    );
    assert!(
        matches!(
            &outcome,
            V2ReadOutcome::Refused(V2ReadRefusal::Envelope(refusal))
                if refusal.code == CheckedPackageRefusalCode::MalformedWire
        ),
        "expected Refused(Envelope(MalformedWire)) once nesting passes serde's recursion cap, got {outcome:?}"
    );
}

#[test]
fn depth_boundary_is_fail_closed_for_both_kinds_of_deepest_path() {
    // QSL-6 M2 (decided fail-closed): `V2ReadLimits::depth` is passed to
    // IR unchanged, in entered-container units. IR's own `json_depth`
    // counts a scalar leaf as one further unit beyond the containers
    // entered to reach it, but counts an empty container as exactly the
    // containers entered including itself -- so at one shared limit the two
    // kinds of deepest path disagree by one (module doc, IR-238 item 1).
    // Bare bytes, not a valid envelope: the depth check runs before schema
    // decode, on any well-formed JSON document.
    let limits = V2ReadLimits {
        depth: 3,
        ..V2ReadLimits::default()
    };
    let is_depth_incomplete = |bytes: &[u8]| {
        matches!(
            read_checked_package_v2(
                bytes,
                identity("pkg"),
                "1".to_owned(),
                limits,
                &evidence(None),
                &no_pins()
            ),
            V2ReadOutcome::Incomplete(V2ReadIncomplete::Limit {
                kind: CheckedPackageLimit::Depth,
                ..
            })
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
