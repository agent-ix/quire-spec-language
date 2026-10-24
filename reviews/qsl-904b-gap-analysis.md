---
id: SR-12260
title: "Gap analysis of quire-spec-language PR #1146: temporal subject proof bounds against QSpec FR-331/FR-368 and SR-12200"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d70693be10d7e284808eea618c782d717b586c48; PR #1146: RequirementItem::{with_subject_proof_bounds, temporal_proof_bounds}, BoundRefusal::{NotTemporal, DuplicateSubjectDomain}, tests/it/requirement_item_proof_bounds.rs; against QSL FR-097 (AC-4, AC-9), TC-438, ADR-014 B-4, QSpec FR-331 and FR-368, and SR-12200 FND-001"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1146

## Summary

Ticket: QSL-904. This review is planless, so plan completion is not
assessed. The trace tags were read from the test source; no matrix status is
claimed. QSpec was read at quire-specification main 8998f318 and is cited by
id.

The ticket asks for typed finite item bounds and temporal `proof_bounds` on
the occurrence-scoped `RequirementItem`, read by the driver without
synthesis, and no compatibility layer. PR #1043 delivered the extent bounds.
This PR delivers the subject-only substitution that SR-12200 FND-001 found
missing.

- FR-331 (STD-213, QSpec #309) says a subject's `proof_bounds` are
  independent of `extent`. A subject bound may be supplied while `extent`
  stays `unbounded`, and it moves nothing into `extent.bounds`. The new test
  `subject_proof_bounds_are_independent_of_the_extent` supplies two
  population bounds on an `infinite-trace` item. It asserts the key order,
  and that occurrence, index, extent, `unbounded` and `proof_bounds` are
  unchanged. That is a real oracle: had the bounds been mirrored or merged
  into the extent, the test would fail.
- FR-331 says the checked requirement carries no proof bounds and the caller
  supplies them (ADR-014 B-4). `items_from_requirements` writes an empty
  `subject_proof_bounds`, which matches.
- No compatibility layer was added. The old mirroring was removed, not kept
  behind a flag.

SR-12200 FND-001 had two parts. (a) No subject-only substitution: this PR
closes it. (b) QSL does not decide which temporal records are over a model
subject, so a temporal item over a supplied trace still returns `Some(..)`:
this PR does not close it. FND-001 below carries (b) forward. SR-12200
FND-001 should stay open until (b) is fixed.

## Verdict

PASS with no major finding. Two medium gaps should be fixed in this PR.
First, `temporal.subject` still applies to every `temporal-satisfaction`
item, not only those over a model subject. QSL already classifies this, and
the new test exercises the wrong case. Second, the new public API and its
refusals have no owning QSL requirement, and the test is tagged with criteria
that do not state them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Part (b) of SR-12200 FND-001 is still open. FR-331 gives `temporal` (with `subject`) to each "`temporal-satisfaction` request item over a model subject", and FR-368 does the same. `temporal_proof_bounds()` still returns `Some(..)` for every temporal item, and `with_subject_proof_bounds` accepts subject bounds for a temporal item over a supplied trace. QSL already decides this when it checks the clause: `model_subject` is set from the binding's type (`ValueType::Reference`) at `qsl-semantics/src/check/temporal_clause.rs:916`, and `property_form()` is `None` for a clause that is not over a model subject. But that decision never reaches `RequirementItem`, so the driver would have to work it out itself. The new test supplies subject bounds on `over (c: Counter)`, where `Counter` is a record, not a model reference. So the API is shown working on an item that FR-331 says has no `subject`. The PR also deleted the interim doc caveat that recorded this gap. The fix is to carry the model-subject classification onto `RequirementItem`, then return `None` from `temporal_proof_bounds()` and refuse `with_subject_proof_bounds` for a temporal item that is not over a model subject. Then move the test to a `model`-subject clause and add one refusal case over a record. Close SR-12200 FND-001 only when that lands. | qsl-route/src/request.rs:420-460; tests/it/requirement_item_proof_bounds.rs:175-200; qsl-semantics/src/check/temporal_clause.rs:916 |
| FND-002 | medium | The new public method and its two refusals have no owning QSL requirement. `subject_proof_bounds_are_independent_of_the_extent` is tagged `#[trace("TC-438", "FR-097-AC-9")]`. FR-097-AC-9 is about each proof bound carrying its domain kind, and TC-438 is about the request writer's availability and bad bounds. Neither states the subject-only substitution, its independence from the extent, or the `NotTemporal` and `DuplicateSubjectDomain` refusals. The test also never checks AC-9's "never none" clause on this path, and the code breaks it (SR-12259 FND-001). Only ADR-014's B-4 row mentions `temporal.subject.proof_bounds`. The fix is to add an acceptance criterion, in FR-097 or the FR that owns the temporal request item, for subject proof bounds. It should say they are caller-supplied, carry a QSL-written population kind, are independent of the extent, apply only over a model subject, and list their refusals. Add a TC-438 step (or a TC of its own) and retag the test to that criterion. | tests/it/requirement_item_proof_bounds.rs:170-176; spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:85 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 951a2d989 | `Requirements` now carries `model_subject`. The checked temporal clause sets it when its `over` binding is a model reference, and hyper, relation and state-graph claims always set it. `RequirementItem` copies it. `temporal_proof_bounds()` is `Some` only for a temporal item over a model subject, and `with_subject_proof_bounds` refuses any other item with `NoModelSubject`. The test now uses a model-reference clause (`fairness_item_path::graph_of`), and the record-subject item is a refusal case. This closes part (b) of SR-12200 FND-001, so that finding can be closed. Verified by the reviewer in round 1 at 951a2d989. |
| FND-002 | fixed 951a2d989 | FR-097-AC-10 states the API, that QSL writes the population kind, that the bounds are independent of the extent, that they apply only over a model subject, and the refusals. TC-438 step 5 covers it, and the test is retagged `#[trace("TC-438", "FR-097-AC-10")]`. Verified by the reviewer in round 1 at 951a2d989. |
