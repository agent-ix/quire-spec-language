---
id: SR-17141
title: "Gap analysis of quire-spec-language PR 1512 (QSL-982 PR 2a): Büchi state keys and FR-338 tags"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@aa96d51839c63574fd759bae580957ea24e2943b; PR #1512 diff against origin/main; FR-338 Canonical octet vectors, Guarded TP-4 clause, FR-338-AC-2, FR-338-AC-4, FR-338-AC-5; qsl-eval/src/automaton/key_buchi.rs; qsl-eval/tests/it/automaton_key.rs; qsl-analyze/tests/it/closure_certificate.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-338
---
# Gap analysis of quire-spec-language PR #1512

## Summary

Ticket: QSL-982. This is a planless audit of the requirement surface the PR
touches, checked against `quoin matrix` (quoin 0.29.1) at the head, the trace
tags, the tests and the source. The PR does not change the spec. Plan
completion: not assessed.

Examined:

- **FR-338-AC-4** (examined). It is now `tagged` through one test,
  `every_canonical_octet_vector_is_the_authored_octets`. Each row of the
  "Canonical octet vectors" table was checked against that test, with
  these results:
  - Bad prefix: asserted by body suffix.
  - Carried root at u64::MAX: asserted by body suffix.
  - Bounded `eventually[0,1]` origin row: asserted as the full key.
  - `accept`/`reject`: asserted.
  - Deadlines 1 and 3 in both orders, and the refused extent: asserted.
  - Two interval occurrences: asserted, including the counter changes.
  - The offsets set and its refused reversal: asserted.
  - The TP-4 `buchi` carry: asserted.
  - Both `guarded-buchi` rows: asserted.
  - The deadlock complete `K_C`, with the profile and activation
    replacements refused: asserted.
  - The table's "clause-root lexical vector", which checks string ordinal
    zero/max versus number zero/overflow: **not asserted** by this test.
    See FND-001.
- **FR-338-AC-5** (examined). The new Büchi admission-negative test is
  tagged. Every listed Büchi negative is present, but the "waiting branch
  with a window entry" negative is refused for another reason (SR-17140
  FND-003).
- **FR-338-AC-2** (examined). The two `closure_certificate.rs` tests are
  newly tagged. See FND-002.
- **FR-338-AC-1** (context_only). Untagged. AC-2 is defined against AC-1's
  certificate.
- **FR-338-AC-16** (context_only). Untagged. Its body octets match the
  guarded vectors, but it is a checker-level (`ShapeMismatch`/`BadState`)
  vector that this PR does not reach.

Reverse gap: every new production path (`BuchiStates`, `BuchiPhase`,
`BuchiState`, `ContextActivation::EachWhen`, `FormulaShape::is_liveness`, the
`reachable_invariant` activation refusal) is owned by FR-338's Canonical
translation state and Guarded `on each` sections. None lacks a requirement.
The semantic disagreements are recorded in SR-17140.

## Verdict

Findings. The FR-338-AC-4 tag claims vectors its test does not assert. The
FR-338-AC-2 tags contradict their own module's statement that the AC-2
vectors are not verified as written. No reverse gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `#[trace("TC-525","FR-338-AC-4")]` tag on `every_canonical_octet_vector_is_the_authored_octets` overclaims, and it is AC-4's only binder. (a) The clause-root lexical vector is absent. The string ordinals `"0"` and `"18446744073709551615"` inside a full `K_C` are asserted only by the untagged `a_reachable_invariant_keys_its_clause_root_and_state_body`; the tagged test checks only body suffixes. JSON-number zero and `"18446744073709551616"` refusing as a root ordinal are asserted by no test in qsl-eval. (b) AC-4's "reversing subset ... insertion order leaves bytes equal" is not exercised for branches (subsets) in the tagged test, only in the untagged `safety_keys_are_the_authored_octets_and_a_function_of_the_facts`. (c) The bounded row's "a false fork before that counted step leaves clock/deadline 1 and does not reject" is asserted only in `automaton_key_bounded.rs`, under FR-338-AC-6. (d) The TP-4 row's "this differs from a positive F carry lacking discharge" relation is not asserted. Fix: tag the tests that hold (a) to (c) with FR-338-AC-4 as well, and add the number-zero/overflow root refusals and the (d) inequality. | qsl-eval/tests/it/automaton_key.rs:1517-1531; qsl-eval/tests/it/automaton_key.rs:85-139; qsl-eval/tests/it/automaton_key_bounded.rs:99-118; spec/functional/FR-338-check-an-en-1-closure-certificate.md:744-773 |
| FND-002 | medium | The new FR-338-AC-2 tags contradict the module doc of `closure_certificate.rs`, which still says "ADR-018 §6's `ConfigVersion` subject ... has no model-check fixture; the Counter subject stands in for it, so the FR-338-AC-1 and AC-2 vectors are not verified as written". AC-2's first two vectors are "AC-1's certificate" (§6 subject, `always holds(c.versionNumber <= 1000)`) with model state `(1, 0)` removed, and with the initial state removed. `a_certificate_missing_a_reached_state_is_rejected_at_it` checks the same rules on a two-counter Counter subject with `always holds(c.value <= 3)`. The deadlock half of `a_deadlock_is_bad_for_the_deadlock_item_alone` is AC-2 as written. `quoin matrix` now counts AC-2 `tagged`. Either build the §6 fixture, or keep only the deadlock test's tag and leave the stand-in untagged, with the module doc kept true. | qsl-analyze/tests/it/closure_certificate.rs:12-14; qsl-analyze/tests/it/closure_certificate.rs:385-391; spec/functional/FR-338-check-an-en-1-closure-certificate.md:936 |

## New findings (disposition pass 1)

Verified against PR #1515 head `9e96822d`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new TP-4 inequality check has a tautological line. `assert_ne!(positive_carry, states_key(r#"["buchi",...]"#))` compares two hand-written literals, so no code under test is involved. The `live.admit(positive_carry)` → `WrongForm("safety")` assertion next to it is real evidence, so the vector is still covered. Drop the `assert_ne`, or compare written keys instead. | qsl-eval/tests/it/automaton_key.rs:1717-1722 |

## Dispositions

Round 1, verified against PR #1515 head `9e96822d` (fix `5f5484aea`). The reviewer reran `quoin matrix` (quoin 0.29.1) at the head. FR-338-AC-4 is `tagged` via `every_canonical_octet_vector_is_the_authored_octets`. FR-338-AC-2 is `tagged` via `a_deadlock_is_bad_for_the_deadlock_item_alone` only.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The tagged test now asserts the remaining rows. Clause-root ordinals `"0"` and `"18446744073709551615"` appear in the full `K_C` and are admitted back. A JSON-number `0` refuses `ForeignContext`, the number u64::MAX refuses `NotCanonical`, `"18446744073709551616"` refuses `ForeignContext`, and a bare `18446744073709551616` refuses `NotCanonical`. Branch insertion order is equal both ways, and reversed canonical input refuses `Order`. The false fork leaves the origin state. The TP-4 carry is not the safety carry (`WrongForm`, see FND-003 for one tautological line). |
| FND-002 | fixed | The AC-2 tag is removed from the Counter stand-in `a_certificate_missing_a_reached_state_is_rejected_at_it`, and kept on the deadlock test, which is AC-2's deadlock vector as written. The module doc now says exactly this. The ConfigVersion missing-state vectors stay unverified as written, as the doc states. |
