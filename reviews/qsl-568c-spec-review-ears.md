---
id: SR-13598
title: "ears-conformance review of the FR-186 and FR-143 spec amendments (PR #1117)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@fd004b67b7c804e521a23e15237462c6d557bb6e; spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md, spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md, spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
---
# SR-13598: ears-conformance review of PR #1117 (QSL-568, QSL-547)

## Summary

Tickets: QSL-568 and QSL-547. This review covers the requirement statements
that the diff adds or edits. In FR-143: the Description sentence, the second
and third "When the half runs" bullets, the "No violating SCC" bullet and
the fault bullet. In FR-185: "Package emission". In FR-186: Description, the
trace-subject, temporal-profile and workload-name bullets, the unbounded-event,
horizon-overflow, literal-threshold and fraction-range bullets, the
probability-threshold bullet, and the seven Normal form bullets. The engine
check (`make check-spec-validation`) gives no `[ears:*]` warning on any
touched file. Semantic review found three low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Four of the FR-186 Normal form bullets carry no SHALL, yet FR-186-AC-4 checks them as requirements. These are "Subject, profile, ... are content: a claim that differs in one of them has another normal form", "... are sets: the order in which they are written is not content", "The `over` binder's name is not content ..." and "A predicate is recorded in its normal form (FR-174) ...". They read as definitions, so no normative statement makes S3 responsible for the invariance the AC tests. Fix: phrase them as one ubiquitous requirement, for example "S3 SHALL give two checked claims equal normal forms exactly when they agree on that content, where the order of weights, fairness constraints and reward operations, the `over` binder's name and the operand order of a commuted equality are not content". | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:208-220 |
| FND-002 | low | FR-143's third "When the half runs" bullet is compound. It holds two SHALLs (`liveness` SHALL be `Unsupported`; no second phase SHALL run), a statement about the safety half, and a rationale ("The second phase reads an `F_A` class as weak ... this build does not decide that reading"). The rationale is not a requirement and belongs in the Description, which already says it. Fix: keep the condition and its two SHALLs, and move the two rationale sentences out of the bullet. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:89-95 |
| FND-003 | low | FR-186 "Events and measures" now states the comparison-threshold type rule twice. "A comparison `M <= c` or `M >= c` SHALL have a threshold `c` of `M`'s type: ... an exact rational in `(0, 1)` for `fraction`" is followed by "A comparison threshold SHALL be a literal of the measure's type". The fraction range then appears both as a type rule (first bullet) and as a `qualitative-threshold` refusal (later bullet), so a reader cannot tell whether `3/2` is a type error or a range error. Fix: merge the first two bullets into one ("SHALL be a literal of `M`'s type: a natural number for `steps`, the reward's dimension for `accumulate`, a rational for `fraction`"), and keep the `(0, 1)` range only in the refusal bullet. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:157-168 |

## Verdict

Mergeable as far as EARS goes. All three findings are wording fixes that
can go into the fix round for SR-13599. The other edited statements conform:
the fault bullet and each refusal bullet are well-formed unwanted-behaviour
statements, and the FR-185 "Package emission" statement and the S4 bullet of
FR-186 are well-formed event statements.
