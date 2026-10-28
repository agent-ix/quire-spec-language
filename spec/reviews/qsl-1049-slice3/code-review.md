---
id: SR-17015
title: "Code review of QSL-1065 state-local EN-1 classification"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1065 diff from <sha removed>: qsl-analyze/tests/it/{litmus.rs,weak_memory.rs}, qsl-eval/src/simulation/{protocol.rs,protocol/access.rs,protocol/footprint.rs,protocol/steps.rs}"
review_set: subset
---

## Summary

The direct Rust review covered enabled application construction, exact weak-branch thread observations, fork-time reader preview, access-shape settlement, transition footprints, and the focused TSO/RA integration tests. The behavior is grounded in FR-220 AC-11/14/15 and ADR-025 MA-1; no applicable AssuranceProfile was found. The changed behavior introduces no unrelated implementation surface. The scoped computed matrix tagged FR-220-AC-11, FR-220-AC-14, and FR-220-AC-15. Quoin 0.29.1 has no `gap-analysis` command, so the reverse-gap check was performed directly over the six-file diff and those requirements; the full-root matrix emitted diagnostics without a matrix, so the scoped `quire matrix` input contained only FR-220, ADR-025, TC-665, and the touched weak-memory test source. No optional semantic review was run.

## Verdict

**PASS.** State-local classification now drives access shape refusal and runtime attempt/footprint handling. Fork-time observations are previewed through the same settling logic, while potential locations remain backing reservations and do not change classification. The prior QSL-1064 review finding is disposed as fixed in the QSL-1065 diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No actionable findings | - |

## Coverage

The scoped matrix reports `tagged` for FR-220-AC-11, FR-220-AC-14, and FR-220-AC-15. Validation passed: 35 weak-memory integration tests; 95 `protocol_` integration tests; 18 release/acquire integration tests; `cargo check -p qsl-eval -p qsl-replay -p qsl-analyze`; `cargo clippy -p qsl-eval -p qsl-replay -p qsl-analyze --all-targets --all-features -- -D warnings`; `cargo fmt --all -- --check`; and `git diff --check`. Plan completion: not assessed.
