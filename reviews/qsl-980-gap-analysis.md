---
id: SR-12369
title: "Gap analysis of quire-spec-language PR #1366: application-claim replay by occurrence key (QSL-980)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4fd9be8a6; git diff origin/main(e5554bc9f)...4fd9be8a6; FR-098-AC-22..26 and FR-098 'The claim a function request decides' / 'Application claim' statements, FR-071-AC-15, FR-290-AC-12, FR-290-AC-15, FR-404-AC-2..7 and AC-9, TC-444, TC-774, TC-1173, FR-062 Extent; QSpec FR-323-AC-22 / TC-278 as cited by FR-071"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-404
    type: reviews
---
# Gap analysis of quire-spec-language PR #1366 (QSL-980)

## Summary

Ticket: QSL-980. Plan completion: not assessed. `quoin matrix --json`
(quoin 0.28.3) did not finish within 600 s, so the AC-to-test mapping below
was done by hand from `#[trace]` tags and by reading each test against its
AC text. Sources: Linear QSL-980 and its two QSL-1006 comments (data),
FR-098, FR-290, FR-404, TC-1173, FR-062, and the code at the head.

| Criterion | Test | Status |
| --- | --- | --- |
| FR-098-AC-22 | `tc_444_the_occurrence_key_selects_the_predicate_or_the_application_claim` | covered; the obligation identity is arbitrary bytes, not one minted for `p`'s contract, and nothing asserts it is carried (FND-005) |
| FR-098-AC-23 | `tc_444_each_occurrence_of_one_node_is_a_claim_of_its_own` | covered; fails if matching were by node |
| FR-098-AC-24 | `tc_444_binder_bodies_are_observed_and_unread_parameters_are_filled` | covered (Input and Witness fill, binder `UnknownParameter`, extent-root `UnboundParameter`) |
| FR-098-AC-25 | `tc_444_a_key_selecting_no_claim_of_the_function_refuses` | covered; all four `ClaimOccurrence` cases, the code and cause, `NotAPredicate`, `WrongValueKind` |
| FR-098-AC-26 | `tc_444_claim_selection_comes_after_the_function_and_before_the_join` | covered for stale id, unknown function and stray assignment; the frame clause is covered by AC-21's existing test |
| FR-098 observation: recursive calls not observed | none | violated at the head (SR-12368 FND-001) |
| FR-098 unread parameters: `Boolean` to `false`, range without 0 | none | untested (FND-003) |
| FR-071-AC-15 / QSpec FR-323-AC-22 | `request.rs` TC-185 test (already on main) | covered; not changed by this PR |
| FR-290-AC-12 application item of `q` | `tc_774_function_contract_and_application_items_replay` | covered (selected function, key, process-provider identity, `replay-parity`) |
| FR-290-AC-15 | `tc_774_an_undefined_application_item_is_refuted` | partly covered (FND-001) |
| FR-404-AC-2 | `a_completed_members_request_settles_replay_parity`, `a_members_item_bounds_build_a_request_that_replays` | covered apart from the unobservable "completes `10`" (FND-004); the member-by-member comparison is partial (FND-006) |
| FR-404-AC-3 | `an_undefined_calls_member_refutes_...`, in-crate `a_member_request_carries_the_process_provider_identity` | covered |
| FR-404-AC-5, AC-6, AC-7 | `another_runs_package_is_a_foreign_run`, `only_the_spine_limits_bound_the_run`, `foreign_run_comes_before_not_a_call_item` | covered |
| FR-404-AC-9 / TC-1173 step 9 | `call_replay_context_root_only.rs` | not updated; its header is now false (FND-002) |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The test tagged FR-290-AC-15 leaves out three of the AC's bullets. (1) The synthesized-owner case: the `value-validity` item of FR-395-AC-12's operation `body` `+` must refuse `NotReplayable` and settle `ReplayRefused(unknown_required_feature)`. This is the guard on the new `callable(owner)` filter, and nothing tests it. (2) `r`'s request carrying the `process-provider` identity of its node and occurrence (only `q`'s is asserted, in AC-12). (3) `p`'s `x > 3` application item settling `replay-parity` while `p`'s function-contract item settles `refuted` for the same `x = 0`. Fix: add these assertions. (1) is the one that matters, because a synthesized owner that is `callable_by_name` would now build and replay a request. | qsl-replay/src/proof_result/plugin_tests.rs:876-909; qsl-replay/src/proof_result/plugin_replay.rs:166-177 |
| FND-002 | medium | TC-1173 step 9 (FR-404-AC-9's root-only consumer) is not done. QSL-980's own ticket comments list step 9, "the built request", as completing in this PR. `call_replay_context_root_only.rs` still asks only for request index 99 (`NotACallItem`), and its header still says "a member's request is refused `NotReplayable` until QSL-980", which this PR makes false. Fix: build AC-2's request for the member through root paths only, and update the header. If the parameter node cannot be reached without `qsl_foundation`, say so in the header and on the ticket, because that would be a facade gap. | qsl-replay/tests/call_replay_context_root_only.rs:1-9,60-88 |
| FND-003 | low | The FR-098 unread-parameter rule is tested only for an `Int[0, 9]` parameter, which fills 0. The `Boolean` to `false` arm, the range-end arm (`Int[3, 9]` to 3, `Int[-9, -2]` to -2) and the refusal for an unread parameter of a non-integer type are untested. Fix: one TC-444 case per arm. | qsl-replay/src/execute.rs:1522-1547 |
| FND-004 | low | Spec defect: FR-404-AC-2 and TC-1173 expected result 2 say `replay` "completes `10`", but an application-claim replay result carries the claim's Boolean (FR-098 "The result carries the claim's Boolean"), so 10 cannot be observed. FR-290-AC-12 says the same for `q`. Fix (spec): reword to "the application completes (claim `true`) and settles `inconclusive` with `replay-parity`". | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:191 |
| FND-005 | low | The FR-098-AC-22 identity clause is weakly tested. The AC wants an identity minted for `p`'s function contract, and it says the identity is carried unchanged. The test sets `[9; 32]` and asserts only the settlement. Fix: mint the ADR-013 O-09 function-contract identity, and assert the request's identity where the result exposes it (the witness arm's record). | qsl-replay/src/execute/tests/application_claim.rs:229-243 |
| FND-006 | low | FR-404-AC-2 says "equal, member by member", but `member_request` compares a subset. It skips `source` (the assignments), `profile_selections`, `state_environment`, `supplied_limits` and `object_environment`, and compares `declared_domains` only by length. Because `ReplayRequestWire` has no `PartialEq`, a difference in those members would go unnoticed. Fix: compare every member, or derive `PartialEq` under test. | qsl-replay/tests/call_replay_context.rs:245-296 |
| FND-007 | low | Spec gap: FR-062 "Extent" has no name for the new public accessor `RequirementRecord::extent_roots()`, which FR-098 "unread parameters" depends on. Fix (spec): name it in FR-062's record members. | qsl-semantics/src/check/claims.rs:338-343 |

## Verdict

**Not mergeable** as it stands. The five FR-098 ACs are covered by real
tests that fail on the obvious mutations: node-only matching, selecting the
claim after the join, and the predicate path for value items. The FR-404
strengthening covers AC-2, 3, 5, 6 and 7. FND-001 and FND-002 are untested
or unfinished parts of the ticket's own scope and should be fixed in this PR.
FND-004 and FND-007 are spec edits and should go to the spec lane.
