---
id: SR-12204
title: "Gap analysis of quire-spec-language PR #1053: S3 parts of FR-212, FR-219 and FR-220"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0dd3dd31d15290d493e356497bb06ce6b290f2f2; PR #1053: every #[trace] added or changed in qsl-semantics/src/check/protocol_memory/tests.rs, qsl-semantics/src/check/protocol_controls.rs and qsl-forms/tests/it/protocol_control_forms.rs, against FR-219-AC-1..5, FR-220-AC-1..6, FR-212-AC-1..4, TC-657, TC-664, TC-665, TC-1065 and ADR-025 section 10; qsl-cst/tests/it/protocol_memory_refinement.rs (FR-220-AC-6)"
review_set: subset
---

## Summary

Ticket: QSL-558 (E15a). Planless. Plan completion: not assessed. Scope is the
S3 parts of FR-212, FR-219 and FR-220. The PR body declares the request-side,
EN-1, V-8 and S4-identity remainder, and this review does not block on it.

Trace bindings examined:

- `a_memory_default_is_recorded_and_a_nested_one_must_match_the_outermost`:
  TC-664, FR-219-AC-1. Correct. It covers Sc/Tso recording, a nested
  parallel with no clause or the same clause, and both directions of a
  differing default, refused at the nested clause and naming both controls.
- `a_weak_default_requires_join_all_without_outstanding`: TC-664, FR-219-AC-1.
  Correct. It covers `ra` with `any`, `tso` with `quorum`, and `tso` with
  `all … outstanding continue`, each at the join and naming the policy and
  the model.
- `each_fence_ordering_is_recorded`: TC-665, FR-220-AC-2. Correct for the
  AC's fence clause.
- `an_ordering_its_access_kind_does_not_take_refuses`: TC-665, FR-220-AC-2.
  Correct. It covers load+release, store+acquire, RMW with none and local+relaxed,
  each refused `ill_typed`/`operator-ineligible` and naming the attempt.
- `store_buffering_classifies_stores_loads_and_read_modify_writes`: TC-665,
  FR-220-AC-1, TC-664, FR-219-AC-1. See FND-001.
- `a_two_field_access_refuses_under_a_weak_default_and_checks_under_sc`: TC-665
  only. Correct. AC-3's request clause is pre-check work.
- `the_scheduling_member_is_part_of_the_checked_protocol`: TC-657 only.
  Correct. FR-212-AC-1..4 are model-checker work beyond the S3 member.
- `a_predicate_join_is_not_compiled_unchecked`: TC-1065. Correct. It was
  narrowed from the deleted memory/fence cases.
- `a_memory_default_and_an_attempt_ordering_are_built_as_written` (qsl-forms):
  no tag. It is an S2 build test.

FR-219-AC-1 and FR-220-AC-2 are S3 criteria in full, and their AC tags are
justified. FR-220-AC-1 is also wholly S3, but its test does not build what
the AC names (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-220-AC-1 tag is on a fixture that is not ADR-025 section 10's `SB`. In section 10, `Sx` stores cell `x`, `Sy` stores cell `y`, `Ly` is `read(y)` on `c1` and `Lx` is `read(x)` on `c2`. In the fixture, both stores run `by X` (one role, one cell). The loads are `M::Core::read` with `src` bound to nothing, so the x/y distinction the AC's "(x, v)… (y, v)" sharing rests on is absent. The test also asserts only the four attempt classes, never that `(c1, r)` and `(c2, r)` are branch-local. Because places collapse objects (SR-12203 FND-001), a faithful two-cell `SB` would classify identically, so this test cannot detect the sharing defect. Build the section 10 model (two cells x/y, roles c1/c2, `read(c: Cell)` bound to the other thread's cell), or drop the AC tag to TC-665 only. | qsl-semantics/src/check/protocol_memory/tests.rs:346-430, 504-589 |
| FND-002 | medium | No test covers the cases where the implemented sharing rule departs from FR-220/MA-1. There is no write-write test (two stores of one cell, nothing reads it) and no test of a register read again in its own branch. MA-4's any-object-location and population-membership refusal branches are also untested; only the two-field branch is. The membership branch is nearly unreachable as built, because single-object membership reads are dropped (`SymbolicLocation::Membership` in reads → `{}`) and sharing needs a reader. Concurrent create/delete in one population therefore never refuses at S3. | qsl-semantics/src/check/protocol_memory.rs:230-266, 411-432 |
| FND-003 | low | FR-220-AC-6 has a test but no tag. `a_relaxed_fence_is_a_parse_refusal_at_the_relaxed_word` asserts the S1 refusal at `relaxed` and says "claims no criterion". It covers the AC in full. The TC-665/FR-220-AC-6 tag was on the closed duplicate #1052 and is not carried here, so the matrix shows FR-220-AC-6 uncovered. This is S1, outside the ticket's S3 scope, but the tag is a one-line change. | qsl-cst/tests/it/protocol_memory_refinement.rs:208-215 |

## Verdict

PASS WITH FINDINGS for gap analysis. No major here: the tags on FR-219-AC-1
and FR-220-AC-2 are correct full-criterion tags, and the TC-only tags on
FR-220-AC-3 and FR-212 are right. FND-001 is the over-tag risk in this lane.
The AC-1 tag stands on a fixture that differs from the model the AC names.
Fix it in the same round as SR-12203 FND-001, since the faithful fixture and
the write-write/register tests are the tests that fix needs.

Declared remainder, not blocking: FR-219-AC-2..5, FR-220-AC-3 (request
clause), AC-4, AC-5, FR-211-AC-2..4, FR-212-AC-1/2/4 and the class parts of
AC-3, EN-1/request-writer memory selection, V-8 `WeakAccessShape`, S4
emission of protocol bodies (FR-383).
