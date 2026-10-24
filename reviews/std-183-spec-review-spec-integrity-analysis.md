---
id: SR-13493
title: 'Spec review of PR #1088: S3 timed-facet gate (FR-230), spec-integrity-analysis'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@9cdbed126; git diff refs/remotes/origin/main...9cdbed126 (PR #1088, branch std-183-spec): spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md, spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md, spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; ticket STD-183'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-685
  type: reviews
---

## Summary

Ticket: STD-183. PR: quire-spec-language#1088, head 9cdbed126. Method: spec-review/spec-integrity-analysis. Checks: consistency of the gate with FR-230-AC-6, FR-361 items 5, 8, 10, 11 and 12, FR-393, and TC-685's other steps; completeness of AC-7 and TC-685 step 7 against the gate's four constructs; traceability edges. `tools/check-index-completeness.sh` exit 0.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-230 Timed facet gate (bullets 1-3) | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-6 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-7 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230 frontmatter relationships and Dependencies | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230 time invariants and urgency (missing `time` member refusal) | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-361 items 5, 10, 11, 12 | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md | context_only |
| FR-361 item 8 | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md | examined |
| FR-393 S2 facet bullet | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md | examined |
| TC-685 steps 1-7 and expected results | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md | examined |
| TC-1078 step 6 | spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-230-AC-6 contradicts the gate. AC-6, and the "Until it lands" bullet it restates, refuse every `Clock` field of a package-admitted object type `not-yet-implemented` with no facet condition. The gate and FR-230-AC-7 refuse the same field, `Rpc`'s `Call.x`, `declaration-form` and nothing else when the facet is absent. Bullet 2 ("no other FR-230 refusal") settles the precedence in prose, but AC-6 is left stating the opposite. AC-6 should be limited to units that select the facet, and with FR-361 item 2 unchanged no such unit reaches S3 (SR-13491 FND-001). | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:152-154,245 |
| FND-002 | medium | "Retaining the construct's family" names no family. FR-361 item 5 defines family names only for the four model constructs, and the gate gives none for `time`, `time invariant`, `urgent` or `Clock`. The gate also says it retains "as FR-361 items 10 and 11 do", but leaves out the owning declaration that item 11 retains, which matters for a `Clock` parameter or result. Two implementers would emit different refusal payloads. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:217-224 |
| FND-003 | medium | FR-230-AC-7 and TC-685 step 7 cover two of the gate's constructs. `Rpc` has no `urgent` declaration and no `Clock` parameter or result, so a gate that skips `urgent` or `Clock`-typed parameters and results passes AC-7. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:246; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:27,39 |
| FND-004 | low | FR-230's relationships and Dependencies section gain no edge to FR-361 (whose items 10 and 11 the gate adopts), FR-393 (the S2 side it relies on) or FR-110 (the header resolution that decides selection). The gate's behaviour depends on all three, yet a blast-radius query from them does not reach FR-230. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:1-27,248-259 |

## Verdict

Findings. Consistent: FR-393's "S2 never refuses for facet; S3 decides" now names the gate; FR-361 item 8 points at it; the gate's suppression of other FR-230 refusals (including the missing-`time`-member refusal for an invariant or urgency) matches FR-361 item 12's one-refusal rule; TC-685 steps 1 to 6 are unchanged and step 7 does not conflict with them. TC-1078 step 6 (S2 builds equal forms under any facet) is unaffected. FND-001 to FND-003 should be fixed before merge.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-230's new Dependencies bullet links `FR-110-header-profile-selection.md`, which does not exist; the file is `FR-110-resolve-header-profile-selections-at-e3.md`. `make check-spec-validation` does not catch it. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:291 |
| FND-006 | low | FR-230-AC-7 forbids only another FR-230 refusal, while TC-685 step 7's expected result says "with no other refusal", which also excludes a refusal from any other requirement. The two disagree on what a test asserts for `g(): Clock`'s body. | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:40 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
