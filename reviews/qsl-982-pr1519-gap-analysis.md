---
id: SR-17161
title: "Gap analysis of quire-spec-language PR 1519 (QSL-982): #1516 rulings, FR-338-AC-4 and AC-5 tags"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c57e5c5d7; PR #1519 diff against origin/main; FR-338-AC-4, FR-338-AC-5, FR-338-AC-6; qsl-eval/tests/it/automaton_key.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-338
---
# Gap analysis of quire-spec-language PR #1519

## Summary

Ticket: QSL-982. This is a planless audit against `quoin matrix` (quoin
0.29.1) at the head, the trace tags, the tests and the source. The PR does
not change the spec. Plan completion: not assessed.

Examined:

- **FR-338-AC-5** (examined). It is newly tagged on
  `an_unguarded_on_each_tp4_clause_is_keyed_each_buchi`. Each listed
  negative is asserted there:
  - a waiting `each-buchi` state carrying the well-formed `shift` window of
    its `eventually[1,*]` root refuses `Members`, while the same window is
    admitted in the active phase;
  - a `buchi` state under the `"each"` context refuses
    `WrongForm("buchi")`, and a `guarded-buchi` state there is refused as
    well;
  - an `each-buchi` state under the origin and `each-when` contexts refuses
    `WrongForm("each-buchi")`;
  - "a TP-4 key under a timed profile": the timed clause gets no
    `BuchiStates` (`NotTranslated`), and the key rewritten with the timed
    profile refuses `ForeignContext`;
  - the same-shaped state under a foreign activation (`origin`,
    `each-when`, `infinite`) or profile refuses `ForeignContext`.

  The foreign-*item* half is asserted by the AC-5 tests from earlier
  rounds (`safety_states_refuse_foreign_contexts_and_other_forms`,
  `a_bounded_key_is_admitted_against_its_own_context_only`). AC-5 is not
  overclaimed by this test or by the AC-5 tests from earlier rounds. With
  this PR, SR-17151 FND-002 (AC-5 tagged although `each-buchi` did not
  exist) is resolved.
- **FR-338-AC-4** (examined). Its vector table gained two `each-buchi` rows
  in #1516 (FR-338 :781-782). The only AC-4 binder,
  `every_canonical_octet_vector_is_the_authored_octets`, does not assert
  them. See FND-001.
- **FR-338-AC-6** (context_only). Its clause "an ordinary open-upper
  `F/U[a,open]` gains its least-fixed-point acceptance set only after the
  lower shift completes" is successor behaviour. The new
  `an_open_upper_eventuality_is_accepted_once_its_shift_completes` asserts
  the matching key-level membership and is tagged `TC-525` only, which is
  appropriate.

Reverse gap: every new path (`BuchiForm`, `BuchiState::Each`, `is_tp4`, the
`"each"` context arm, the open-upper acceptance arm) is owned by FR-338's
amended state table and acceptance rule. None lacks a requirement.

## Verdict

One medium trace finding. The FR-338-AC-4 tag misses the two `each-buchi`
vector rows, which are asserted only in a test not bound to AC-4. The fix is
one tag or two assertions. The AC-5 tag is correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-338-AC-4 vector table now holds the `each-buchi` rows: `["each-buchi","waiting",[[],[],[],[]]]` with context activation `"each"`, and `["each-buchi","active",[[[[],"negative"]],[],[],[]]]`, which "differs from the phase-less origin `buchi` key". `every_canonical_octet_vector_is_the_authored_octets`, AC-4's only binder, has no `each-buchi` assertion. Its doc still claims "every canonical octet vector". The rows are asserted exactly in `an_unguarded_on_each_tp4_clause_is_keyed_each_buchi`, which is tagged FR-338-AC-5 only. Either add `FR-338-AC-4` to that test's tag, or assert the two rows in the aggregate. | qsl-eval/tests/it/automaton_key.rs:2373-2405; qsl-eval/tests/it/automaton_key.rs:1564-1580; spec/functional/FR-338-check-an-en-1-closure-certificate.md:781-782 |
