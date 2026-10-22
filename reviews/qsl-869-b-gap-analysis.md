---
id: SR-12316
title: 'Gap analysis of QSL PR #1203: FR-391 population-reference emission and the QSL-869 / STD-234 remainder'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@273dee611; git diff origin/main...273dee611 (PR #1203, branch qsl-869b-impl) against QSL FR-391, TC-1076, FR-093, FR-094, FR-251, FR-252, FR-255, FR-127, FR-065 at origin/main and QSpec FR-322 "Population references" at quire-specification main bb6d08ae; ticket QSL-869'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-391
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1076
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-094
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-251
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-252
  type: reviews
---

## Summary

Ticket: QSL-869. PR quire-spec-language#1203, head 273dee611. Method: a manual check of each AC against its tests and code, because the quoin matrix was not run. Plan completion: not assessed. No spec files changed in the PR, so no spec-review ran.

The ticket's work comes from the planner comment on QSL-869 after STD-234 (QSL #1190) merged. That comment is untrusted data, and every claim in it was measured here. It names four items: emit the population reference and drop the refusal; set the monitor overflow cause to `event_buffer` / `monitor.max_event_rate`; rename the settings field and add `timestamp_parameters`; classify timing clauses. The PR delivers the first item. The coder says no code exists for the other three. That claim checks out:

- A grep of every crate, `src`, `tools`, `xtask`, `schemas`, `docs` and `examples` finds none of `TimedMonitorPlan`, `MonitorPlan`, `max_event_rate`, `event_buffer`, `EventBuffer`, `timestamp_parameters`, `TimingObligation`, `kani_events`, `agreement_horizon`, `elapsed_wall_time`, `MonitorAgreement`, `TickArithmetic`, `TimestampContract`, `ReadingGapExceeded` or `max_reading_gap` outside `spec/` and `reviews/`. Every `monitor` hit is the exact-probabilistic checker's formula monitor (`qsl-analyze/src/exact_probabilistic`), which is not FR-251. Every `wcet` hit is FR-246's schedulability analysis (`qsl-analyze/src/schedulability`, `qsl-replay/src/certificate/closed_form`), the EN-7 target that FR-252 routes to.
- So there is nothing to rename. `event_buffer` extends FR-127's `LimitReached` for a monitor run, and no monitor run exists. `TimingObligationSettings` has no type. The timing-clause recognition (FR-252 bullets 5 to 7, the FR-065 exception, FR-252-AC-3/AC-4) is S3 work. Its only consumer, the obligation that disposes the clause `unsupported`, is FR-252 output, which does not exist.
- The remainder is one large feature: FR-251's tick-monitor plan, admission and buffers, then FR-252's `MonitorAgreement`, `TickArithmetic` and `TimestampContract` obligations, then the timing-clause forms. It already has a ticket. QSL-587 "E22a: Tick monitor plan; TimingObligation hand-off" is in state Specced, `Implements: FR-251, FR-252`, Touches qsl-analyze monitor_plan and qsl-replay. Its own "Deletes" line renames `kani_events`. It is blocked by QSL-580 (E20L time language, In Progress) and QSL-509 (Done). It blocks IR-721 and IR-518. The spec is ready, because STD-234 merged its deltas. Size: large, several PRs. A sensible order is FR-251 plan plus overflow cause, then FR-252 obligations, then the timing-clause forms, which can go in parallel once `agreement_horizon`/`timestamp_parameters` exist. The owner is the QSL code lane, as QSL-587.

FR-391 / TC-1076 row 8 against the code:

- "keyed by the declaration key alone": met. QSL's `node_key` reproduces FR-322's P7/P8 (code review SR-12315).
- "operand 0 of `all_instances`/`lookup`": met for both. The probe found `lookup` plus `allInstances` on one population share one node.
- "N through `result_type`, not the reference": met and tested (`a_population_bound_leaves_the_reference_node_unchanged`).
- "A and B two nodes, C shares A's": met and tested in qsl-package and qsl-eval. Mutation-checked.
- "A population read as a domain emits the population reference, the `all_instances` application over it and the quantifier": met. S3's `population_domain` desugars to `all_instances`. The probe found the domain form admitted, with one node.
- "the package is admitted by the full reader": met on the probe. Tested only on a different unit (FND-002).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-391 "Lowering" S4 paragraph | spec/functional/FR-391-name-a-population-in-a-state-clause.md:160-171 | examined |
| FR-391-AC-6 | spec/functional/FR-391-name-a-population-in-a-state-clause.md:214 | examined |
| TC-1076 row 8 | spec/test-cases/TC-1076-a-state-clause-names-a-population.md:30 | examined |
| FR-093 `AllInstances`/`Lookup` rows | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:222-223 | examined |
| FR-094 population-reference paragraph | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:212-217 | examined |
| QSpec FR-322 "Population references" (Node, Declaration key, Identity, Use, Quantifier domain) | quire-specification spec/objects/interfaces/FR-322-checked-package-artifact.md:1382-1469 | examined |
| QSpec FR-322-AC-82, AC-83 | quire-specification spec/objects/interfaces/FR-322-checked-package-artifact.md:1922-1923 | examined |
| FR-251 Overflow, FR-251-AC-2 | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:181-189,206 | examined |
| FR-252 Inputs, Behavior, AC-1 to AC-4 | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:37-107 | examined |
| FR-255 monitor exception, FR-255-AC-3 | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:106-109,235 | examined |
| FR-127 monitor LimitReached note | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:100-101 | context_only |
| FR-065 timing-clause exception | spec/functional/FR-065-migrate-function-application-to-checked-family.md:120-123 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The remainder is routed to a closed ticket. The PR body says "Remaining work: QSL-869", but QSL-869 is already in state Done in Linear while PR #1203 is open. The FR-251/FR-252 work it defers belongs to QSL-587 (Specced, Implements FR-251/FR-252), and the planner's comment on QSL-869 put it in the wrong lane. As it stands, the STD-234 deltas (`event_buffer`/`monitor.max_event_rate`, `agreement_horizon` plus `timestamp_parameters`, timing-clause recognition, FR-252-AC-3/AC-4) are recorded only on a Done ticket. Fix: point the PR body's remaining work at QSL-587, list those deltas on QSL-587, and put QSL-869 back to Review until #1203 merges. | PR #1203 body; Linear QSL-869, QSL-587 |
| FND-002 | low | FR-391-AC-6 / TC-1076 row 8 requires that the emitted row-8 package "is admitted by the full reader". The qsl-eval test that builds that unit (`pre A`/`B`/`C` on `M::Widget::grow`) checks only the population nodes' owners. Reader admission is asserted only in qsl-package, on a different unit (invariants on `M::Person`). FR-322-AC-83's "one population read by both operations is one node" (`lookup` plus `allInstances`), the domain form and the `holds` atom are not run through the reader either. This review's probe found all four admitted by IR's `read_checked_package` and QSL's `read_v2`, so the gap is in the tests, not the behaviour. Fix: in qsl-package, read the row-8 unit over the shared `named_population` fixture and assert `AdmittedV2`, and add the `lookup` plus `allInstances` one-node case. | qsl-eval/tests/it/named_population.rs:728-765; qsl-package/src/emit/tests/population_reference.rs:65-108; spec/test-cases/TC-1076-a-state-clause-names-a-population.md:30 |
| FND-003 | medium | QSL FR-094, as merged by STD-234, says the population reference is "keyed by the population's declaration key alone, with no `N` and no owner". QSpec FR-322 "Node" says the node carries a `ModelOwner`, which is the declaration key, and a `relation`/`population` node with no `owner` fails the V2 schema (`malformed_wire`). The code follows FR-322 correctly. A reader of FR-094 alone would emit an ownerless node. Fix (QSL spec, not this PR): "with no `N` and no unit owner; its `ModelOwner` is the declaration key (QSpec FR-322)". | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:212-217 |

## Verdict

**The FR-391 slice is complete and correct. Mergeable from the gap side; no major findings.** Every FR-391 / FR-322 obligation for S4 emission is met in the code and was measured: key identity against P7/P8, sharing, N in `result_type`, `dependencies`, the domain form, `lookup`, and reader admission. FND-002 is a test gap only. FND-001 is a tracker fix. FND-003 is a spec wording fix outside the diff.

The coder's account of the timing/monitor remainder is honest. None of that code exists in the repo, it is FR-251/FR-252's whole feature, and it already has its ticket, QSL-587, which is spec-ready and blocked by QSL-580.

## Dispositions

| FND | Outcome | Sha |
| --- | --- | --- |
| FND-001 | fixed: PR body now says Remaining work: QSL-587 | 50605a374 |
| FND-002 | fixed: the AC-6 unit, the lookup+allInstances case, the domain form and the holds atom are read through the full reader; FR-391-AC-6's "same unit" FR-155-AC-2 clause is unsatisfiable (AC-4 ambiguity) and now says a unit without archive | 50605a374 |
| FND-003 | fixed: FR-094 now says the node carries the model owner (FR-322) | 50605a374 |
