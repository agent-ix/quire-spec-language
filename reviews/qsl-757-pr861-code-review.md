---
id: SR-11185
title: "code review of quire-spec-language PR 861 (same-identity collisions refuse conflicting-binding)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ea401b9cb7d94ff86c5ba9bf20dbeacbb40ef75a; qsl-semantics/src/model/intake.rs, qsl-semantics/tests/it/tc757_same_identity_collisions.rs, qsl-semantics/tests/it/variant_type.rs, qsl-semantics/tests/it/tc235_business_meanings.rs"
review_set: subset
---

## Summary

Ticket: QSL-757. Code review with the rust-review lane, read in place, no build.

- `duplicate_process_step_refusal` now also maps the reader's
  `DUPLICATE_IDENTITY` at `/fields/j/identity`, `/operations/k/identity`,
  `/operations/k/params/s/identity` and `/variants/i/identity` to
  `invalid_model_binding`/`conflicting-binding`. The key is the shared identity
  and the member path is the later member. The pinned reader
  (`rules.rs` `identity_declarations` + `duplicate_identities`) emits at exactly
  those pointers, and in the order type, constraints, fields, variants,
  relationships, operations (+params), clauses, states, transitions, steps. So
  for a field against an operation, the operation is named, which matches the tests.
- No panics in the new code. Every index parse and lookup is `?`. A malformed
  position returns `None` and falls back to the existing typed
  `malformed-declaration` path.
- No previously admitted program is newly refused. The mapping fires only
  when the reader has already emitted `DUPLICATE_IDENTITY`, and before this PR
  that always refused as `malformed-declaration`.
- Red evidence: each of the four collision vectors fails on `origin/main`.
  The old code returned `None` for these positions, so the generic
  `malformed-declaration` was retained and normalized, and `collision()`
  asserts `conflicting-binding`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Vacuous test. `distinct_identities_in_the_same_positions_are_admitted` asserts only `if let Err(..) = read_k2(..)`. `read_k2` calls `read_domain_package`, which never returns `Err`: every intake refusal is kept on `DomainPackage.intake_refusals` and comes out at normalization. This is the same fact that made the first collision helper go red for the wrong reason. If distinct identities were wrongly refused as `conflicting-binding`, this test would still pass. Fix: normalize the package and assert the outcome is admitted, or that `intake_refusals` is empty. | qsl-semantics/tests/it/tc757_same_identity_collisions.rs:106-120; qsl-semantics/src/model/intake.rs:4289-4505 |

## Verdict

CHANGES REQUESTED (one medium). The mapping is correct and has no panics. The
four collision vectors and the two retargeted variant vectors can fail. The one
admission guard can never fail.

Recorded only, not blocking. The function and its locals (`process`, `step`)
now cover fields, operations, parameters and cases, so the names mislead. If a
member duplicates a type identity, the key is that type, so the type's own
reader refusals are deduplicated away.

## Dispositions

Round 1, reviewed at bd32608341bda0788418f2cdde5a90371a5b7310.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bd3260834. `assert_admitted` normalizes and requires `NormalizeOutcome::Completed`. A kept intake refusal or a normalization refusal now fails it. `the_admitted_helper_rejects_a_collision` (`should_panic`) shows it fails on a colliding package. |
