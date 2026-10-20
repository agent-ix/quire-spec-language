---
id: SR-12176
title: "Gap analysis of quire-spec-language PR 1017: FR-337-AC-5 and FR-338-AC-8 clause-root resolution against the closure-certificate tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@00ca7115731e145fefe3ec1e5856be7005a09b75; PR 1017: FR-337-AC-5, FR-338-AC-8, FR-126-AC-5, FR-126-AC-9, TC-1079, TC-1080 against qsl-analyze/tests/it/{closure_certificate,model_check,counter}.rs and qsl-replay/src/model_check/identity.rs"
review_set: subset
---

## Summary

Ticket: QSL-880. This is a planless gap analysis of the PR diff. Plan
completion: not assessed. Criteria were mapped from the `#[trace]` tags in the
diff and checked against the TC procedures. `quoin matrix --json` (quoin 0.28.3) was run on the reviewed head to
check how the criteria are bound.

- FR-337-AC-5 / TC-1079. `the_checker_resolves_each_clause_s_identity_from_the_package`
  compiles a real Counter package with two structurally identical `on origin`
  clauses `C` and `D`. It asserts the following. They share one node. Each
  resolved identity's node, `claim` origin and `over` node equal that
  declaration's package node. The two ordinals differ. `C`'s engine
  certificate is accepted for `C` and rejected `ShapeMismatch` for `D`. This
  is TC-1079 step 3, taken through the checker. Steps 1, 2 and 4 (I2 triple,
  foreign-triple refusals) are the existing qsl-package tests tagged
  `TC-1079`/`FR-337-AC-5`. The parameter-free half waits on STD-186 (declared
  gap).
- FR-338-AC-8 last sentence / TC-1080 step 6.
  `a_request_whose_clause_root_does_not_resolve_settles_failed` offers a
  clause the request's package does not declare. It asserts the typed
  `ClosureRefusal::Item(ItemRefusal::ClauseRoot(Undeclared{name}))`, the same
  refusal from `check_model` before any exploration, and a `Failed` settlement
  (value, label and execution) of a real `Holds` outcome. The control (the
  declared clause's certificate is `Accepted`) is asserted first, so the
  refusal cannot pass vacuously.
- Trace honesty. Both new tests carry only the TC id (`TC-1079`, `TC-1080`).
  That matches what each test verifies, so neither tag over-claims. The
  TC-1079 test adds a checker-side vector to TC-1079 step 3. The TC-1080 test
  is exactly step 6. The tags are incomplete, though: see FND-003.
- Cancellation. The pre-cancelled `check_closure` vectors
  (`a_stopped_check_names_what_stopped_it`,
  `key_admission_is_bounded_and_cancellable`, and the settlement `Deadline`
  vector) now stop inside identity resolution. They still assert
  `Stopped(Cancelled{source})` / `Incomplete(Cancelled)`, so the
  `check_closure` arm is covered. The `check_model` side is not covered (see
  SR-12175 FND-001; not repeated here).
- Formula rewrite. The FR-126-AC-9 TP-1 vector and the FR-338
  `ContractUndetermined` vector now write `6 mod (2 - c.value)`. The atom
  oracle (`Letters`) is unchanged on every asserted state. The test is not
  weakened (see SR-12175 Summary).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Only the `Undeclared` arm of `clause-root-unresolved` is tested through the checker. `ClauseRootRefusal::Unresolved` (the I2 lookup misses the package's own triple, for example because emission dropped the node) and `ClauseRootRefusal::Package` (emission or read-back refuses, including a subject whose `packages` lacks the domain package) have no vector that reaches `check_closure`, `check_model` or the `Failed` settlement. The I2 lookup's own refusals are tested in qsl-package. | qsl-replay/src/model_check/identity.rs:76-98; qsl-analyze/tests/it/closure_certificate.rs:593-657 |
| FND-002 | low | FR-126-AC-9's text still writes `always holds(6 / (2 - c.value) >= 0)`. The TP-1 test now verifies `6 mod (2 - c.value)`, because S3 refuses `/` there with `AmbiguousLiteral`. The deadlock-half test's doc comment records the same rewrite ("its division written `mod`"). The TP-1 test's doc comment just changed the formula with no note, and the AC was not updated. So the AC as written is verified by no test. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:360; qsl-analyze/tests/it/model_check.rs:438-451 |
| FND-003 | medium | FR-338-AC-8 is `untagged` in the computed matrix (`binders: []`), even though this PR adds the test for its last sentence. TC-1080 says "Tag the tests `#[trace("TC-1080", "FR-338-AC-8")]`". The new `a_request_whose_clause_root_does_not_resolve_settles_failed` test and the existing `a_refused_key_rejects_the_certificate_at_the_least_refused_state` test carry only `TC-1080`, so nothing binds FR-338-AC-8 and the matrix reports it untested. The tag is honest but incomplete: adding `"FR-338-AC-8"` to both tags closes it. The new TC-1079 test also omits the `"FR-337-AC-5"` its TC asks for. That AC is still bound by the three qsl-package tests, so only the checker-side vector is missing from its binders. | qsl-analyze/tests/it/closure_certificate.rs:593,986,521; spec/test-cases/TC-1080-a-refused-automaton-state-key-rejects-the-closure-certificate.md |

## Verdict

PASS. FR-337-AC-5's checker-side half and FR-338-AC-8's unresolved-root
sentence are covered by real compiled packages with typed oracles and positive
controls. The fixture identity is gone. Both tags are honest. The `on each` /
`when` / captures roots and the parameter-free clause are declared gaps
(S3/S4, STD-186) and are not counted here. FND-003 (medium) leaves
FR-338-AC-8 unbound in the matrix because a criterion id is missing from the
tags. It is a one-line fix and not a major. FND-001 and FND-002 are low.
Nothing blocks.
