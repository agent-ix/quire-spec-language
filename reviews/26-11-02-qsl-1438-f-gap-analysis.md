---
id: SR-202611020540
title: "Gap analysis — QSL-1438 F: derived partial applications and dev opt-level 3"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2089 head 959952ac6; FR-120 (candidate budget, MaxCandidates stop); qsl-eval/src/simulation/model/application.rs; qsl-eval/src/simulation/model/frames.rs; qsl-eval/src/simulation/protocol/steps.rs; qsl-analyze/tests/it/rc11_oracle.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-120, type: references }
---

## Summary

Ticket: QSL-1438. PR: quire-spec-language#2089, head 959952ac6. Planless gap analysis of the PR diff against the ticket's acceptance criteria and FR-120's candidate-budget rule. FR-120 says that when the count would exceed `max_candidates`, the expansion stops with `resource_exhausted`/`insufficient-next-charge`, naming `MaxCandidates`. Plan completion: not assessed.

The PR adds no tests. It relies on the existing LT-12/13/16 oracle and weak-memory suites.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Ticket AC: LT-16 at most 60 s in the debug harness | qsl-analyze/tests/it/rc11_oracle.rs:167 | examined |
| Ticket AC: exact 7,607 states / 19,574 edges | qsl-analyze/tests/it/rc11_oracle.rs:167 | examined |
| Ticket AC: no caps; QSL-381 limits stay caller-configurable | Cargo.toml; qsl-eval/src/simulation/model/application.rs | examined |
| Ticket AC: LT-12/LT-13 unchanged | qsl-analyze/tests/it/rc11_oracle.rs | examined |
| FR-120 MaxCandidates stop | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:641-643 | examined |
| Derived partial-application charge | qsl-eval/src/simulation/model/application.rs:654-669 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test fails if the derived partial application charges the wrong count. Charging 0 instead of `charged - unselected` passes 231 tests (rc11_oracle, weak_memory, release_acquire, protocol, litmus, tso), although the derived branch runs about 100k times in them with nonzero unselected charges. The weak-memory harness sets `max_candidates: 1_000_000`, so the charge never binds. A wrong charge would change when FR-120's MaxCandidates stop fires and, under the parallel explorer, make it depend on memo order. | qsl-eval/src/simulation/model/application.rs:654-669 |

## Verdict

The ticket's acceptance criteria are met as far as this review measured. LT-16 and LT-12/13 pass with their oracle assertions unchanged. The `MUTANT-DERIVED` baseline, which only adds a stderr print, passed 68 tests in 18 s with 4 threads. The PR reports 39–42 s single-core and 8.5 s all-core for LT-16. No cap is introduced. `max_candidates` is still the caller's.

The derived charge is exact by inspection (see SR-202611020539), but no test pins it. The test that closes FND-001 builds a constrained attempt with a multi-message read whose full application is memoized first. It then applies only the selected transitions under a `max_candidates` exactly equal to a fresh selected application's total, and asserts that this succeeds. It also asserts that `total - 1` stops with `MaxCandidates`, and that the derived result's successors and findings equal a fresh `apply_operation_matching` over the same selection.
