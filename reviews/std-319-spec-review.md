---
id: SR-15640
title: "spec review of STD-319 FR-262 Begin handle, child-request fault and cancellation rule (QSL PR #1243)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@dba30ca1e8ae27b971f894e844b082f65b116c0a; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/functional/FR-276-cancel-a-lifecycle-operation.md, spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md, spec/test-cases/TC-757-a-cancelled-operation-stops-within-one-charge-and-emits-nothing.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-757
    type: reviews
---
# SR-15640: spec review of STD-319 FR-262 Begin handle, child-request fault and cancellation rule (QSL PR #1243)

## Summary

Ticket: STD-319 (blocks IR-583; context quire-eval#2). The diff is `origin/main...dba30ca1e` (main 48108c9c1 is an ancestor; `git ls-remote` confirms the head). Six files, spec only. It changes FR-262 (Begin row, a no-`Cancel` paragraph, the child-request validation paragraph, the cancellation paragraph, AC-7, new AC-10 and AC-11), FR-276 (a Behavior bullet and AC-6), TC-757 step 6, FR-401-AC-4 and AC-6, FR-403 (Inputs, the Begin-failure paragraph, parity item 4, AC-4) and FR-402 (the `cancelled` payload note and the suite-scope list).

These claims hold against QSL main and quire-exact 1c6cbcb (the locked rev):

- `Meter::with_cancel` installs the handle, and `check_injected` runs first at every `charge`: a polled cancelled handle returns `Incomplete { limit_kind: WorkUnits, limit: consumed, consumed, next_charge, charge_point }` before any counter is written, so the denied charge is unrecorded and earlier spend stays. `Cancel::tripped` reports the cause a charge saw.
- The reference `call`/`evaluate` return `Ok(Evaluation)` with an `Incomplete` outcome and `tripped() == cause` for a pre-cancelled meter handle (model_reference_queries.rs:4110-4140, equality_matrix.rs:378-387 at `ChargePoint::FunctionCall` with no admitted charge, hyper_tuple.rs:403-415, evaluate/invariants.rs:178-215). qsl-eval has no path that returns `CallFailure::Cancelled` except `From<SuppliedFailure>` mapping `SuppliedCause::Cancelled` (expression/mod.rs:335-345), which is the supplied-value run's admission cancellation, so the "only a pre-charge Begin failure from the supplied-value run" statement is true of the reference.
- Lifecycle `execute` (spine/lifecycle.rs:617-640) returns `Cancelled` on `cancel.cause()` before any charge and on `tripped()` after the call; replay (execute.rs:727-743) does the same with `ReplayRefusal::Cancelled`. FR-276-AC-3, AC-6 and TC-757 steps 3 and 6 agree with this.
- FR-262-AC-10, AC-11, FR-276-AC-6 and FR-403-AC-4 are testable and can fail (a core that converts the denied charge to `Cancelled`, or reports a bad child request as `Input`, fails them). FR-090 names the category `internal failure`; FR-403-AC-13 already uses stage `quire-eval`.
- FR-401-AC-4's parenthetical does not change any expected value. FR-402's note agrees with FR-262 and the version-1 case rules.
- Ids FR-262-AC-10, FR-262-AC-11, FR-276-AC-6 and `child-request-not-admitted` appear on no open PR head and not on main. `git merge-tree` of the head with #1237 (std-302) is clean, and no #1237 hunk touches the changed FR-402/FR-403 lines. No status or changelog wording. `make check-spec-validation` exits 0 with no finding on a touched file.

Lead defaults: (1) Begin takes no `Cancel` is sound and removes the mismatch case. (2) The `Fault` mapping, stage and invariant are sound. The `Storage` default is acceptable but under-specified (FND-001, FND-005). (3) Reference-wins cancellation matches the code and the recorded tests; it is the only reading that keeps FR-401-AC-4 true. (4) AC-7's rewording is consistent with the Resume row and the delivery rule.

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | major: Child-request validation says each argument "is admitted against its parameter's `ValueType`" but not by what, so its non-`Input` stops are undefined. Begin's membership check is `SuppliedRun::admit` (FR-403 Begin check 3), which charges the run's admission budget and polls the run's `Cancel`; lifecycle `execute` builds that run with the caller's handle. If child validation reuses it (the PR body tells quire-eval#2 to "map a nested `admit_call` input refusal to the fault"), a child request can stop with `SuppliedCause::Cancelled` (then the core returns `CallFailure::Cancelled` mid-session, contradicting "only as a pre-charge Begin failure" and "observe cancellation only there"), `Limit`, `Capacity`, `UnsupportedHelper` or a run `Fault`, none of which the paragraph maps. Only `Input` and `Storage` are mapped. Whether validation charges the meter is also unstated, while AC-7 and AC-11 assume it does not. <br>Fix: state the mechanism: either membership through the type environment with no supplied-value run, no meter charge and no handle poll, or through the session's run with every `SuppliedFailure` arm mapped (Input to the fault; say what Cancelled, Limit, Capacity and UnsupportedHelper give). | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:164-177 |
| FND-002 | medium | major: TC-1121, the test case of FR-403-AC-4, still expects one cancelled vector: step 4 "The cancelled call is `CallFailure::Cancelled`, with no charge." FR-403-AC-4 now has two: a pre-cancelled meter handle gives the located `Incomplete` at `function.call`, and a pre-cancelled supplied-value run gives `CallFailure::Cancelled`. A coder following TC-1121 writes the expectation FR-262 now forbids. <br>Fix: split TC-1121 step 4's cancelled bullet into the two vectors with FR-403-AC-4's expected results. | spec/test-cases/TC-1121-the-no-std-evaluator-in-quire-eval-matches-the-std-reference.md:86 |
| FND-003 | medium | minor: AD-001 still states "Cancellation is observed at each charge and before each scheduler transition", and that "These rules apply identically to the reference and no_std implementations". FR-262 now says the core observes cancellation only at a charge and a transition with no charge runs to its end. The assurance case and the requirement contradict. <br>Fix: reword AD-001 to the FR-262 rule (observed only at a charge; a denied charge is the located incomplete; the lifecycle operation reports `Cancelled` from `tripped()`). | spec/assurance/AD-001-shared-evaluation-core-interface.md:138 |
| FND-004 | low | minor: FR-402's note says AC-10 and AC-11 are compared "by running those criteria's tests against both implementations", but FR-403's rerun obligation (line 376), FR-403-AC-7 and TC-1121 step 7 still name FR-262-AC-1 to AC-8 only, so nothing requires AC-10 or AC-11 to pass on the `no_std` evaluator. #1237's FR-405-AC-2 uses the same AC-1 to AC-8 range. <br>Fix: extend FR-403-AC-7, the line-376 sentence and TC-1121 step 7 to AC-10 and AC-11 (and tell #1237 to follow when it rebases). | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:376, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:401 |
| FND-005 | low | minor: The release rule lists terminations "with completion, refusal, undefinedness, incomplete or fault", but child-request validation can now end a running session with `CallFailure::Storage`, which the list omits; whether that termination keeps earlier spend (as the fault does) and which parameter index `Storage.parameter` carries (the callee's) is unstated. <br>Fix: add storage to the release list and state "keeps earlier spend; `parameter` is the callee parameter index". | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:175-177, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:225-227 |

## New findings (disposition pass 1)

Head 6c2f9a772 (`origin/main` 48108c9c1 is an ancestor; `git ls-remote` confirms the head). `make check-spec-validation` exits 0 with no finding on a touched file.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | major: The FND-001 fix decides child-argument membership "in the type environment of the session's checked closure" and says "No other failure arises from validation". That check cannot admit a `Population` argument. `TypeEnvironment::admits` (quire-semantic-value ac0e137, declaration.rs:44-52 and 2709) uses kernel `ValueType::admits`, which "refuses every `(ValueType::Population(_), Value::Population(_))` pair outright" (quire-exact 1c6cbcb value.rs:24-31, FR-089-AC-6). Begin handles this with a caller-layer check that resolves the `PopulationId` through the object environment and compares its declared maximum (FR-403 Begin check 2; qsl-eval expression/mod.rs:431-439). As written, a well-formed child request that passes a `Population` to a `Population<T>[N]` parameter (FR-153) ends the session with the `child-request-not-admitted` fault. <br>Fix: for a `Population<T>[N]` parameter, decide membership through the environment contract's population resolution and declared maximum, as Begin check 2 does. That check also charges nothing and polls no handle. An unresolved or mismatched population is the same fault. Optionally add a `Population` child vector to AC-11. | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:166-171 |

## New findings (disposition pass 2)

Head c4543768c (fix commit 3ef49e5be plus a merge of `origin/main` 43f4e2086, which is an ancestor). `make check-spec-validation` exits 0 with no finding on a touched file.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | minor: FR-262-AC-11 says "in three runs" but now lists four fault requests, followed by a "fifth run". The declared-maximum branch the prose adds ("whose binding's declared maximum differs") has no vector: the only fault vector for `h` is an unresolved `PopulationId`. <br>Fix: say "four runs", and add a run in which `h` is requested with a `Population` the environment resolves to a binding whose declared maximum is not `3`, ending with the same fault. | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:269 |
| FND-008 | low | minor: `CallFailure::Storage` now covers "the storage failure ... of the environment read", but the FR-403 environment contract's read 6 (whether a binding is recorded, and its declared maximum) returns C/SV/K values or boxed coded causes and defines no storage failure. The reference's Begin check (`resolve_population(..).is_some_and(..)`) has none either. The clause names a failure that no read can produce. <br>Fix: drop "or of the environment read". A storage failure then comes only from the type-environment check. | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:186-191 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-262 now decides membership in the type environment with no supplied-value run, no meter or budget charge and no handle poll, and says validation never charges or observes cancellation; the only failures are the fault and `CallFailure::Storage`. `TypeEnvironment::admits(&ValueType, &Value) -> Result<bool, EnvironmentFailure>` exists, charges no work and takes no `Cancel`, and its `EnvironmentFailure` is exactly `CallFailure::Storage`'s `failure` type, so it can be implemented without a run. The `Population` case is a regression: see FND-006. |
| FND-002 | fixed | TC-1121 step 4 now has both vectors. The meter-handle vector matches `check_injected` (Begin admission uses a run with a fresh `Cancel`, so the first poll is at `function.call`, giving `limit = consumed`, no admitted charge, and `Cancel::poll` records `tripped()`). The supplied-run vector matches `run.check_cancelled` at expression/mod.rs:453, which runs for every visited value, the scalar `3` included. |
| FND-003 | fixed | AD-001:138 now says "Cancellation is observed only at a charge", with the located incomplete result and the lifecycle `tripped()` report. |
| FND-004 | fixed | FR-403 (the rerun sentence, AC-7 and Dependencies), FR-262 Dependencies and TC-1121 (description and step 7) include AC-10 and AC-11. Both can run on the `no_std` evaluator through its registry (FR-403-AC-3) and the `quire-eval` entry. #1237's FR-405-AC-2 range still says AC-1 to AC-8, which is a planner note for that PR. |
| FND-005 | fixed | The release list includes storage failure. The `Storage` termination comes before the child's `function.call`, keeps earlier spend, and `parameter` is the callee's parameter index. |
| FND-006 | fixed | Disposition pass 2: FR-262 now decides a `Population<T>[N]` child argument as Begin check 2 does, through the environment contract's population resolution and declared maximum (FR-403 contract read 6), with no run, no charge and no poll. An unresolved population or a different declared maximum gives the `child-request-not-admitted` fault. FR-262-AC-11 adds an unresolved `h` fault run and a resolved `h` run applied with one `function.call` charge, and TC-1121 steps 7 cover both. The remaining nits are FND-007 and FND-008. |

## Verdict

Disposition pass 2: FND-006 is fixed and there is no new major finding. FND-007 and FND-008 are low-severity wording and coverage nits. The PR is mergeable once they are fixed, or with them deferred to the S0 coder's test pass if the lead accepts that.

Disposition pass 1: FND-001 to FND-005 are fixed, but FND-006 is a new major regression from the FND-001 fix, so the PR is not mergeable until it is fixed.

Review pass: not mergeable as is: FND-001 and FND-002 are major and should be fixed in this PR. FND-003 to FND-005 are minor and cheap to fix in the same round. The three reference-behaviour claims, the incomplete-result shape (`work_units`, `limit == consumed`, denied charge unrecorded), the new ACs' testability, id uniqueness, the #1237 non-conflict and the FR-090 category name are all verified. Outside this diff, qsl-eval's `CallFailure::Cancelled` doc comment ("the call stopped at its next charge") describes the behaviour the spec now rejects; that is a code-doc fix for the S0 coder, not a spec defect.
