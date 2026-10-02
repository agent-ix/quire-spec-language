---
id: SR-327
title: "Code review of composed guarded-definedness proofs"
type: SpecReview
analysis: code-review
scope: "FR-040 / TC-119; src/checking/composed/proofs/; src/checking/proof.rs; src/checking/proof/; tests/composed_proofs.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/TC-119, type: references }
---

## Summary

Claude Opus rechecked the PR #51 corrections using the actual code-review,
rust-review and rust-style skills. The coordinating agent transcribed its
completed findings and executed its gate batch when Claude's CLI denied the
`flock` invocation. No false discharge or correction regression was found.
The original review remains in Git history.

## Verdict

**CONDITIONAL** — only the two low findings below remain. The supported
value-definedness stage is ready to merge. Full query/family/runtime admission
and source-to-B emission remain open; see [SR-328](26-09-10-composed-proofs-gaps.md).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Bounded implementation cost: the shared materializer clones each node Kind and field-key lookup allocates an owned name. Charges bound both; optimization is deferred. | src/checking/proof/graph.rs:36; src/checking/composed/proofs/engine/walk.rs |
| FND-002 | low | The composed materializer's invariant-error adapter retains an unsupported classification but drops the underlying IR diagnostic detail; the historical adapter keeps it. No newly reachable failing input was found. | src/checking/composed/proofs/engine/graph.rs:75 |

## Correction disposition

| Initial finding | Current disposition |
| --- | --- |
| SR-327 FND-001: refused proof reported complete | Resolved. Refused leaves, locally unproved callers and refused dependencies keep `complete()` false; the strengthened public tests exercise these cases. |
| SR-327 FND-002: opaque witnesses treated as executable IR | Withdrawn as an executable-export defect. This API exposes a symbolic definedness environment, explicitly documented as non-executable. Non-numeric comparisons produce independent Boolean witnesses; they never become numeric/comparison operands. New controls retain actual native types and show an unrelated opaque guard cannot prove a numeric obligation. |
| SR-327 FND-003: historical representation panics | Resolved. A private typed adapter error distinguishes unsupported representation from an actual IR diagnostic; no private IR constructor, fabricated diagnostic or panic supplies the refusal. Historical source/code mapping is preserved. |
| SR-327 FND-004: cross-module assumptions panic | Resolved in the identified adapters. Missing type/binder/source facts return existing typed causes, original declaration loci remain available, and source-work dimensions are exhaustively mapped. Remaining closed-enum unreachable arms stay within already selected operator groups. |
| SR-327 FND-005: repeated arena arithmetic | Resolved. The shared parser-owned range helper and checked child offsets enforce the declaration window. |
| SR-327 FND-006: clone/key allocation cost | Retained as current low FND-001; no premature optimization gate. |

The shared materializer and actual IR prover remain the only proof kernel.
Caller argument definedness, callee totality, immutable capture/let/pre origins
and unsupported ordered-query dependencies remain distinct. No proof graph is
an executable package or family-admission authority.

## Local gates

All gates below executed successfully against the corrected code. The coordinator
ran the reviewer-authored batch with its single internal lock; an additional
outer lock was deliberately omitted to avoid acquiring the same lock twice.
One build job, `nice -n10`, the shared target cache, `--locked`, and one test
thread were used throughout. The metadata-only library touch occurred inside
the lock before the first Cargo command.

| Gate | Result |
| --- | --- |
| Strict all-target Clippy, no default features | exit 0 |
| Strict all-target Clippy, all features | exit 0 |
| Full tests, no default features | exit 0 — 478 passed, 0 failed, 4 existing ignored |
| Full tests, all features | exit 0 — 494 passed, 0 failed, 4 existing ignored |
| Formatting | exit 0 |
| cargo-deny | Not applicable: no deny.toml |

Logs: `/tmp/quire-composed-proofs-rereview-20260910-234908.log` and its
`-summary.log`. Both full runs include 16 composed proof tests, 24 historical
checker tests and three compile-fail doctests. The initial review's claim of
a two-doctest difference at the same tree was incorrect: its `--all-targets`
runs excluded three doctests, and the earlier root run preceded a parent test
addition. Current totals above come from executed full suites.
