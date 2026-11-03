---
id: SR-2026110217004
title: "Code review of QSL-1514 progressed trace carrier (QSL PR #2219)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9e5fbc2935438b72a25edd1100eeeda8419369dc; PR #2219 origin/main...HEAD; QSL-1514, QSL-1495; qsl-eval/src/temporal/finite_window, qsl-eval/src/temporal/observed.rs, qsl-replay/src/spine/clause/temporal.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-504
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: reviews
---
# SR-2026110217004: Code review of QSL-1514 progressed trace carrier (QSL PR #2219)

## Summary

Ticket: QSL-1514 (QSL-1495 in part). Diff `origin/main...9e5fbc293`, rust lane (rust-review) folded in. Examined: the new `admit_progressed`, the evaluator profile widening, `PendingNextEvent`, silent-sample letters, the `TraceInput::Progressed` carrier and its binding check, and the two restated tests. No `allow`, `unwrap`, `expect` or `panic` on a production path was added (one `unwrap_or(0)` on a conversion that cannot fail). Root `src/temporal` is untouched; `qsl-eval/src/temporal` is the live evaluator and is touched as FR-504 requires. The restated tests are honest: `run_clause_returns_the_updated_ledger...` now asserts the FR-504 verdict, `a_clause_under_another_profile_is_refused` now uses an infinite-trace clause. Tests run: qsl-eval and qsl-replay tagged tests pass; remaining failures are the known clean-main set (QSPEC_DIR-gated, layers::equality, frame_occurrence_region, dispatch_link_compile, one doctest).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | admit_progressed materializes one String position per silent sample up to min(seen, last + reach) before any meter is charged, so a clause with reach 10^12 and W 10^12 allocates unbounded memory at admit. The bounded-work test only uses reach 3. Fix: charge the silent samples to the meter, or represent the silent tail as a range the evaluator reads without allocating, and add a test with a large reach. | qsl-eval/src/temporal/finite_window/progressed.rs:184-214 |
| FND-002 | low | lower_boundary is carried on TraceInput::Progressed beside progress, but FR-504 Inputs lists it inside progress. The carrier and the spec disagree on the shape; either the spec or the type must change. | qsl-replay/src/spine/clause/temporal.rs:50-53 |

## Verdict

Behavior matches FR-504-AC-1..8 on the tested paths and the old behavior (ProfileNotEvaluated) is gone. FND-001 is a real resource-bound gap on the new path. Mergeable after FND-001 is fixed or consciously deferred; FND-002 is resolved by a spec ruling.
