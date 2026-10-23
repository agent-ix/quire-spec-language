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

## New findings (disposition pass 1)

Round 1 was reviewed at f239768f617ad00b6e77a04e76ce2fcb5ac69faa, against
FR-262 and FR-401 as amended by STD-335 (ac178fd85), which merge 854ea3b26
brought into the branch.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The adapter's own `admit_request` has two branches no test reaches: the `Population` branch (unresolved id, wrong declared maximum, matching maximum) and the `Storage` mapping. The adapter test says these "are quire-eval's toy closure tests". The toy tests cover the scheduler, not this adapter code. | qsl-eval/src/value/expression/closure.rs:197-233; qsl-eval/tests/it/reference_adapter.rs:436-484 |
| FND-007 | medium | FR-401-AC-3 as amended requires `qsl_replay::Category` to fail to compile from another crate. It compiles, because of the re-export in SR-12354 FND-001, and the `compile_fail` doctest list in qsl-eval/src/lib.rs does not include it. | qsl-eval/src/lib.rs:18-71; qsl-replay/src/lib.rs:236 |
| FND-008 | medium | Two criteria added by STD-335 have no test. FR-401-AC-8 (host-body dispatch from an interpreted call site through the adapter) has none, and its implementation is SR-12354 FND-003. FR-262-AC-12 (Begin polling the meter's handle and a caller-supplied run's handle, over `call`/`evaluate` and over Begin) has none, and its implementation is SR-12354 FND-008. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md (AC-8); spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md (AC-12) |
| FND-009 | low | `begin_observes_the_meters_cancelled_handle` is traced to FR-262-AC-10, but it checks the Begin rule that AC-12 states, not AC-10's charge-time cancellation. Retag it `FR-262-AC-12`. | qsl-eval/tests/it/reference_adapter.rs:410-434 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed d726f6d26 | `registration_is_decided_from_the_packages_admitted_signature`, traced FR-401-AC-5, asserts `f` `Accepted`, the range-only `Int[0, 10]` `SignatureMismatch`, dependency `g` `Accepted` through `link_with`, a not-callable-by-name function `CallableNotAdmitted`, and an unknown key `CallableNotAdmitted`. |
| FND-002 | fixed d726f6d26 | Adapter tests are now traced to FR-262-AC-4, AC-6, AC-7 and AC-11 (unknown key, two arguments, `f(10)`, and `f(9)` applied with one charge). The residual branches are FND-006. |
| FND-003 | fixed 86590fbe4 | qsl-eval/src/lib.rs has one `compile_fail,E0432` doctest for each of the nine original old paths, plus a positive doctest of the new paths. The amended AC-3 entry is FND-007. |
| FND-004 | fixed 86590fbe4 | `a_lower_layer_naming_quire_eval_fails_the_placement_check` asserts that the fixture fails with "qsl-semantics names quire-eval in [dependencies], which sits above it", naming both crate and table. |
| FND-005 | fixed 86590fbe4 | `a_normal_edge_to_the_quire_eval_leaf_is_clean` (tools/arch-lint/metadata.rs), traced FR-401-AC-7 and TC-156. |

## New findings (disposition pass 2)

Round 2 was reviewed at a05545fba3c2b25301496b1d3ba1c1e138591b33, against
STD-339 (62d5f232a).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | Two of FR-401-AC-9's named vectors are not bound to it. `received_equality_source_cause_crosses_actual_public_call_and_evaluate_unchanged` (the TC-918 `Compare` run in invariants.rs) and `empty_admitted_member_faults_through_public_call_and_evaluate_with_exact_prefixes` (the population query, with its n-th-poll prefixes) both assert the AC-9 behaviour and pass. Neither is traced `FR-401-AC-9`: only the equality_matrix vector and the new `call_supplied` test are. Add the tag to both. | qsl-eval/src/value/expression/evaluate/invariants.rs:1357; qsl-eval/tests/it/model_reference_queries.rs:4020 |

