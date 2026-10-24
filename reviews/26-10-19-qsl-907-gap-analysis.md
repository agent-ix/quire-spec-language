---
id: SR-13339
title: "Gap analysis of QSL-907 guard facts for deref(r).f in a function body (PR #1051)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6cce08195; qsl-semantics/src/check/facts.rs, qsl-semantics/tests/it/state_clauses.rs; FR-104-AC-12, FR-381-AC-10, FR-056-AC-9, FR-098-AC-14, TC-1084"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1084
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: reviews
---
# SR-13339: Gap analysis of QSL-907 guard facts for deref(r).f in a function body (PR #1051)

## Summary

Ticket: QSL-907. PR quire-spec-language#1051, head 6cce08195. This analysis does not use a plan, so plan completion was not assessed. The Test Matrix comes from `quoin matrix --json` (quoin 0.28.3) on the PR head.

Criteria examined:

- FR-104-AC-12 and TC-1084. TC-1084's scope is FR-104-AC-12.
- FR-381-AC-10, which is bound to TC-1058 tests in `protocol_obligation/tests.rs`. This PR does not change it.
- FR-056-AC-9 and FR-098-AC-14: the existing `deref` function forms. Their bound tests are unaffected, and the `qsl-package` and `qsl-replay` lib suites pass.

Code findings are in SR-13338.

## Verdict

**FAIL.** The new test's `TC-1084` tag is not truthful. TC-1084 is the only test-case binding of FR-104-AC-12, so the matrix now lists FR-104-AC-12 as `tagged`, with this test as its only binder. Yet the test runs none of TC-1084's procedure. The PR body says "FR-104-AC-12 is not claimed for the deref form", but the matrix binds it all the same.

The behaviour the PR adds is also not owned by any requirement. FR-104 "Definedness facts" extends stable paths through model reads only for state clauses. No AC states the function-body `deref(r).f` form. The ticket asks for tests that back the existing ACs and nothing beyond them, but no existing AC covers this form. That needs a /specify cycle, not a test tag.

A let/shadowing test is not required. Shadowing is refused at name resolution, and let aliases are probed as sound in SR-13338.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_guard_on_deref_of_a_reference_is_a_range_fact_in_a_function_body` is tagged `TC-1084`, so the matrix binds it to FR-104-AC-12 as that criterion's only binder. It runs none of TC-1084's steps. Step 1 has a `pre` and an `invariant` over `Counter`, plus the record `Pair` function form `earlier.tally < 3 and later.tally = earlier.tally + 1`. Step 2 is `later.tally = earlier.tally`. Step 3 is `-earlier.tally` and `n` refusing `type-mismatch`. The test instead checks `deref(earlier).versionNumber` over object references. FR-104-AC-12 therefore reads as covered while its own text is unverified, which is coverage inflation. Untag the test, or tag it to a criterion that states the deref form (see FND-002). TC-1084 still needs a test that runs its own procedure. | qsl-semantics/tests/it/state_clauses.rs:5570; spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md |
| FND-002 | medium | No requirement owns the function-body extension. FR-104 "Definedness facts" defines stable paths through `deref(value(p)).f` keyed by (observation, path) for state clauses only. The PR applies this to every function body, and through `Definedness` to standalone `CheckMode::Linked` expressions too. That changes range facts, presence facts (`present(deref(r).p)` now discharges `value(deref(r).p)`; it refused `unproved-presence` before), nonzero facts, `let` aliases, and termination argument shapes (SR-13338 FND-001). There is no AC, and no test pins the presence or let-alias effects. Specify the function-body rule (one `Current` observation, no `pre`) with ACs for range, presence and the termination exclusion, then tag tests to those ACs. | qsl-semantics/src/check/facts.rs:474-490; spec/functional/FR-104-check-state-clauses.md:216-232 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 0fa5ac4a6 removes `#[trace("TC-1084")]` from the test. Its doc now says it claims no criterion because no requirement owns the behaviour yet. FR-104-AC-12 goes back to having no binder, as on main. |
| FND-002 | deferred | STD-219 (Backlog) is filed to specify "FR-104: range facts for deref(r).f hold in function bodies". The fix also narrows the behaviour to integer range facts only. Two new tests pin the narrowing: `the_function_body_deref_fact_is_a_range_fact_only` (presence still refused, a let alias carries no fact) and `recursion_through_a_deref_read_is_not_a_structural_decrease`. The function-body rule itself still has no AC until STD-219 lands. |
