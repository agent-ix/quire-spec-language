---
id: SR-1594002
title: "gap analysis of PR #1594 (QSL-580a, FR-230 timed members at S3)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1594 git diff origin/main...origin/qsl-580a-timed-s3; spec/functional/FR-230 (AC-1, AC-2, AC-4, AC-8 claimed); qsl-semantics/tests/it/timed_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# SR-1594002: gap analysis of PR #1594 (QSL-580a, FR-230 timed members at S3)

## Summary

Ticket: QSL-580. Plan completion: not assessed. This is a planless audit of the PR diff's ACs, tags and code. The PR claims FR-230-AC-1, AC-2, AC-4 and AC-8, partly. Each tag below names an AC that its test only partly asserts, and one AC vector is unreachable as specified. Coder note on the AC-2 `time-invariant-shape` vector: confirmed as a real spec gap. QSpec's shared grammar defines `clock-bound = postfix, ( '<' | '<=' ), quantity-literal` (quire-specification proposals/quire-v1/shared-grammar.md:958), and QSL's `TimeInvariant` production follows it. A body `x >= 1` is therefore an S1 parse error, while FR-230-AC-2 says the vector is grammar-valid and refused at S3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-230-AC-2 lists "a time invariant body `x >= 1`" as a grammar-valid vector that S3 refuses `invalid_timed_model`/`time-invariant-shape`. The shared grammar's `clock-bound` admits only `<` and `<=`, so S1 refuses it and no S3 test can exercise the vector. Spec fix: drop or move the vector, and reach `time-invariant-shape` through a form the grammar admits (FR-230-AC-21's `exists(...)` body, or AC-25's `min` constraint), or widen `clock-bound` in QSpec FR-415. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:582; qsl-cst/src/grammar.rs:1410-1440 |
| FND-002 | medium | Two tests carry FR-230-AC-1 but do not assert that the `s` unit's package identity differs from the `ms` unit's. That half needs S4 emission, which this PR does not have. The S2-form half is also not asserted here. A partial test should not carry the full AC tag. | qsl-semantics/tests/it/timed_model.rs:168,240 |
| FND-003 | medium | Three tests carry FR-230-AC-2, but they cover six of its ten vectors. `time-invariant-shape` is unreachable (FND-001). The duplicate-source, `unit m` and untimed-`urgent when` vectors are in pre-existing tests tagged only `TC-685`. `a_clock_under_tick_time_refuses` asserts the span of the time declaration, not the field's span that FR-230 names. A package field has no source span, so that is a second spec gap. | qsl-semantics/tests/it/timed_model.rs:312,350; qsl-semantics/src/check/timed_model.rs:1184-1206 |
| FND-004 | low | FR-230-AC-8 is on `rpc_checks_to_its_clocks_guards_resets_and_invariants`. That test does not check the E3 resolution to the temporal layer, or AC-7's refusals for the same unit with no header. It only shows that the layer-selected unit checks. | qsl-semantics/tests/it/timed_model.rs:168 |
| FND-005 | low | FR-230-AC-4 is on two tests that do not assert that each part's data predicate is `self.ready` (only that one exists). The 6,400-place count comes later, in PR #1596's test, so neither PR alone carries the whole AC. | qsl-semantics/tests/it/timed_model.rs:255,292 |

## Verdict

**CONDITIONAL.** No untagged AC was introduced and no stub was found. The reverse-gap check found production code with an owning requirement for every new path. The trace findings are partial tags. Split each partial test into its own AC-n row, or drop the tag until the rest lands. FND-001 needs a spec fix ticket: the AC is unreachable as written. With code-review FND-001, the PR is not mergeable as is.

## Coverage

Plan completion: not assessed. Semantic review: done by hand for the four claimed ACs, not the opt-in fan-out. FR-230-AC-9 to AC-38 are declared out of scope in the PR body: no tags, no code. A full `quire matrix` run is in the reviewer report.
