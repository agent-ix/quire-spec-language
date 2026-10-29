---
id: NFR-009
title: "Bound composed state evaluation work and retained values"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-046, type: constrains }
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: constrains }
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: constrains }
---
# NFR-009: Bound checked-term evaluation work and retained values

## Statement

If the next charged checked-term evaluation operation exceeds its selected
caller ceiling, then the owning evaluator SHALL stop before performing or
retaining that operation and SHALL return its typed incomplete/resource-limit
outcome without a completed partial value.

## Scope

One checked-term call, family evaluation or protocol-slot operation, using the
caller-owned `quire.value.accounting/v1` meter. The ten `ScalarLimits` counters,
setting names and defaults are the single table in FR-255 and QSpec FR-323;
this requirement adds no second counter set or alternate accounting identity.
Each applicable counter is inclusive, independently caller-settable above or
below its default, and has no ceiling (ADR-030 D-1). A protocol operation that
evaluates several slots shares its meter as FR-390 specifies; a new evaluation
retry uses a fresh meter. Exhaustion exposes no completed or partially
materialized value. Checked packages and immutable environments are reusable.

Input document reading and model normalization are separate admission stages:
FR-106 owns snapshot/invocation byte, object and value limits, and FR-056 owns
model normalization limits. Their counters are not evaluation meter counters.
This NFR does not preserve the retired `quire.state.evaluation-work/1`
accounting identity or its `StateView` input-validation charges.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Checked-value arithmetic and normalization | Applicable `ScalarLimits` fields | FR-255 accounting table | negative-abuse-testing |
| Checked-term execution, sequence occurrences and graph work | `work_units`, `value_occurrences` and other applicable `ScalarLimits` fields | FR-255 accounting table | negative-abuse-testing |
| Materialized result | `result_units` and other applicable `ScalarLimits` fields | FR-255 accounting table | negative-abuse-testing |
| Source/model/observation intake | Stage-specific limits owned by FR-056 and FR-106; not meter dimensions | FR-056 / FR-106 | negative-abuse-testing |

## Counter definitions

The caller's meter and FR-255/FR-323 counter definitions govern all checked-term
charges. FR-046 owns predicate and ordered-query operation order; FR-047 owns
finite graph traversal and identity; FR-107 owns state-clause evaluation; and
FR-390 owns protocol-slot evaluation and shared-meter ordering. Each owner
defines its charge point and typed result. A charge is checked before the
operation it governs, zero is an effective limit, and a denied charge does not
perform that operation. This requirement does not restate those operation
tables or introduce a separate input-validation counter.

Nesting depth is not itself a limit (ADR-030 D-1). The checked-term, family and
graph implementations use explicit heap stacks where their owning requirements
specify deep-input behavior; TC-137 and TC-131 exercise the deep cases on a
512 KiB stack.

## Verification

Rust public-API tests invoke checked-term, family and protocol-slot entry points
and independently count representative inputs for each applicable
`ScalarLimits` counter. They run zero/no-work, exact, one-short and above-default
requests while holding other dimensions sufficient. Tests cover ordered
sequences, nested predicate calls, checked-value comparisons, materialized
results and cyclic/diamond graphs. Mutation controls remove individual charge
sites and require the exact-boundary case to fail. A sufficient retry after
exhaustion returns the same value with a fresh meter and unchanged inputs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-009-AC-1 | Every applicable `quire.value.accounting/v1` counter is one of FR-255's ten accounting rows, uses that row's units/default/setting name, and can be lowered or raised independently; zero remains effective and a value above the default is used as given. The retired `quire.state.evaluation-work/1` identity is not exposed by checked-term evaluation. | Test (TC-137, TC-1073) |
| NFR-009-AC-2 | For each applicable charged dimension, the independently counted exact case completes and the one-short case stops before the denied operation, retaining successful usage only and no completed partial value. | Test (TC-137, TC-131, TC-1073) |
| NFR-009-AC-3 | Counter overflow and allocator refusal reached by a checked-term operation preserve the meter's typed cause and affected counter; no caller classifies them from diagnostic text. | Analysis |
| NFR-009-AC-4 | Re-evaluation starts with a fresh meter, leaves the checked package and immutable environment unchanged, and returns the same result and usage when inputs and effective limits are equal; sequential protocol slots share one meter as FR-390 requires. | Test (TC-137, TC-1073, TC-142) |

## Dependencies

- [FR-049](../functional/FR-049-admit-composed-evaluation-inputs.md) owns the typed input and report boundary.
- [FR-046](../functional/FR-046-execute-predicates-and-ordered-queries.md) owns predicate/query operator order.
- [FR-047](../functional/FR-047-evaluate-finite-object-reference-graphs.md) owns graph identity, Boolean meaning and its shared-meter charge points.
- [FR-255](../functional/FR-255-name-the-setting-that-raises-a-reached-limit.md) owns the single public setting table and the ten accounting settings. FR-056/FR-106 own pre-evaluation intake limits; NFR-006 remains the historical native runtime accounting contract. Their identities and counters are not reused or silently reinterpreted.
