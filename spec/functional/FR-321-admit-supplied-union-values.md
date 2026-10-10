---
id: FR-321
title: "Represent union values in the kernel and admit supplied union values"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-319
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
---
# FR-321: Represent union values in the kernel and admit supplied union values

## Description

The kernel SHALL represent a union value as one `Value` variant carrying the
union node key, the active member's `VariantId` and the payload values
(ADR-013 §8 OQ-I, SC-Q1 option (a)); and when a caller supplies a union value
as a run argument or witness binding, QSL argument admission SHALL admit it
only when it is a well-formed value of a union of the evaluating package,
and otherwise SHALL refuse it before evaluation or any charge (ADR-012 §16.3
SC-R3).

## Inputs

- A supplied union value (a run argument or witness binding), admitted
  under QSpec FR-143, or a v2 `union_value` node (QSpec FR-440); the
  evaluating package's type environment and member index.

## Outputs

- An admitted kernel union value.
- Or `invalid_runtime_input`/`wrong-value-kind`, or the reference walk's
  existing refusal.

## Behavior

- The union's kernel type SHALL be `ValueType::Composite(union node key)`,
  with its member list in the layer-3 type environment, as for records.
- Argument admission SHALL refuse a supplied union value with
  `invalid_runtime_input`/`wrong-value-kind`, before evaluation or any
  charge, when:
  - its union node key is not a union of the evaluating package;
  - its `VariantId` is not a member of that union;
  - its payload count differs from the member's declared arity; or
  - a payload value is not admitted by the member's declared position type.
- Argument admission SHALL walk union payloads for nested references exactly
  as it walks record slots, so a dangling reference inside a payload is
  refused by the existing reference walk.
- A kernel union value SHALL convert to its v2 `union_value` node spelling
  (QSpec FR-440) and back without loss.
- Argument admission, conversion and kernel equality SHALL complete on union
  values of any nesting depth, stopping only at a caller-configured value
  limit of the run, which they report by name with its configured value; a
  host stack limit is never an outcome.

## Acceptance Criteria

Supplied admission SHALL use the distinct public `supplied.admission_work_units`
ceiling and caller-owned cumulative traversal budget of FR-277, retaining
the original Cancel across helpers and all arguments. Its membership/type
comparison units and invalid-stop order SHALL follow the reviewed QSV
FR-109 contract. Membership work SHALL NOT use the evaluator's work ceiling
or fabricate a FunctionCall charge. "Before any charge" in this requirement
means before any semantic evaluation charge; truthful helper spend up to a
refusal remains observable in its owning pre-call phase. Union-specific
units depend on the reviewed QSV declaration/member/type-comparison baseline,
not the released bool-only admission API or source-predicted totals.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-321-AC-1 | Running `area` with a supplied `Shape` argument refuses `invalid_runtime_input`/`wrong-value-kind`, with no evaluation and no charge, for each of: a union value whose key is a union of another package; a `VariantId` that is not a `Shape` member (another union's, or an enum's); `Rect` with one payload value; `Circle` with payload `true`. A `Rect(2, 3)` argument is admitted and the run returns 6. | Test (TC-829) |
| FR-321-AC-2 | Over a union `Holder { Some(Reference<M::T>), Nothing }`, a supplied `Holder::Some` whose reference has no target in a declared-complete population is refused by the reference walk with its existing code, before evaluation. | Test (TC-829) |
| FR-321-AC-3 | `Shape::Empty`, `Shape::Circle(4)` and a `Tree` value of depth 3 each convert to their v2 `union_value` spelling and back to an equal kernel value. | Test (TC-830) |
| FR-321-AC-4 | A supplied `Tree` value 10,000 levels deep is admitted, converted and compared equal to itself under the default run limits without a host stack overflow. With the run's value-occurrence limit set one below the value's occurrence count, admission returns that limit by name with its configured value. | Test (TC-830) |
| FR-321-AC-5 | For TC-830's unchanged full Tree, semantic occurrence limits 29,998 and 29,999 respectively deny and admit. With occurrence and other bounds permitting progress, admission helper ceiling A-1 denies its actual next event and A admits, where A is the reviewed membership event sequence counted independently of the evaluator. Each pre-call failure has FR-277's typed phase payload and zero evaluation consumption. Multi-argument controls retain cumulative helper spend and the original caller Cancel. | Test |

## Dependencies

- QSpec FR-143 (union values, their runtime admission and the foreign-key
  refusal), FR-440 (`union_value` nodes), FR-441 (member key).
- ADR-013 §8 OQ-I (SC-Q1), O-13, C-07.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.3 SC-R3, §16.4 S6a row,
  §16.11.
- Union value spelling (SC-G2): QSpec FR-440 `union_value` nodes; runtime
  admission QSpec FR-143 (specification tickets STD-142, STD-115).
