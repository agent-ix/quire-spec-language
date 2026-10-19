---
id: SR-13340
title: "base review of STD-215 hybrid solver budget wire spelling (PR #1056)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9ba8384c57223d2cbc3f592bd7e5b7b03a1159d1; spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md, spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-705
    type: reviews
---
# SR-13340: base review of STD-215 (PR #1056)

## Summary

Ticket: STD-215. Base checklist over `git diff refs/remotes/origin/main...9ba8384c5`
(FR-250 verdict-map rows, budget bullets, AC-2, AC-3; TC-705 step 3 and its
expected result), compared against merged QSpec `refs/remotes/origin/main`
FR-331 "Hybrid solver budgets" (and its incomplete-cause row and AC-29) and
FR-193 "Verdict map" / "Solver budgets" / AC-8 / AC-9.

The wire spellings match QSpec: budgets in `hybrid.limits.max_solver_steps` /
`hybrid.limits.max_solver_time`, setting names `hybrid.max_solver_steps` /
`hybrid.max_solver_time`, step stop
`LimitReached{limit: max_solver_steps, value, setting: hybrid.max_solver_steps}`,
`max_solver_time` exhaustion and provider `deadline_ms` overrun both
`TimedOut`, the frontend deadline `Cancelled{source: deadline}`, writer rounds
fractional time up and writes an integer, a fractional wire value refused,
`max_solver_time` outside request identity / the FR-292 key and never stored.
Positive counts are stated for both budgets. The text describes behaviour,
not a changelog. `make check-spec-validation` exits 0 with no diagnostic on
FR-250 or TC-705 (pre-existing warnings elsewhere only);
`tools/check-index-completeness.sh` exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The normative verdict-map table (the "map SHALL be") still says the `TimedOut` row is "distinct from `Cancelled(Deadline)`". `Cancelled(Deadline)` is the FR-276 `StageFailure`/`CallFailure` spelling, not the record cause; the record cause is `Cancelled{source: deadline}` (FR-127, ADR-018 V-7, QSpec FR-331), which the bullets, AC-3 and TC-705 now use. The table also has no row for the frontend-deadline case, which QSpec FR-193's verdict map gives its own row (`incomplete`, execution `resource-incomplete`, `cancelled{source: deadline}`); and the bullet that introduces it says "SHALL settle `Cancelled{source: deadline}`" without the `incomplete` result. Respell the cell and add the row. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:75,100-102 |
| FND-002 | medium | FR-250 now states each budget is a positive count and that `hybrid` and its `limits` are never empty, but no AC or TC step exercises it: AC-3 and TC-705 step 3 cover only the fractional wire value. QSpec FR-331-AC-29 refuses a zero budget, an unknown `hybrid.limits` member and an empty `hybrid` or `limits` before consumption. A `max_solver_steps` of 0 and an empty `limits` could be admitted without any FR-250 test failing. Add those cases to AC-3 and TC-705. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:85-88,104-105,119; spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md:23,31 |
| FND-003 | medium | The table's "Provider result" column and AC-2 ("Fixture results map ...") now include "a provider overrun of its `deadline_ms`", but Inputs defines the provider result as `SolverResult` whose `SolverOutcome` is `Unsat`, `DeltaSat`, `EnclosureDisjoint`, `EnclosureMeetsUnsafe`, `Unknown` or `Stopped(limit)`, and TC-705's fixture returns "each `SolverOutcome`". A deadline overrun is not one of them, so the FR does not say how QSL observes it (a `Stopped` variant, or an envelope-level event the host detects). Two implementers would model it differently and the TC fixture cannot produce it as written. State the observation point in Inputs and the fixture. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:46-50,75,118; spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md:19 |
| FND-004 | low | QSpec FR-331 is now cited as the authority for the budget layout ("(QSpec FR-331)"), but FR-250 has no `relationships` edge to `ix://agent-ix/quire-specification/FR-331` and the Dependencies section lists only QSpec FR-193. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:4-20,88,128 |
| FND-005 | low | The bullet calls `TimedOut` "the time-limit cause". QSpec FR-331-AC-29 refuses a `time-limit` cause on a hybrid item as an unknown spelling, so the phrase reads like the refused wire spelling. Say "the wall-clock cause" or drop the qualifier. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:99 |
| FND-006 | low | The touched budget bullets keep "published default 60 seconds" / "1,000,000 steps" without saying, as QSpec FR-193 does, that the request writer omits an unset member and the provider applies its descriptor `bounds`. Since `max_solver_steps` is part of request identity, a writer that writes 1,000,000 and one that omits it produce different key bytes for the same request. State that the defaults are QSL's built-in solver descriptor bounds and an unset budget is omitted. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:89-91,104-106 |
| FND-007 | low | Three rewrapped Behavior lines run well past the file's wrap width (lines 91, 95, 102), leaving the bullets ragged. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:91,95,102 |

