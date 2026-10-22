---
id: SR-15970
title: "Spec review of quire-spec-language PR #1248: STD-277, code and locus for the FR-194 S3 window refusals"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@48d91126b1857f2bf4a03bf8ec136fef3239f2c7; PR #1248 (git diff origin/main...48d91126b): spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, spec/test-cases/TC-629-window-aggregates-in-monitors.md; read for context: QSpec FR-414 (AC-4, AC-6), native-diagnostics.md (invalid_package, ill_typed rows); QSL FR-218 (join count invalid-value precedent), FR-258-AC-4; agent-ix/quire-specification main 5ee0071f"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-629
    type: reviews
---
# SR-15970: Spec review of quire-spec-language PR #1248: STD-277, code and locus for the FR-194 S3 window refusals

## Summary

Ticket: STD-277. Sub-analyses folded in: integrity, EARS, criterion strength (by hand: no Jev client), failure-domain. The head 48d91126b is behind main 423ac6aa0 and merges cleanly. `make check-spec-validation` on the merge exits 0 with no diagnostic on FR-194 or TC-629. FR-194-AC-7 is new, and no other open PR touches FR-194 or TC-629. The codes are catalogued pairs: `invalid_package`/`invalid-value` and `ill_typed`/`unit-mismatch` are both in native-diagnostics, and FR-218 already uses `invalid_package`/`invalid-value` for an out-of-range source count at S3 (the join `quorum` count). The model-subject and unit-mismatch codes match QSpec FR-414-AC-6 and AC-4. Every AC-7 vector agrees with the S3 bullets: `past[0, 9]` under timestamped-event refuses at `0`, the first bound of the wrong kind; `past[0, 300 s]` under event-position refuses at `300 s`; `quantile 0`, `1` and `3/2` are outside `0 < q < 1`. Each vector can fail, for example on a refusal at the atom instead of the literal, or on a report of the first fault only. The lead defaults are sound: the whole window as the `a > b` locus (neither bound alone is wrong), and every fault reported in source order. That rule is consistent with FR-361 item 11's every-construct reporting. The QSpec follow-up is stated in the PR body: FR-414 has no AC for the window-kind, `a > b`, `quantile` and `min_count` codes.

Units examined:

- `FR-194-AC-7` (examined): spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md
- `FR-194` (examined): spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md
- `TC-629` (examined): spec/test-cases/TC-629-window-aggregates-in-monitors.md
- `FR-414-AC-6` (context_only): agent-ix/quire-specification:spec/functional/temporal/FR-414-evaluate-window-aggregates.md
- `FR-218-AC-23` (context_only): spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md

## Verdict

**MERGEABLE after a sync with main, with one minor finding to fix or accept.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | [minor] The several-faults rule leaves three interactions open. (1) The threshold belongs to the atom, not the aggregate term, so 'an aggregate term with several of these faults' does not clearly cover a wrong-dimension threshold next to a `quantile 1`. (2) A window with one bound of the wrong kind, such as `past[5, 2 s]` under event-position, has no defined `a > b` comparison; say whether the `a > b` check runs once a kind refusal is reported. (3) For a term over a model subject, which has no window profile, say whether the window-kind check still runs next to the `expression-form` refusal. Say 'an aggregate atom' and state the two skips, or add one vector each. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:71-86 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | [minor] The second vector, `quantile 1 of e.latency over past[0, 9] <= 5 m`, names no profile. 'Under event-position' opens the previous sentence only. Under timestamped-event, `past[0, 9]` adds a `unit-mismatch` at `0`, and TC-629 step 8 checks 'exactly the refusals', so the profile decides pass or fail. Write 'Under event-position' in that sentence too. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:165 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 2731cb83c: the rule now covers 'an aggregate atom (the aggregate term with its comparison and threshold)'; a wrong-kind bound suppresses `a > b` (`past[5, 2 s]` gives one `unit-mismatch` at `2 s`); a model-subject term skips the window-kind and `a > b` checks. FR-194-AC-8 has one vector per case, checked by hand against the bullets. |

Disposition pass 1 verdict (head 2731cb83c): **MERGEABLE, with one low finding to fix or accept.**
