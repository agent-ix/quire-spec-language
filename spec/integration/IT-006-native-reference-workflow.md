---
id: IT-006
title: "Qualify the native source to reference-result pipeline"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: verifies
---
# IT-006: Qualify the native source to reference-result pipeline

## Objective

Execute healthy, violating, refused and incomplete parent/aggregate state cases
through actual native model admission, parsing, linking, checking, runtime input
construction, validation and reference evaluation. Preserve authored and original
source correspondence at every stage. This is a native API milestone in LC03;
[IT-002](IT-002-native-state-workflow.md) still requires the real compiled
ConfigVersion model and qualified existing backend in the full assignment.

## Target Integration

The compiler library consumes the real pinned Contract IR declaration/proof API
and the source-derived Rust native-rule-model producer qualified by IT-005.
The test invokes actual public compiler APIs in one Rust process. It does not
mock an IR environment or reconstruct a second expression interpreter in setup.
There is no network service or external producer process in this test.

## Preconditions

Record compiler revision, model/source digests, IR pin 690bde7, Rust 1.98.1 and
the adopted state semantics at e897f81. The LC03 spec-review gate is resolved
before implementation. Construct the positive model and source through the same
already qualified APIs as the negative controls; setup must succeed before the
stage under test is invoked. A missing implementation leaves a test planned.

## Inputs

The exact source-derived Node rule model, independently authored native parent
ordering, reachability and duplicate-preserving sequence predicates, selected
authored requirement/clause identities and explicit immutable snapshot artifacts.
Include complete acyclic parent populations, an order violation, a cycle, a
dangling parent, an incomplete population, a stale selected digest, wrong
invocation roles and insufficient evaluation work. Operation cases use exact
pre/post snapshots, the admitted step anchor and original frame permissions.

Expected truth, diagnostics and event/cost vectors are authored independently
of the implementation. No authored JSON expected-output table is executed as
the evaluator. The generated small-graph oracle computes mathematical closure
independently of the runtime's traversal implementation.

## Test Procedure

1. Admit the actual source-derived model and parse/link/check native clauses.
   - IT-006-SC-01: setup succeeds with exact model/source and authored bindings.
2. Construct and validate the healthy complete current snapshot.
   - IT-006-SC-02: validation returns a constructor-private context and evaluation returns true.
3. Run the independently altered parent-order and cycle cases.
   - IT-006-SC-03: expected true/false results distinguish ordering from one-or-more-edge reachability.
4. Run admitted dangling/stale/wrong-invocation inputs through validation.
   - IT-006-SC-04: the specified runtime refusal has native/model/input correspondence and no Boolean.
5. Run incomplete-population, exact/insufficient work and cancellation controls.
   - IT-006-SC-05: incomplete outcomes retain actual usage/events with no Boolean; retry has fresh budgets.
6. Run pre/post frames, captured parameter/result and removed-object cases.
   - IT-006-SC-06: evaluation follows the selected observation only after complete frame/delta validation.
7. Record commands, exact revisions, source/model/input references and stage outcomes.
   - IT-006-SC-07: no setup refusal, static proof, backend result or authored expectation substitutes for the requested reference observation.

## Expected Results

Every step's criterion is observed through the real library. Native completed
truth, observed activation, input assumptions, provenance and incompleteness
remain distinct. No full LC02/LC03 closure, portable method/result adoption,
backend parity or Quire integration is claimed from this milestone alone.

## Metadata

Priority: High. Status: planned. Local Rust Integration tests, one Cargo job and
one test thread; no additional agents, hosted dispatch or producer-language work.

## Dependencies

- [IT-005](IT-005-qualify-native-model-consumption.md) supplies admitted model/checker setup.
- [FR-007](../functional/FR-007-validate-runtime-inputs.md) validates snapshots and invocations.
- [FR-008](../functional/FR-008-evaluate-state-reference.md) owns reference execution.
- [FR-018](../functional/FR-018-construct-native-runtime-inputs.md) binds exact input bytes.
