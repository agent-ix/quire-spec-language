---
id: SR-17223
title: "code-review (with rust-review lane) of QSL-1081 capture emission and observation (quire-spec-language PR 1554)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@dc5789a5f3435e744ecea1089f0db734677bc2bc; git diff origin/main...dc5789a5f (PR 1554, branch qsl-1081-capture-code): qsl-semantics/src/check/{mod,assemble,refusal,temporal_clause}.rs, qsl-semantics/src/check/temporal_clause/condition.rs, qsl-semantics/src/check/lowering/temporal.rs, qsl-eval/src/{temporal,simulation/model,model_subject.rs,value}, qsl-analyze/src/model_check/{check,liveness}.rs, qsl-replay/src/{certificate/closure.rs,model_check}, and their tests; ticket QSL-1081"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
---

# Code review of QSL-1081 capture emission and observation (quire-spec-language PR 1554)

## Summary

Ticket: QSL-1081. PR: quire-spec-language#1554, head dc5789a5f3435e744ecea1089f0db734677bc2bc. One code-review pass with the rust-review lane over the whole diff (29 files, +1633/-173), read against the merged FR-337 (AC-8 to AC-14), TC-1267, FR-218-AC-16 row 23a, FR-125, FR-126, FR-156, FR-338 and QSpec FR-370.

No wrong-verdict defect was found in the production code. Traced and found faithful to FR-337:

- S3 order. The guard is typed first, then each capture initializer in source order, then the `holds` conditions (`check_conditions`). A capture refusal located after the binder reroutes through `guard_refusal`, so a defect in the guard and a capture refuses at the guard. Slot counts are `1 + e + j` for capture `j` and `1 + e + k` for a `holds` condition. The trigger keeps its slot in a `holds` condition under the unwritable name " trigger", so `q` stays out of scope there. A capture sees `c`, `q` and earlier captures only, and the guard sees `c` and `q` only.
- S4. The captures aggregate holds one named binding per capture in source order. Each capture gets its own `value`/`parameter` node at level `first_capture + j`, typed by its declared type, with `expression` and `type` occurrences at the capture's name. `holds` reads of a capture lower to a reference to that node. The `requirements` call now receives every parameter node of the clause.
- Engine. Captures are fixed once, only where the guard read true (or at position 0 for `on origin`, or with no guard), after the guard and before the atoms, in the evaluator, the model-check monitor (`read`) and the lasso reader (`reading_at`, cache key includes the origin flag). An undefined capture ends the run with `UndefinedEvaluation` located at `UndefinedLocus::Capture(name)`; where the guard is false no capture is read.
- CaptureRead ordering. `refuse_capture_read` runs after `prepare`, the admission `ModelSystem::new`, `check_model_bounds` (RequiresBound) and the instance resolution in all three routes (`check_model`, `check_protocol`, `check_reduced`) and in both liveness entries, and before any state is expanded. `captured_read` considers only `holds` conditions, computed from the slot arithmetic (slots at or after `first + count` are condition-local), so an initializer reading an earlier capture is not a `holds` read.
- Footprint and POR. `activation_reads` joins the guard's and each initializer's reads with the binders `[over, q, earlier captures]`, and the `guarded` flag of the ample-set `Atoms` is true for a clause with a capture.
- FR-338. The closure checker evaluates the captures where the engine does (position 0 for `on origin`, every position for `on each`) and rejects `BadState` at the reached product state. Guarded clauses get no certificate (`certified = !guarded`), so ignoring the guard in `captured` is sound. A capture-reading clause that reaches the checker unbound faults closed (`evaluation-local-value-unbound`), not open.

Remaining findings are test-coverage gaps against FR-337's ACs, one ordering corner, and two low code points. The coder's known residuals are judged below.

Judgement of the coder's known residuals:

