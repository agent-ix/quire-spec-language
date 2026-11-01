---
id: SR-1988009
title: "Code review — QSL-588 timed statistical sampling PR 1998 rebase"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/, qsl-analyze/tests/it/, qsl-eval/src/, qsl-eval/tests/it/timed_subject.rs, qsl-replay/src/, qsl-semantics/src/, qsl-semantics/tests/it/probabilistic_declarations.rs; FR-253-AC-1..4, FR-254-AC-1..4"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-253", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Reviewed the rebased PR #1998 Rust diff and its FR-253/FR-254 implementation, test oracles, typed-fairness integration, and Rust gates. The previous free-delay race and trace issues are fixed; a compound timed formula can still be sampled beyond its actual horizon because the checker adds every operator bound.

## Verdict

**CONDITIONAL** — one medium behavior finding remains in timed horizon calculation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Timed formula horizon sums every temporal operator bound, overestimating branching formulas and allowing post-horizon free-delay or time-lock refusals to change a bounded event's result. | qsl-semantics/src/check/probabilistic/claim.rs:1318-1329 |

## Coverage

- FR-253-AC-1 and AC-2: the test asserts exact windows and all three race outcomes; `#[trace]` directly names both criteria. The probability oracle keys outcomes in `BTreeMap<Vec<(String, String)>, Rational>` and asserts exact `9/10`, `1/20`, and `1/20` values.
- FR-253-AC-3: declaration and unbounded-uniform refusal paths remain covered by timed-declaration tests.
- FR-253-AC-4: the race preserves the scheduler-choice MDP, the free timeout has `distribution: None`, and the same state is refused as `NotStochastic` by sampled execution; the test directly binds AC-4.
- FR-254-AC-1..4: checked exact measured decisions, draw-grid replay and altered-delay rejection, elapsed timestamps, free-delay refusal, sample budget, and local time-lock/no-witness outcomes. Gap analysis records missing subclauses in AC-3 coverage.
- Typed fairness integration: `every_scheduler_carries_its_fairness_set` remains present and passed in the changed `probabilistic_declarations` integration target. Statistical execution still admits `ProbScheduler::Workload` only.
- EN-5 protected paths: no changed file under `qsl-analyze/src/exact_probabilistic/` or exact certificate/witness paths. CI/workflow paths are unchanged.
- Gates: `cargo fmt --check` passed; strict workspace all-target/all-feature Clippy passed offline using the existing target; PR no-default-features `cargo check` passed; `cargo deny check bans` passed. Full `cargo deny check` failed on AGPL-3.0-or-later licenses in unchanged `filament-core-data` dependencies. The first online Clippy attempt could not update a shared Cargo git checkout; the offline rerun completed successfully.
- Focused PR tests: `timed_subject` passed 73; `probabilistic_declarations` passed 52. `statistical_model` passed 27 and `statistical_witness_replay` passed 12; all four focused test commands passed.

## Findings detail

FND-001: `claim.rs` computes the timed horizon by summing the upper bound of every operator. For Boolean branches such as `eventually[0,1] p && eventually[0,3] q`, the bounded event horizon is the longest branch (3), not the sum (4). The statistical reader then samples through that inflated horizon; an unsupported free-delay race or time lock after the real formula horizon can incorrectly make the claim unsupported. Compute the maximum path horizon and add a branching-formula regression test.

## Bindings

- `TC-708` → `FR-253-AC-1`, `FR-253-AC-2`: correct and direct.
- `TC-708` → `FR-253-AC-3`: correct.
- `TC-708` → `FR-253-AC-4`: correct and direct.
- `TC-709` → `FR-254-AC-1..4`: criterion tags are direct; the gap artifact records the compound AC-3 oracle limitation.
