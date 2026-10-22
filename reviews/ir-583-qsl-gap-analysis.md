---
id: SR-12355
title: "Gap analysis of quire-spec-language PR #1269: moved evaluation vocabulary repoint and reference adapter (IR-583 S0)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a999bdf3d587cbf14307e08388e6b8ae61cd594d; quoin 0.28.3 matrix over the PR head; acceptance criteria FR-401-AC-2, AC-3, AC-4, AC-5, AC-7 and FR-262-AC-4, AC-6, AC-7, AC-10, AC-11 (spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md, FR-262-evaluate-values-and-calls-at-any-depth.md); TC-170, TC-452 context; tests qsl-eval/tests/it/reference_adapter.rs, tests/it/family_outcome_layering.rs, xtask/src/definition_scan.rs, tools/arch-lint/api_surface.rs; reviewed together with quire-eval PR #3"
review_set: subset
date: '2026-10-22'
---
# Gap analysis of quire-spec-language PR #1269

## Summary

Ticket: IR-583 (S0). I ran `quoin matrix --json` (quoin 0.28.3) over the PR
head, and read each FR-401 and FR-262 row against the tests. Plan
completion: not assessed.

Matrix rows: FR-401-AC-2, AC-3 and AC-5 are tagged. FR-401-AC-1 is quire-eval's
own, and AC-4 and AC-7 are untagged. FR-262-AC-1 to AC-11 are all
`untagged`.

Examined and clean:

- **FR-401-AC-4.** I scanned every test diff for a changed expected value
  and found none: each string or number change is a path repoint or a
  reindent. qsl-eval passes all 778 tests at the head. That includes the
  pre-cancelled-meter `Ok(Incomplete)` assertions that FR-262 keeps as an
  evaluation. I did not run the qsl-replay, qsl-analyze, bench or
  conformance runs; the leader's gate covers them.
- **TC-170.** The definition-scan allow-list adds `closure` and nothing
  else. The FR-401-AC-3 definition scan finds no QSL definition of
  `FamilyOutcome`, `FamilyResult` or `EvalOutcome`.
- **TC-452 (FR-100-AC-8).** The arch-lint probes now re-export
  `qsl_eval::value::QualifiedName` in place of the moved `Evaluation`, so
  they still exercise a `qsl_eval` path that resolves. The probe still
  fails as it should.
- **FR-401-AC-2, positive half.** qsl-eval and qsl-replay name quire-eval in
  `[dependencies]`, and the seven lower crates name it in no table.

## Verdict

Coverage is short on the adapter, which is this PR's new behaviour.
FR-401-AC-5 is the one adapter criterion the PR tags, and it covers two of
its five cases. The adapter's request validation, including its
`Population` branch, has no test at all, and that is how SR-12354 FND-002
went unnoticed. Fix FND-001 to FND-004 in this PR. FND-005 can go to a
ticket if RT's edge belongs to a later slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-401-AC-5 covers two of its five registrations. The fixture has one function, `keep(a: Integer): Integer`, so the test covers an unknown key and a whole-type mismatch. It does not cover the AC's `f(x: Int[0, 9]): Boolean` with the range-only mismatch `([Int[0, 10]], Boolean)`. It does not cover the dependency package's `g(): Integer` → `Accepted`, which is the only test of `declaring`'s dependency walk. Nor does it cover the FR-151 synthesized candidate's key → `CallableNotAdmitted`, the only test of `callable_by_identity`'s `callable_by_name` filter. The test is also bound only by a doc comment, not `#[trace("FR-401-AC-5")]`. Fix: build the AC's fixture (f, a dispatched member call, a dependency g) and assert all five results. | qsl-eval/tests/it/reference_adapter.rs:95-124; qsl-eval/src/value/expression/closure.rs:93-109; qsl-semantics/src/check/mod.rs:4209-4225 |
| FND-002 | medium | The reference adapter has no FR-262 evidence. The matrix reports FR-262-AC-1 to AC-11 untagged, and quire-eval's toy tests point here for the criteria (SR-12353 FND-003). The adapter's new paths have no test: `admits_request` (AC-11: unknown key, arity, `f(10)`, both `Population` mismatches, the matching `Population` run applied with one `function.call`), a child request into an interpreted callee through the adapter (AC-4, AC-6), and cancellation through the adapter (AC-7, AC-10). Fix: add adapter-level tests for AC-11's six runs and for one AC-7 or AC-10 cancellation, each with `#[trace]`. | qsl-eval/src/value/expression/closure.rs:194-221, 259-310; qsl-eval/tests/it/reference_adapter.rs |
| FND-003 | medium | FR-401-AC-3 is proven only for definitions. The test shows that no QSL crate defines the three family types. It never shows that the nine old paths "each fail to compile from another crate". A `pub use` at any old path would pass the definition scan. I checked by hand that none exists at this head, but nothing stops one from coming back. A `compile_fail` proof is feasible: one `compile_fail` doctest per old path (`use qsl_eval::value::Evaluation;`, `use qsl_foundation::InternalFault;`, and so on), in a crate that depends on qsl-eval, qsl-semantics and qsl-foundation. One doctest per path stops one failure masking another. Pair them with a positive test that `use`s the new paths. | tests/it/family_outcome_layering.rs:49-74 |
| FND-004 | medium | FR-401-AC-2's negative half is missing. The AC requires that "a fixture manifest in which `qsl-semantics` depends on `quire-eval` fails the check, naming the crate and the table". The test reads only the real workspace metadata, so the check is never shown to fail. Its message names the crate but not the table. Fix: run the same predicate over a fixture dependency set with `qsl-semantics` → `quire-eval`, and assert that the failure names both the crate and the table. | tests/it/family_outcome_layering.rs:696-740 |
| FND-005 | low | FR-401-AC-7 is untagged: no test shows that FR-059's direction check reports RT's normal edge to `quire-eval` as clean, with no FB-11 cycle. If S0 owns it, add the fixture test. Otherwise record which slice does. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md (AC-7) |
