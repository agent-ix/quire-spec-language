---
id: SR-1339
title: "Code review of quire-spec-language PR #640: out-of-range post-state value as a violation witness (QSL-634)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6a09ff93786ec1607cd08af6ccc79e54cf991c81; PR #640 diff against origin/main: qsl-eval/src/value/expression/evaluate.rs, qsl-replay/src/execute/state_clause.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/state_clause_replay.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
---
# Code review of quire-spec-language PR #640

## Summary

Ticket: QSL-634. Reviewed head 6a09ff93, against the ruling recorded on
QSL-634: inputs are refused, outputs are evidence. The Rust lane
(rust-review) is folded into this file. No build was run (the leader's merge
batch runs make ci).

**Soundness of the widening: sound for pre-state and arguments.**
`admit_parameters` and `admit_result` pass no witness sink. Pre snapshots,
`PreCall` snapshots and `Current`/invariant snapshots (`admit_populations`)
are admitted under `PostStateRange::Refuse`. A frame run's post snapshot is
admitted under `Refuse` (`post_self` false). `finish_populations` widens
only when that environment's own `out_of_range` is non-empty, which only
`Witness` can produce. Pre and post are finished separately, so a widened
post closure never widens the pre closure that `pre(..)` reads.
`widen_ranges` rebuilds name, presence, redefinition, supertypes,
composites and units faithfully (checked against quire-semantic-value
660a126 `FieldDeclaration`/`ObjectTypeDeclaration`). `run_clause` keeps
`ObservationLimits::default()`, so it still refuses.

**The two qsl-eval relaxations are not gated** on post-state witnessing
(FND-001). The settlement only covers clauses that complete a value
(FND-002), and sequence elements stay refused (FND-003).

## Verdict

Changes requested. The wrapping-debit path, the pre-state refusal and the
in-range path are correct and tested. FND-001 and FND-002 should be fixed or
ruled on before merge. FND-003 needs a ruling or a fix. FND-004 is a low
naming/typing point.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The evaluator relaxations apply to every evaluation, not only a witnessed post snapshot. The `Equality` arm compares any integer outside its node's static type by value, and `project` now builds `OptionValue::from_admitted` for any present optional slot with no check at all (not only an out-of-range integer: a wrong-kind value is accepted too). Before, both were `CheckedInvariant`/`invariant()` faults that caught an operand the checker's types do not admit, from any source. Now they pass silently in `run_clause`, function calls and every other path. Fix: gate both on the witnessed case (for example, check the integer against the field's declared range and only relax for `Value::Integer` in `project`), or keep the fault everywhere the observation was not admitted under `Witness`. | qsl-eval/src/value/expression/evaluate.rs:1288-1310, 1885-1895 |
| FND-002 | medium | A range violation settles a violation only when the clause completes a value (`value.is_some()`). The ruling makes the out-of-range post value itself the witness of the violation. A clause that passes the exact value through a typed kernel op that refuses or faults on it (a `Coerce` to the field's range refuses `IntegerOutOfDomain`; any op still checking `admits` raises `CheckedInvariant`, which FR-122 turns into an `InternalFault` refusal) settles `inconclusive`/`NoValue` or refuses. CG then lands on Inconclusive, the outcome QSL-634 exists to remove. Fix: settle the violation whenever `range_violations` is non-empty, whatever the evaluation did. If the witness-arm settlement cannot hold a reproduced result with no evaluated value, get a ruling and say so in FR-122. | qsl-replay/src/execute/state_clause.rs:293-305 |
| FND-003 | medium | An out-of-range integer in a post-state sequence field is still refused `invalid_runtime_input` (`admit_scalar(.., None)` for collection elements). It is subject output just like a scalar field, so by the ruling it is evidence, not an input defect. The carve-out is a choice the coder made, not part of the ruling. Fix: witness elements as well (record the element index), or get the carve-out ruled. | qsl-semantics/src/model/observation/document.rs:1083-1089 |
| FND-004 | low | `OutOfRange::field` is a bare `String`, on a public type CG consumes. Under redefinition and supertypes a name does not say which declaration the range came from. A typed field reference (`FieldRef`, or the declaring type key plus name) carries that, and it matches how the evaluator already names fields. | qsl-semantics/src/model/observation.rs:118-128 |
