---
id: FR-023
title: "Run native packages with retained request provenance"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: references
---
# FR-023: Run native packages with retained request provenance

## Description

When a caller executes a native package with supplied runtime artifacts and an authored selection, the runtime shall validate and evaluate that request while retaining its exact provenance in the returned report.

## Inputs

An immutable NativePackage, owned RuntimeInput, exact ExecutionSelection,
caller-lowered validation and evaluation limits, and a cancellation poll.

## Outputs

An immutable native report borrowing the package and owning the original offered
input and selection, including on failed validation. Its outcome is either the
original failed ValidationReport or the actual EvaluationOutcome with measured
usage, ordered implication events and reference cost-model version.

## Behavior

The runtime shall call the existing validator before executing the predicate.
If validation fails, then the runtime shall preserve its status, detailed
diagnostics, terminal reason and usage without starting evaluation.
When validation succeeds, the runtime shall execute the existing evaluator once.
The runtime shall return a Boolean only for a completed evaluation.
The runtime shall retain the supplied package, input bytes and authored selection
on every returned outcome without substituting diagnostic or model ownership.
The runtime shall retain actual validation and evaluation counters separately.
The runtime shall forward one cancellation poll through both stages and start
fresh stage budgets on each call.

Caller poll panics unwind as in the existing APIs and do not manufacture a report.
This in-process Rust API does not define B's portable result envelope, serialize
an attestation, or qualify a backend or C's Quire extraction. Input construction
remains a separate fallible step. Existing validate/evaluate callers retain their APIs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-023-AC-1 | Healthy and violating parent/aggregate requests return true and false respectively with their exact package, original input bytes and authored selection retained. | Test |
| FR-023-AC-2 | Invalid inputs or foreign selections retain the original validation classification, details, terminal reason and usage, with no evaluation result or Boolean. | Test |
| FR-023-AC-3 | Validation and evaluation exhaustion or cancellation remain distinguishable, retain actual usage/event prefixes, and fresh retries can complete. | Test |
| FR-023-AC-4 | An operation request retains exact invocation and pre/post inputs while actual capture/frame semantics determine completion or refusal. | Test |

## Dependencies

- [FR-007](FR-007-validate-runtime-inputs.md): existing model-aware validation.
- [FR-008](FR-008-evaluate-state-reference.md): existing native evaluation.
- [FR-019](FR-019-package-checked-native-clauses.md): immutable native package.
