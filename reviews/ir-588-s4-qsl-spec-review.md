---
id: SR-16278
title: "Spec review of quire-spec-language PR #1382: FR-395 re-export list amendment (IR-588 S4)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@571dfa7005fd8d75134224e48fdb8f58291a16cf; spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md; FR-403 S4 disposition table and ADR-011 X-16 as context"
review_set: subset
date: '2026-10-23'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: reviews
---
# Spec review of quire-spec-language PR #1382 (FR-395 amendment)

## Summary

Ticket: IR-588 (S4). The PR removes `SuppliedLimits` from FR-395's
facade-boundary re-export list. In its place it says that `Call.supplied` is
`quire_semantic_value::supplied_stop::SuppliedLimits`, which a consumer imports
from its owner, because the root re-exports no moved vocabulary (FR-403). This
matches the code (the `pub use` is gone), FR-403 ("no item is re-exported")
and ADR-011 X-16. `quire validate` on the file exits 0.

Examined: FR-395's "Facade boundary" statement and FR-395-AC-9. FR-403's
disposition table and ADR-011 X-16 were read as context.

## Verdict

**Amendment is correct, but incomplete.** FR-395-AC-9 still promises what the
amended statement no longer gives.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-395-AC-9 still reads "A consumer reaches the entry, its result types and FR-100's call types through the `qsl_replay` root alone". `Call.supplied`'s type is one of FR-100's call types, and after the amendment it comes from `quire_semantic_value`. TC-1089's own test file (qsl-replay/tests/call_requirements_root_only.rs) now imports it from there. So the AC contradicts the amended statement, and the test that backs it no longer meets its literal text. Fix: amend AC-9 to except `SuppliedLimits`, for example "... through the `qsl_replay` root alone, except `SuppliedLimits`, which comes from `quire_semantic_value::supplied_stop`". | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:178 |

## Dispositions

Round 1 was reviewed at b5e0c0d03, and the fix commit is 31f5d6c09. `quire validate` passes on FR-395 (exit 0). TC-1089 does not repeat the "root alone" wording, so it needs no change.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 31f5d6c09 | FR-395-AC-9 now adds "except `Call.supplied`'s `SuppliedLimits`, which the consumer imports from `quire_semantic_value::supplied_stop`". That matches the amended statement and call_requirements_root_only.rs. |
