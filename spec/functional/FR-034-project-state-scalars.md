---
id: FR-034
title: "Project validated context fields at their checked observations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: references
---
## Description

When state-scalar-ir/v1 is selected, the compiler shall project primitive context fields and their checked observations through the existing executable IR binder.

## Inputs

The selected target uses FR-033's shared catalog and command parsing. Public
target, read-origin and materialization-stop enums are non-exhaustive.

The existing native package and lowering limits; optional runtime input
materialization takes a constructor-private ValidatedContext from that exact
checked package, a caller-lowered work ceiling (hard maximum 100,000) and a
cancellation poll. Existing runtime validation remains responsible for exact
model/snapshot/invocation identity, population closure and operation frames.
As in native runtime validation, a true poll means cancel and a callback panic
unwinds. Provenance borrows immutable objects/artifacts rather than copying
their potentially large identity strings for every read.

## Outputs

The existing NativeProjection includes typed origins for direct declarations
and fields of the selected self object. Materialized Boolean/integer inputs
retain the projection, validated context, exact artifact reference, optional
object identity and arena location. These are local backend inputs, not a
portable evidence envelope or a generated execution result.
Materialization failures distinguish ContextMismatch, ResourceExhausted,
Cancelled and defensive InvalidCorrespondence, with completed work and a
boundary-specific explanation. Existing lowering failures retain their codes.

## Behavior

The compiler shall admit the integer-ir/v1 expression domain plus primitive fields of self and pre expressions over that domain.
The compiler shall retain separate pre/post read correspondences for the same declaration.
The compiler shall derive field types from checked model declarations and use deterministic aliases that cannot collide with model value names or other field aliases.
The compiler shall preserve linked field identity, model digest, source coordinates and native/IR observation correspondence.
The compiler shall count every traversed native node, including field receivers and pre/group wrappers, against the existing lowering limits.
If a receiver is not self wrapped only in groups or pre, or a field is nonprimitive, then the compiler shall refuse the complete projection.
The compiler shall preserve the existing Boolean and integer target contracts.
The standalone lower command shall accept the explicit state-scalar-ir/v1 target and emit its exact IR bytes.
When inputs are requested, the materializer shall accept only a context validated against the projection's exact in-memory checked package and selected authored clause.
The materializer shall read primitives from the selected snapshot or captured invocation without evaluating predicates or manufacturing values for unavailable data.
The materializer shall retain inputs in projection read order for the selected clause only.
If materialization reaches its work ceiling or is cancelled, then the materializer shall return a classified failure without partial inputs.
The materializer shall charge each inspected read, parameter, field and arena value against fresh per-call work.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-034-AC-1 | Actual IR readers bind ConfigVersion's unchanged-version rule with one bounded field declaration and distinct pre/post references; linked field/source/model identities, deterministic collision-free aliases and operand order survive. | Test |
| FR-034-AC-2 | Validated unchanged/changed ConfigVersion inputs materialize the actual pre/post integers with exact artifact/object/arena provenance; independent native execution returns true/false, and forbidden frame changes produce no validated context. | Test |
| FR-034-AC-3 | Direct state and captured Boolean/integer parameters retain their values and observations; another checked package is rejected, and only the selected clause's inputs are returned. | Test |
| FR-034-AC-4 | Unsupported receivers, nonprimitive fields and later unsupported clauses refuse atomically; exact and insufficient lowering/materialization budgets, cancellation and fresh retries behave as specified. | Test |
| FR-034-AC-5 | The actual standalone command exports the new target bytes for the concrete update fixture; prior targets retain their field/pre refusals and default behavior. | Test |

## Dependencies

- [FR-033](FR-033-lower-bounded-integer-ir.md): existing bounded integer IR domain.
- [FR-032](FR-032-realize-config-version-workflow.md): concrete update workflow.
- [FR-007](FR-007-validate-runtime-inputs.md): complete native runtime validation.
