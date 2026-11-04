---
id: SR-20261104091645
title: "Code review — QSL-1507 spine profile and proof paths"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/spine/protocol_proof.rs, qsl-replay/src/spine/lifecycle/tests.rs, qsl-semantics/src/library/profile_closure.rs, qsl-semantics/src/library/bundle.rs; FR-036-AC-8, FR-040-AC-11, FR-048-AC-4, FR-064"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: references
---

## Summary

Reviewed PR #2292's profile closure resolver, spine result paths, protocol proof correspondence, tests, and ADR boundary against QSL-1507's acceptance scope. The proof path accepts an arbitrary Handler name as exact, and two new string comparisons bypass the marked string edge boundary.

## Verdict

**CONDITIONAL** — the two medium findings need fixes before merge. Coder receipts report focused tests, both workspace clippy configurations, and formatting green; the aggregate gate is deferred until the final reviewed head per the code lead.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The protocol proof accepts any Handler name as the exact execution point. | qsl-replay/src/spine/protocol_proof.rs:199-202 |
| FND-002 | medium | New string-based dispatch comparisons are outside a marked `string_edge`. | qsl-semantics/src/library/profile_closure.rs:77; qsl-replay/src/spine/lifecycle.rs:1488 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `ProtocolProofCorrespondence` now carries `expected_handler`, the execution point must match it, and the adverse test verifies a different Handler name is refused. |
| FND-002 | fixed | Profile spellings now map to `ProfileIdentityKind` inside the marked `profile_identity_kind` edge; callers branch on the closed enum. |
