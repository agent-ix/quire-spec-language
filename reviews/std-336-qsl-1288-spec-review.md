---
id: SR-16196
title: "spec-review of the HP-2 SMT solver-stop precedence (quire-spec-language PR 1288)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@09a19b63; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/functional/FR-182-settle-a-hyper-verdict.md; spec/test-cases/TC-1190-the-hp-2-product-identity-binds-each-smt-outcome-to-its-item.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1190
    type: references
---

# Spec review of the HP-2 SMT solver-stop precedence (quire-spec-language PR 1288)

## Summary

Ticket: STD-336 (follow-up to merged #1282). One spec-review pass over
`git diff origin/main...09a19b63`. Oracles: FR-250's deadline rule, FR-276,
FR-293, FR-127's `IncompleteCause`, `qsl-replay`'s `ReachedLimit` and
`IncompleteCause::TimedOut`, and QSpec #388 at head 4374ec62 (FR-363
"Undecided k-induction runs", FR-363-AC-10, FR-399-AC-12, TC-294 step 9).

Gates: `make check-spec-validation` (QSPEC_DIR=~/dev/quire-specification)
exits 0 with no finding on the touched files. origin/main is an ancestor of
the head.

Examined, clean:

- The order is sound. There are four rules: a tripped cancellation handle,
  then the backend's wall clock or a deadline overrun, then a limit the
  request names, then the solver's own `unknown`. They are applied first
  match wins, so no stop gets two causes. Cancellation comes first, which
  keeps FR-250's rule that a caller's deadline is `Cancelled{deadline}` and
  never `TimedOut`. FR-250 likewise maps a provider's `deadline_ms` overrun
  to `TimedOut`.
- The causes already exist. `IncompleteCause::TimedOut` (FR-293) and
  `LimitReached{limit, value, setting}` exist, and `ReachedLimit` is
  `qsl-replay`'s `model_check::limits` type. The new V-7 `TimedOut` row uses
  the same label, basis and category as the other V-7 rows.
- It matches QSpec #388. FR-363's four rules are the same, in the same
  order, and use the same wording ("a resource stop internal to the solver
  that no request limit names").
- ADR-018 V-6 keeps the "not closed at `k`" meaning and states the same
  order.
- Every rule has a vector. FR-182-AC-12 and TC-1190 step 7 cover
  `TimedOut`, a named limit and an internal stop, and each vector can fail
  because it names its rule's cause and the cause it must not get. The
  earlier vectors are unchanged.
- Ids. The PR adds no new id. `LimitStopped` and `TimedOut { method` appear
  in no other open PR.
- The lead defaults (the order, and `TimedOut` with no new cause) are
  labelled in the PR body.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | [minor] The named-limit vector uses `max_states`, which does not govern an SMT run. FR-182-AC-12 and TC-1190 step 7 settle "a stop under the request's `max_states`, reported `LimitStopped`" as `LimitReached{MaxStates, …}`. `max_states` is EN-1's explicit-state limit, and no `ModelCheckLimit` bounds CG's SMT solver yet. `SolverSteps` is FR-250's hybrid budget, and IR-705 owns the SMT ceiling. The vector tells CG that it may attribute an SMT stop to a limit the run never used, and the record would then state that limit's value and setting. Rule 3 also does not say whether `settle_hyper_smt` checks that the limit is one of the run's limits. Fix: use a limit an SMT run is actually bounded by, or say that rule 3 applies only once a `ModelCheckLimit` governs the solver and keep the vector abstract. State what happens when `LimitStopped.limit` names a limit that does not govern the run. | spec/functional/FR-182-settle-a-hyper-verdict.md:107, :245-248, :291 | correct-requirement-no-evidence |
| FND-002 | low | [minor] Rule 2's "a deadline the request gave it" names no field. QSpec FR-363 names `deadline_ms`, and FR-250 names the hybrid provider's `deadline_ms`, but neither `HyperQueryRequest` nor `ModelCheckLimits` carries a deadline. The order also does not cover a stop that is not a resource stop, such as a solver crash or malformed solver output. A reader could report it as `Unknown`, which settles V-6 `InductionNotClosed`, where an internal fault (`failed`) is meant. Fix: name the request field the rule reads, or say the overrun clause applies when the backend has a request deadline. State that a solver fault is not a stop and settles as an internal fault. | spec/functional/FR-182-settle-a-hyper-verdict.md:237-252 | missing-requirement |

## Verdict

Mergeable. The order is sound and covers every stop CG reports. It agrees
with FR-250, FR-276 and FR-293, and with QSpec #388's FR-363 order. Its ACs
can fail. The two low findings can be fixed in this PR or deferred. Neither
blocks a merge.
