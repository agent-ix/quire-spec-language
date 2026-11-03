---
id: SR-20261103154606
title: "QSL-1522 r1 gap analysis of PR #2202 (new ACs and TC rows of FR-497, FR-505, FR-509)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2202 head 8bf8bbec7; FR-497-AC-1..8 / TC-1287; FR-505-AC-1..9 / TC-1300; FR-509-AC-1..10 / TC-1304; code trace tags in *.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/TC-1304
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1300
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1287
    type: references
---

# QSL-1522 r1 gap analysis of PR #2202

## Summary

Tickets: QSL-1522, QSL-1526. Plan completion: not assessed. This PR is
spec-only.

Every AC is in its TC's trace tag and Scope:

- TC-1287 covers FR-497-AC-1 to AC-8.
- TC-1300 covers FR-505-AC-1 to AC-9 (rows 1 to 23).
- TC-1304 covers FR-509-AC-1 to AC-10 (rows 1 to 18).

Code tags exist only for FR-497-AC-1 to AC-7. FR-497-AC-8, all of FR-505 and
all of FR-509 have no tagged test yet, which is expected: the implementation
belongs to QSL-1522 and QSL-1526. The existing `[ 2 , 2 ]` reader test
contradicts FR-509-AC-8, as FR-509's Dependencies says.

Each new AC was checked for whether a wrong implementation could pass it:

- FR-509-AC-5 can fail. The `d = 3` step appending two entries, leaving
  `pending` `"1"` and binding only on the continuation step separates
  correct behaviour from both "refuse `d = 3`" and "block the whole send".
- FR-505-AC-2, AC-3 and AC-8 pin the code, cause, path and the one reported
  row.
- FR-497-AC-8 pins the charge point and excludes `inexact_conversion`.

## Verdict

**MINOR GAPS.** One clause cannot fail as written, and two oracles are too
loose.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-509-AC-9's clause "two sends pending with count 1 and payloads `m1` and `m2` reach states with different keys" cannot fail on the property it targets, that the `send-pending` entry carries the payload. TC-1304 row 17 runs it as capacity 1, `[2,2]`, one appended entry each. In each pending state the queue already holds the appended `m1` or `m2` entry, so the two keys differ by queue content even if `send-pending` held no payload or no count. Compare the states after a receive has emptied the queue, with the receiver's binder unread, when the payload lives only in `send-pending`. Add a pair that differs only in pending count. Row 17's two-run setup rests on a false premise: one exploration reaches both states, through a payload domain {`m1`, `m2`} at one send node or two send nodes in parallel branches (QSpec TC-378 CH-18). | spec/functional/FR-509-declare-a-channel-s-delivery-count-interval.md:140; spec/test-cases/TC-1304-declare-a-channel-s-delivery-count-interval.md:41 |
| FND-002 | low | FR-509-AC-8 and TC-1304 row 13 accept "a `ViewRefusal` naming the `delivery` member". `ViewRefusal` has both `MissingMember` and `MalformedMember`, so an implementation that reports a malformed text as `MissingMember { delivery }` passes. Name `ViewRefusal::MalformedMember`, the variant the `delivery` doc comment already uses for a text that is no policy word or interval. | spec/functional/FR-509-declare-a-channel-s-delivery-count-interval.md:139; qsl-package/src/protocol_view.rs:820-823 |
| FND-003 | low | FR-509-AC-10 states no oracle for its charge and frontier claims. It gives no work-limit value, no expected consumed count, and no check that the unexplored frontier holds the state whose expansion stopped, which the Behavior bullet and QSpec FR-426-AC-21 both state. "Charges one work unit per generated step and one per processed entry" therefore cannot fail. Give a concrete limit, for example a `[0,n]` send under limit `L`, with the expected consumed units and the frontier state. | spec/functional/FR-509-declare-a-channel-s-delivery-count-interval.md:141; spec/test-cases/TC-1304-declare-a-channel-s-delivery-count-interval.md:42 |
