---
id: SR-13460
title: 'Code review of PR #1093: refinement history fields and AX-2 candidate sets (code-review with rust-review lane)'
type: SpecReview
scope: 'agent-ix/quire-spec-language@498b438cf; git diff refs/remotes/origin/main...498b438cf (PR #1093, branch qsl-545-history): qsl-replay/src/refinement/{step,history,map}.rs, qsl-replay/src/model_check/refinement_replay.rs, qsl-analyze/src/model_check/{refine,check}.rs, qsl-eval/src/simulation/model/refine.rs, qsl-eval/src/value/expression/{mod,evaluate}.rs, qsl-semantics/src/check/{check,refinement,refinement/history,state_model,step_simulation,ir,lowering}.rs, qsl-semantics/src/model/object_environment.rs, qsl-forms/src/refinement.rs and the tests; measured against QSpec FR-377/FR-378 at /Users/peter/dev/quire-specification ad2b37e; ticket QSL-545'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-138
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-141
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-142
  type: reviews
---

## Summary

Ticket: QSL-545. PR: quire-spec-language#1093, head 498b438cf. Method: code-review with the rust-review lane. Evidence on head 498b438cf (scratch clone, own CARGO_TARGET_DIR): `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics` exit 0 (3577 passed, 0 failed); root `cargo test --test it` exit 0 (975 passed, 6 ignored); `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `make string-edge`, `make arch-lint-api-surface-qsl`, `make arch-lint-canonical-encoder` and `make arch-lint-qualified-core` each exit 0. Two scratch probe tests (reverted after) reproduce FND-001 and FND-002. No HiddenFields, UnsupportedFeature::HistoryRow or `history-row` remnant is left in the Rust tree, and no shim.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| judge_operation | qsl-replay/src/refinement/step.rs | examined |
| decide_step (Any arm, read order) | qsl-replay/src/refinement/step.rs | examined |
| completions | qsl-replay/src/refinement/step.rs | examined |
| check_initial | qsl-replay/src/refinement/step.rs | examined |
| advance / carried / step_history | qsl-replay/src/refinement/history.rs | examined |
| HistoryValues::key | qsl-replay/src/refinement/history.rs | examined |
| map_state history input | qsl-replay/src/refinement/map.rs | examined |
| Run::create product key | qsl-analyze/src/model_check/refine.rs | examined |
| history_headers / history_rows | qsl-semantics/src/check/refinement/history.rs | examined |
| Typer History lookup | qsl-semantics/src/check/check.rs | examined |
| step_simulation_records hook | qsl-semantics/src/check/step_simulation.rs | examined |
| evaluate_history_update | qsl-eval/src/value/expression/mod.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AX-2: an undecided completion or successor is dropped from the candidate set when another is accepted, so a later step can refute on a set that lost a member. Probe: abstract `toss` post `self.tossed and 1 mod self.side = 0` makes side 0 undecided and side 1 accepted; the set after `toss` is [1], and `reveal` to face 0 returns Fails(AbstractStepRejected) where the honest verdict is Undetermined(UndecidedSuccessor). | qsl-replay/src/refinement/step.rs:786-811, 428-440 |
| FND-002 | high | A failed history update keeps the pre-state value as a stand-in, and the post-state rows are mapped over it; an undefined row at the stand-in then outranks the update's undetermined outcome. Probe: `writes` = pre(self.writes) + 2 (out of type) and `prev` reading `1 mod (1 - self.value + self.writes)`: `write(r, 1)` returns Undefined at `prev` (a refutation), though the row is defined at the computed value 2 and the update alone settles the step Undetermined. | qsl-replay/src/refinement/history.rs:156-163, 299-301; qsl-replay/src/refinement/step.rs:333-366 |
| FND-003 | medium | An abstract-operation row whose post candidate set is empty with hidden fields returns AbstractStepRejected (first by transition key, then enumeration order) instead of NoAbstractMatch, which FR-141 ("for any row whose post candidate set is empty") and QSpec FR-378 ("A step after which the set is empty fails with NoAbstractMatch") both state. | qsl-replay/src/refinement/step.rs:812-828 |
| FND-004 | low | S3 admits Enum and Decimal history types (`bounded`), whose values FR-181's encoder cannot key, so a well-formed refinement reaches a product Fault (settles failed) instead of an S3 unsupported refusal. | qsl-semantics/src/check/refinement/history.rs:73-93; qsl-eval/src/simulation/model/encode.rs:257-261 |
| FND-005 | low | The typer resolves a history field before the object's own fields, and the S3 conflict check reads only the type and its ancestors (`types.attribute(object.id, name)`), so a subtype that declares a field with the history field's name has that field shadowed in every refinement row. | qsl-semantics/src/check/check.rs:2735-2750; qsl-semantics/src/check/refinement/history.rs:125-144 |

## Verdict

Not mergeable: FND-001 and FND-002 each give a wrong `refuted` verdict, and both were reproduced. Correct: the history row checks (type, bounds, static literal, conflicts, duplicate updates, typing with `pre()`), update evaluation through one evaluator with a fresh meter and pre/post observations, receiver-only writes, the read order of receiver, arguments, history and post rows, the product key (concrete, history, sorted candidates; deterministic BTreeMap order), completion enumeration (odometer over FR-120 domain order, optional unset first, candidate bound), the union of accepted completions, the FR-146 hook, and the complete deletion of the HiddenFields refusal with no shim. Rust lane: idioms match the repo (a `reason`-carrying `#[allow(clippy::result_large_err)]`, `#[must_use]` builders, no unwrap on production paths, saturating counters). Mutation probes in SR-13461.
