---
id: FR-007
title: "Validate runtime snapshots and invocations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: depends_on
---
# FR-007: Validate runtime snapshots and invocations

## Description

When runtime validation is requested, the compiler runtime shall establish the selected clause's immutable input context before predicate evaluation.

## Inputs

A borrowed CheckedPackage, owned RuntimeInput inventory, exact authored
RequirementRef/ClauseId and current-self or invocation selection, ValidationLimits
and a cancellation poll. Snapshot/Invocation artifacts come from
[FR-018](FR-018-construct-native-runtime-inputs.md). Exact fields, role-specific
references and error rules are defined in
[the native input contract](../../docs/native-runtime-inputs.md).

## Outputs

A constructor-private ValidatedContext borrowing the checked package and owning
the immutable inventory/indexes, or a report retaining classified diagnostics
and usage with no validated context. The report distinguishes Refused from
Incomplete; neither carries a predicate Boolean.

## Behavior

Validation checks exact model/artifact/clause/operation correspondence, required
observations, all supplied selected values and transitive population closure.
It checks known invalid data even when other required data is unavailable.
The completeness flag is an input assumption, not a deployment guarantee.

Invariants select current. A recorded operation supplies both pre and post for
frame/delta validation, including when selecting a precondition. Self is required
in pre and additionally in post for a postcondition. Parameters capture pre and
the declared result captures post. Retrospective validation does not authorize
a future mutation. Runtime input cannot enlarge the admitted model's frame.

The frame compares every surviving object's fields using storage equality,
including options and ordered duplicate-preserving sequences. The current frame
has no State-root write permission: corresponding supplied/required State roots
must be preserved across pre/post. This closes the existing object-only frame;
a future permission for root replacement requires a reviewed model extension.

Malformed/stale input refuses; missing observations, incomplete populations,
work exhaustion and cancellation are incomplete unless a known invalid defect
also exists. Every individual diagnostic keeps its own classification.
Diagnostic detail capacity and a separate terminal stop reason follow
[NFR-006](../non-functional/NFR-006-bound-native-runtime.md).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-007-AC-1 | A dangling target in a complete universe receives dangling_reference. | Test |
| FR-007-AC-2 | An incomplete required population receives incomplete_population without inferring missing targets to be dangling. | Test |
| FR-007-AC-3 | Mismatched invocation anchors, selected snapshot roles or pre/post correspondence receive wrong_snapshot. | Test |
| FR-007-AC-4 | An unauthorized changed object field or created/deleted object type receives frame_violation. | Test |
| FR-007-AC-5 | Failed validation returns no ValidatedContext or predicate Boolean, including mixed invalid/unavailable inputs. | Test |
| FR-007-AC-6 | Foreign authored clauses, model owners, conflicting input identities and stale selected digests refuse without first-match or latest-revision fallback. | Test |
| FR-007-AC-7 | Every supplied selected object, State value, parameter and result satisfies exact fields, nominal types, scalar bounds, enum variants, option tags and sequence bounds, including skipped fields. | Test |
| FR-007-AC-8 | Duplicate object identities and wrong reference type/universe refuse; reference cycles terminate validation without merging equal-valued objects. | Test |
| FR-007-AC-9 | Missing required observations or State roots are incomplete; missing self in a complete required population refuses, with post self required only for postconditions. | Test |
| FR-007-AC-10 | Parameters/results match the selected operation exactly; a pre-captured reference can resolve an object deleted at post without being retagged. | Test |
| FR-007-AC-11 | Declared created/deleted sets are duplicate-free, disjoint and equal the actual complete population differences; wrong sets receive population_delta_mismatch. | Test |
| FR-007-AC-12 | Each validation ceiling stops before its next charged operation; cancellation returns the actual partial diagnostics and usage with no successful context. | Test |
| FR-007-AC-13 | With sufficient budget, inventory/population/field permutations preserve sorted diagnostics, including original native/model loci and exact structured runtime locations. | Test |
| FR-007-AC-14 | Frame storage equality preserves State-root values and detects option-tag, nested-record or sequence-order/multiplicity changes; missing counterpart roots are incomplete. | Test |
| FR-007-AC-15 | Repeated validation has fresh accounting and leaves the checked package/input bytes unchanged after success, refusal, exhaustion and cancellation. | Test |

## Dependencies

- [FR-015](FR-015-project-native-model-semantics.md) owns admitted model roles.
- [FR-016](FR-016-check-native-clauses.md) owns static judgments and input obligations.
- [US-003](../usecase/US-003-evaluate-bounded-state.md) traces to StR-001.
- [IT-006](../integration/IT-006-native-reference-workflow.md) exercises the native pipeline.

## Status

Validation is qualified at 45ed1b4 by SR-097: all fifteen acceptance criteria
have executed evidence in 35 public API tests. Reference execution is qualified
at 48f53ae by SR-098. Task-015 owns the complete native API review/handoff;
remaining LC03/backend/Quire issue-level acceptance is separate.
The adopted standard pin is e897f810a7356d4ce8fd19026221ebda7b65596f.
Remaining LC02/FS03 issue acceptance is separate from the landed checker API.
