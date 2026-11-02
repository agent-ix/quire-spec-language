---
id: SR-202611021110
title: "Fresh code review (rust lane) - QSL-1461 explicit Text literals"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2084 head ecb0d795b, diff origin/main...HEAD (non-review files)"
review_set: subset
---

## Summary

Ticket: QSL-1461. Independent review of PR #2084 (code-review with rust-review lane, diff only, no cargo run). Grammar, S2 mapping, typing hint, normalization via quire-exact's TextNormalization::apply, bounds check on profile.length, typed refusal and lowering are correct for nfc/nfd/binary-utf8. Lock discipline holds. Two defects: an unrelated behavior change for qualified UUID/Timestamp calls, and weak vector-test guards.

## Verdict

**CONDITIONAL** - mergeable once FND-001 is answered with a test (or reverted) and FND-002 is tightened.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | typing.rs now skips the native constructor for any qualified name, which also changes `M::UUID(..)` / `M::Timestamp(..)` (previously native candidates); no test pins either behavior. | qsl-semantics/src/check/check/typing.rs:1392 |
| FND-002 | low | The two vector tests never assert the case count (file has 15 cases incl. TX-13s, not 14), so an empty or truncated `cases` array passes vacuously; the oracle also ignores each case's `length`/`normalized`/`expected`/`cause`. | qsl-package/src/emit/tests.rs:2499 |
| FND-003 | low | The test oracle uses the same unicode-normalization crate the production path reaches through quire-exact, so normalization correctness is not independently checked; the new dev-dependency also lacks the justification comment its neighbours carry. | qsl-package/Cargo.toml:67 |

## Review Evidence

Checked: Cargo.toml (root) not in diff, branch="main" unchanged, no new rev= pin; Cargo.lock diff is exactly quire-contract-model -> IR f93d7277 plus one unicode-normalization line under qsl-package (already a locked crate); no #[allow] added; production normalization reuses quire-exact TextNormalization::apply, the new crate is dev-only; InvalidTimedModel -> InvalidPackage follows the existing grouped mapping and the "no per-IR-variant Code" comment, no new Code minted; emit.rs schedulability_claims fields are the IR relock's required struct fields. Missing vector file panics (not skip); QSPEC_DIR unset falls back to the sibling checkout and still panics when absent. Compilation and tests not run (no cargo, by brief).
