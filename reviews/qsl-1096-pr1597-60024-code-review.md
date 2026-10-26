---
id: SR-60024
title: "code-review (with rust-review lane) of QSL-1096 replay of an authored clause's undefined evaluation (quire-spec-language PR 1597)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@325f2f32b27f1ae22554d332504353f4ca270be6; git diff origin/main...325f2f32b27f1ae22554d332504353f4ca270be6 (PR 1597, branch qsl-fr488-residuals): qsl-replay/src/model_check/replay.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/tests/it/{lasso_replay,model_check_captures,model_world,partial_order,reduction_differential,analyze}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
---

# Code review of QSL-1096 authored undefined-evaluation replay (PR 1597)

## Summary

Ticket: QSL-1096 (QSL-1092 decided, spec only; QSL-1094 untouched). Method: code-review with the rust-review lane over `git diff origin/main...325f2f32b27f1ae22554d332504353f4ca270be6`. The replay now runs the FR-125 evaluator over the replayed positions with letters that record the locus of the first undefined reading, and `undefined()` compares that position, locus and cause with the payload. `ModelTraceRefusal::AuthoredUndefined` is removed with no remaining reference outside historical reviews. The replay is fail-closed: a mismatch of position, locus or cause settles inconclusive, an evaluator verdict or meter stop gives no value, and an evaluator/letter disagreement is a fault.

Own mutations (each reverted; tests lasso_replay, model_check_captures, model_settle, 30 tests):

- Compare ignoring `locus`: 3 tests fail.
- Compare ignoring the `Capture` name: `replay_reproduces_an_undefined_capture_located_at_it` fails.
- Capture locus recorded as `Activation`: the same test fails.
- Contract conjunctions read before the letter at one position: all 30 pass (FND-001).
- Drop the evaluator-position fault guard: all 30 pass (defensive, no finding).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| undefined_letter, ReplayedLetters.note | qsl-replay/src/model_check/replay.rs:1220-1510 | examined |
| undefined, ItemReads | qsl-replay/src/model_check/replay.rs:1809-1940 | examined |
| replay_protocol_trace and settle call sites | qsl-replay/src/model_check/replay.rs:700-740, 1020-1040 | examined |
| engine letter and constraint order | qsl-analyze/src/model_check/check.rs:2205-2260, 2340-2450 | context_only |
| analyze.rs, partial_order.rs, reduction_differential.rs comments and exemptions | qsl-analyze/tests/it | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new read order (an authored clause's letter before the contract conjunctions of the same position) is asserted by FR-128 but no test fails if it is reversed. Mutation: in `undefined()` take the expansion's contract finding before the letter; all 30 tests of lasso_replay, model_check_captures and model_settle pass. No fixture has the letter and a contract conjunction both undefined at one position, so a payload naming the contract there (which the engine never emits) is untested as inconclusive. Add that case (the AC-6 `CanInc` subject with an atom undefined at value 2, payload located at `CanInc` then at the atom). | qsl-replay/src/model_check/replay.rs:1899-1935 |
| FND-002 | medium | Within one position the replay reads a state constraint before the letter, the engine reads the letter first: `create` calls `advance` (the letter) and reads the constraint only `if bad.is_none()`. For an authored clause, a payload located at `Constraint(name)` at a position where the clause's letter is also undefined reproduces, though the engine would have reported the letter. Traced by reading, not run. FR-128 states the constraint is read when the state is reached; say where it falls relative to the letter and make the replay and engine agree, with a test. | qsl-replay/src/model_check/replay.rs:1878-1897; qsl-analyze/src/model_check/check.rs:2227-2252 |
| FND-003 | medium | The PR says the authored replay also serves `replay_protocol_trace` (a new `ItemReads::Authored` arm and `undefined_letter` call in `ProtocolReplay`), but no test replays a protocol clause's undefined evaluation; the only protocol `UndefinedEvaluation` test is the deadlock item. The arm is unexercised. | qsl-replay/src/model_check/replay.rs:713-731; qsl-analyze/tests/it/protocol_system.rs:1836 |
| FND-004 | low | Stale comments say `replay_model_trace` refuses an authored clause: analyze.rs lines 9-11 and 22-23 (module header, "Open" notes for FR-281-AC-2 and AC-7), and 311-317 (the FR-281-AC-7 test doc, which still builds the replay result by hand for that reason). They are false after this PR. The AC-2 and AC-7 "Open" items may now be closable by a real replay; at minimum the text must be corrected in this PR. | qsl-analyze/tests/it/analyze.rs:9, 23, 312 |
| FND-005 | low | `partial_order.rs` `vector` and `reduction_differential.rs` still skip replay for an `UndefinedEvaluation` counterexample ("held to its prefix"), though FR-160-AC-6 says every counterexample from either run replays and replay now exists. The edit only reworded the comments. | qsl-analyze/tests/it/partial_order.rs:930-940; qsl-analyze/tests/it/reduction_differential.rs:321-324 |
| FND-006 | low | `replay_reproduces_an_undefined_capture_located_at_it` asserts `Inconclusive(_)` for the altered payloads; FR-337-AC-13 and TC-1267 step 7 name `ReplayParity`. An inconclusive of another cause passes. The lasso test checks `DisagreementCause::Verdicts`; do the same here. | qsl-analyze/tests/it/model_check_captures.rs:487-491 |
| FND-007 | low | `model_refusal` groups `CaptureRead` under "The forms and subjects EN-1 does not decide yet" and returns `Unsupported(NOT_YET)`, so a standing decision is reported on the wire as `not-yet-implemented`. Either keep it and say so in the spec, or give the standing refusal its own cause. | qsl-analyze/src/analyze/items.rs:191-199 |

## Verdict

Mergeable after FND-001 to FND-003 (medium) are fixed or ticketed with a reason; FND-004 to FND-007 are low and FND-004 should be fixed in this PR. No panic, unsafe, unchecked integer conversion or string comparison was added. Gate: `make ci` exit 0 at this head.
