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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | An out-of-type update value is now written to the history, and the post-state rows are evaluated over it. A row that reads the failed field and is undefined at the computed value refutes the step. Merged FR-138 and FR-141 "Undetermined history update" say such a row is an undetermined read, never evaluated. Probe: `writes` = pre(self.writes) + 2 with `prev` reading `1 mod (2 - self.writes)`: `write(r, 1)` returns Undefined at `prev`, where the spec requires Undetermined naming `writes`. | qsl-replay/src/refinement/history.rs:288-296; qsl-replay/src/refinement/step.rs:344-369 |
| FND-007 | medium | An update with no value (undefined, refused or incomplete) now skips the whole post-state mapping, so an undefined row that does not read the failed field no longer wins. Merged FR-141 skips only rows that read that field, and "an undefined read anywhere in the order ... wins". Probe: Reads with an over-budget `H` and an undefined `items` (which does not read `h`) returns Undetermined naming `H` instead of Undefined at `items`. Mutation M21 (map despite no value) survives. The coder's "the row cannot be read" holds only for rows that read the field; the spec is right. | qsl-replay/src/refinement/step.rs:344-369 |
| FND-008 | high | The certificate checker merged from QSL-549 carries no history values: `CertifiedPosition` has only `concrete` and `abstract_states`, `matches` compares only those, and its module doc still says "HistoryValues are not carried, because history fields are refused (FR-138)", which is false once this PR lands. Two product states that differ only in history collapse to one listed position, so the checker can accept a certificate that never checks the second history variant. FR-149 lists `history: HistoryValues` in a position. Either carry and compare the history, or refuse history refinements in the checker. Not reproduced. | qsl-replay/src/certificate/simulation.rs:31-32, 66-72, 466-476 |
| FND-009 | low | The `any` arm returns Undetermined for any ContractUndetermined finding in a member's expansion. Merged FR-141 restricts this to an undecided successor "whose visible part equals the mapped one". The code over-reports inconclusive (sound but off spec), the finding's `candidate` digest is not compared, and mutation M19 survives, so no test covers the rule. | qsl-replay/src/refinement/step.rs:436-443 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | judge_operation and the any arm return Undetermined(UndecidedSuccessor) for an undecided member with hidden fields; the round-0 probe now gives Undetermined; M18 is killed (any-arm restriction: FND-009) |
| FND-002 | fixed | the stand-in is gone; the round-0 probe (FR-138-AC-8's case) gives Undetermined naming `writes`; the replacement behaviour has its own defects, FND-006 and FND-007 |
| FND-003 | fixed | a hidden-field row whose set comes out empty returns NoAbstractMatch; M20 is killed |
| FND-004 | still-open | the Fault is now an S3 refusal, but it names a fifth feature `history-value-key`; merged FR-135 and QSpec FR-375 define a closed set of four names, and this build must hold `history-row` and omit the other three. Enum literals are FR-375-AC-9-legal. Needs a spec ruling (add the name to FR-375/FR-135, refuse by another code, or encode enum/decimal in FR-181) |
| FND-005 | fixed | the conflict check now covers subtypes' declared fields; M23 is killed |
| FND-004 | deferred | spec follow-up on QSL-545 (no new Linear issue can be filed): the refusal now uses the closed-set name `history-row` with a new `subject: enum/decimal` payload member, but FR-135 says this build holds `history-row`, and FR-138 / FR-375-AC-9 make enum literals legal; FR-135/FR-138 must state the partial support and the payload member |
| FND-006 | fixed | an out-of-type value is no longer written and its field joins `failed`; rows that read it are not evaluated; probe 3 returns Undetermined; M24 and M21 are killed |
| FND-007 | fixed | only rows reading a failed field are skipped; probe 4 returns Undefined at `items`; M27 (skip every row) is killed |
| FND-008 | fixed | `CertifiedPosition.history` is filled by the producer and compared in `matches`; a_certificate_position_carries_its_history; M25 is killed |
| FND-009 | fixed | the any arm counts an undecided postcondition only for a candidate digest among the mapped completions; M26 is killed (the positive half is untested: SR-13461 FND-010) |
| FND-010 | fixed | `failed` is keyed by (object, field) and a row of another object is evaluated; a_failed_update_skips_the_rows_of_its_own_object_only; the name-only mutant (M28) is killed. The stated reference-reach limit is a new defect: FND-011 |
| FND-011 | fixed | advance drops failed (object, field) entries from the post history, and the evaluator's unresolved History read is a typed refusal, so any row reaching the field, of any object and by any path, is undetermined; the row-skip machinery is removed; a_row_reaching_a_failed_field_through_a_population_is_undetermined (probe 5); M29, M30 and M31 are killed |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | `map_object` skips a row when `row.expression.reads_history(name)` matches by field name alone, for any object. A row that reads the same-named field of an object whose update did not fail is also skipped. When the failed update is undetermined and that row would be undefined, the merged FR-141 rule says Undefined wins, but the code returns Undetermined: a missed refutation, never a wrong one. | qsl-replay/src/refinement/map.rs:431-434; qsl-semantics/src/check/refinement.rs:241-247 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | high | A row of another object that reaches a failed history field (through a reference or a population read) is evaluated at the stand-in: the failed field "holds the value the step started from", and the object-keyed skip matches only the row's own object. That is FND-002's wrong refutation on a cross-object path. Probe: Reads with `h` out of type on `s0` and the ring's `items` row reading `s.h` through `only(s in R::slots ...)` as `1 mod (0 - s.h)`: undefined at the stand-in 0, defined at the computed 2. `put(1)` returns Undefined at `items`; merged FR-138/FR-141 require Undetermined naming `h`. Fix: drop the failed (object, field) entries from the post history environment, and make the evaluator's unresolved History read an undetermined outcome rather than the `history-value-unresolved` invariant fault. That decides per read, for any path. | qsl-replay/src/refinement/map.rs:430-439; qsl-replay/src/refinement/history.rs:159-161; qsl-eval/src/value/expression/evaluate.rs:1813-1822 |
