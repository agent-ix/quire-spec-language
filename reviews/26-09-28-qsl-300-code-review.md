---
id: SR-774
title: "QSL-300 code review of PR 513 (FR-115 Frame selection, ProtocolClause S6a evaluate arm)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; spec/spec.md; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/state_clause.rs; qsl-semantics/src/model/observation.rs; qsl-semantics/src/model/observation/frame.rs; qsl-semantics/src/model/population.rs (read, unchanged); qsl-eval/src/value/expression/mod.rs; qsl-eval/src/value/expression/causes.rs; qsl-eval/src/value/expression/s6a/protocol_clause.rs; qsl-eval/src/value/mod.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/call.rs (read, unchanged); qsl-replay/src/spine/clause/tests.rs; qsl-replay/src/spine/clause/tests/frame.rs; examples/config-version/spine.rs; tools/arch-lint/api_surface.rs (read, unchanged); spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-514
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---
## Summary

Ticket: QSL-300. PR: quire-spec-language#513. This review covers
code and Rust (the rust-review lane is folded in here).

What was checked:

- **frame_violation is a violation with a witness, not a Refusal (QSL-296
  ruling).** `frame::verdict` maps a `decide_frame` finding whose code is
  `Code::FrameViolation` to `FrameVerdict::Violation(FrameWitness)`. Every
  other code goes to `Refused`. `decide_frame` (`population.rs:1440-1546`)
  raises `frame_violation` only through `frame_violation()` with the four
  causes `verdict` maps (create, retype, delete, field write), so the
  `frame-violation-cause-has-no-witness` fault arm cannot be reached. The S6a
  arm turns `Violation` into `EvalOutcome::Kernel(Completed(false))` and stores
  the witness in `env.witness`. `evaluate_frame` returns it as
  `FrameEvaluation.witness`. `run_frame` maps `witness: Some` plus
  `Completed(Boolean(false))` to `ClauseDisposition::FrameViolation`. That
  variant is `Evaluate` / `Violation` / `truth: false` / exit 10 in `stage`,
  `category`, `truth` and `exit_code`, and all four are total matches with no
  `_` arm on the disposition. No later step turns it back into a refusal.
- **Three outcomes stay separate.** `Holds` goes to `Completed(true)`, then
  `convert_outcome`, then `Evaluate(Completed(true))`: exit 0.
  `Violation` goes to `FrameViolation`: exit 10. `Refused(record)` goes to
  `FamilyResult::Refused(ProtocolClauseFrameRefusal)`, then
  `convert_outcome`'s `FamilyEvaluated(Refused)` arm, then
  `Evaluate(Refused(Family{code}))`: exit 20 from the
  `population_delta_mismatch` catalog code. Each has its own match arm, and
  TC-514 hits each one.
- **FR-106 checks 1 and 3 to 10 are shared.** `admit_invocation_documents`
  is one function with two callers: `admit_operation` (clause run) and
  `admit_frame_invocation` (Frame run). They differ only in `post_self`:
  postconditions only for a clause run, and always false for a Frame run,
  since the operation may delete `self`. That matches FR-106 check 9. Check 2
  stays in `admit_observations` and does not apply to a Frame run. Check 11 is
  one `frame::run` loop, wrapped by `enforce` (clause run) and `verdict`
  (Frame run). The two differ only in how a `decide_frame` finding is mapped.
- **Clause runs are unchanged.** `enforce` maps every finding with
  `refuse(admission_record(..))`. `admission_record` builds the same record
  as the old `admission_refusal`: the same code, cause, population, object and
  field lookup. The one reorder is `clause-declares-no-operation`, which moved
  from check 5 to before check 1. Check 2 lets only pre- and postconditions
  reach `admit_operation`, and FR-104 gives each of them an operation, so the
  fault cannot happen on checked input. The TC-465 FR-106-AC-5 tests
  (`qsl-semantics/tests/it/state_clauses.rs`) still pass in `make ci`.
- **Arch-lint fix (FR-100-AC-8).** `OperationName { model, object, operation }`
  is three `String`s defined in `qsl-replay/src/spine/clause.rs`, with its own
  `Display`. It is not a re-export and does not depend on qsl-eval. The new
  public items `ClauseRunSelection::Frame`, `ClauseDisposition::FrameViolation`
  (qsl-semantics `FrameWitness`) and `ClauseRunProvenance::frame` (`NodeKey`)
  name no `qsl_eval` path. `FrameEvaluation` and `CallFailure` are imported
  inside the private `run_frame` only. The `api_surface.rs:2623` test passes
  in `make ci`.
- **QSpec FR-013-AC-3: the frame comes only from the package.** The effect
  comes from `operation_frame_by_identity(key).operation().declaration.effect()`.
  The request carries no effect.
- **Rust idioms.** No `unsafe`, casts or locks. `too_many_arguments` allows
  follow the existing `admit_operation` precedent. The large `Decision` value
  is boxed in `FrameStop`. The hand-written `Debug` on `AdmittedInvocation`
  uses `finish_non_exhaustive`. There are no production `unwrap`s or panics.
  `#[trace]` tags are on all 9 TC-514 tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `run_frame` guards the case of a witness with a verdict that is not false (`frame-witness-without-a-false-verdict`), but not the reverse. With `witness: None`, a `Completed(Boolean(false))` goes through `convert_outcome` to `Evaluate(Completed(false))`: a violation with exit 10 and no witness. The arm makes this impossible today, but FR-115 requires a violation to carry its witness. Make that case an `EvaluateFault` too, so a future regression fails loudly instead of losing the counterexample. | qsl-replay/src/spine/clause.rs:906-929 |
| FND-002 | low | FR-115 Inputs still says the `Frame` selection's `operation` is "the `QualifiedName` `M::T::op`". The public type is now the spine-owned `OperationName { model, object, operation }`, a change forced by FR-100-AC-8. Update the spec text so it names the shipped type. | spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md:46-49 |
| FND-003 | low | FR-115's row in the spec index still says "not yet implemented -- TC-514 planned". The PR updated FR-115's Status and the TC-514 row in tests.md, but not this row. | spec/spec.md:541 |

## Verdict

PASS with three low findings. The QSL-296 ruling is met: `FrameViolation`
carrying a `FrameWitness` is a real outcome at every step, and nothing turns
it into a refusal. The three outcomes are separate. Checks 1, 3 to 10 and 11
are shared code, not copies. Clause-run behaviour is unchanged.

## Dispositions

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `run_frame` maps `witness: None` with `Completed(Boolean(false))` to `EvaluateFault("frame-false-verdict-without-a-witness")` |
| FND-002 | fixed | FR-115 Inputs names the spine-owned `OperationName` |
| FND-003 | fixed | the spec.md FR-115 row says implemented under QSL-300, TC-514 passed locally |
