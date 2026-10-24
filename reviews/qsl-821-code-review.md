---
id: SR-12091
title: "QSL-821 PR #942 code review: TypeAdmission carries its LimitExceeded"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ab50faf64693b520352b2dcac6627ccd12f3fc04; qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/mod.rs, qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine/clause/tests/refinement.rs, qsl-foundation/src/diagnostic/locus.rs, qsl-foundation/src/diagnostic/stage.rs"
review_set: subset
---

## Summary

Ticket: QSL-821. PR quire-spec-language#942, diff `origin/main...HEAD` (8 files, +88/-6). Code review with the rust-review lane. Read-only: no build, no cargo run. The author has not compiled the change, so this pass read for compile errors.

Checked:

- **Hash chain.** `CheckCause` derives `Hash`, so the new `limit: Option<LimitExceeded>` field needs `Hash` all the way down. `LimitExceeded` has fields `Setting` (derives Hash), `u64`, `u128` and `Option<Box<Locus>>`. `Locus` has variants `SourceRegion` (a manual `Hash` over the same key as its manual `Eq`), `quire_exact::Location`, `DigestRecord` and `JsonPointer`. In the locked quire-exact checkout (`21349174d`), `Location`, `Origin` and `Role` all derive Hash. The other types derive Hash in qsl-foundation. No field is a float or order-dependent, and each `Hash` agrees with its `Eq`.
- **Consumer audit by signature.** I grepped the whole workspace, `.rs` and other files, excluding target/ and .git/. `CheckCause::TypeAdmission` is constructed in one place, `type_admission` in check/mod.rs:679. Its two match sites in refusal.rs (lines 1091 and 1145) use `..`. There are no other struct patterns, `matches!`, `if let`, macros, serde or Display impls on its fields, and no other test initialisers. The other hits are in qsl-replay and are part of this diff. Nothing in root src/tests, xtask or tools/arch-lint names the variant.
- **Imports and calls.** `AssemblyCause` is `pub use`d at check/mod.rs:195. `qsl_foundation::diagnostic::LimitExceeded` is the type `AssemblyCause::TypeLimit` holds. `limit.as_ref().map(ReachedLimit::of)` fits `fn of(&LimitExceeded) -> Self`. The tests' imports (`LimitExceeded`, `Setting`, `Code`, `ReachedLimit`, `RefusalCause`, `ClauseRunReport`) are already in the test module, the `ClauseRunReport` fields match the existing initialiser at line 496, and `Setting::EnvironmentWorkUnits` and `Setting::EnvironmentAncestorSteps` exist with the asserted names.
- **Classifier arm.** `check_limit` has an explicit `TypeAdmission { limit, .. }` arm and no wildcard, and `TypeAdmission` was removed from the `None` list. The match stays exhaustive, `cfg(seam_probe)` arm included. A type-admission ceiling names `setting().name()` and `configured_bound()`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. I found no compile error by reading, and no major defect. The `Hash` derives only add a trait and are sound. The classifier arm is explicit and exhaustive. The producer fills `limit` only for `AssemblyCause::TypeLimit`. `environment_cause` never yields `IdentityLimit` and the storage causes carry no `LimitExceeded`, so `None` is correct for them.
