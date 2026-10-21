---
id: SR-12248
title: "Gap analysis of QSL-560 E15c protocol scheduler fairness and lasso replay (PR #1134)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@39307eec1 (qsl-560-impl, diff origin/main...HEAD); FR-212-AC-1..4, FR-217-AC-1, FR-217-AC-4, FR-206, ADR-027 PA-1/PA-2/PA-3, TP-4; QSpec FR-430 (by id); TC-657, TC-662 against qsl-analyze/tests/it/scheduler_fairness.rs and qsl-eval/src/simulation/protocol/fairness.rs tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-657
    type: reviews
---
# SR-12248: Gap analysis of QSL-560 E15c protocol scheduler fairness and lasso replay (PR #1134)

## Summary

Ticket: QSL-560 (PR quire-spec-language#1134). This is a planless gap analysis. It compares the listed acceptance criteria with the tests the diff tags to them, checks how strong each tagged test's oracle is, and judges the six remainder items the PR declares blocked. Plan completion: not assessed.

## Verdict

**No major gaps.** The tags are honest. Only FR-212-AC-3, FR-212-AC-4 and FR-217-AC-4 carry AC tags. FR-212-AC-1 and FR-212-AC-2 are tagged `TC-657` only, with a doc comment stating what is not covered. FR-217-AC-1 stays untagged.

The declared remainder items were judged as follows:

1. **FR-212-AC-1 `proved` waits on FR-339: true.** `LivenessRefusal::ComponentCertificateNotBuilt` is the pre-existing outcome of every liveness run that finds no fair accepting cycle, model subjects included. The test asserts the whole product was explored (`product_states > 0`), and that no fair counterexample exists.
2. **The FR-212-AC-2 identity difference waits on FR-383: true.** FR-383 (emitting protocol nodes, `scheduling` included) is not implemented on main: only `qsl-package` emit tests mention `protocol_clause`. `package_id` therefore cannot bind the member yet.
3. **Authored protocol-target constraints end to end: true.** There is no emitted fairness member for a protocol target. Refusing at S4 (`ProtocolFairnessNotEmitted`) is the sound choice, because compiling the clause with an identity that drops the target would mis-bind the obligation. Compensation templates: `ProtocolSystem` refuses `compensate` (`Unsupported::Compensation`), so their scheduler constraints cannot arise. The eval-side attempt-node class was not built, and it is not blocked (SR-12247 FND-005).
4. **Liveness over `tso`/`ra`: true.** Memory constraints (FR-228, QSpec FR-434 origin `memory`) are not read by the engine, and refusing is sound. The refusal has no test (SR-12247 FND-002).
5. **FR-217-AC-1 envelope serialization: true.** No counterexample wire encoder exists (QSpec FR-433). AC-1 is untagged.
6. **FR-212-AC-3 spelling: acceptable as code.** FR-212 "Dependencies" says QSpec owns the fairness targets, and QSpec FR-430's table spells `<protocol>::<parallel>::<branch>` and `<protocol>::<node>`. The QSL AC text still reads bare `s` and `Done`, though, which this repo now refuses as `missing-name`. The AC must be edited in this PR (FND-001).

Nothing unblocked was skipped beyond FND-005 (eval attempt-node class) and FND-003 (authored operation constraint over a protocol, never exercised).

## Examined Scope

- FR-212-AC-1: examined. `chatter_carries_three_scheduler_constraints_and_no_fair_counterexample` asserts exactly three weak/whole/scheduler members over `s`, `r` and the root, and a full-product run with no fair counterexample. `proved` is blocked (item 1). Tagged TC-657 only, which is correct.
- FR-212-AC-2: examined. `scheduling_adversarial_admits_the_lasso_that_starves_a_branch` asserts an empty fairness set, the stem `fork(Both)`, `send(Tx)`, the loop `duplicate`, `lose`, `s`'s attempt enabled at every loop state, and `refuted`. The identity difference is blocked (item 2). Tagged TC-657 only, which is correct.
- FR-212-AC-3: examined. Both targets resolve, with the target, granularity and kind asserted. `Missing` refuses `missing_declaration`/`missing-name`. A second `scheduling` refuses `ambiguous_declaration`/`ambiguous-name`. The root and `left` classes over Fill are asserted as exact sets. See FND-001 and FND-002.
- FR-212-AC-4: examined. `a_safety_verdict_is_the_same_with_and_without_the_scheduler` asserts `proved` both ways, with equal certificates and stats.
- FR-217-AC-4: examined. The adversarial lasso replays as `Reproduced` and settles `refuted`. With the default scheduler set against the default package it refuses `UnfairProtocolLasso`, `invalid_runtime_input`/`invalid-value`, naming `r`'s member, which is first in JCS order. The AC names `s` as the starved thread, and the test asserts `s` is enabled at every loop state in the same run.
- FR-217-AC-1: context only (untagged, item 5).
- FR-217 Behavior (fairness-set recomputation): examined. `a_protocol_payload_without_the_scheduler_constraints_refuses` asserts `ProtocolFairnessSet` with both sets, `stale_dependency`/`content-mismatch`.
- ADR-027 PA-1 (a branch excludes nested-parallel threads; the root `each` is per thread): examined. These are the `fairness.rs` unit tests.
- QSpec FR-430 member encoding: examined. The unit test asserts the exact JCS bytes for a branch member and the root member.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-212-AC-3 text reads `fair weak whole s` and `fair weak each Done`. The tagged test checks `Chatter::Both::s` and `Chatter::Done`. As written, the AC's bare names refuse `missing_declaration`/`missing-name` in this implementation, so the tag claims an AC that the code contradicts word for word. The spelling is right per QSpec FR-430. Edit the AC (and TC-657 step 3) to the qualified spelling in this PR. | spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md:108, qsl-analyze/tests/it/scheduler_fairness.rs:519-560 |
| FND-002 | low | FR-212-AC-3 says the second `scheduling` member refuses "naming the first". The test asserts only the code and cause, not that the refusal names the first member's span. | qsl-analyze/tests/it/scheduler_fairness.rs:572-580 |
| FND-003 | low | Authored operation constraints over a protocol subject (origin `authored`, the FR-212 Outputs, and their merge beside the scheduler members in JCS order) are not exercised by any test. Every test runs with no authored constraint. A strong or `each` operation constraint over `Chatter`'s `Done` application is reachable today: operation targets resolve and emit. | qsl-eval/src/simulation/protocol/fairness.rs:282-310 |

## Coverage

| AC | Tag | Oracle |
| --- | --- | --- |
| FR-212-AC-1 | TC-657 only | partial: set shape and no fair counterexample; `proved` blocked on FR-339 |
| FR-212-AC-2 | TC-657 only | partial: lasso and empty set; identity blocked on FR-383 |
| FR-212-AC-3 | FR-212-AC-3 | full, with a spelling divergence from the AC text (FND-001) |
| FR-212-AC-4 | FR-212-AC-4 | full |
| FR-217-AC-1 | none | blocked on FR-433 wire |
| FR-217-AC-4 | FR-217-AC-4 | full |
