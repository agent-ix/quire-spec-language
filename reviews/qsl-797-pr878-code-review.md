---
id: SR-11409
title: "Code review (with rust-review lane) of quire-spec-language PR #878: FrameChange::Link wire and replay decode"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b35563aa0d7a42adf308f5eaf3156d3829ed19b0; qsl-semantics/src/model/observation.rs, qsl-replay/src/spine/native_result.rs, qsl-replay/src/witness/frame.rs, qsl-replay/src/lib.rs, qsl-replay/src/spine/clause/tests/frame.rs"
review_set: subset
---
# Code review of quire-spec-language PR #878

## Summary

Ticket: QSL-797 (implementation slice a). One pass of code-review with the
rust-review lane over `git diff origin/main...HEAD`. No builds were run (brief).
Only defects that give a wrong result, a panic, a failed AC, a vacuous or
weakened test, a broken gate or an unupdated exhaustive match block. Mediums and
lows are recorded in the Verdict.

Examined: `FrameChange::Link` and `LinkDifference` (qsl-semantics);
`FrameChangeRead::Link`, `LinkDifferenceRead`, `ObjectRefRead`, the
`FrameWitnessRead::typed` population check, `FrameChangeWire::Link`,
`ObjectRefWire` (qsl-replay native_result); `ClaimedChange::Link` and its
`of_witness` arm; the three new tests and the extended decode unit test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for code-review and rust-review. The trace-tag finding on the routing
test is recorded once, in the gap-analysis artifact (SR-11410).

Checked and correct:
- Wire shape matches FR-267-AC-12 and the FR-267 `link` row: tag `link` from the
  enum's `rename_all = "snake_case"`, members `relationship`, `difference`
  (`added`/`removed`), `source` and `target` as closed `{population, key}`
  objects, `modifies` as ordered declaration keys. The round-trip test compares
  the written `change` with a JSON literal built from the spec, so a wrong tag,
  a swapped difference spelling, a swapped end or a dropped member fails it.
- The reader refuses an unknown or null `difference` (unit enum), a missing or
  unknown change member (`deny_unknown_fields` on the internally tagged enum), a
  missing end member and an extra end member (`deny_unknown_fields` on
  `ObjectRefRead`), and a bare-key end. Each TC-1037 row 14 and QSpec TC-255
  NR-11 edit is in `native_result_two_refuses_broken_link_changes`, and each
  starts from a valid document, so each refusal comes from its edit.
- The added population refusal is consistent with the spec. FR-267 Frame
  witness wire says "For a `link` change the top-level `population` is the
  population of `source`", and FR-115 Outputs says the same. A witness made by
  FR-115 always meets it, so it cannot refuse a legitimate witness. It gives
  `NativeRunReadError::Invalid` located at
  `disposition.refusal.frame_witness.population`, and it is tested. The string
  compare sits in the existing `#[string_edge]` `typed`, so the string-edge
  gate holds.
- The writer's `"added"`/`"removed"` literals come from an enum match, not a
  string match, so no string-edge marker is needed.
- Exhaustive matches: the only `FrameChange` matches are `of_witness`, the
  reader `typed` and the writer `From`, and all three have a `Link` arm. Other
  uses are `let ... else` or have a wildcard arm (frame_replay.rs:423). There
  is no match on `ClaimedChange`.
- Routing: `ClauseDisposition::{stage, category, truth}` match on
  `FrameViolation(_)` without looking at the change, so a Link witness is
  evaluate, violation, `truth: false`, exit 10 with no code change.
- Feature lanes: `spine/clause/tests/frame.rs` is not behind `quire-extraction`,
  so the tests run in the default-workspace lane and in the all-features lane.

Recorded (not blocking):
- low: the population check is not tested against a mutant that compares the
  target end. The fixture puts both ends in `Q`, so `target.population` would
  pass every test. The spec vectors are single-population too.
- low: the round trip uses `p1`-`t1` `added` and a non-empty `modifies`. Row 13
  says row 2's report (`modifies []`) and row 4's (`p3`-`t1` `added`). The wire
  coverage is the same or stronger.
