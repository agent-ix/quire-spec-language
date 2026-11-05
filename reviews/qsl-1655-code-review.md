---
id: SR-20261105101003
title: "Code review — QSL-1655/1665 IEEE negation"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2370; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/lowering.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/tests/it/ieee_node_charges.rs; qsl-replay/tests/it/compile_ieee_profile.rs; FR-397-AC-21/27"
review_set: subset
---

## Summary

Ticket: QSL-1655 / QSL-1665. PR #2370 adds IEEE unary negation through the checked, lowered, and evaluated QSL paths, using the locked `quire-exact` operation. I reviewed the five changed Rust files against FR-397-AC-21 and AC-27, including the Rust safety, API, test, and boundary checks.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| IEEE type check for unary negation | qsl-semantics/src/check/check.rs:3846-3915 | examined |
| IEEE lowering and profile law | qsl-semantics/src/check/lowering.rs:4018-4054 | examined |
| Machine evaluation and meter/result handling | qsl-eval/src/value/expression/evaluate.rs:2216-2254 | examined |
| F32/F64 bit-preservation and meter tests | qsl-eval/tests/it/ieee_node_charges.rs:263-410 | examined |
| Lowering and mode-mismatch test | qsl-replay/tests/it/compile_ieee_profile.rs:802-829 | examined |
| FR-397-AC-21 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:339 | examined |
| FR-397-AC-27 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:345 | examined |

## Verdict

**CONDITIONAL** — implementation uses the owned sign-bit operation and preserves the type/profile contract, but FR-397-AC-21's state-clause case has no value-asserting test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new evaluation tests exercise negation through checked function bodies, but FR-397-AC-21 also requires `evaluate_clause_set` to return `Completed(true)` for a clause holding `-qnan != -qnan`. No test exercises that state-clause route, so a clause-specific wiring regression would pass. Add an AC-tagged state-clause test asserting the specified outcome. | qsl-eval/tests/it/ieee_node_charges.rs:263-410; spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:339 |

## Rust Review

The new evaluator delegates to `quire_exact::IeeeOperation::Negate`, whose pinned contract flips only the sign bit without rounding or changing the payload. The checker admits Float negation only with the IEEE profile; lowering uses the width-specific operation identity and `ieee_profile` law without a rounding mode. The `ValueType` matches are exhaustive. I found no new unsafe code, lock edits, vendoring, unchecked wire conversions, or production panic paths.

The added bit-level tests cover F32/F64 signed zero, subnormals, finite values, infinities, NaN payloads, rounding-mode independence, double negation for F32, and F33 operand/result budget stops. The lowering test checks the operation, law, absent mode, and mismatched return mode. Coder-reported checks: evaluator negation tests 3/3, replay lowering test 1/1, touched-crate clippy in both configurations, formatting, and diff checks passed; these focused commands were not independently rerun in this review.

## Review Method Limits

No applicable AssuranceProfile was found for the changed paths. The repository matrix commands are recorded in the separate gap-analysis review; semantic intent review was not requested.
