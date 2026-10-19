---
id: SR-13085
title: "Failure-domain review of PR #1030: resource limits and binding order in the ConfigVersion rows"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@ffc3b9d9f; git diff origin/main...HEAD: ADR-018 §6, FR-126-AC-1/AC-2, TC-521"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
---

## Summary

Ticket: STD-204. Walked the failure cases the restated rows depend on: the candidate limit, the result-pair charge, the missing component certificate (Holds rows refuse `ComponentCertificateNotBuilt` today, with the stated counts), the undecided TP-2 form, and the cross-binding order. Measured on PR #1024's head (qsl-530-e9, a2ad83a6b), `cargo test -p qsl-analyze --test it example_units`: 6/6 pass, reproducing 15 product states for one instance and 30 for both (refused `ComponentCertificateNotBuilt`, FR-339 not built), the no-granularity lasso (empty stem, length 3, all steps one receiver, bound object stays 0, the other cycles 0,1,2,0), and the shipped-unit `each` loop `upd(a), upd(b)` at `(0,0)`. Reviewer probes in a scratch clone of that head: the no-granularity run binds `over` to `a` and loops `upd(b)` x3; `always holds(c.versionNumber <= 1000)` returns `Holds` with a closure certificate (TP-1 row reproduced); `eventually[0,5] holds(c.versionNumber = 2)` under `quire.temporal.event-position.false-extension/v1` returns `FormNotDecided{form: BoundedMltl}` (TP-2 is not decided at that head; QSL-528 / PR #1029 is open); the `(pre(self.versionNumber) + 1) mod 3` post clause is refused `IllTyped(TypeMismatch)` at S3 (the ADR's reason holds); with the example's no-granularity request, `max_candidates` 1000, 2002 and 3003 stop `Stopped{ResourceExhausted, MaxCandidates}` at depth 0, and 6006 and above give the AC-1 lasso.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-126-AC-1 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126-AC-2 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| ADR-018 §6 Frame | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 post clause | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 Subject | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 Verdict under fair weak each | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 A bounded variant | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| TC-521 Test Procedure fixtures | spec/test-cases/TC-521-explicit-state-model-check.md | examined |
| FR-120 candidate count | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | context_only |
| FR-154 | spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ConfigVersion rows hold only above an unstated `max_candidates`. Under the receiver-scoped frame each application still enumerates the receiver's 1001 `versionNumber` values and their result pairs, so an expansion charges about 6,000: measured, the no-granularity request stops `Stopped{ResourceExhausted, MaxCandidates}` at depth 0 with `max_candidates` 1000, 2002 or 3003, and returns the AC-1 lasso at 6006 and above (the fixture uses 1,000,000). TC-472 uses 1,000, which would stop this example the way §6 says the shipped frame stops. Neither AC-1/AC-2/AC-5, §6 nor TC-521 names the setting, and `max_candidates` has no published default. Add the subject's `max_candidates` (e.g. 1,000,000) to TC-521's fixtures and §6's Subject, and state that the rows assume it. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:352-354; spec/test-cases/TC-521-explicit-state-model-check.md:22-25; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:400-406 |

## Verdict

Findings: FND-001 (medium). The engine-measured rows (15/30 product states, the no-granularity lasso shape, the shipped-unit `each` loop) match the spec; FR-120 is unchanged and the receiver-scoped frame is FR-154's; AC ids are kept and the text reads as behaviour.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | "a smaller setting such as TC-472's 1,000 stops the run ... at the first expansion" reads as any setting below 1,000,000 stopping it. Measured, 1000, 2002 and 3003 stop at depth 0 and 6006 already returns the AC-1 lasso; the threshold is about 6,000 (two receivers, 1001 candidates and their result pairs each). Say "a setting below about 6,000, such as TC-472's 1,000, stops ...". | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:375-379 |

## Dispositions

Round 1, against head 439d37769 (fix commit 6cdb4afe3).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-521 fixtures: "and the subject's `max_candidates` set to 1,000,000 (TC-472's 1,000 would stop the example at its first expansion)"; FR-126-AC-1: "and the subject's `max_candidates` 1,000,000"; ADR-018 §6 **Candidate limit** paragraph. |
