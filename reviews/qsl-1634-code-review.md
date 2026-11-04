---
id: SR-2288001
title: "Code and Rust review of QSL-1634 temporal decisive information (quire-spec-language PR 2288)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/value/expression/mod.rs; qsl-replay/src/spine/clause/temporal.rs; qsl-replay/src/witness/derivation.rs; qsl-replay/src/result.rs; qsl-replay/src/result/wire.rs; qsl-replay/src/spine/native_result.rs; qsl-replay/src/outcome.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/clause/tests/temporal.rs"
review_set: subset
evaluated_revision: "997eedb4f737c555e66544546924324f2a80159b"
---

## Summary

Ticket: QSL-1634. Code review with the rust-review lane over `git diff origin/main...HEAD`
of PR 2288 at 997eedb4f. Checked against FR-283 Outputs, FR-330 Outputs, FR-327 and
FR-328 (violation position is the activation position), FR-286-AC-10.

Soundness of the verdict: `evaluate_holds` and the observed-letter path are untouched;
`evaluate_holds_traced` resolves the same condition and slots and binds the same
argument (`Value::Reference(over)`) as `ObservedLetters::atom`, only adding a trail.
`decisive_information` runs after the verdict is fixed, on its own meter, and only for
`Violated`, so it cannot change any verdict and the untraced path has no added cost.
The `derive_separating_witness` refactor into `decisive_stop` is behaviour-preserving
(every former `closed_scope()` exit is now `Ok(None)`, mapped back to `closed_scope()`).

## Verdict

CONDITIONAL. The plumbing is sound and the untraced path is unchanged, but the
decisive search swallows faults and falls through to a later atom (FND-001), and the
on-origin position mismatch (FND-002) is a spec contradiction that must be routed to a
spec ticket before the FR-283-AC-10 / FR-286-AC-10 tags are claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | decisive_information's find_map turns every failure of the deciding atom (evaluator fault, decisive_stop InternalFault, missing receiver path, missing condition) into None and moves on to the next atom, so a fault yields a decisive from a different, non-deciding atom instead of failing or yielding none | qsl-replay/src/spine/clause/temporal.rs:1148-1165 |
| FND-002 | high | Violated{at} is the activation position (FR-327, FR-328-AC-1), but the FR-283/FR-330 decisive rule evaluates atoms at that position; an on-origin `always p` violated later reports no decisive (contradicting FR-283's Violated row) and a clause with shifting operators can report an atom false at the activation that did not decide the violation; FR-283-AC-1/AC-7/AC-10, FR-286-AC-10 and TC-765 expect position 2 where FR-328-AC-1 gives 0 | qsl-replay/src/spine/clause/temporal.rs:1095-1106; spec/functional/FR-283-monitor-a-supplied-trace-offline.md:266 |
| FND-003 | low | The decisive evaluation runs on a fresh Meter::new(run.accounting) per holds atom and is excluded from usage, so a violation can do up to atoms x budget of unaccounted work | qsl-replay/src/spine/clause/temporal.rs:1149 |

## Fixes

- FND-001: once the first atom that evaluates `Completed(false)` is found, the search
  ends: its `decisive_stop` fault or missing receiver path returns `None` for the whole
  report (or an `EvaluateFault`, which FR-283 asks for an internal failure); an atom whose
  traced evaluation faults should end the search too, not skip to the next atom. Use a
  loop or `find_map` returning `Option<Option<_>>` so only "atom not false" continues.
- FND-002: engine bug or spec mismatch: a spec mismatch. The engine follows FR-327/FR-328
  (FR-328-AC-1 states `always holds(c.value <= 1)` is `Completed(false)` at position 0).
  The decisive rule in FR-283 and FR-330 assumes `position` is the position where the
  atom fails. File a QSL spec ticket to fix FR-283/FR-330 (take decisive at the letter
  position where the deciding atom fails inside the failing activation, which the
  evaluator has to report, or restate AC-1/AC-7/AC-10, FR-286-AC-10 and TC-765 with an
  `on each always[0,0]` clause). In this PR, keep the PR-body note, link that ticket, and
  narrow the test's trace tags to what it proves (see SR-2288002 FND-002).
- FND-003: charge the decisive evaluation to one shared meter for all atoms (or the run's
  remaining budget), so its work is bounded by one budget.

## Rust checks

No unsafe, no panics on library paths, no new public string errors. `TemporalDecisive`
is a pub data struct with a manual `PartialEq` over `same_element`, consistent with
`SeparatingWitnessRecord`. `DecisiveWire` uses `skip_serializing_if` for absent
`quantifier` and `index`, matching FR-283's presence rule. `item_record` writes
`decisive` only in the `Violated` arm; FR-267's writer is untouched. The lasso slot
mapping is checked with `usize::try_from` and guards an empty loop.

## Test evidence

`cargo test -p qsl-replay --lib -- spine:: outcome:: witness` at the PR head: 628 pass,
15 fail. The same failures reproduce at the merge base f01c91b00 (sampled:
`frame_occurrence_region_is_the_first_naming_clause_or_attempt_in_source_order`,
`layers::equality::core_option`, `timestamped_counter_cover_uses_half_open_boundary_rules`).
The PR body says 14; it is 15, all pre-existing. The new test passes.
