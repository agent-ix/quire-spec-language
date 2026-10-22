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

## Verdict

Not mergeable as is: FND-001 and FND-002 are major and should be fixed in this PR. FND-003 to FND-005 are minor and cheap to fix in the same round. The three reference-behaviour claims, the incomplete-result shape (`work_units`, `limit == consumed`, denied charge unrecorded), the new ACs' testability, id uniqueness, the #1237 non-conflict and the FR-090 category name are all verified. Outside this diff, qsl-eval's `CallFailure::Cancelled` doc comment ("the call stopped at its next charge") describes the behaviour the spec now rejects; that is a code-doc fix for the S0 coder, not a spec defect.
