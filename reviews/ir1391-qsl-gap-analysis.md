---
id: SR-20261106200941
title: "IR-1391 QSL facade gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; IR-1391 QSL division selection facade; FR-495-AC-8 and TC-1284; qsl-replay compile/application/execute/lifecycle"
review_set: subset
---

## Summary

Reviewed source head 0b40ef26a and the implementation diff from origin/main. The posted FR-495-AC-8 narrowing is context: source-backed execution only, with independent strict-read law retention treated as inspection, never byte-only execution.

## Verdict

**CONDITIONAL** — Two concrete public-path/negative-selection test gaps require fixes and genuine execution before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new public replay_with_integer_division entry and replay recompilation are not exercised by the original tests; add a genuine request showing selected-law success and missing/wrong-law refusal so losing the selection in replay cannot pass. | qsl-replay/src/execute.rs:1185-1197 |
| FND-002 | medium | The original conflicting/unknown test never supplies an unknown definition identity under the valid authority; add that owner-refusal control rather than relying only on a foreign authority and conflicting known profiles. | qsl-replay/src/application/division_tests.rs:188-212 |

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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The unused newly added replay API and its selection plumbing were removed. The final reduced implementation only supplies the admitted law to compile and source-backed closure acquisition. Existing replay remains unchanged; no untested new replay path, forged request law binding, or byte-only execution credit is claimed. |
| FND-002 | fixed | The owner admission test now changes only the identity under the genuine authority and asserts native IncompatibleDefinition, separately from foreign authority, conflicting profiles and incompatible modulo binding. The focused test genuinely passes. |

## Current outcome

**PASS** on 5e9282fa7 — four genuine focused tests pass in /tmp/IR-1391-qsl-tests.log. Three selected laws execute nine signed tuples through source-backed closure calls (quotient and remainder), and independent strict reads assert the original law views. Missing selection, mod/non-division controls, unknown/foreign/conflicting/incompatible owner references, and a substituted unselected emitted law are distinct actual refusals; the latter asserts OperationLawUnselected and its exact definition pointer. The new tests use bare TC-1284/FR-495-AC-8 trace attributes. Matrix attempt exited 124 with no JSON, so this is manual touched-scope verification, not computed or whole-repository coverage credit. Full CG/integration agreement remains separately owned. Final aggregate gate remains with the lead.

## Final focused evidence

At source5e9282fa7, /tmp/IR-1391-qsl-tests.log reports4 passed/0 failed, and /tmp/IR-1391-qsl-clippy.log reports qsl-replay library/test-target Clippy success with warnings denied. Coder formatting and diff checks pass. No reviewer Cargo build or final aggregate gate was run.