## Dispositions (round 2)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed 76a6c0986 | `the_adapter_validates_a_population_request_by_its_declared_maximum` (traced FR-262-AC-11 and FR-089-AC-5) asserts `Ok` for `Population<5>` with a maximum-5 binding, `NotAdmitted` under `Population<6>`, and `NotAdmitted` for an unresolved id. The `Storage` mapping cannot be reached from QSL, because the kernel reservation hook is private. It is covered by quire-eval's toy test. |
| FND-007 | fixed 76a6c0986 | qsl-replay/src/lib.rs has a `compile_fail,E0432` doctest of `use qsl_replay::Category;`, and the re-export is gone. |
| FND-008 | still-open | FR-262-AC-12 is now tested by `begin_of_a_call_and_of_an_expression_observes_the_meters_handle`, but only for run 1, the meter's handle cancelled before Begin (with both causes), plus the uncancelled and no-handle sessions. Run 2 (the meter's handle tripping at the first poll point for the second argument) is untested. Runs 3 and 4 and the last sentence of the AC describe a caller-supplied run over a different handle. `quire-eval`'s Begin with contract admission takes no run, so those runs cannot be expressed. That needs a spec ruling (draco). FR-401-AC-8 as amended still binds the std reference evaluator ("the runs over `f(3)` and over the requesting body hold as stated"). No test shows the factory invoked once per application, `f`'s interpreted body uncharged, or a requesting body resumed with the host body's `7`, and no test is tagged AC-8. |
| FND-009 | fixed 76a6c0986 | `begin_observes_the_meters_cancelled_handle` is retagged `#[trace("FR-262-AC-12")]`. |

## New findings (disposition pass 3)

Round 3 was reviewed at 551a919e8db696feaea6748bb5a84cccea67e488. The fix
commit is 2ad3aec61. The governing text is STD-339's follow-up (04cee5579,
#1292, merged into the branch). It cuts FR-262-AC-12 to two runs and adds
FR-401-AC-10 and TC-1191.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | FR-401-AC-10, new in #1292, has no test bound to it. `call_supplied_polls_the_callers_handle_and_the_meters_at_the_first_charge` asserts most of the `g` package half, but it is traced `FR-401-AC-9`. It never asserts that the caller run's handle is not tripped, because the run's `Cancel` is moved in and dropped. It does not cover the TC-918 `Compare` package that the AC names. Fix: retag it `FR-401-AC-10` and `TC-1191`, keep the run's handle so it can assert `tripped() == None`, and add the `Compare` package run. | qsl-eval/tests/it/reference_adapter.rs:622-679 |

## Dispositions (round 3)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-008 | still-open | Two parts are resolved. The FR-401-AC-8 std half is fixed in 2ad3aec61 by `a_registered_body_for_f_runs_for_the_root_and_the_requesting_body`. It asserts 7 for `f(3)` and for the requester, the factory begun once, and one or two admitted charges. #1292 dropped the caller-supplied-run variants of AC-12. One part stays open: AC-12's second run, the meter's handle tripping with `Deadline` at the supplied-value run's first poll point for the second argument, still has no test. The test comment says such a handle "cannot trip at the supplied-value run's poll point (`Cancel`'s observer fires on meter charges only)". That is not so. `Cancel::poll` calls the observer at every poll, and quire-semantic-value's `check_cancel` and quire-exact's membership `poll` call `cancel.poll()`. A local probe at this head shows it: an observing handle on the meter saw 18 polls during Begin admission of `g(3, 4)`, and cancelling at poll 10 made `begin_call` return `CallFailure::Cancelled(Deadline)` with zero work. Add that run. Also update the comment, which still says runs 3 and 4 await a draco ruling. |
| FND-010 | fixed 2ad3aec61 | `received_equality_source_cause_crosses_actual_public_call_and_evaluate_unchanged` and `empty_admitted_member_faults_through_public_call_and_evaluate_with_exact_prefixes` now carry `FR-401-AC-9`. |
