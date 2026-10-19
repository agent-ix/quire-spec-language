---
id: SR-13324
title: "Spec review of PR #1050: integrity analysis"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@c5407fabd; git diff origin/main...HEAD (PR #1050)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-541
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-551
    type: reviews
---

## Summary

Ticket: STD-212. Consistency and completeness across the changed artifacts at c5407fabd. TC-541 scope AC-1..AC-5 and TC-551 scope AC-1..AC-8 match the AC tables; every new AC has a procedure step and an expected result.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136 Order of the checked rows bullet | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-5 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| TC-541 step 5 | spec/test-cases/TC-541-s3-checks-a-refinement-s-step-rows.md | examined |
| FR-146 Inputs invariants bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Outputs StepObligation comment | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 hypotheses bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Row order bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Occurrence key bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Obligation identity bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 settlement bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 rejected-certificate bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Dependencies ADR list | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-1 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-5 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-6 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-7 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-8 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| TC-551 steps 5-8 | spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md | examined |
| TC-551 expected steps 5-8 | spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-136-AC-5's tie clause ('two rows whose first named declarations have equal digests are ordered by member name bytes') cannot be shown with `CasRefinesCounter`, and 'member name' of a step row is undefined. Each `CasRefinesCounter` row's first named declaration is its own concrete operation, a distinct declaration node, so no two rows have equal digests, and TC-541 step 5 names no other fixture. #1040 breaks ties on the operation's node name, which is never reached. Give a fixture where a tie occurs, for example a protocol subject's `duplicate ch` and `lose ch` rows, which share the channel, and say what a step row's member name is. Otherwise drop the clause. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:150 |
| FND-002 | low | FR-146-AC-1 still says each record lists 'the concrete package's invariant clauses (none)', the wording the PR replaces in the behaviour, the Outputs comment and the Inputs. Say 'the invariant clauses whose context is a concrete-model object type (none)'. The id is unchanged. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:175 |
| FND-003 | low | The Dependencies prose still names only ADR-013 O-09 and ADR-018 V-3/V-6/V-8, although the new text relies on O-07, ADR-018 PC-2/PC-6, ADR-020 RM-1 and QSpec FR-375. Add O-07, PC-2, PC-6, RM-1 and QSpec FR-375 to the list. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:186-188 |

## Verdict

Findings: 1 medium, 2 low.

## Dispositions

Round 1, head 7cfb5530b (fix commit 7cfb5530b over merge 2b96e20e4).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
