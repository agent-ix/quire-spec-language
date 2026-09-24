// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-079: the `#185` registry swap changes routing, never lowering output.
//!
//! **Scope note on TC-203's literal test procedure.** TC-203 asks for a
//! pre-swap output snapshot committed in a commit that precedes an
//! in-place rewrite of `src/lowering/target.rs`'s fixed catalog, with the
//! two compared byte-for-byte. That procedure fits a ticket that rewrites
//! `target.rs` itself. This ticket does not: ADR-011 §6.2 (SEAM-1's own
//! row) and ADR-012 §14.1 both assign the actual deletion/rewrite of
//! `lowering::target`, `ProjectionTarget` and the `--target` argument to a
//! *later* ticket (`#217`, SEAM-1 M-6b) -- "`M-6b deletes the rest of
//! `lowering` with #217`". This ticket (`#185`) is additive only: it adds
//! the new `#185` registry (now `qsl_route`) alongside the untouched lowering
//! pipeline, in preparation for `#217` to route through it later. There is
//! therefore no in-place "swap" of `target.rs`'s own code for TC-203's
//! two-commit snapshot-and-diff procedure to compare across -- see this
//! ticket's own final report for the citations.
//!
//! What this file verifies instead, non-vacuously, is FR-079's actual
//! governing sentence: *"The registry changes how a target is selected and
//! how a capability is matched to a registrant; it SHALL NOT change what IR
//! or output a selected target produces."* It does this two ways:
//!
//! 1. **Untouched corpus, unmodified assertions (TC-203's real intent).**
//!    `src/lowering.rs`, `src/lowering/target.rs`, `src/lowering/inputs.rs`,
//!    `src/lowering/wire.rs`, `src/command.rs` and `src/cli.rs` are not
//!    edited by this ticket at all (verified by this PR's own diff, which a
//!    reviewer checks against the file-ownership list in the ticket). The
//!    existing lowering corpus (`tests/integer_lowering.rs`,
//!    `tests/state_scalar_lowering.rs`, `tests/native_lowering.rs`), each of
//!    which already asserts byte-exact expected IR inline, continues to
//!    pass completely unchanged -- itself the strongest form of "byte-
//!    identical before and after", since there is no "after" edit to those
//!    files to diverge from "before" at all.
//! 2. **Architectural decoupling (this file).** `lowering::lower_for`'s own
//!    signature takes no [`qsl_route::Registry`] parameter, so a registry
//!    populated with a representative Kani `BackendDescriptor` cannot
//!    reach it. This test constructs such a registry, lowers the same
//!    input with and without it having been built at all, and confirms the
//!    lowering output -- serialized to bytes -- is identical either way.

use ix_trace_rs::trace;
use qsl_foundation::digest::ByteDigest;
use qsl_route::{
    BackendDescriptor, BackendId, Candidate, ManifestDigest, Mode, Registry, ToolIdentity,
};
use qsl_semantics::check::Capability;
use quire_spec_language::lowering::{lower_for, LoweringLimits, ProjectionTarget};
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::syntax::ClauseKind;
use serde_json::json;

use crate::support::runtime_setup as setup;

fn boolean_model(input: bool) -> NativeModel {
    setup::authored_model(|model| {
        model["values"].as_array_mut().unwrap().push(json!({
            "name":"enabled", "kind":if input { "input" } else { "state" },
            "type":{"kind":"boolean"}
        }));
        if input {
            model["operations"][0]["parameters"] = json!(["enabled"]);
        }
    })
}

fn package<'a>(models: &'a [NativeModel], expression: &str, kind: ClauseKind) -> NativePackage<'a> {
    NativePackage::new(
        setup::checked_kind(models, expression, kind),
        PackageLimits::default(),
    )
    .expect("valid native package before lowering")
}

/// A representative Kani `BackendDescriptor`, registered "as one backend"
/// (the ticket's own wording) advertising a capability kind, purely to
/// prove the registry's existence has zero effect on `lower_for`'s output.
/// This is not a claim about which capability kind Kani actually
/// advertises in production -- that is `#217`'s wiring, once it routes the
/// legacy targets through this registry.
fn kani_backend_registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(BackendDescriptor::new(
            Candidate::new(
                BackendId::new("kani"),
                ManifestDigest::from_digest(ByteDigest::of(b"kani-manifest").as_bytes()),
            ),
            ToolIdentity::new("kani"),
            [(Capability::OperationContract, Mode::Bounded)],
        ))
        .expect("a fresh registry accepts its one registration");
    registry
}

/// Untagged (PR #305 review finding 5): TC-203's own procedure is a
/// pre/post byte-diff across an in-place rewrite of `src/lowering/
/// target.rs` that this ticket does not perform (see this file's own doc
/// above) -- a `#[trace("TC-203", "FR-079-AC-1")]` tag here could not
/// actually fail on the property TC-203 names, and would falsely mark that
/// TestMatrix row backed. The real catalog-replacement procedure TC-203
/// describes belongs to #217 (ADR-011 §6.2, M-6b); FR-075's Description
/// and FR-079 still need amending to say so (spec text, not this fix).
#[test]
fn lower_for_output_is_identical_whether_or_not_a_registry_was_ever_built() {
    let models = [boolean_model(true)];
    let native = package(
        &models,
        "enabled implies (not enabled or true)",
        ClauseKind::Precondition,
    );

    // "Before": no registry object exists anywhere in this call.
    let without_registry = lower_for(
        &native,
        ProjectionTarget::BooleanOracleV1,
        LoweringLimits::default(),
    )
    .expect("lowering succeeds");

    // "After": a populated registry exists in scope, including a Kani
    // descriptor -- but nothing plumbs it into `lower_for`, because its
    // signature has no such parameter.
    let registry = kani_backend_registry();
    assert_eq!(registry.backends().count(), 1);
    let with_registry_in_scope = lower_for(
        &native,
        ProjectionTarget::BooleanOracleV1,
        LoweringLimits::default(),
    )
    .expect("lowering succeeds identically with a registry merely in scope");

    assert_eq!(
        format!("{without_registry:?}"),
        format!("{with_registry_in_scope:?}"),
        "lower_for's output must not depend on whether a qsl_route::Registry was built"
    );
}

/// TC-204 (FR-079-AC-2): each of the three legacy target names parses,
/// round-trips and appears in the published target list, and an unknown
/// name is rejected -- identically to `main` before this ticket, because
/// `src/lowering/target.rs` is not edited by this ticket at all.
#[test]
#[trace("TC-204", "FR-079-AC-2")]
fn legacy_target_names_parse_round_trip_and_list_identically() {
    use std::str::FromStr;

    for name in ["boolean-oracle/v1", "integer-ir/v1", "state-scalar-ir/v1"] {
        let parsed =
            ProjectionTarget::from_str(name).unwrap_or_else(|_| panic!("{name} must still parse"));
        assert_eq!(
            parsed.name(),
            name,
            "{name} must round-trip through Display"
        );
        assert_eq!(parsed.to_string(), name);
        assert!(
            ProjectionTarget::ALL
                .iter()
                .any(|target| target.name() == name),
            "{name} must still appear in the published target list"
        );
    }

    assert!(
        ProjectionTarget::from_str("nonexistent-target/v1").is_err(),
        "an unpublished name must still be rejected"
    );
    assert_eq!(
        ProjectionTarget::ALL.len(),
        3,
        "no target was added, dropped or merged"
    );
}