- AuthoredUndefined replay refusal (QSL-898 remainder): not acceptable to leave silent. `replay_model_trace` refuses every authored-clause `UndefinedEvaluation` (replay.rs `AuthoredUndefined`), so the AC-13 sentence "the replay reproduces UndefinedEvaluation at position 2 located at z" cannot pass. This is a pre-existing limitation, not caused by this PR, but AC-13 is merged text; FND-003 asks the lead to either track it (ticket) and amend AC-13 or land it.
- Capture type mismatch has no payload: acceptable. The spec says retain `capture`, `expected`, `actual` (AC-8 S3 bullet), but the refusal path reuses the existing initializer type mismatch, and its code, tag and locus are right. Not a finding.
- FR-339 BadState for an undefined capture not tested separately: acceptable only as part of FND-003 (no FR-339 test exists at all for AC-13).
- "No state charged" tested indirectly: acceptable. `max_states = 0` makes any exploration fail, and the test expects `CaptureRead`; that is a real check of "before exploration". It does not cover precedence against RequiresBound (FND-002).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-337 Captures at S3 / S4 / Capture observation | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-8 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-9 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-10 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-11 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-12 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-13 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-14 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| check_conditions / check_clause capture typing | qsl-semantics/src/check/temporal_clause/condition.rs:93-180, qsl-semantics/src/check/temporal_clause.rs:890-945 | examined |
| lowering of captures | qsl-semantics/src/check/lowering/temporal.rs:140-285 | examined |
| TemporalClauseNode::capture, captured_read | qsl-semantics/src/check/mod.rs:699-731 | examined |
| engine capture fixing (eval) | qsl-eval/src/temporal/engine.rs:585-690, qsl-eval/src/temporal/observed.rs | examined |
| ClauseLetters::captures, activation_reads | qsl-eval/src/simulation/model/letters.rs:323-420 | examined |
| model-check read, refuse_capture_read | qsl-analyze/src/model_check/check.rs:463-485, 2344-2430 | examined |
| lasso reading_at | qsl-analyze/src/model_check/liveness.rs:855-950 | examined |
| closure checker captured | qsl-replay/src/certificate/closure.rs:1172-1210 | examined |
| replay of an authored UndefinedEvaluation | qsl-replay/src/model_check/replay.rs:868-871 | context_only |
| tests: model_check_captures, lasso_replay, emit temporal_clause, protocol_requirement | qsl-analyze/tests/it, qsl-package/src/emit/tests, qsl-semantics/src/check | examined |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-337-AC-10's requirement half is untested: the only test over `requires temporal Due;` with a capture asserts the requirement's position and the absence of a temporal refusal, never the key. Nothing asserts that the node the protocol's `requirements` set holds for `Due` is the emitted clause node, that its key equals the clause key, is the same for the clause emitted twice, and differs from the key of the same clause without captures. The test is also tagged only `FR-218-AC-16`, so no `FR-337-AC-10` tag covers that half. The production change (the `requirements(...)` call now passes every clause parameter node) is exactly what could change that key, so it is unguarded. | qsl-semantics/src/check/protocol_requirement/tests.rs:198-229 | correct-requirement-no-evidence |
| FND-002 | medium | FR-337-AC-12's precedence sentence is untested: "a clause whose unit has an unbounded root returns RequiresBound, not CaptureRead". No test runs a capture-reading clause over an unbounded subject. The code order is right (check_model_bounds precedes refuse_capture_read), but a later reorder would pass every test. | qsl-analyze/tests/it/model_check_captures.rs:156-204 | correct-requirement-no-evidence |
| FND-003 | medium | The AC-13 halves that need the replay of an undefined capture and the FR-338/FR-339 BadState of the AC-11 `z` clause are not met. `replay_model_trace` refuses any authored-clause `UndefinedEvaluation` with `AuthoredUndefined`, so "the replay reproduces UndefinedEvaluation at position 2 located at z" fails, and no test runs FR-339's component checker. The closure test uses a different unguarded clause over a different start value and only the FR-338 half. Guarded clauses get no closure certificate, so "with when (c.value = 7) it is Holds and FR-338's checker accepts the ClosureCertificate" cannot be exercised either. The coder reports the replay residual (QSL-898 remainder); the PR needs a ticket for it and a decision on the AC text, otherwise AC-13 reads as satisfied by a test that proves a weaker claim. | qsl-replay/src/model_check/replay.rs:868-871 | correct-requirement-no-evidence |
| FND-004 | low | The capture path of the lasso reader is untested (a throwaway probe by the reviewer, not committed, ran `eventually holds(c.value = 3)` under infinite-trace with the AC-11 capture `z` and found the right behaviour: `UndefinedEvaluation` at position 2 for an unguarded and a `when (c.value = 2)` `on each`, none for `on origin`, none where the guard stays false; so this is a test gap, not a defect). All capture vectors are bounded `eventually[0,1]` or safety `always`; none is an infinite-trace `eventually` (liveness.rs). `reading_at` keys its cache on the origin flag, reads captures only when no earlier undefined reading exists, and feeds `Reading::Undefined { locus: Capture }`, and `product.captured != Never` changes the ample-set `guarded` flag. A wrong key or condition there gives a wrong verdict (cached reading reused at position 0 versus later) and no test would fail. | qsl-analyze/src/model_check/liveness.rs:855-950 | correct-requirement-no-evidence |
| FND-005 | low | FR-337-AC-14 is asserted weakly. The test checks that `activation_reads()` has one element with the capture and zero without; it does not check that the location is `Counter::value`, that its receiver is the `over` parameter, or that FR-126's partial-order run joins it (no POR run). A read of `q.value` that landed on the wrong receiver would pass. | qsl-analyze/tests/it/model_check_captures.rs:206-238 | correct-requirement-no-evidence |
| FND-006 | low | S3 order of capture-type resolution versus an earlier capture's initializer. `check_clause` resolves the type of every capture (and checks duplicate names) before any initializer is typed, and `refuses_after_activation` reroutes only guard refusals. A clause `capture a: Int[0, 9] = <ill-typed>; capture b: M::Absent = 1;` therefore refuses at `Absent`, not at `a`'s initializer, although FR-337 says captures are checked in source order. Same for a formula-level fault (an inverted interval) versus a capture initializer. Low: both are single refusals with correct codes, and the spec's own AC-8 vectors hold. | qsl-semantics/src/check/temporal_clause.rs:890-945, qsl-semantics/src/check/mod.rs:2865-2872 | implementation-bug-despite-evidence |
| FND-007 | low | `ModelLetters::captures` and `TemporalLetters::captures` default to `Ok(CaptureReading::Fixed)` and `captured_read` to `None`. A future implementor that forgets to override them silently ignores an undefined capture (fail-open), where the neighbouring `activation` has no default. The four existing production implementors are overridden; the risk is only for new ones. | qsl-eval/src/model_subject.rs:203-221, qsl-eval/src/temporal/letters.rs:114-120 | implementation-bug-despite-evidence |
| FND-008 | low | `CapturedAt` is defined twice with the same three variants, privately in qsl-eval engine.rs and `pub(super)` in qsl-analyze check.rs. One typed answer from the checked clause (for example a method on `CheckedTemporalClause`) would remove the drift risk between the engine, the monitor and the lasso reader. | qsl-eval/src/temporal/engine.rs:337, qsl-analyze/src/model_check/check.rs:283 | implementation-bug-despite-evidence |

## Verdict

PASS with findings: no high finding, production behaviour matches FR-337 on every path traced; three medium findings are missing tests or an unmet AC half (FND-001 to FND-003) and should be fixed in this PR or ticketed with an amended AC before merge.

Gate: `make ci` stops at the first red, ci-default-features, on `qsl-eval` `type_invariants::a_record_invariant_follows_only_object_reading_helpers`. That test also fails at the PR's merge base d905d880f and passes on origin/main (c8ac2dd69), where #1552 aligned its fixture, so it is not caused by this PR; the branch needs main merged or rebased. fmt and `clippy --workspace --all-targets -D warnings` pass; qsl-analyze (including model_check_captures), qsl-semantics, qsl-package, qsl-replay and qsl-forms tests pass.