## Verdict

Not mergeable as is: three medium findings (FND-001 to FND-003), no high.
The core wire spellings the ticket targets all match QSpec FR-331 and FR-193.
Examined and clean: the `max_solver_steps` table row, the
`hybrid.limits` / setting-name bullet's spellings, the rounding/wire-integer
bullet, the `max_solver_time`-outside-the-key statement and AC-3's equal-key
clause, the `TimedOut` vs `Cancelled{source: deadline}` split in the bullets,
AC-3 and TC-705, the step-stop `LimitReached` spelling in the bullet, AC-2,
AC-3 and TC-705, and the "positive" wording for steps (was "non-negative").

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | The two new refusal bullets say the hybrid provider refuses "as FR-331 refuses any out-of-domain value". In a QSL file a bare FR id is a QSL artifact, and QSL FR-331 is "Replay a temporal counterexample over an observed trace", so the citation points at the wrong requirement. Write "QSpec FR-331". | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:98,107 |
| FND-009 | low | The new Inputs bullet says the host "observes it as the provider's deadline event on the call", and TC-705's fixture repeats "the provider deadline event". QSpec FR-462 has no such event: the host enforces `deadline_ms`, ends the provider process and settles each open item `incomplete`, `timed-out`. "the provider's deadline event" reads as something the provider emits, and "that item" understates "each open item". Say the host ends the provider at `deadline_ms` (QSpec FR-462) and settles each item still open `TimedOut`. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:53-55; spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md:19 |
| FND-010 | low | TC-705 step 3 writes "an empty `hybrid` and `limits`" and expects "the empty `hybrid` and `limits`" refused, which reads as one request. AC-3 says "an empty `hybrid` or `limits` are each refused", which needs two requests (an empty `hybrid`, and a `hybrid` with an empty `limits`). Name both requests. | spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md:23,31 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | `Cancelled(Deadline)` is gone and the bullet now states `incomplete`, execution `resource-incomplete`, but the new frontend-deadline row sits after a blank line (line 82), so it is not part of the verdict-map table. Markdown ends a table at a blank line, so the row renders as a separate paragraph and no table parser sees it. Delete the blank line at line 81. |
| FND-002 | fixed | AC-3 and TC-705 step 3 now cover a zero budget, an unknown `hybrid.limits` member and an empty `hybrid` or `limits`, each refused by the hybrid provider before consumption, and a new bullet states the refusal. |
| FND-003 | fixed | Inputs now says a `deadline_ms` overrun is not a `SolverOutcome`, that the host observes it on the call, and that no `SolverResult` is read. TC-705 adds a fixture that overruns. This agrees with QSpec FR-462 (the host ends the process and settles the item `timed-out`); the wording nit is FND-009. |
| FND-004 | fixed | A `depends_on` edge to ix://agent-ix/quire-specification/FR-331 and a "QSpec FR-331" Dependencies entry were added. |
| FND-005 | fixed | The bullet now reads "with the cause `IncompleteCause::TimedOut`". |
| FND-006 | fixed | A new bullet states that the writer omits an unset budget, that the provider applies its descriptor `bounds`, and that 60 s and 1,000,000 steps are QSL's built-in solver bounds. AC-3 and TC-705 check the omission. |
| FND-007 | fixed | The Behavior bullets are rewrapped within 80 columns; only table rows exceed it. |
