---
id: SR-12174
title: "Gap analysis of quire-spec-language PR #1010: FR-336 AC-1 and AC-4 against TC-846, FR-125 and FR-380"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@14cde82acd99ffe92f31fcdb44cf6bb87c1c74de; PR #1010: temporal requirement record population domain (qsl-semantics), ModelRequest (qsl-route) and their tests; against QSL FR-336 (AC-1 to AC-4), TC-846, FR-125, FR-380, and QSpec FR-361-AC-8"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1010

## Summary

Ticket: QSL-876. Planless gap analysis over the PR diff. Plan completion:
not assessed. Read-only: nothing was built or run. Test-to-AC bindings were
read from the `#[trace]` tags in the tree at the reviewed sha.

Per acceptance criterion of FR-336:

- **FR-336-AC-1.** Covered. `model_subject_bound::an_unbounded_population_stays_in_the_requirement_record`
  (the claim API) and
  `emit::tests::temporal_clause::the_emitted_record_carries_the_population_domain_and_no_universe`
  (the real emission: `Unbounded`, exactly the population and
  infinite-trace domains, and the bounded variant with one domain). The
  "for `{a, b}` and for `{a, b, c}`" clause holds by construction, because
  the record has no universe input (FND-003).
- **FR-336-AC-2.** Refusal half: `model_check::a_population_with_no_universe_is_refused_before_exploring`,
  tagged `TC-846` only, before this PR. Explores half: no test, blocked on
  EN-1 (`FormNotDecided` for `always eventually`), as the PR declares
  (FND-002).
- **FR-336-AC-3.** Covered before this PR by
  `model_check::a_proof_bound_does_not_make_an_unranged_parameter_finite`.
  Not in scope.
- **FR-336-AC-4.** Tagged on
  `universe_obligation::two_universes_are_two_obligations_and_each_result_joins_its_own_index`.
  That test can fail on `ModelRequest`'s real logic: dedup by obligation,
  distinct indices, and the index-to-obligation join for each `proved`
  result. It cannot fail on "the obligation identity binds the universe"
  (FND-001).

FR-380 bound rule: a `bound` removes the population domain, which matches
FR-380 "Meaning" and is tested in both the claim test and the emission
test. FR-125 (the model subject) and QSpec FR-361-AC-8 (`no-instance`
versus `requires-bound`) were read for context. Neither is changed by this
diff. Code with no owning requirement: none. `ModelRequest` is owned by
FR-336 but has no production caller (SR-12173 FND-002).

## Verdict

Mergeable. No high finding. AC-1 is fully evidenced through the real
emitter. AC-4's tag is honest about `ModelRequest`'s join but over-claims
the universe binding, which no production subject type carries yet. AC-2's
explores half stays open on EN-1, as declared.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-336-AC-4 test's "two obligation identities that differ only in the subject's universe" holds by the test's own `Subject { universe, initial }` struct. `ModelRequest<S>` is generic and compares `S` with `==`, so the test cannot fail on whether a real subject's identity binds its universes. No production subject type can be the `S` (`qsl_eval::ModelSubject` is not `Eq`). The tag is honest for "each `proved` result joins only its own request index" and over-claims the universe half. Keep the tag and state the seam in the doc comment, or move the universe half to the EN-1 driver test once a real subject identity exists. | qsl-analyze/tests/it/universe_obligation.rs:23-28,96-107; qsl-route/src/model_request.rs:36-60 |
| FND-002 | medium | FR-336-AC-2's explores half ("with `{a, b}` it explores and returns FR-126-AC-1's outcome") has no test. EN-1 refuses `always eventually` as `FormNotDecided`. Declared as blocked on EN-1. The refusal half's test carries only `TC-846`. Deferred to the EN-1 ticket. | qsl-analyze/tests/it/model_check.rs:772-800 |
| FND-003 | low | FR-336-AC-1 names two requests, with universe `{a, b}` and with `{a, b, c}`. No test pairs the record with a request of each universe. The AC-4 test writes the one `requirements` value for both items, so its equal-extent check compares a value with itself. The property holds structurally (the record has no universe input), so this is weak evidence, not a wrong result. | qsl-semantics/tests/it/model_subject_bound.rs:63-104; qsl-analyze/tests/it/universe_obligation.rs:62-70,108-113 |
