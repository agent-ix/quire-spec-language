---
id: SR-13083
title: "Spec review of PR #1030: ADR-018 §6 ConfigVersion example under the receiver-scoped frame"
type: SpecReview
scope: "agent-ix/quire-spec-language@ffc3b9d9f; git diff origin/main...HEAD: ADR-018 §6, FR-126-AC-1/AC-2, TC-521"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
---

## Summary

Ticket: STD-204. Base spec review of `git diff origin/main...HEAD` at ffc3b9d9f: ADR-018 §6, FR-126-AC-1/AC-2 and TC-521's fixtures and step 1. FR-120 is unchanged and the example's `modifies_receiver` frame is FR-154's receiver scope; AC ids are kept; the added text states behaviour and rationale, not a change log. Measured on PR #1024's head (qsl-530-e9, a2ad83a6b), `cargo test -p qsl-analyze --test it example_units`: 6/6 pass, reproducing 15 product states for one instance and 30 for both (refused `ComponentCertificateNotBuilt`, FR-339 not built), the no-granularity lasso (empty stem, length 3, all steps one receiver, bound object stays 0, the other cycles 0,1,2,0), and the shipped-unit `each` loop `upd(a), upd(b)` at `(0,0)`. Reviewer probes in a scratch clone of that head: the no-granularity run binds `over` to `a` and loops `upd(b)` x3; `always holds(c.versionNumber <= 1000)` returns `Holds` with a closure certificate (TP-1 row reproduced); `eventually[0,5] holds(c.versionNumber = 2)` under `quire.temporal.event-position.false-extension/v1` returns `FormNotDecided{form: BoundedMltl}` (TP-2 is not decided at that head; QSL-528 / PR #1029 is open); the `(pre(self.versionNumber) + 1) mod 3` post clause is refused `IllTyped(TypeMismatch)` at S3 (the ADR's reason holds); with the example's no-granularity request, `max_candidates` 1000, 2002 and 3003 stop `Stopped{ResourceExhausted, MaxCandidates}` at depth 0, and 6006 and above give the AC-1 lasso.

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
| FND-001 | medium | The TP-2 row (`eventually[0,5]` ... `Violated` with a five-step prefix) has no engine run behind it: at PR #1024's head the request returns `FormNotDecided{form: BoundedMltl}`, and the bounded-MLTL engine (QSL-528, PR #1029) is unmerged. STD-204 asks for the AC-2 rows recomputed from the real engine and "a vector that the engine reproduces", and the PR title says the rows reproduce. The row is consistent with CX-1 (a bounded prefix covers the horizon, six positions) and the 9-state graph, so the spec text need not change; the gap is the ticket's acceptance. Keep STD-204 (or a named follow-up gated on QSL-528) open for the TP-2 reproduction rather than closing it on this PR. The TP-1 row was reproduced by this review (Holds with a closure certificate). | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:353; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:458-463 |
| FND-002 | low | AC-1 and §6 still name the reported binding `c = b` with the loop on `a`, which neither the engine (measured: `over` = `a`, loop `upd(b)` x3) nor STD-203's default ranking (shortest counterexample, tie by canonical binding order, so `a` first for two length-3 lassos) reports. The "up to exchanging `a` and `b`" clause lets either pass, so AC-1 no longer fixes which binding is reported. That deferral does not pre-empt STD-203, but the literal `c = b` reads as the expected value. When STD-203 lands, AC-1/AC-2 and §6 must name the ranked binding and drop the exchange clause; until then say the reported binding is the one STD-203's ranking selects. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:352-353; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:406-410,432 |
| FND-003 | low | "15 product states for one bound instance and 30 for both": the AC's request is `over (c: Config::ConfigVersion)` over universe `{a, b}`, which always binds both, and `RunStats.product_states` reports the total (30). No request the AC names yields the 15; the fixture gets it by restricting the letters to `b`. State the 30 as the request's count and the 15 as the per-binding product size, or name the request that binds one instance. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:352; spec/test-cases/TC-521-explicit-state-model-check.md:50-51 |

## Verdict

Findings: FND-001 (medium), FND-002 (low), FND-003 (low). The engine-measured rows (15/30 product states, the no-granularity lasso shape, the shipped-unit `each` loop) match the spec; FR-120 is unchanged and the receiver-scoped frame is FR-154's; AC ids are kept and the text reads as behaviour.
