---
id: SR-12264
title: "Gap analysis of quire-spec-language PR #1149: run_with_requirements against FR-395, TC-1089, FR-062-AC-14 and FR-075-AC-17 (QSL-938)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@80a871782a879f25764737aa4b0c72a68bccef89; PR #1149: run_with_requirements, CallRequirements, the check-stage owner field and their tests; against QSL FR-395 (AC-1 to AC-11), TC-1089, FR-062-AC-14 and FR-075-AC-17"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1149

## Summary

Ticket: QSL-938 (step 2). This is a planless gap analysis scoped to the PR
diff. Plan completion: not assessed. Every AC in scope has at least one
tagged test, and the tags name the AC they assert:

- FR-395-AC-1, AC-3 to AC-8 and AC-11 are tested in
  `qsl-replay/tests/call_requirements_facade.rs` (8 tests, all pass at this
  head).
- FR-395-AC-2 and FR-075-AC-17 are tested in
  `tests/it/call_requirements_items.rs`, against `items_from_requirements`
  as an independent oracle.
- FR-395-AC-9 is tested in `call_requirements_root_only.rs`, which imports
  only from the crate root. The T12-A rule forbids any `qsl_replay::spine`
  spelling, so a root-only consumer passes it.
- FR-395-AC-10 is a unit test of `associate` with the owner altered to
  `None` and to `ghost`.
- FR-062-AC-14 is tested in `claims/tests.rs`, where the owner oracle is
  read from the unit text and not from the record, and in
  `tests/it/call_requirements_items.rs` (operation-contract and
  temporal-satisfaction records).

No stubs, no tautological asserts. The AC-6 outcome comparison goes through
`Debug` text, because `CallOutcome` has no `PartialEq`. That is acceptable.

## Verdict

PASS with findings. There is no major gap. One medium and one low finding
are fixed in this PR. The gap that hid SR-12263 FND-001 (no fixture with an
operation body or precondition holding a scalar application) is covered by
that finding's fix, not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-062-AC-14 requires "for each fixture RR-1 to RR-17, every record has exactly one owner". `tc_1089_every_value_validity_record_has_the_declaring_function_as_owner` iterates `fixtures()`, which holds RR-1 to RR-13 and RR-15 to RR-17 only (its own doc says so at line 227). So RR-14 (`function all_positive ... { forall(v in s: v + 1 > 0) }`, the only fixture whose claim sites sit inside a quantifier binder) is never owner-checked, yet the test's doc comment claims RR-1 to RR-17. Fix: add RR-14's unit (as the TC-160 step 10 RR-14 test at line 1279 builds it) to the owner loop. | qsl-semantics/src/check/claims/tests.rs:227; qsl-semantics/src/check/claims/tests.rs:567-591 |
| FND-002 | low | FR-062-AC-14 and TC-1089 step 6 name the `operation-contract` vector as QSpec TC-198 L07's `post Q using V on M::A::remove` over its F1 domain package. `only_value_validity_records_have_an_owner` substitutes `post Q using v on Config::ConfigVersion::attemptUpdate { true }` over the ConfigVersion package. It asserts the same property, but the tagged test does not run the vector the criterion names. Fix: use the TC-198 L07 unit, or amend TC-1089 step 6 and FR-062-AC-14 to name the ConfigVersion `post` vector, in this PR. | tests/it/call_requirements_items.rs:108-143 |

## Dispositions

Round 1, reviewed at 7218f53c928b4de8adfc52f538348793099d1381. Each fix was verified against the code and spec at that sha.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7218f53c9. The owner test now chains RR-14's unit (`function all_positive ... { forall(v in s: v + 1 > 0) }`) onto `fixtures()`. |
| FND-002 | fixed | 7218f53c9. FR-062-AC-14 and TC-1089 step 6 now name the `post Q using v on Config::ConfigVersion::attemptUpdate { true }` vector that the test runs. |
