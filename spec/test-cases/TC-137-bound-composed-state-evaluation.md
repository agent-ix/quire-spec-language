---
id: TC-137
title: "Bound composed evaluation and retry immutable inputs"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/NFR-009, type: verifies }
---
# TC-137: Bound composed evaluation and retry immutable inputs

## Description

Verify every `quire.state.evaluation-work/1` counter and its charge-before-work
boundary through the public evaluator.

## Test Procedure

Independently count fixed valid fixtures exercising input nodes/entries/text and
structural depth; expression work/depth and predicate-call depth; sequence
occurrences; retained outputs; comparison pairs/depth; and graph objects/edges/
depth. Use nested record/option/sequence values, grouped/branched expressions,
nested calls, recursive record comparison and a graph chain so each depth limit
has a distinct oracle. Run each dimension at zero when no work is required,
exact, one-short and above-hard while other limits remain sufficient.
After every exhausted run, repeat with sufficient limits over the same borrowed
package and state view. Mutation controls remove one charge site at a time.

## Expected Results

Exact capacities complete and report the independent counts. One-short, overflow
and hard-ceiling excess stop before the next operation with the exact dimension
and no completed partial value. Above-hard options clamp. Every sufficient retry
starts from zero usage, returns the same value and leaves all input identities
and contents unchanged. Removing a required charge makes its mutation control
fail.
