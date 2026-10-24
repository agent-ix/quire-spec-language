---
id: SR-12200
title: "Gap analysis of quire-spec-language PR #1043: RequirementItem proof bounds against FR-097, FR-075, ADR-014 and QSpec FR-331/FR-368"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b42753456ff98a90765e0b16e6116a6ac7ddf333; PR #1043: RequirementItem::{proof_bounds, temporal_proof_bounds, bounded}, the shared substitute helper, and the two tests in tests/it/requirement_item_proof_bounds.rs; against QSL FR-097 (AC-3, AC-4, AC-9), FR-075-AC-8, FR-335-AC-3, FR-336, TC-438, ADR-014 §4 and B-4, and QSpec FR-331 and FR-368"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1043

## Summary

Ticket: QSL-904. This review is planless, so plan completion is not
assessed. It was read-only and ran no tests. `quoin matrix` (quoin 0.28.3)
did not finish within the review window. The trace tags were therefore read
from the test source directly, and no matrix status is claimed. QSpec was read at its current
main and is cited by id. The consumer need comes from the ticket (data): the
driver wants the FR-331 `extent.bounds` and `temporal.subject.proof_bounds`
as typed `ProofBound`s on the occurrence-scoped `RequirementItem`.

- ADR-014 B-4 does say the caller supplies a proof bound, and FR-336 says
  the record is computed from checked types alone. So the ticket's wish to
  "source the accessors from the checked requirement record" cannot be met
  literally, because the record holds no bound. `RequirementItem::bounded`
  is the QSL-owned substitution step that makes the accessors non-empty.
  This traces to ADR-014 §4 "Bounded request" and FR-097's bounded-request
  outputs. It is justified.
- FR-097-AC-9 (each emitted proof bound carries its domain kind, paired
  with its variant) is implemented and covered for `collection` and
  `integer` by the new tests, and for all kinds by the existing TC-438
  writer tests. The binding is correct.
- FR-097-AC-4 (refusals and their order) is unchanged for `RequestWriter`,
  because `substitute` is a faithful extraction (SR-12199). The same rule now
  backs `RequirementItem::bounded`, but no test exercises it through that
  method (FND-002).
- FR-097-AC-3, FR-075-AC-8 and FR-335-AC-3 (the follow-up has its own
  request index) are contradicted by `RequirementItem::bounded` (SR-12199
  FND-001), and the FR-097-AC-3 tag on the new tests is wrong (SR-12199
  FND-002).
- QSpec FR-331 / FR-368 `temporal.subject.proof_bounds`: only the case where
  the subject's bounds equal the extent's substitution is served. The PR
  body declares the rest honestly (FND-001).
- Production code without an owning requirement: none. The three methods
  trace to FR-097 / ADR-014 §4 and QSpec FR-331.

## Verdict

FAIL on SR-12199 FND-001, the follow-up index. Nothing else blocks. This
review adds two medium gaps: the temporal subject bound has no QSL source
outside the extent substitution, and the new method's refusal and
multi-record paths have no tests. Neither is major. The first needs a
follow-up ticket so that QSL-904 does not close as if it fully served
`temporal.subject.proof_bounds`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `temporal_proof_bounds` returns the extent substitution for every `temporal-satisfaction` item. QSpec FR-368 puts `subject.proof_bounds` only on an item over a model subject, and QSpec FR-331's vectors show a temporal item whose extent stays `unbounded` (an infinite-trace formula) while `subject.proof_bounds` holds a population bound. No QSL FR defines that subject-only substitution, so `bounded` refuses such an item (`UnboundableDomain`) and the driver can never write its subject bounds. A temporal item over a supplied trace also gets `Some(..)`, although it has no FR-368 subject. The PR body declares both gaps. The fix is to file a QSL spec ticket for subject proof bounds as distinct from extent bounds (FR-336 / FR-368 alignment) and to keep QSL-904's acceptance limited to the extent-equals-subject case. | qsl-route/src/request.rs:392-402 |
| FND-002 | medium | The new public method's own paths have no tests. Nothing calls `RequirementItem::bounded` on an already-bounded item (its own `self.extent` guard, separate from `bounded_item`'s `ClaimExtent` guard) or on an infinite-trace or loop item. That last case backs the PR's claim that an infinite-trace item's `temporal_proof_bounds()` stays `Some(&[])`. There is also no test with two or more domains (key order of `proof_bounds()`) or with several records. A multi-record test would have caught SR-12199 FND-001. The fix is to add one refusal case, one multi-domain case, and one multi-record follow-up to `requirement_item_proof_bounds.rs`. | tests/it/requirement_item_proof_bounds.rs:41-97; qsl-route/src/request.rs:410-421 |
