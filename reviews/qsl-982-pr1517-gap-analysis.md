---
id: SR-17151
title: "Gap analysis of quire-spec-language PR 1517 (QSL-982 PR 2b): protocol-subject key forms"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7e8126932; PR #1517 diff against origin/main; FR-338 past memory and acceptance rules, FR-338-AC-4, FR-338-AC-5, FR-338-AC-6; qsl-eval/src/automaton/key_state.rs; qsl-eval/tests/it/automaton_key.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-338
---
# Gap analysis of quire-spec-language PR #1517

## Summary

Ticket: QSL-982. This is a planless audit against `quoin matrix` (quoin
0.29.1) at the head, the tags, the tests and the source. The PR does not
change the spec. Plan completion: not assessed.

Examined:

- **FR-338's distance-history paragraph and protocol finite-F/U acceptance
  rule** (examined). No acceptance criterion names the key forms
  themselves. The "Additional semantic vectors" (protocol bucket
  transforms `[true,true]`/`[false,false]`) are successor semantics. The PR
  adds only keys, so it does not reach them. The new tests carry
  `TC-525` only.
- **FR-338-AC-5** (examined). It is `tagged` through earlier tests. This PR
  adds admission negatives (wrong past form per subject, an out-of-range
  distance, the wrong tail, duplicate or unsorted buckets, a malformed
  transform, a non-canonical decimal), but they are not bound to it.
- **FR-338-AC-6** (context_only). It covers zero-distance protocol steps and
  the ordinary post-shift acceptance set: successor behaviour, not this
  PR.
- **FR-338-AC-4** (context_only). Unchanged by the PR.

Reverse gap: every new path (`DistanceBucket`, `PastMemory::DistanceHistory`,
`Reader::buckets`, `Reader::has_acceptance`, the subject-selected arms) is
owned by FR-338's past-memory and acceptance text. None lacks a requirement.

## Verdict

Findings, both low. The new admission negatives are not bound to
FR-338-AC-5. FR-338-AC-5, as amended by #1516, lists `each-buchi`
negatives that no code implements, yet the matrix reports it `tagged`. That
gap predates this PR, so it is recorded here to defer, not to block. No
reverse gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The protocol-subject admission negatives in `a_protocol_past_window_is_distance_history` carry only `#[trace("TC-525")]`, so the matrix binds them to no criterion. Those negatives are: `history`/`shift-history` refused on a protocol subject, `distance-history` refused on an ordinary one, a distance beyond `b`/`a`, a non-null finite tail, a null shift tail, two buckets at one distance, unsorted buckets, a short or non-Boolean transform, and `"01"`. Every earlier admission-negative test in this file is tagged FR-338-AC-5. Split the negatives into their own test tagged `FR-338-AC-5`, or tag this one. | qsl-eval/tests/it/automaton_key.rs:1903-1905; qsl-eval/tests/it/automaton_key.rs:1997-2062 |
| FND-002 | low | Since #1516, FR-338-AC-5 names these negatives: a waiting `each-buchi` state carrying its root's shift window, `buchi` under an unguarded `on each` context, `each-buchi` under an origin or `each-when` context, and a TP-4 key under a timed profile. `quoin matrix` reports AC-5 `tagged`, but the `each-buchi` form, and the `"each"` context for unguarded TP-4, exist nowhere in the code: `StateContext::clause` still gives that clause `"infinite"`, and `BuchiStates` refuses it. The tag overstates coverage until the ruled forms land. Not introduced by #1517; defer to the QSL-982 remaining-work PR. | qsl-eval/src/automaton/key.rs:173-178; qsl-eval/src/automaton/key_buchi.rs:87-96; spec/functional/FR-338-check-an-en-1-closure-certificate.md:974 |
