---
id: SR-202611030201
title: "Code review — QSL-1563 one-list check keeps a refused protocol's refusals"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/mod.rs (PR #2171, head 8355f64cb); FR-383, FR-218-AC-30, FR-136-AC-8"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
---

## Summary

Ticket: QSL-1563. Reviewed `git diff origin/main...HEAD` of PR #2171 (one file, qsl-semantics/src/check/mod.rs) with the Rust lane. PR #2157 moved a protocol's own refusals into `CheckedGraph.protocol_refusals`, so stages after the protocol loop no longer stop at a refused protocol and the one-list `check`/`check_with_cancel` returned only those later stages' consequences. The fix splits the body into `check_retaining_protocols(.., independent_protocols, &mut protocol_check_refusals)`: `check_with_cancel` passes `false`, returns after the refinement checks when a protocol refused, and returns the protocol refusals first then the refinement refusals; `check_with_protocol_results[_and_cancel]` passes `true` and keeps running past a refused protocol (FR-383). No `#[allow]`, `unwrap` or pin added.

Evidence: the four named tests pass; qsl-semantics lib 751 passed, 0 failed; `--test it --all-features` 1314 passed; qsl-replay and qsl-analyze pass; whole-workspace `cargo check`, clippy default and `--no-default-features` (-D warnings) and arch-lint qualified-core pass. qsl-package fails 27 tests and the root crate 5 tests, all on clean origin/main too (control run; the branch's qsl-package failures are a strict subset of main's 30, the three extra main failures being emit tests this fix repairs).

## Verdict

**PASS** — mergeable. The fix restores the pre-#2157 one-list result for `check()` without changing the FR-383 per-protocol path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
