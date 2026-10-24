---
id: SR-12154
title: "Gap analysis of quire-spec-language PR #989: C1b2 FR-218, FR-308-AC-1 and FR-381 recorded halves"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f29f1ebc4f33bb512f205b4323b9351a73884ebc; FR-218-AC-1..AC-9, AC-11, AC-12, AC-17..AC-20, AC-22, AC-24, AC-26; FR-308-AC-1; FR-381-AC-1, AC-5, AC-6, AC-7, AC-11, AC-12; TC-663, TC-887, TC-1056, TC-1057, TC-1058, TC-1065; qsl-semantics/src/check/protocol_clause/tests/graph.rs, qsl-semantics/src/check/protocol_controls.rs, qsl-semantics/src/check/protocol_controls/model_state_tests.rs, qsl-semantics/src/check/protocol_obligation/tests.rs, qsl-semantics/src/check/protocol_obligation/tests/recorded.rs, qsl-forms/tests/it/protocol_control_forms.rs, and the production files they test"
review_set: subset
---
# Gap analysis of quire-spec-language PR #989

## Summary

Ticket: QSL-850 (the dispatching brief's choice; the PR body names no
ticket). PR quire-spec-language#989. Planless, diff-scoped gap analysis.
Plan completion: not assessed. I traced criteria to tagged tests by
reading. No matrix or test was run (static brief).

Trace bindings in the diff. Each was read against its AC and judged on
whether the test can fail on the behaviour the AC states:

- FR-218-AC-2 (`a_guard_reads_no_model_state_and_partitions_its_facts`):
  correct. It covers all three clauses.
- FR-218-AC-4, AC-5, AC-6, AC-7 (graph tests): correct. AC-5 asserts
  `loop_proof.is_some()` and not the recorded invariant and variant text.
  That is weak, but it can fail.
- FR-218-AC-11: correct. It covers the missing channel, both mismatch
  directions with retained channel, expected and actual, the matching case,
  and the cross-channel receive.
- FR-218-AC-12, AC-20, AC-26 (model-state tests): correct. AC-20 cannot
  assert the `operator` member, because the cause has none (SR-12153
  FND-005).
- FR-218-AC-17: correct for every clause the code can reach. The `finish`
  record half is unreachable, as the PR body says. The undeclared-owner
  half lives in an untagged test. See FND-002.
- FR-218-AC-18, AC-19: correct. Some assertions are partial; see FND-003.
- FR-218-AC-22: correct. It now includes the two-case `choice` that checks.
- FR-308-AC-1: correct. It uses `until(w.done)` and `when (true)`, not the
  AC's braces. That is the PR's spec conflict 3.
- FR-381-AC-1 (refusal test and recorded test), AC-5 (refusal and
  recorded), AC-6 (two tests), AC-7 (zero refusal and recorded counts; S1
  overflow in `qsl-forms/tests/it/protocol_member_forms.rs`), AC-11, AC-12
  (three tests): correct. AC-11 asserts every retained member of the
  dominance refusal (profile, compensation, member, commit, effect, reason,
  span) and the three admitted placements.

No binding is wrong, so there are no `check: trace` findings. The
earlier-lane over-claim pattern (tagging an AC whose behaviour the test
cannot fail on) does not recur.

Not claimed, matching the PR body: FR-218-AC-1 (replicated roles and
FR-208 `until`), AC-3 (the replicated-role region; the reader-set half is
tested untagged), AC-8 and AC-9 (protocol references, other tests), and
AC-24 (process and state-machine roles).

Code with no owning AC: the template frame-population refusal (FND-001).
The await members that no checker reads are SR-12153 FND-001.

## Verdict

