---
id: TC-126
title: "Preserve exact predicate meaning at cross-family calls"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: verifies
---
# TC-126: Preserve exact predicate meaning at cross-family calls

## Description

Verify that the real native pipeline preserves a checked predicate's identity
and explicit environment across state, temporal and protocol call sites under
[FR-046](../functional/FR-046-execute-predicates-and-ordered-queries.md).
Existing binding/type/emission controls are candidate evidence, not automatic
proof of the complete stage.

## Test Procedure

Construct an acyclic typed Boolean predicate with immutable explicit arguments
and call it from each family in a multi-unit package. Run native binding,
checking, definedness and admitted emission; inspect the callee identity, each
call site's original source span, parameter order, type/unit and anchor/capture
bindings in their public reports and emitted records.

Change one axis at a time: substitute a same-named foreign callee, wrong arity,
wrong nominal type/unit, ambiguous name, non-Boolean body, direct cycle,
two-predicate cycle or unguarded optional operation. Pair the partial-operation
case with its valid preceding guard. Select the historical no-call definition
and a narrower callee definition under a wider caller, including an unused
prohibited declaration. Keep all non-mutated inputs fixed.

## Expected Results

The positive retains one exact callee and distinct original call-site
environments. Each adverse case refuses its affected dependency without
granting family authority or changing an unrelated checked declaration.
The guarded total counterpart admits. No capture is retagged or read from an
ambient receiver. Emission evidence does not claim runtime execution.

This newly scoped test case is planned for trace reconciliation and execution;
its existence does not mark the criteria passed.
