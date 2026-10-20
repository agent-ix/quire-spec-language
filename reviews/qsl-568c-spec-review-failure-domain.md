---
id: SR-13600
title: "failure-domain review of the FR-186 and FR-143 spec amendments (PR #1117)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@fd004b67b7c804e521a23e15237462c6d557bb6e; spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md, spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
---
# SR-13600: failure-domain review of PR #1117 (QSL-568, QSL-547)

## Summary

Tickets: QSL-568 and QSL-547. This review looks for unstated failure modes
in the paths the diff adds: the FR-143 second-phase fault, the overlap of
the strong and hidden-field conditions, and the FR-186 horizon overflow,
unresolved names, and normal-form identity confusion. The overlapping
FR-143 conditions are reported in SR-13599 (FND-001 and FND-002), and the
unresolved-name refusals in SR-13599 (FND-007 and FND-008), so they are not
repeated here. Identity confusion was checked. FR-186 now keeps the normal
form apart from O-09's identity, and two declarations of equal content stay
two obligations, so there is no collision. Two findings remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A second-phase fault leaves the refinement item's settlement unstated. FR-143 now says `check_refinement` returns a refusal, "the safety half already decided is not returned with it", per the owner ruling. FR-144 settles an item from a `RefinementOutcome`, and it names `failed` only for a replay fault and a `StepVerdict::Fault`. It says nothing about a `check_refinement` refusal. So for a refinement whose safety half was proved, the item's terminal value, and whether any FR-331 record is written, is unspecified. Nothing tests the path either: no AC or TC step covers the fault bullet. Fix: one FR-144 settlement statement for a refusal from `check_refinement` (presumably `failed` with the fault, no half values), and a note in FR-143-AC that the path is verified by inspection or by a fault-injection test. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:137-140; spec/functional/FR-144-request-and-settle-a-refinement-item.md:93-113 |
| FND-002 | low | The horizon-overflow refusal has no locus requirement. FR-186's locus rule ("retains the claim's locus and names the offending part") covers only refusals with a bracketed cause. The overflow bullet is a resource-exhausted limit, so a refusal located at the package root meets the spec. The #1116 code does exactly that (`horizon_refusal` builds `Location::root`, SR-13595 FND-010). A temporal clause's overflow is located at the operator. Fix: the bullet says the refusal is located at the interval operator whose bound makes the sum overflow, or at the claim. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:149-151; qsl-semantics/src/check/probabilistic.rs:582-593 (qsl-568-fr186-typing) |

## Verdict

Not mergeable, together with SR-13599. FND-001 needs one FR-144 statement
that this PR can add beside FR-143's fault bullet. FND-002 is a one-line
addition.
