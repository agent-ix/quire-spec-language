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

Independently count fixed valid fixtures exercising input nodes/entries/text;
expression work; sequence occurrences; retained outputs; scalar/key comparison
pairs and equal-length text comparison bytes; and graph objects/edges. On a
thread with a 512 KiB stack, evaluate an input nested 100,000 records deep, a
chain of 10,000 nested predicate calls and a 10,000-object graph chain under
limits sized for them.
Use nested record/option/sequence inputs, grouped/branched expressions, nested
calls, supported scalar/enum/reference/object-identity comparisons and a graph
chain so each selected dimension has a distinct oracle. Run each dimension at
zero when no work is required,
exact, one-short and raised above its default while other limits remain sufficient.
After every exhausted run, repeat with sufficient limits over the same borrowed
package and state view. Mutation controls remove one charge site at a time.

## Expected Results

Exact capacities complete and report the independent counts. One-short, overflow
stop before the next operation with the exact dimension and no completed
partial value. A limit raised above its default admits input past the
default. Every sufficient retry
starts from zero usage, returns the same value and leaves all input identities
and contents unchanged. Removing a required charge makes its mutation control
fail.
