---
id: FR-530
title: "Carry and check liveness dependencies and fairness premises in native composition"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-031
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-489
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-348
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-380
    type: depends_on
---
# FR-530: Carry and check liveness dependencies and fairness premises in native composition

## Description

QSL SHALL carry QSpec FR-489's typed premise lists in a native composition
request over checked component packages, and SHALL record and check the
premise discharge record in the result, reusing the TL-9 assume-guarantee
algorithm (QSpec FR-176) and QSpec FR-348 and FR-380 for the rules and not
re-deriving them.

## Inputs

A composition request over checked component packages: the component subjects,
the selected rule, and the `environment`, `relies`, `liveness_dependencies`
and `fairness` lists of QSpec FR-489.

## Outputs

The composed result with its premise discharge record, or the distinct
outcome of QSpec FR-489's settlement table.

## Behavior

| Shape | Existing owner | Native addition |
| --- | --- | --- |
| Composition negotiation and rule selection | QSpec FR-348 | none |
| Assume-guarantee algorithm, fixed point, refinement | QSpec FR-176, FR-177 (TL-9) | none |
| Refinement transfer | QSpec FR-380 | none |
| Typed request carrying the four premise lists | none | the request type and its compile, read-back and admission |
| Premise discharge record | none | the record, the subject-and-strength check and its validation |

- Compile, read-back and negotiation of a composition request shall keep each
  premise list distinct and its source correspondence.
- The result shall record, for each required premise, the proved guarantee that
  discharges it, and shall refuse to discharge it from a guarantee whose
  subject differs, that is weaker, or that is of another kind.
- A mutually dependent progress pair shall not be accepted as self-justifying;
  a supported cycle shall require the rule's independently justified premises
  as recorded premises.
- Missing premise, trusted assumption, unsupported rule, failed check and
  resource exhaustion shall keep their distinct outcomes, trust basis and
  extent, and a result that only references another result shall not be
  evidence of validity.
- Admission shall reject a forged or mismatched premise.
- The request shall continue settling its unrelated supported claims.
- Backend execution stays with the TL-9 providers; Quoin consumes the results
  and does not validate the theorem anew.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-530-AC-1 | A request identifies the component subjects, the selected rule, the environmental assumptions, the relied-on peer guarantees, the liveness dependencies and the fairness premises as separate lists, and compile and read-back preserve each list and its source correspondence. | Test (TC-1330) |
| FR-530-AC-2 | The result records which proved guarantee discharges each required premise; a mismatched subject, a weaker guarantee and a guarantee of another kind cannot discharge it. | Test (TC-1330) |
| FR-530-AC-3 | A two-component fixture composes under a supported checked rule; a mutually dependent progress pair does not self-justify; a supported cycle needs the rule's independently justified premises. | Test (TC-1330) |
| FR-530-AC-4 | Missing premise, trusted assumption, unsupported rule, failed check and resource exhaustion remain distinct with their trust basis and extent, and a referenced result alone is not accepted as evidence. | Test (TC-1330) |
| FR-530-AC-5 | Compile, read-back, negotiation and result tests reject a forged or mismatched premise while unrelated supported claims continue. | Test (TC-1330) |

## Dependencies

- QSpec FR-489, FR-348, FR-176, FR-177 and FR-380.
- This requirement adds no parallel evidence store and no theorem import.

## References

- Linear QSL-1542; TL-9.
