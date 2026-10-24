---
id: SR-11177
title: "QSL-502 quire-spec-language PR 857 gap analysis: FR-316, FR-318, FR-323 trace and AC coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9290f614ae36ee7c4986a8024dd3917294443667; qsl-semantics/src/check/sum_case/tests.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/tests/it/variant_fields.rs, qsl-forms/tests/it/sum_case_forms.rs; spec FR-316, FR-318, FR-323, FR-324, FR-360"
review_set: subset
---

## Summary

Ticket: QSL-502. Planless gap analysis of the PR's tagged tests against the
ACs they claim. Done manually by reading each test, not through
`quoin matrix` (brief: no builds). Plan completion: not assessed.

Bindings checked:

- FR-316-AC-1, AC-2: correct. AC-4: correct for the 5,000-member and
  1,000-level checks. The limit is the assembly environment work ceiling,
  not a `CheckingLimits` field (recorded below).
- FR-316-AC-3: the binding is wrong on the code (code-review SR-11176
  FND-001).
- FR-318-AC-1, AC-2, AC-3, AC-4, AC-6: correct for the AC text. The
  arm-level loci are the arm body's region (recorded below).
- FR-318-AC-4 on the variant_fields test: correct (enum-typed scrutinee).
- FR-323-AC-3: correct for the keyed refusal. `Sequence<F>` admission is in
  the sibling test.
- FR-318-AC-5: wrong (FND-001).
- FR-323-AC-1 and FR-323-AC-2: wrong (FND-002).

Recorded, not blocking (medium):

- The arm-level exhaustiveness loci are the arm body's span, not the arm's,
  because S2 has no arm node. This is recorded in the PR body.
- Assembly resolves type forms inside arm bodies before the typer runs. So
  `convert<Missing>` in an unknown-member arm refuses `missing_declaration`
  instead of `unknown-member`. This is recorded in the PR body as remaining
  FR-318-AC-4 work.
- FR-316-AC-4 names a `CheckingLimits` field, but the test drives
  `AssemblyLimits.environment.work_units`.
- FR-318-AC-5 and FR-318-AC-6 run through the private `Typer`, because
  lowering refuses unions until FR-319/FR-320.

## Verdict

FAIL for this method: two blocking trace overclaims. In each, a criterion
that the PR body itself lists as not done is tagged on a test that does not
assert it, so the computed matrix would report it covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-318-AC-5 is tagged on `a_case_checks_under_the_clause_kind_it_stands_in` and `a_case_adds_no_claim_and_its_arm_bodies_are_claim_sites`. Neither checks a `case` inside a state clause body or a `decreases` measure. The first only passes four `ClauseKind`s to the typer over a function body, with no decreases kind. Neither asserts the clause's one `value-validity` record. The PR body lists FR-318-AC-5 as not done. Remove the tag, or add the state-clause and measure checks and the record count. | qsl-semantics/src/check/sum_case/tests.rs:339-357, 439-466 |
| FND-002 | medium | FR-323-AC-1 and FR-323-AC-2 are tagged on `collections_of_a_union_are_admitted`, which asserts only that the parameter types are admitted. The ACs require built Set/Bag/OrderedSet/Sequence values with element counts, union-key order and first-occurrence order. These are runtime halves that the PR body says are not done. Remove the two tags, or narrow them to what is asserted. | qsl-semantics/src/check/sum_case/tests.rs:477-487 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 0d98c5aae removes the FR-318-AC-5 tag from both tests. Round 1, reviewed at 0d98c5aae. |
| FND-002 | fixed | 0d98c5aae narrows the collection test's tag to `TC-833` alone. A TC-only tag claims no AC row. Round 1, reviewed at 0d98c5aae. |
