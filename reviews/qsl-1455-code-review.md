---
id: SR-1455006
title: "Code review — QSL-1455 FR-496 omission when an effect names an unemitted node (PR 2093, re-review)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit.rs; qsl-package/src/emit/tests/operation_effects.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/lowering/model/tests.rs; qsl-semantics/tests/support/receiver_frames.rs; Ticket: QSL-1455"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
---

## Summary

Ticket: QSL-1455. Re-review of the full diff origin/main...HEAD at head 7357d8fa6 (base 347e9769e, which is current origin/main). The prior low finding SR-1455004 FND-001 (omission test covered only the relationship node) is addressed by commit c01476f0e.

## Verdict

**PASS** — mergeable. No high or medium findings.

## Rust review

`operation_effects_are_carried` checks the declaring node, every Field/Relationship modifies declaration, creates and deletes against the emitted node set; the whole optional member is dropped when any is missing. The `rows.sort_by` change in `checked_model_operation_effects` sorts by `declaring` structural identity as FR-322 and FR-496 require, and the `Vec` now holds rows directly. The restated TC-417 expectation (Order, Invoice, Sub) is correct: `Sub` is the declaring node of an FR-496 row and no function names it. No `#[allow]`, pin, or `unwrap` on a production path was added; the `unwrap`/`expect` calls are in test code only. `pub(crate) mod base` widens a test-support module only.

## Verification

- `cargo test -p qsl-package --lib operation_effects` — 9 passed.
- `cargo test -p qsl-semantics --lib reference_types_key_over_their_model_nodes_and_record_the_correspondence` — passed.
- `cargo clippy -p qsl-package -p qsl-semantics --all-targets` — clean.
- Whole-workspace cargo check not required: merge-base equals origin/main, so no trial merge applies.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | low      | No findings (placeholder) | -    |

## Dispositions

| FND     | Outcome | Reason |
| ------- | ------- | ------ |
