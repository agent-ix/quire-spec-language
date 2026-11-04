---
id: SR-20261104130232
title: "Code review — QSL-1642 timed population refusal"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/timed_model.rs, qsl-semantics/tests/it/timed_model.rs, edited FR-230/391/480, TC-1331, QSL-1642 review notes"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

## Summary

Reviewed the Rust refusal path, focused integration test, linked requirements and test case, and the four specification-review notes included in the PR. The production check is scoped to time-invariant and urgent predicates, but the new test and AC reuse an existing FR-230-AC-55 identity, making the trace binding ambiguous.

## Verdict

**FAIL** — the changed test trace names an acceptance criterion id already used by a different clock-guard criterion, so the test cannot be attributed unambiguously.

## Assurance Context

No applicable AssuranceProfile exists in the repository; the reviewed spec/assurance items are measurement plans and architecture notes, not an active AssuranceProfile.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new timed-population criterion reuses FR-230-AC-55, already assigned to the quantified clock-guard criterion; the new trace tag and FR-480-AC-17 cross-reference therefore have ambiguous ownership. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:886; qsl-semantics/tests/it/timed_model.rs:291 |

## Scope

- Examined: FR-230-AC-55, the newly added refusal criterion: each named-population read in invariant/urgency predicates refuses at Config::calls; self-only predicates and unit-level preconditions remain admitted.
- Examined: the pre-existing FR-230-AC-55 clock-guard criterion: quantified guards range over live sequence slots and preserve empty-sequence semantics.
- Examined: FR-391's new timed-predicate boundary and unchanged population behavior for state clauses and holds.
- Examined: FR-480-AC-17's clarified type-resolution-only assertion.
- Examined: TC-1331's refusal cases and positive controls.
- Examined: the changed typed_alternative_in guard, named_population_read traversal, and the new trace-tagged integration test.
- Context only: all four changed spec/reviews/qsl-1642/*.md files.
- No CI workflow file changed.

## Verification

- cargo clippy --locked -p qsl-semantics --all-targets --no-default-features -- -D warnings: passed.
- cargo clippy --locked -p qsl-semantics --all-targets --all-features -- -D warnings: passed.
- cargo test --locked -p qsl-semantics --all-features: pending at review write time.
- cargo deny check --disable-fetch: skipped; advisory database lock is on a read-only path.
- quire matrix --scope . --format json: timed out after 120 seconds with no output.
- quoin advise --json: timed out after 90 seconds with no output.

## Dispositions

Round 1, reviewed against final head f9e37ff46e798976cb75b87bf15f5b6b2a6f8a67. The duplicate id is corrected: the existing quantified clock-guard criterion remains FR-230-AC-55; the named-population refusal now has the unique id FR-230-AC-58, uses TC-1334, and the implementation test carries both corrected trace tags. The focused timed-population integration test passed (1 test).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: In a timed model whose domain package declares calls, time invariant HasCalls when { forall(c in Config::calls : true) } { self.x <= 1 ms } and urgent CallsExist when { exists(c in Config::calls : true) } each refuse unsupported_construct/not-yet-implemented at Config::calls, naming the missing timed-predicate population scope, and produce no TimedModel. Time invariant and urgency predicates that read only self still check. The same named-population reads in a unit-level precondition remain admitted under FR-391. |
