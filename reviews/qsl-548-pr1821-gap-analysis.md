---
id: SR-1821002
title: "gap analysis of PR #1821 (QSL-548 slices B and C): per-step simulation settlement"
type: SpecReview
analysis: gap-analysis
date: 2026-10-29
scope: "agent-ix/quire-spec-language; PR #1821 at 748406e67 against FR-146-AC-4, 7, 8, 10 and QSpec FR-468-AC-2..4, 6, 7, 9, 10"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1821002: gap analysis of PR #1821 (QSL-548 slices B and C)

## Summary

TC-551 and TC-893 cover the following:
- **FR-146-AC-4.** Verified gives `Certified`; no certificate gives `Uncertified`; `CasRefinesCounterAdd` gives `Uncertified`; a refused `Initial` is rejected at `Base`.
- **FR-146-AC-7.** A rejected part rejects only its own record.
- **FR-146-AC-10.** Kind-only routing to a bounded-only SMT backend, and the refinement record as an SMT unrolling candidate.
- **FR-468 rule order**, including an initial counterexample through replay.
- **Other results** and **stopped or faulted checks**.
- **The `hypotheses-not-established` wire form** and its plugin read.
- **QSL-905**, which this PR now delivers:
  - the records are written into `requirements()` (slice A);
  - each per-step record routes by kind to an `operation-contract` backend, carrying the extent negotiation settles `requires-bound` from (`per_step_records_route_by_kind_to_an_operation_contract_backend`);
  - with no such backend, a record has no candidate;
  - the refinement record is an SMT unrolling candidate beside EN-1 (`the_refinement_record_is_also_an_smt_unrolling_candidate`).

Gap:
- **FND-001.** FR-146-AC-8's hypothesis cases are tested only through `settle_step_records`, with a made-up `NodeKey`. The `Uncertified` hypothesis item and `inconclusive` `hypotheses-not-established` are never tested through `settle_step_simulations` with CasRefinesCounter carrying an invariant. AC-8 names `self.value < 0`, and FR-146-AC-3 names `CasInv`. The analyze test's fixture lists no hypotheses, so the link from the records' `hypotheses` to the hypothesis items is untested. That is where SR-1821001 FND-001's hypothesis half sits.
- **FND-002.** Five settlement tests carry only `#[trace("TC-551")]` and name no AC, so the trace matrix shows FR-146-AC-8 as untested although its rules are exercised:
  - `the_safety_half_settles_by_the_first_rule_that_applies` covers AC-8;
  - `other_backend_results_settle_the_record_and_the_half` covers QSpec FR-468-AC-7, under AC-8;
  - `step_records_are_certified_only_by_a_verified_certificate` covers AC-4;
  - `a_stopped_or_faulted_check_settles_incomplete_or_failed` covers AC-4 and AC-7;
  - `a_verified_certificate_settles_every_record_certified` (qsl-analyze) covers AC-4 and AC-8.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-146-AC-8's hypothesis cases are never run end to end: there is no `settle_step_simulations` test over CasRefinesCounter with an invariant (`self.value < 0` or `CasInv`) whose hypothesis item is unproved or `Uncertified` | FR-146-AC-8, FR-146-AC-3 |
| FND-002 | low | Five settlement tests name no FR-146 AC in `#[trace]`, so FR-146-AC-8 shows untested in the matrix. Tag each with the AC it exercises (AC-8 on the safety-half and other-result tests, AC-4 on the certification tests) | FR-146-AC-4, FR-146-AC-8 |

## Verdict

Mergeable as far as coverage goes. Add the FND-001 test with the SR-1821001 FND-001 fix, and the FND-002 tags with it.
