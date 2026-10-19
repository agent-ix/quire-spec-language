---
id: SR-13265
title: "EARS review of QSL PR #1039 (STD-157)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@a6f4a796f; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: reviews
---

## Summary

Ticket: STD-157. The PR adds one FR-381 behaviour bullet and leaves the FR statement alone. `quire validate` on FR-381 and TC-1058 reports no EARS diagnostic. Its first, third and fourth sentences are well-formed SHALL / SHALL NOT clauses. One sentence states the conditional typing rule in descriptive form.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-381 expected-type bullet, sentence 1 ("S3 SHALL type an operand ...") | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md | examined |
| FR-381 expected-type bullet, sentence 2 ("When one operand has type ...") | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md | examined |
| FR-381 expected-type bullet, sentence 3 ("S3 SHALL refuse an unproved obligation ...") | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md | examined |
| FR-381 expected-type bullet, sentence 4 ("The rule adds no conversion ...") | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The trigger sentence "When one operand has type `Int[lo,hi]` and the other ... is a binary `+`, `-` or `*` ..., the `Int[lo,hi]` operand supplies the arithmetic node's expected type: the node is typed `Int[lo,hi]` ..." has EARS event shape but no SHALL, so the conditional typing it states binds only through sentence 1's reference to QSpec FR-149. EARS form: "When ..., S3 SHALL type the arithmetic node `Int[lo,hi]` with the obligation that ...". | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:224-230 |

## Verdict

Mergeable on this lens; FND-001 is wording. Sentence 3's refusal (code, cause, locus at the arithmetic node, never the equality) and its SHALL NOT clamp/wrap/widen are each one determinate, testable response.
