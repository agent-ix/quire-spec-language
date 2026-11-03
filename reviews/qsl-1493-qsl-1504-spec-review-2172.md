---
id: SR-2172001
title: "spec-review of batch 6 coder gaps FR-527, FR-501, FR-330, FR-312, FR-285, FR-502, FR-520, FR-045 (quire-spec-language PR 2172)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-045, FR-285, FR-312, FR-330, FR-501, FR-502, FR-520, FR-527 and spec/test-cases/TC-769, TC-1160, TC-1295, TC-1296, TC-1315, TC-1322 (14 files)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-527
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: references
---

# Spec review of batch 6 coder gaps (quire-spec-language PR 2172)

## Summary

Tickets: QSL-1493 (FR-501, FR-504, FR-285, FR-330, FR-312), QSL-1504 (FR-527, FR-525), QSL-1548
(FR-520); QSL-1494 (FR-502) had no findings. One spec-review pass over `git diff origin/main...HEAD`
at head bfc5f0ce3 (EARS, integrity and cross-reference checks folded in; no Jev client).

Verified TRUE of the code: `is_cumulative` of quire-exact's ten-counter `LimitKind` (the revision
the lock pins) is exactly `WorkUnits | ResultUnits`, so FR-520-AC-7's sum for those two and maximum
for the other eight is right (the `is_cumulative` in qsl-semantics `accounting.rs` is a different
five-counter model enum, WorkUnits only); `Category` in quire-semantic-value has eight values and no
Indeterminate or Pending (FR-285 interim text); the QSL #2170 `mapping_support` returns
`NoProfileSelection` from `TemporalProfile::from_identity`, classifies timed as `Unmatched::TimedProfile`
and adds `MixedDirection` for past plus future, keeps the retained subject as declaration, profile
row and activation, and spells the surrounding closure `Complete`/`Open`; the Contract IR reader
refuses a temporal-profile law the lock's `profile_selections` did not select (FR-527-AC-21); five
temporal profiles are admitted at check (FR-326, FR-508). IR-1278, IR-1280 and IR-1281 exist in
Backlog. The merged FR-501 canonical order (stamp, authority, key, observation identity) is total,
so "the lowest canonical position of the stamp" is well defined and independent of listed order;
FR-501-AC-4 (stamps 0, 2, 2, 5; `eventually`, not order-sensitive) and AC-23 do not contradict
AC-7, AC-20, AC-21. Spec ahead of code: #2169's `conflicting_timestamped_valuation` reports the
position of a true reading that precedes a false one at a stamp, not the lowest position of the
stamp, and orders by authority and key only; QSL-1493 must conform to FR-501. `quire validate`
over the 14 files exits 0 with only the pre-existing archetype warnings. No other open PR touches
these files. Renumbering checked: FR-527 AC-1..24, FR-502 AC-1..11 and TC-1296 rows 1..15, FR-520
AC-1..13 and TC-1315 rows 1..14, FR-501 AC-1..23 and TC-1295, FR-285 AC-1..11 and TC-769 steps
9..10, FR-312-AC-8 and TC-1160 scope.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-501 now says the cover comes from the request's trace input, but the Counter rule (line 283) and FR-501-AC-9 and AC-13 still say "the clock binding declares" / "With the clock binding `Counter { uncertainty: n }`"; the three statements contradict the restated Inputs bullet and FR-330. | spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md:283 | wrong-requirement |
| FND-002 | medium | FR-525 was not updated: row 109 still reads "a profile outside the four admitted is refused at check" and maps `Unmatched::TimedProfile` to FR-527-AC-18 (now five profiles; the timed classification is AC-22 and AC-23); rows 79 and 122 map FR-502 as "AC-1 to AC-8" (now AC-11); no row carries FR-527-AC-21 to AC-24. | spec/functional/FR-525-map-each-legacy-temporal-evaluator-criterion-to-its-spine-successor.md:109 | wrong-requirement |
| FND-003 | medium | FR-504 (event-position and fixed-sample traces) still treats "conflicting valuations as FR-501 Verdicts states"; FR-501 now defines `ConflictingValuation` only between observations that share a stamp, and those traces carry no stamps, so FR-504 has no defined conflict. | spec/functional/FR-504-carry-progress-on-event-position-and-fixed-sample-traces.md:98 | wrong-requirement |
| FND-004 | low | FR-285-AC-10 asserts two claims: that the report and the ranking give equal pairs, and that each of five outcomes maps to a named label and exit; either can fail alone. | spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:152 | wrong-requirement |
| FND-005 | low | FR-520's table lists AC-6, AC-12, AC-13, AC-7 in that order; the new ACs sit between AC-6 and AC-7. | spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md:103 | wrong-requirement |
| FND-006 | low | FR-330 adds `execution` and `cover` to the Timestamped input and says the entry takes the cover from it; FR-501-AC-22 tests only an omitted cover, and no criterion asserts that a supplied `Counter` cover is the one used or that `execution` is carried. | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:58 | missing-requirement |

## Verdict

Not mergeable until FND-001 to FND-003 are fixed: each is a leftover statement that now contradicts
the PR's own restatement. FND-004 to FND-006 are low. Everything else checked against the code and
the merged specs holds (see Summary).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-007 | low | The rewritten FR-504 "Missing valuations and history" bullet reads "SHALL treat a missing valuation as FR-501 "Verdicts" states it with a possibility set", which is garbled, and one sentence then runs on into "and its `Indeterminate` carries the `instance` and `also_held_by` members"; the no-conflict rationale should be its own statement. | spec/functional/FR-504-carry-progress-on-event-position-and-fixed-sample-traces.md:97 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-501 line 283 reads "When the trace input's cover is `Counter { uncertainty: u }`"; AC-9 and AC-13 read "With the trace input's cover `Counter { ... }`". No "clock binding declares" cover wording remains in FR-501. |
| FND-002 | fixed | FR-525: FR-043-AC-12 and FR-052-AC-13 map FR-502-AC-1 to AC-11; FR-052-AC-14 maps FR-501-AC-1 to AC-25; TimedProfile maps to FR-527-AC-22, AC-23; a new row maps a profile outside the admitted set to AC-18 and AC-16; FR-045-AC-6 adds AC-21; a new FR-045-AC-3 row maps to AC-24 gated on IR-1278. |
| FND-003 | fixed | FR-504 now says a position of these traces is one document at its own coordinate, no two share one, so there is no conflicting-valuation case (position i has coordinate i, FR-504 line 57). Wording is poor; see FND-007. |
| FND-004 | fixed | FR-285-AC-10 asserts only report equals ranking; the five label and exit pairs are FR-285-AC-12, and TC-769 scope reads AC-1 to AC-12. |
| FND-005 | fixed | FR-520 table order is AC-1 to AC-13 in sequence. |
| FND-006 | fixed | New FR-501-AC-24 (a supplied `Counter` cover is used; `Exact` gives a different verdict and result identity) and AC-25 (`execution` carried, same verdict), with TC-1295 rows 38 and 39 and scope AC-1 to AC-25. |
