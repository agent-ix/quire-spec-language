---
id: SR-20261106200940
title: "IR-1391 QSL facade code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; IR-1391 QSL division selection facade; FR-495-AC-8 and TC-1284; qsl-replay compile/application/execute/lifecycle"
review_set: subset
---

## Summary

Reviewed source head 0b40ef26a and the implementation diff from origin/main. The posted FR-495-AC-8 narrowing is context: source-backed execution only, with independent strict-read law retention treated as inspection, never byte-only execution.

## Verdict

**PASS** — Static Rust/API review found no implementation defect. Actual owner admission types are re-exported without a mirror carrier; optional selections reach root checking and replay recompilation; existing entries use None. Root alone owns the aggregate final gate. Runtime tests are not yet qualified at this original review checkpoint.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

The Rust lane uses the repository AGENTS/CLAUDE plus rust-style API, implementation, testing, safety and wire checks. Inspected every changed file listed above, actual semantic admission/checked selection ownership, root-only selection application, existing None callers, typed error propagation and public source-backed execution. No unsafe, dependency, wire-schema or CI changes. No applicable AssuranceProfile found.

The computed matrix was attempted with a 30-second bound and produced no JSON before timeout; no computed coverage credit is claimed. Manual touched trace/code verification records direct #[trace("TC-1284", "FR-495-AC-8")] binders and real owner paths. Optional whole-repository semantic audit was not requested. No spec/plan artifacts changed in this implementation diff, so spec-review methods are not applicable; the separate posted spec PR is context only.

## Validation evidence

Focused library check passed in /tmp/IR-1391-qsl-check.log. At original review time the cold focused test build failed on missing test imports; no runtime pass is inferred. Subsequent fixes and genuine runtime results require disposition. No reviewer Cargo run or aggregate gate was started; this avoids a duplicate build in the shared ticket worktree.

## Examined scope

- `FR-495-AC-8` — `spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md`: Through the existing public `qsl-replay` compile facade, compile genuine bounded-operand `div` and `rem` functions with each owner-admitted Euclidean, Floor and Truncating selection. The supplied capability, checked nodes and emitted `operation.laws` retain that exact owner law into CG; the existing source-backed QSL/CG agreement path matches QSpec TC-192 at positive and negative operands, including negative-divisor controls that distinguish the laws. This obligation supplies the admitted law to CG/source-backed IR-1391 agreement; a new byte-only execution surface is outside this scope. Missin
- `qsl-replay/src/compile.rs` — `qsl-replay/src/compile.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.
- `qsl-replay/src/application.rs` — `qsl-replay/src/application.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.
- `qsl-replay/src/execute.rs` — `qsl-replay/src/execute.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.
- `qsl-replay/src/spine/lifecycle.rs` — `qsl-replay/src/spine/lifecycle.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.
- `qsl-replay/src/lib.rs` — `qsl-replay/src/lib.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.
- `qsl-replay/src/application/division_tests.rs` — `qsl-replay/src/application/division_tests.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.
- `qsl-replay/src/spine/clause/tests/temporal.rs` — `qsl-replay/src/spine/clause/tests/temporal.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.
- `qsl-replay/src/spine/lifecycle/tests.rs` — `qsl-replay/src/spine/lifecycle/tests.rs`: Reviewed complete changed lines, callers, and relevant owner contracts.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | The replay-surface removal commit left a call to removed recompile_with_owner_selections with an undefined integer_division variable, preventing qsl-replay compilation. | qsl-replay/src/execute.rs:1193-1196 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | 593c7ee50 restores the existing recompile_with_temporal_selections entry. The final 5e9282fa7 diff adds no public replay entry and the focused four-test suite genuinely passes. |

## Current outcome

**PASS** on 5e9282fa7 — reviewed the final reduced diff. Only owner-admitted compile and source-backed checked-closure acquisition are new public entries. Ordinary replay retains its original wire and None selection behavior; no law-bound replay identity or byte-only execution claim remains. No new mirror carrier, unsafe or package-global law fallback. The actual checked owner law reaches root checking, emission and source-backed execution; independently read law views retain it. Imported libraries are not silently assigned the root's law. No new findings remain.

## Final focused evidence

At source5e9282fa7, /tmp/IR-1391-qsl-tests.log reports4 passed/0 failed, and /tmp/IR-1391-qsl-clippy.log reports qsl-replay library/test-target Clippy success with warnings denied. Coder formatting and diff checks pass. No reviewer Cargo build or final aggregate gate was run.
