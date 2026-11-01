---
id: SR-202610310137
title: "Code review — QSL-1414 AC-6/7 checked-package v2 tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs; FR-048-AC-6; FR-048-AC-7; TC-134"
review_set: subset
---

## Summary

Reviewed PR #2011 at `8d059304a0be4c86a3d40ee63eec77cb39f414e6` against `origin/main`, including the AC-6/7 partial-claim boundary. The positive path emits the checked package and requires both strict checked-package readers to admit it; the negative cases do not exercise reader rejection, and the purported commit-before-recovery mutation moves commit before the forward effect.

## Verdict

**CONDITIONAL** — the focused static positive is strictly read back, but the adverse cases do not test checked-package reader refusal and one AC-7 mutation does not exercise the named recovery-order failure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The negative provenance mutations call `check_tc134` and stop at source assembly/checking; none emits a checked package, mutates its wire, or invokes the checked-package reader. The PR therefore provides no adverse reader evidence for the checked-package v2 claim. Add a reader-facing mutation fixture and assert the intended typed reader refusal, or keep the negative claim explicitly scoped to source-check refusals. | qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs:435-479 |
| FND-002 | medium | The mutation named as commit/effect ordering at lines 469-479 moves `commit Finalized` before the repeat containing `Applied`; it tests commit-before-forward-effect, not the FR-048-AC-7 commit-before-recovery mutation. Add a distinct mutation that puts commit before the recovery prerequisite and assert its typed refusal. | qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs:469-479 |

## Review evidence

The positive at lines 193-349 calls `lower_with`; its helper emits bytes, invokes `quire_contract_model::read_checked_package`, and invokes the QSL `read_with` route. `lowered.ir.is_ok()` and `lowered.verified` therefore establish strict read-back admission. The coder reported the focused `cargo test -p qsl-package --lib protocol_requirements` passed at this frozen head. I attempted the same focused command with the shared review target, but stopped during fresh dependency compilation when free disk fell to 29 GiB; no test result is claimed from my attempt. `git diff --check origin/main...HEAD` passed. No applicable AssuranceProfile exists in `spec/`.
