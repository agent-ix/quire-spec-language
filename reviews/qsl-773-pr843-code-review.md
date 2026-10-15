---
id: SR-9121
title: "Code review of quire-spec-language PR #843: adapt to QSV fallible TypeEnvironment::admits"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6426347a1c5709993e094bda667c6ff5890168e6; PR #843 diff vs main: Cargo.lock, qsl-eval/src/value/expression/{mod.rs,evaluate.rs}, qsl-replay/src/{execute.rs,execute/argument.rs,execute/state_clause.rs,outcome.rs,result/wire.rs,result/wire/value.rs,spine/call.rs,spine/clause.rs,spine/native_result.rs}, qsl-semantics/src/value/environment_stage.rs, src/command/output.rs, tests"
review_set: subset
---
# Code review of quire-spec-language PR #843

## Summary

Ticket: QSL-773 (QSL half). Code review with the rust-review lane, scoped to
commit 6426347a1 over main. QSV main 60107c5f changed
`TypeEnvironment::admits` to `Result<bool, EnvironmentFailure>` and added
`ConstructionCause::StorageDenied` / `StorageCapacity`. This PR bumps the lock
and adapts the callers.

Examined:
- Cargo.lock: only `quire-semantic-value` moves (b7d5531 to 60107c5f). No
  Cargo.toml change (clean).
- Every `.admits(` in the workspace (`git grep`, all crates incl. bench,
  fuzz, xtask, src/command). The five `TypeEnvironment::admits` sites
  (eval mod.rs:203, evaluate.rs:2033, replay argument.rs:233, wire
  value.rs:283, spine call.rs:757) are adapted. The rest are kernel
  `ValueType::admits` or lowering `target.admits` (clean).
- `validate` return type changed from InputRefusal to CallFailure. Both
  callers already used `?` into `CallFailure`. Arity, WrongValueKind and
  DanglingReference keep their order, code and cause through `From` (clean).
- `CallFailure::Storage` is handled in every match: replay
  `call_failure_to_replay_refusal`, `state_clause::separate`, spine
  `convert_call_failure`, `check_clause_admitted`, `check_frame`. There is
  no wildcard arm that could absorb it (clean).
- `.all` to `for` / `all_fit` in argument.rs and wire value.rs: order is
  the same and a `false` still stops the walk before a later failure. The
  record length guard and the tuple Absent/Null to false behaviour are kept
  (clean).
- `membership_failure_code` / `_cause`: Allocation/Capacity map to
  resource_exhausted with allocation-failed / reservation-size-overflow,
  the same causes as `stage_failure` for environment assembly. Limit and
  Refused map without a panic (clean).
- The native-result writer, the CLI output Details/cause and the outcome
  stage handle `RunRefusal::Storage`. The reader accepts resource_exhausted
  at stage call (clean).
- The evaluate.rs:2033 field-option payload route returns unique, typed
  InternalFault identifiers. The old bool `admits` aborted on allocator
  failure, so this is no regression (clean).
- Non-test `ConstructionCause` / `ConstructionRefusal` consumers:
  evaluate.rs:1567-1571 and 1601-1605 (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tuple and record construction sites discard the ConstructionRefusal with `map_err(\|_\| ...)`. A new QSV StorageDenied/StorageCapacity therefore surfaces as `tuple-construction-rejected` / `record-construction-rejected`, which reads as an ill-typed construction. The PR's own evaluate.rs:2033 rule says an undecided admission is never read as rejected | qsl-eval/src/value/expression/evaluate.rs:1567-1571, qsl-eval/src/value/expression/evaluate.rs:1601-1605 |

## Verdict

**FAIL, 1 blocking finding (FND-001).** Everything else holds. The lock bump
is the only dependency change. All five `TypeEnvironment::admits` callers
send Allocation/Capacity to a typed storage variant and never to `false` or
WrongValueKind. Short-circuit order is preserved, the validate change keeps
existing refusals unchanged, and the gate log (make ci, 13:37 after the
11:57 commit, exit 0) is on the final head.

Fix: match `refusal.cause` at both construction sites. Map StorageDenied /
StorageCapacity to their own fault identifiers, as evaluate.rs:2033 does, and
keep `*-construction-rejected` for the ill-typed causes.

The following are recorded and do not block:
- The five new tests cover the mapping functions and conversions. The
  call-site `map_err` wiring has no end-to-end test because QSV's
  `ReservationPolicy` is private. Mutating a call site to `unwrap_or(false)`
  would go undetected.
- The new `field-option-payload-admission-*` fault identifiers have no
  FR-090 producer probes.
- The evaluator route is exit 30 (InternalFault) while the call-admission
  route is exit 22 (Incomplete). This is a documented limit of the kernel
  `Stop`.
- On the contract-broken Refused/Limit arms, a CLI command-error document
  gets runtime_invariant/stage_limit_exceeded with no cause and position
  details. The FR-100 reader rejects that at stage call. The path is
  unreachable per QSV's contract.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 278d1e170 |
