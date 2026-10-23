---
id: SR-16266
title: "Round-3 spec-review of the hybrid reader and encoding fix-forward (quire-spec-language PR 1364)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4169782619fe8a516aa9ef2ee40b2c0ced49603c (fix commit 33501726 against SR-16258; origin/main...HEAD; cross-checked against every open PR head, including #1365); spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md; spec/test-cases/TC-1187-a-hybrid-declaration-checks-to-a-classified-model-and-routes-its-claim.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1187
    type: references
---

# Round-3 spec-review of the hybrid reader and encoding fix-forward (quire-spec-language PR 1364)

## Summary

Ticket: STD-385. This round re-reviews #1364 at head 41697826: fix commit
33501726 against SR-16258, plus a merge of main. Main is an ancestor of the
head. The companion QSpec round is SR-16265 (#414 at e55b847b).

Gates run by the reviewer with `QSPEC_DIR` at QSpec e55b847b:

- `make check-spec-validation`: exit 0.
- `cargo test -p qsl-package --lib checked_v2`: 41 passed, 6 failed. Every
  failure is `UnsupportedNodeTag` at `/semantic_graph/nodes/39/node_tag`,
  the missing reader code only.

Disposition of SR-16258: FND-001 is fixed.

- `HybridInterval` holds exact rational bounds with open and closed ends,
  and the horizon is `Option<ExactRational>`.
- S4 emits the form FR-370 admits under the claim's profile.
- S3 refuses an interval the profile does not admit, at a span that names
  the profile.
- FR-447-AC-15 and TC-1187 step 8 pin both admissions and both refusals.

Hand derivation of AC-15:

- Under `timed/v1`, `[0, 10]` becomes `{lower: 0/1, upper: 10/1, closed,
  closed}` with horizon 10. FR-370 admits it, and so does QSpec HY-21.
- `[0, *]` under `timed/v1` has no finite upper bound, so FR-370 does not
  admit it and the S3 refusal is right.
- `always` with no interval under event-position needs an integer `[a,b]`,
  so the S3 refusal at the operator's span is right and matches FR-326.

Ids: FR-447-AC-13 to AC-15 are clear of #1365, which holds AC-6 to AC-12,
and of every other open PR head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | [ambiguous] Two rules overlap with open #1365 (STD-409..415) and need one reconciliation when the second PR rebases. (a) The horizon type: #1364 has `Option<ExactRational>`, and #1365 has `Option<HybridHorizon>` with `Seconds(ExactRational)` and `Positions(u64)`. (b) The source interval: #1364 says "an `interval`'s `uint` bounds SHALL count seconds, each end closed". #1365 parses a `timed-interval` with rational bounds and open ends in place of `interval`. Under #1364 alone, its own refusal of "an open end or a bound that is not a whole number of seconds" under the non-timed profiles cannot be reached, because no source form produces one. Neither PR is wrong alone. Fix: merge #1364 first and have #1365 replace the horizon type and the "Intervals" bullet in its rebase, so that the open-end and non-whole refusal becomes reachable. No change to #1364 is needed if that order is kept. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:146; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:157; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:278-289 | wrong-requirement |

## Verdict

Mergeable on its own text. There is one low finding, which concerns merge
order with #1365 only. #1364 is spec-only, so merging it turns no
gate red: QSL conformance is red only through the QSpec checkout. Its
AC-15 and AC-13, however, cite FR-481 behaviour that exists only at #414's
head, the `timed/v1` admission and the `Tank` fixture. So merge it in the same window as
QSpec #414, which SR-16265 blocks, and the QSL reader-code PR.