Coverage of the claimed ACs is sound, and every binding is honest. One
medium gap (FND-001): a new refusal path with no test and no AC. The rest
are low: partial assertions, and a spec ambiguity about non-Boolean
`visible` entries. No finding here blocks on its own. The blocking item for
the PR is SR-12153 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Checker::frame_population` adds `ObligationCause::AmbiguousPopulation` (`ambiguous_declaration`/`ambiguous-name` at a template's operation, when the declaring type is in two or more populations). No test reaches it, and no FR-381 AC states it. It is only reachable for a template whose operation no attempt names; otherwise the assembler refuses first. Add a test and state the refusal in FR-381, or an AC. | qsl-semantics/src/check/protocol_obligation.rs:1049-1066; qsl-semantics/src/check/protocol_obligation/cause.rs |
| FND-002 | low | The AC-17 test binds `owner: _` and never asserts the retained `owner` member the AC names. The AC's first clause (a choice or repeat owned by an undeclared role refuses `missing-name` at the owner) is asserted only by the untagged `a_choice_owner_is_a_role_and_its_constant_guards_partition`. The `over`-read case asserts the slot and node but not the code or cause. | qsl-semantics/src/check/protocol_clause/tests/graph.rs:644; qsl-semantics/src/check/protocol_clause/tests/graph.rs:761 |
| FND-003 | low | AC-19 asserts `!cases.is_empty()` rather than the case names the AC says are retained. AC-18 never asserts the `missing_declaration`/`missing-name` code or cause. Its comment says the reads check "on the await's then path", but only the all-branch-join positive case is written. | qsl-semantics/src/check/protocol_clause/tests/graph.rs:907; qsl-semantics/src/check/protocol_clause/tests/graph.rs:848 |
| FND-004 | low | FR-218 "Choice and repeat decisions" says "A decision fact is a Boolean field read of a record binder", but `visible` entries are typed with `Goal::Infer`, so a non-Boolean entry such as `attempted.n` is admitted. AC-2's own test relies on this (`visible (attempted.n)` with guards `attempted.n > 0` and `attempted.n > 1`). The partition proof stays sound, because guard subterms are independent atoms. The spec and AC-2 disagree on whether non-Boolean facts may be listed, and the spec should settle it. | qsl-semantics/src/check/protocol_slots.rs:700; qsl-semantics/src/check/protocol_controls/model_state_tests.rs |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | TC-1058 row 19d says `join predicate(Ready) [a, b]` checks. While `join predicate` is refused as `not-yet-implemented` (coverage.rs `JoinPredicate`), that half cannot pass and is untested. The AC-11 test covers only the `quorum(1)` half. The AC-11 text does not name `predicate`, so the trace is still honest. Record it as the row's unreachable half until a slice checks `join predicate`. | spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md:59; qsl-semantics/src/check/protocol_clause/coverage.rs |

## Dispositions

Round 1, reviewed at 837a476a67d36fb7f671e1ed35ae2bbd3f80854b: fix commit
56c826467, plus merge 837a476a6 of origin/main. The merge leaves every
claimed `#[trace]` binding in place, FR-308-AC-1 included. It adds fence,
memory-clause and access-ordering refusal tests, which are untagged.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 56c826467 |
| FND-002 | fixed | 56c826467 |
| FND-003 | deferred | Partly fixed in 56c826467, which now asserts the AC-19 case names and the AC-18 code. The await-`then` positive case that the comment at graph.rs:969-970 claims is still not written. It is low and does not block. |
| FND-004 | deferred | Spec ambiguity. FR-218 says a decision fact is a Boolean field read, while AC-2's own example lists `m.n`. Nothing changed in the fix round, and it is low. Route it to the same FR-218 spec follow-up. |

Round 2, reviewed at 765ac39919e7edda36b18e56b74ca87e4cc9e2bd (commit
765ac3991; merged STD-184 0bbbba485). Re-read against the merged text:

- FR-218-AC-2's test now lists `attempted.n` as an individual non-Boolean
  fact, asserts both arm faults, and checks an atom against its
  parenthesized negation.
- FR-218-AC-17's test now asserts that a `finish` record, and a read that is
  both out of scope and unlisted, refuse only `missing-name` (TC-1056 rows
  49 and 50).
- FR-381-AC-11's test covers rows 19, 19a, 19b, 19c, 19d (quorum) and 19e.

Every `#[trace]` stays honest.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | 765ac3991 |
