---
id: SR-12160
title: "Gap analysis of quire-spec-language PR #996: FR-134 fairness kinds in candidate computation"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ad74f49e477767a0b97002d1c963c6e5965eb974; qsl-route/src/lib.rs, qsl-route/src/request.rs, qsl-route/tests/it/fairness_kinds.rs, spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md, spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #996

## Summary

Ticket: QSL-534. A planless gap analysis scoped to the PR diff and FR-134. For
each claimed criterion, it checks for a test that would fail if the behaviour
were wrong, checks that each `#[trace]` binding claims only what its test
verifies, and checks whether FR-134 holds on the real item path. Plan
completion: not assessed. Static review only: no test was run.

Examined:
- FR-134-AC-1 (TC-535 step 1): `a_backend_without_the_requested_kind_is_not_a_candidate`
  and `every_kind_in_the_fairness_set_must_be_advertised`.
- FR-134-AC-2 (TC-535 step 2): `an_emptied_set_names_the_missing_fairness_kind`.
- FR-134 Behavior (four bullets) and Inputs, checked against
  `candidates_for_fairness`, `descriptor_from_manifest` and
  `items_from_requirements`.
- The untagged tests `an_item_without_fairness_keeps_every_capable_backend`,
  `backend_absence_names_no_missing_fairness_kind` and
  `registrations_differing_in_fairness_kinds_conflict`.
- Context only: QSpec FR-368 "Fairness kinds" and "No candidate", QSpec
  TC-301, `CheckedTemporalClause::fairness`, `Requirements`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `an_emptied_set_names_the_missing_fairness_kind` is tagged `#[trace("TC-535", "FR-134-AC-2")]`, but its own doc says "Settling it `unsupported` is `negotiate_*`'s work". It cannot fail on AC-2's clause "the item settles `Unsupported`, `unsupported-requested-capability`", so the matrix reports AC-2 as covered when half of it has no test. The dispatch brief said AC-2 was untagged; at this head it is tagged. Fix: drop the AC-2 tag, or split FR-134-AC-2 in the spec into the registry half (QSL) and the settle half (negotiate_*, by reference). | qsl-route/tests/it/fairness_kinds.rs:109-119 |
| FND-002 | medium | FR-134 does not hold on the real item path. `items_from_requirements` still calls `registry.candidates(kind, named)` with no fairness set, because `Requirements` and `RequirementRecord` carry none, even though `CheckedTemporalClause::fairness()` holds the checked set. So a requested temporal item with a `strong` constraint still gets a weak-only backend as a candidate, which Behavior bullet 3 forbids. This is not a regression, and QSpec FR-368's `negotiate_*` fairness check at the candidate arm still settles it loudly, so it is not a silent wrong verdict. But the ticket's "negotiation filters candidates by fairness kind" holds only through the new API, which no production caller uses. QSL-534 is FR-134's owning ticket. No other QSL ticket covers carrying the fairness set to the request item. | qsl-route/src/request.rs:397-428 |
| FND-003 | medium | Behavior bullet 1, "The EN-1 provider SHALL register advertising the fairness kinds `Weak` and `Strong`", has no implementation and no test. No EN-1 registration exists in this repo, and `descriptor_from_manifest` cannot carry the kinds (SR-12159 FND-001). The AC-1 test's "EN-1" is a fixture whose `{Weak, Strong}` the test sets itself. AC-1's "EN-1 with fairness kinds `{Weak, Strong}`" would therefore still pass if the real EN-1 registered advertising no kind. The registry half of the AC-1 tag is honest. | qsl-route/tests/it/fairness_kinds.rs:52-78 |

## Verdict

PASS on the registry slice, with no major finding. The AC-1 bindings are
honest for the registry behaviour:

- **Carried unchanged.** The weak query returns both candidates, each with its
  own exact kind set. A filter that dropped or merged kinds fails the
  `fairness_kinds(&candidate)` assertions.
- **`Strong` excludes weak-only.** The strong query returns EN-1 alone, and the
  weak-only backend's kinds are absent from the set.
- **Every kind is required.** The two-kind test fails under an "any kind"
  filter, which the single-kind AC-1 scenario could not detect.
- **AC-2 registry half.** The emptied set is checked for `is_empty`, the
  capability kind and `missing_fairness_kinds == {Strong}`.
  `backend_absence_names_no_missing_fairness_kind` separates that from FR-076
  absence.

**Is leaving the manifest read out acceptable?** Today it does not cause a
silent wrong result. `candidates_for_fairness` has no production caller, so
no manifest-built descriptor is ever filtered. The fairness decision that
QSpec owns is made by `negotiate_*` from the manifest's own `fairness_kinds`
(QSpec FR-368). The trap is the next step: once FND-002 wires the item's
fairness set into `items_from_requirements`, every manifest-registered backend
would be filtered out silently. So the manifest read, or at least a documented
typed-builder path (SR-12159 FND-001), must land no later than that wiring.
FND-002 and FND-003 together mean FR-134 is delivered as a registry API, not
end to end.

## Bindings

| Test | AC | Trace |
| --- | --- | --- |
| a_backend_without_the_requested_kind_is_not_a_candidate | FR-134-AC-1 | correct (registry half) |
| every_kind_in_the_fairness_set_must_be_advertised | FR-134-AC-1 | correct |
| an_emptied_set_names_the_missing_fairness_kind | FR-134-AC-2 | wrong: overclaims the settle clause (FND-001) |
