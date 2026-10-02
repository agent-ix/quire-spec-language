---
id: FR-268
title: "Re-derive and separation-check a state-clause witness on replay"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-265
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-269
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
---
# FR-268: Re-derive and separation-check a state-clause witness on replay

## Description

A state-clause counterexample (FR-122) SHALL carry the clause's separating
witness record, and FR-122's replay SHALL re-derive that record and check
on its own terms that the named element separates the clause from the other
truth value, as ADR-031 SW-8 and SW-12 to SW-14 decide. Replay never trusts
the search that found the element.

## Inputs

- A `WitnessEnvelope<StateClauseCounterexample>` (FR-070, FR-122) whose
  payload gains `witness: Option<SeparatingWitnessRecord>` (FR-265's
  record), present exactly when the producing run's basis was decisive.
  The envelope itself is unchanged (FR-070-AC-5).
- FR-098's request, byte provision and limits.

## Outputs

- FR-122's replay result. An agreeing result carries the re-derived record
  when the basis is decisive and no record otherwise (FR-072).
- A disagreement settles `inconclusive` with `DisagreementCause::Witness`
  (FR-269).

## Behavior

- When FR-122's single evaluation of the admitted clause completes, the
  executor SHALL derive its own basis and record by FR-265 from that
  evaluation.
- If the replayed verdict agrees with the proved one, then the executor SHALL
  compare the payload's `witness` with its re-derived record componentwise
  under QSpec FR-351 identity. The comparison includes the deciding
  quantifier and compares the deciding element under ADR-013 O-13
  semantic equality.
  - If both records are absent, then the executor SHALL settle FR-122's
    agreement for the envelope's arm.
  - If both records are present and equal and the separation check below
    passes, then the executor SHALL settle the agreement:
    `reproduced-with-evaluated-witness` on the `Witness` arm,
    `reproduced-without-witness` on the `Input` arm.
  - If the two records differ or only one is present, then the executor SHALL
    settle `inconclusive` with `DisagreementCause::Witness` carrying the
    payload's record and the re-derived one, each as given or absent.
- The executor SHALL run the separation check over the admitted
  observation, charged to the request's evaluation meter, in these steps:
  1. resolve the record's deciding quantifier in the recompiled package and
     require it to be a `forall` or `exists` occurrence inside the clause's
     claim;
  2. evaluate the quantifier's domain expression in the clause's roots and
     the `let` bindings on the decision path;
  3. read the element at `index` and require its source location to equal
     the record's value path and its value to equal the deciding element;
  4. evaluate the quantifier's body once with the binder bound to that
     element, and require `false` for a `forall` and `true` for an
     `exists`.
  If any step fails, the executor SHALL settle `inconclusive` with
  `DisagreementCause::Witness`, `failure` `Separation` naming the step
  (FR-269). A step fails when its requirement is not met, or when the domain
  evaluation (step 2) or the body evaluation (step 4) ends `undefined` or
  refused. An `undefined` evaluation refutes the witness with cause
  `UndefinedEvaluation { where, cause }`, naming the expression and its
  undefined cause; a refused evaluation carries its refusal record.
- The payload's encoded `witness` SHALL count against the envelope's
  configured reader bound (FR-070-AC-7). An exhausted meter during any step
  of the separation check SHALL settle `inconclusive` with cause `NoValue`,
  as an exhausted evaluation does in FR-122.
- Replay SHALL give the same result for the same request and envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-268-AC-1 | Over FR-265's `witness` unit, an envelope for `AllBelow` over `high` whose payload carries FR-265-AC-1's record settles `reproduced-with-evaluated-witness` on the `Witness` arm and holds that record; the same payload on the `Input` arm settles `reproduced-without-witness`. An envelope for `NestedAll` over `high` carrying FR-265-AC-4's record, and one for `FilteredAll` carrying FR-265-AC-5's, each settle `reproduced-with-evaluated-witness`. | Test (TC-743) |
| FR-268-AC-2 | Re-derivation mismatch: the `AllBelow` over `high` envelope whose record names the equal element 600 at index 2, after the stop, one whose record names the `forall` of `GuardedAll`, and one with no record each settle `inconclusive`, `Witness`, holding the payload's record (or its absence) and the re-derived one (or its absence). | Test (TC-743) |
| FR-268-AC-3 | Separation check, called on its own over the admitted `high` observation and `AllBelow`: FR-265-AC-1's record passes; a record whose deciding quantifier names a node outside the clause's claim, one whose index is 3 (past the domain's end), one whose value path names another member, one whose deciding element is 700 at index 1, and one naming 0 at index 0 (whose body is `true`) each fail, naming the failing step (1, 3, 3, 3 and 4 respectively), each as the `DisagreementCause::Witness` step FR-269 carries. Over variants of `AllBelow` whose domain expression evaluates `undefined` on `high`, whose body evaluates `undefined` for `mid`, and whose domain evaluation is refused (a `count` into `Int[0, 0]` of a one-element collection), the check fails at step 2 with `UndefinedEvaluation`, at step 4 with `UndefinedEvaluation`, and at step 2 with the refusal record, respectively; with the evaluation meter set one unit below step 2's need it settles `NoValue`. | Test (TC-743) |
| FR-268-AC-4 | The FR-122 envelopes whose clauses reach no decisive occurrence (`VersionUnchanged` over changed-version and `ParentOrder` over violating-parent) carry no record and settle as FR-122-AC-1 states, with no QSpec FR-351 record on the result. An `AllBelow` envelope whose encoded record exceeds the envelope's reader bound refuses at decode under FR-070-AC-7. Replaying the FR-268-AC-1 `Witness`-arm envelope twice gives equal results. | Test (TC-743) |

## Dependencies

- [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md) SW-8,
  SW-12 to SW-14.
- [FR-122](FR-122-replay-a-state-clause-counterexample.md) (the replay this
  extends), [FR-265](FR-265-derive-a-state-clause-separating-witness.md)
  (the derivation replay shares with the producer),
  [FR-269](FR-269-settle-a-witness-disagreement-as-a-typed-cause.md) (the
  cause), [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md)
  (the envelope and reader bound), [FR-072](FR-072-implement-typed-replay-result.md)
  (the result).
- QSpec FR-351 (record identity).

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
- QSpec half (Linear STD-144): QSpec FR-351-AC-7 and QSpec FR-352-AC-7. Replay consumer:
  agent-ix/quire-contract-codegen#50.
