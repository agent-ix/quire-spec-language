---
id: SR-13597
title: "base review of the FR-186 and FR-143 spec amendments (PR #1117)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@fd004b67b7c804e521a23e15237462c6d557bb6e; spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md, spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md, spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md, spec/test-cases/TC-548-the-liveness-half-reads-abstract-fairness-through-the-mapping.md, spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-548
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-621
    type: reviews
---
# SR-13597: base review of PR #1117 (QSL-568, QSL-547)

## Summary

Tickets: QSL-568 and QSL-547. This is the base checklist over
`git diff origin/main...fd004b67b` (5 files, +133/-50): FR-143, FR-185,
FR-186, TC-548 and TC-621. The PR changes only spec files, so code-review
and gap-analysis do not apply. Sub-analyses: ears-conformance (SR-13598),
integrity (SR-13599) and failure-domain (SR-13600).

Gates: `make check-spec-validation` exits 0, and no warning is on a touched
file (the 17 warnings are elsewhere, and main has the same set).
`tools/check-index-completeness.sh` passes.

Purpose check. The diff answers the spec follow-ups of the #1116 reviews
(SR-13595 FND-001, 007, 008, 009, 012; SR-13596 FND-001 to 003, 005) and
the #1113 reviews (code-review FND-001 and 003, gap-analysis FND-003 and
005). The new criteria FR-186-AC-5 and FR-143-AC-6 are each traced: TC-621
scope now ends at AC-5 with step 5, and TC-548 scope ends at AC-6 with step
6. ID formats and AC numbering follow the series. No changelog, status or
owner wording was added. "This build" in FR-143 is the repo's usual way to
scope a capability, and "pending QSL-883" was there before this diff.

Can the code meet each statement? Yes, for every FR-186 statement. The #1116
mismatches the coder is fixing (binder name in the bytes, unsorted weights,
`Debug` bytes, TP-2 advice in the fraction message) each need only a direct
change. FR-143's strong-`F_A` rule is not yet in the #1113 code
(refine.rs:213-227 has no strong branch), and it is buildable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-186-AC-4 does not cover some of the normal-form content rules that the Normal form section states and the rulings require. It varies only the confidence and the workload, and checks invariance only under reordered weights, a renamed binder, a commuted equality and equal-content declarations. Nothing checks that reordered fairness constraints or reordered reward declarations keep one normal form ("the constraints of a fairness set and the operations of a reward are sets"). Nothing checks that a claim differing only in its subject, or only in its `over` type, has another normal form. The #1116 review showed both gaps matter: the mutants that drop the subject or the `over` binding from the bytes survive (SR-13595 FND-005). Fix: add these four cases to AC-4 and TC-621 step 4. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:235; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:208-216; spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md:24 |
| FND-002 | low | FR-143-AC-6's second half, "the same refinement with `ensure fair weak whole Spec::Pair::flipB` runs the second phase", names no verdict, and "runs" cannot be observed directly. TC-548 step 6 and its expected result cover only the strong case. With `flipB` enabled only at `a = 0` and `flipA` weakly fair on every loop, the weak variant returns `Checked(HoldsReduced{reductions: []})`. Fix: name that verdict in AC-6, and add the weak variant to TC-548 step 6 and its expected result. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:151; spec/test-cases/TC-548-the-liveness-half-reads-abstract-fairness-through-the-mapping.md:35 |
| FND-003 | low | FR-186-AC-3 states the trace-subject refusal twice: "A `probabilistic` claim over a trace subject is refused, and its diagnostic names window aggregates" and, later, "A claim over a trace subject is refused `trace-subject`". Fix: merge them into one clause, "refused `trace-subject`, its diagnostic naming window aggregates". | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:234 |
| FND-004 | low | FR-186-AC-5 says `steps ... >= rational(1, 2)` and `steps ... >= 5 ms` "are each refused `ill_typed`" with no cause. Behavior pins both to `ill_typed`/`type-mismatch` ("a rational for `steps`, a duration where a natural number is due"). Behavior also routes a dimensioned threshold on an `accumulate` measure to `unit-mismatch`, so the cause is what tells the two apart. An implementation that refused the duration `unit-mismatch` would pass the AC while breaking the statement. Fix: name `type-mismatch` for both in AC-5 and TC-621 step 5. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:236; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:160-163 |

## Verdict

Not mergeable as is, together with the integrity review (SR-13599 FND-001,
high). This base review has one medium finding and three low ones, all
AC or TC wording within this PR.

Recorded clean: FR-186-AC-1 and AC-2 (unchanged in meaning), AC-3's new
`not-temporal-profile` and `unbounded-event` cases, AC-5's literal, fraction
range, horizon-overflow and S4 cases, and TC-621's scope and steps 1 to 5.
FR-143-AC-1 to AC-4 move to the `HoldsReduced{reductions: []}` and typed-cause
forms, and TC-548 steps 1 to 4 do the same. FR-185 "Package emission" gains
`probabilistic-claim`, which FR-186-AC-5 verifies.
