---
id: SR-2698001
title: "code review of PR2698 postcondition range facts from preconditions (FR-577)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/facts.rs, qsl-semantics/src/check/facts/persistent.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/state_clause.rs, qsl-semantics/src/check/refusal.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/call_site.rs, qsl-eval/tests/it/total_functions.rs and the PR's tests; PR2698 b1fa7352c"
review_set: subset
---

## Summary

Ticket: QSL-1901. Code review with the rust-review lane folded in, scoped to `git diff origin/main...HEAD`. The fact transfer is sound on every path the ACs reach. The redefinition-family test covers both directions. Context matching compares resolved `EffectiveId`s, so a subtype or supertype context is excluded. Binder slots at or after `parameters` are dropped. An unreachable precondition or an empty interval intersection transfers nothing. A join keeps the larger `c`. The `let` outcome case is safe because each slot is bound by exactly one `let`. Integer typing in `subject` keeps Rational and Decimal orderings out of the difference facts. The problems found: the preconditions-first loop does not restore source order on its early-return paths; the operand message is filled in for Range refusals that carry no interval; and the result-slot shift has no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The preconditions-first loop restores source order only after it finishes. The limit, cancel, fault and `ResourceExhausted` paths `return Err(refusals)` mid-loop, in processing order and with a different set of clauses checked. `nodes_used` and the contract meter also accumulate in that order, so a package-wide limit can trip at a different clause than in source order. With `[post A refused, pre B exhausted]` the first refusal is now B, not A. This breaks FR-577 Outputs ("refused exactly as FR-104 refuses it") on the limit edge. | qsl-semantics/src/check/mod.rs:3196-3326 |
| FND-002 | medium | `name_operand` fills `operand` for whatever Range refusal comes first. That includes the Rational and Decimal carried-sum refusals, which carry `ProvedInterval::default()` for both intervals. `check_message` then renders "`…` must lie in [-inf, +inf] but is proved only in [-inf, +inf]", which contradicts itself. Limit the operand rendering to Range refusals at an integer arithmetic node, or to refusals whose required interval is bounded. | qsl-replay/src/spine/lifecycle.rs:723-750; qsl-replay/src/spine.rs:1438-1450; qsl-semantics/src/check/facts.rs:1161-1172,1462-1486 |
| FND-003 | medium | The result-slot shift (`with_preconditions(.., usize::from(has_result))`, then `root + shift` in `rekey`) is never exercised, because `withdraw` has no result type. Changing `shift` to 0 passes every test, yet it would key a parameter's precondition facts on the postcondition's `result` slot. That is unsound. Add a variant with a result-typed operation whose postcondition reads both `result` and a parameter. | qsl-semantics/src/check/state_clause.rs:1042-1044; qsl-semantics/src/check/facts.rs:2053-2060 |
| FND-004 | low | `interval_source` was inserted between `check_message` and its doc comment. `interval_source` now carries both doc paragraphs and `check_message` has none. | qsl-replay/src/spine.rs:1052-1067 |
| FND-005 | low | `Obligation::Range.operand` is display text in a semantic refusal type. The checker never fills it, and it takes part in the derived `Eq`/`Hash`, so the same refusal compares unequal before and after the spine stage. Rendering the operand in `check_message` from the sources the caller holds would keep `Obligation` semantic. | qsl-semantics/src/check/refusal.rs:2384-2392 |
| FND-006 | low | `in_redefinition_family` scans every record of every selected view, and the precondition filter scans every typed clause, once per postcondition. That is O(posts x records). Building a set of family keys and a map from (context, operation key) to precondition facts once, before the loop, would make both lookups constant. | qsl-semantics/src/check/mod.rs:1590-1604,3210-3224 |
| FND-007 | low | p09, traced to QSpec-FR-146-AC-8 and named "intervals come only from declared types and literal guards", now asserts that `a < b` proves `b - a`. QSpec FR-146 as written forbids that until STD-662 (Backlog) amends it. The change is justified by FR-577 ("function bodies") and tracked, but the test name is now wrong. The new comment's claim that a bound "reaches only the `-` of the same two paths" is not shown by any assertion. | qsl-eval/tests/it/total_functions.rs:867-921 |

## Scope and evidence

Diff `origin/main...HEAD` at b1fa7352c only. Rust lane: no `unsafe`, no new panics in library code, no recursion (`PersistentMap::entries` walks an explicit stack), no Tokio. The `difference` and `difference_interval` arithmetic was checked by hand for each relation, for negation and for both lower and upper ends. `join` uses an idempotent `max`, which `intersect_with` requires. `conjoin` intersects intervals, takes the union of membership facts and keeps the smaller `c`. The `CallSiteRefusal::Compile.region` plumbing uses `refusal.region()`, and `Source::render` checks the source identity, so `name_operand` cannot slice the wrong unit.

Dependents (concern 5): FR-577 only adds facts, so a unit that checked before still checks. The only tests that could break are ones that expect a refusal. A grep of qsl-analyze, qsl-package, qsl-eval, qsl-semantics, qsl-replay and the root `tests/` for `Obligation::Range` constructors, `CallSiteRefusal::Compile` patterns and `unproved-range` expectations found no expectation of the old no-fact behaviour. `the_function_body_deref_fact_is_a_range_fact_only` is unaffected because its `let` is the body root, not a guard operand. No golden file holds an `unproved-range` message. `dependency_references` are sorted after the loop. The full gate was not run.

- FR-577 (spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md): precondition facts, difference facts, refusal locus and operand. (examined)
- FR-104-AC-6 (spec/functional/FR-104-check-state-clauses.md): Checking the same unit twice, and checking it with its clauses in another order, gives each clause the same node identity and the same requirement record keys. (examined)
- FR-121-AC-37 (spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md): call_site refuses CallSiteRefusal::Compile whose region is Some, names the unit's source and spans exactly the bytes of pre(self.balance) - amount. (examined)

## Verdict

No high-severity soundness defect was found. Fix the three medium findings in this PR before merging.
