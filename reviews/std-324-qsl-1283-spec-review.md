---
id: SR-16193
title: "Spec review of quire-spec-language PR #1283: STD-324 and STD-322, FR-220 References and FR-267 reader refusals mirroring QSpec FR-435 and FR-352"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0eea6c62; PR #1283 (git diff origin/main...0eea6c62): spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md, spec/functional/FR-267-write-run-results-as-native-run-result-2.md, spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md; read for context: QSpec PR #382 head 7df84d72 (FR-435, FR-439), QSpec PR #383 head f30d03b1 (FR-352, TC-255), QSL FR-109 at main 84c61f0ab"
review_set: subset
evaluated_revision: "0eea6c62"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1160
    type: reviews
---
# SR-16193: Spec review of quire-spec-language PR #1283: STD-324 and STD-322, FR-220 References and FR-267 reader refusals mirroring QSpec FR-435 and FR-352

## Summary

Tickets: STD-324 (QSpec #382 SR-16183 FND-002) and STD-322 (QSpec #383 SR-16184 FND-003). Reviewed `git diff origin/main...HEAD` at 0eea6c62. The FR-220 References sentence now cites QSpec FR-435-AC-2, FR-435-AC-9 and FR-439-AC-11, which match QSpec #382's text at 7df84d72; the wording is behaviour-only and the old ticket reference is gone. The FR-267 reader bullets match QSpec FR-352 as amended at f30d03b1 point for point: `temporal` refused on a selection that is not `Temporal` and beside a top-level `refusal`; `insufficient-data` read only in its own row and `max_window_values` only in `window-limit-reached`, each refused in an `undefined` or `incomplete` row and on a non-temporal selection. FR-267-AC-16 has one adverse edit per new refusal, each of which can fail, and TC-1160 step 6 runs it. FR-267-AC-16 is absent from main and from every other open QSL PR; no other open QSL PR touches FR-220, FR-267 or TC-1160. The lead defaults are flagged in the PR body, and the merge order (with or after QSpec #382 and #383) is stated. `QSPEC_DIR=<QSpec main 6f24ddd4> make check-spec-validation` exits 0 with no diagnostic on a touched file. The head is one commit behind QSL main 84c61f0ab and merges with it cleanly, and with #1284.

Out of this diff, for the lead: QSL FR-267-AC-13 writes the observed counterexample's `trace_position` as the string `"0"`, and FR-267-AC-14 refuses it set to `"1"`, but QSpec's backend-provider schema types `TemporalCounterexample.trace_position` as a `Count` integer, and QSpec #383's TC-255 NR-14 now uses the integer 1. The two repos disagree on that member's encoding, which QSL-902's TC-255 conformance will meet. Not a finding of this PR (pre-existing on both mains).

Units examined:

- `FR-220 References` (examined): spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md
- `FR-267 Temporal disposition wire reader bullets` (examined): spec/functional/FR-267-write-run-results-as-native-run-result-2.md
- `FR-267-AC-16` (examined): spec/functional/FR-267-write-run-results-as-native-run-result-2.md
- `TC-1160 step 6` (examined): spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md
- `QSpec FR-352 Temporal disposition` (context_only): agent-ix/quire-specification PR #383
- `FR-109-AC-3` (context_only): spec/functional/FR-109-run-a-state-clause-through-the-spine.md

## Verdict

**MERGEABLE with or after QSpec #382 and #383**, with one low finding to fix or accept.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-267-AC-16 locates two of its documents loosely. 'FR-109-AC-3's state clause run at stage evaluate' picks the exhausted-work report out of AC-3's three only by its stage, and 'a state clause run's undefined outcome' names no run at all (FR-109-AC-3 has no undefined report). Name the runs, for example FR-109-AC-3's exhausted-work report for the `incomplete` edit and FR-109-AC-7's `precondition-false` report for the `undefined` edit, as QSpec TC-255 NR-13 names NR-01. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:406 |
