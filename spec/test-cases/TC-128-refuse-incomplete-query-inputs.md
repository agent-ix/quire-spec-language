---
id: TC-128
title: "Keep incomplete query inputs and exhausted work distinct from values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: verifies
---
# TC-128: Keep incomplete query inputs and exhausted work distinct from values

## Description

Verify that population completeness and finite work remain required for a
completed predicate/query result under
[FR-046](../functional/FR-046-execute-predicates-and-ordered-queries.md).

## Test Procedure

Supply an independently selected complete finite population view and evaluate a
query with a known nonempty result. Remove, one at a time, the required sequence,
a required member and the completeness premise without changing the declared
scope or expected selection. Pair each adverse case with the complete input.

For predicate-call expansion, nested queries and retained output, measure the
successful run's public accounting and rerun with zero, the exact required
capacity and one less in each exercised dimension. Keep unrelated budgets
sufficient. Retry with the original complete input and adequate limits, checking
that each run has fresh counters and preserves the original input identity.

For short-circuiting forms, place missing membership immediately before and
after the first decisive occurrence. For materializing/aggregate forms, make
the second occurrence unavailable after the first occurrence has produced an
intermediate value.

## Expected Results

The complete view yields its independently expected value. Missing data or
completeness never becomes an empty domain or successful aggregate. Exact work
capacity completes; insufficient capacity reports resource incompleteness,
with the exhausted dimension and no completed partial query value. No stage
creates an observation store or narrows declared membership to fit its sample.

Concrete population validation and runtime execution evidence must come from
the actual selected interfaces. This planned case does not claim that currently
emitted population/closure requirements establish those runtime premises.
