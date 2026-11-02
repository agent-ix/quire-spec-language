---
id: SR-202610310155
title: "Code review — QSL-1397 producer dispatch emission"
type: SpecReview
analysis: code-review
date: 2026-11-02
scope: "agent-ix/quire-spec-language; PR #2047; qsl-semantics/src/check/{dispatch_link.rs,lowering.rs,ir.rs,mod.rs}; qsl-package/src/emit.rs; tests/it/dispatch_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-494
    type: reviews
---
# SR-202610310155: Code review — QSL-1397 producer dispatch emission

## Summary

Ticket: QSL-1397. Reviewed PR #2047 at `e3270aa2a60d51dd19bf2cc5a13739e63accfdcf`. The implementation retains dispatch decisions by exact lowered application/location and writes the carrier from those decisions. The Rust review found two acceptance-test gaps: candidate body/precondition identities are not compared element-by-element, and declaration-order invariance is not exercised.

Scope examined: FR-494 and AC-1 through AC-4; TC-1282 and TC-1283; dispatch decision retention, identity mapping, source-map joining, row ordering, refusal paths, and the changed typed-emission tests. No applicable AssuranceProfile was present. No CI workflow or gate was changed.

Verification: the two `dispatch_emission` integration tests passed with the IR worktree supplied as a Cargo config patch; `cargo fmt --all -- --check` passed through `locked-build.sh`. A reviewer Clippy attempt resolved quire-contract-model from remote main instead of the draft branch and failed on the missing API; the PR body records the author's successful Clippy run using a temporary manifest override.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | TC-1282 does not verify each candidate's exact operation-body and effective-precondition identities | FR-494-AC-1, FR-494-AC-3, TC-1282 | correct-requirement-no-evidence |
| FND-002 | medium | TC-1283 does not test that permuting declarations preserves canonical row order and identities | FR-494-AC-2, TC-1283 | correct-requirement-no-evidence |

## Verdict

**CONDITIONAL.** The changed Rust path is coherent on inspection, but the two medium evidence gaps leave required identity-copy and declaration-order invariance behavior unverified by the tagged acceptance tests.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The fix captures pre-emission candidate identities and compares every typed candidate's exact selection key, body identity and nullable effective-precondition identity. |
| FND-002 | fixed | The fix recompiles after declaration permutation and compares producer identities and structuralized canonical rows. |
