---
id: SR-202610310157
title: "Code review — QSL protocol recursion groups"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/protocol.rs, qsl-semantics/src/check/lowering/protocol/controls.rs, qsl-semantics/src/check/lowering/temporal.rs, qsl-package/src/emit/tests/protocol_recursion.rs; FR-383-AC-24/25"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
---

## Summary

Reviewed QSL-851's mixed protocol/function SCC lowering, recursion key settlement, protocol emission/read-back, and FR-383-AC-24/25 tests. The focused tests and library checks pass, but an eligible cycle containing an attempt-bearing protocol is left in `AfterClauses` before its mixed-cycle placeholder is settled.

## Verdict

**CONDITIONAL** — the current acceptance case passes, but a protocol in a cyclic reference component that contains an attempt still cannot emit the required group.

Focused checks run: `cargo test --locked -p qsl-package protocol_recursion -- --nocapture` (3 passed, including 100,000-member cycle); `cargo clippy --locked -p qsl-semantics -p qsl-package --lib -- -D warnings` (pass); `cargo check --locked -p qsl-package --no-default-features` (pass). `quoin matrix --repo . --json` exited 0; FR-383-AC-24/25 are statically tagged, with no run evidence bound in the evidence store.

## Assurance Context

MP-002 (`spec/assurance/MP-002-checker-wall-time.md`) applies because this change modifies the S3 `PackageDeclarations::check` path. I checked its scope and decision rule. No current-main/candidate interleaved A/B benchmark was run for the protected benchmark population, and the plan says its measurement is not a merge gate. The fixed benchmark population does not include mixed protocol SCCs, so this review makes no wall-time performance claim; the O(V+E) algorithm was checked in source and its charged-work vectors.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Attempt-bearing protocols in a mixed recursion component return `AfterClauses` before their cycle placeholder is resolved, so a predicate-join back-reference cannot emit the required group. | qsl-semantics/src/check/lowering/protocol.rs:621-623 | implementation-bug-despite-evidence |
