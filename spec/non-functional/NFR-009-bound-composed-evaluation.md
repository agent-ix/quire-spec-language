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
# NFR-009: Bound composed state evaluation work and retained values

## Statement

If the next charged composed-evaluation operation exceeds its selected ceiling,
then the state evaluator SHALL stop with typed exhaustion before performing or
retaining that operation.

## Scope

One `state::evaluate` or `state::evaluate_v2` call, including request/input
validation, predicate calls, ordered queries, comparison and finite graph
traversal. Limits are inclusive unsigned counters. Callers may lower each
independently; a higher request clamps to the hard ceiling. Exhaustion returns
no completed or partially materialized value. The borrowed version-exact
package and state view remain reusable with fresh limits.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Input value nodes | At most 100000 inspected nodes per call | 100000 nodes | negative-abuse-testing |
| Input aggregate entries | At most 100000 inspected input/static-binding entries per call | 100000 entries | negative-abuse-testing |
| Input text content | At most 8388608 inspected UTF-8 bytes per call | 8388608 bytes | negative-abuse-testing |
| Input structural depth | At most 64 active recursively validated value nodes per call | 64 levels | negative-abuse-testing |
| Expression work | At most 1000000 entered value operations per call | 1000000 operations | negative-abuse-testing |
| Active expression depth | At most 64 active value-operation frames per call | 64 levels | negative-abuse-testing |
| Predicate call depth | At most 64 active predicate calls | 64 calls | negative-abuse-testing |
| Sequence work | At most 1000000 inspected query occurrences per call | 1000000 occurrences | negative-abuse-testing |
| Retained output | At most 100000 materialized result or active-local values per call | 100000 values | negative-abuse-testing |
| Graph expansion | At most 10000 expanded object storage keys per call | 10000 objects | negative-abuse-testing |
| Graph edges | At most 100000 inspected reference occurrences per call | 100000 edges | negative-abuse-testing |
| Active graph depth | At most 64 active recursive graph-expansion frames per call | 64 levels | negative-abuse-testing |
| Value comparison | At most 100000 comparison-work units per call | 100000 units | negative-abuse-testing |

## Counter definitions

The accounting identity is `quire.state.evaluation-work/1`. Request selection
charges one expression operation only when its selected operation is entered.
Each predicate call charges the call operation before checking call depth; its
callee root charges separately. Each ordered-query occurrence charges sequence
work before binding or inspecting that occurrence. Each value added to a map or
filter result charges retained output before insertion; a scalar completed root
charges one retained value when copied into the report.

Input validation charges one node before inspecting a supplied value and one
entry before inspecting each binder, population, object, field or sequence
member. Required-input discovery also charges one input-aggregate entry before
visiting each admitted value, binder or dependency edge and before retaining a
binder-table entry; the same charge precedes any corresponding fallible scratch
allocation. It charges a string's UTF-8 byte length before inspection. Shared
input values are charged at each supplied occurrence because the public view is
a tree, while object storage is indexed once per full key.

Input structural depth starts at one for each supplied binder or object-field
value root and increases by one before entering each record field, option
payload or sequence member. Scalar, reference, object and unavailable nodes are
leaves. Binding/population table containers do not add a value depth. The active
counter is decremented after a child returns and usage retains the greatest
successfully entered depth.

Expression depth starts at one for the selected value operation and increases
by one before entering each selected operand, branch, binder initializer, query
body or predicate callee root. Every operation, including `group` and `read`, is
a frame; a skipped branch or sequence occurrence creates no frame. Predicate
call depth independently starts at one on entry to the first callee, increases
only across an active nested call and decrements when that call returns.

Retained-output work includes each query/map/filter result member, the completed
root copied into the report and each active local/query binding materialized by
evaluation. A local binding is removed when its scope exits, but its successful
retention remains a cumulative charge; its fallible table allocation follows
that charge.

Value comparison charges one unit before inspecting a pair. An equal-length text
pair additionally charges one unit per UTF-8 byte before inspecting its content;
different-length text values require no content scan. Scalar, enum and complete
object-key comparisons require only the pair unit.
The selected composed profile admits equality only for its scalar, enum,
reference and object-identity types; it does not widen FR-040 with recursive
record, option or sequence equality. Text content is charged at the comparison
point described above, independently of input validation; no wall-clock or
locale-dependent work is introduced.

Graph reachability performs deterministic depth-first search in authored edge
order. Entering an unexpanded object charges graph expansion, marks its full
storage key expanded and then scans its outgoing references. Each reference
occurrence charges one graph edge before target comparison. A matching endpoint
returns true before repeat-expansion suppression; otherwise an unexpanded target
is entered recursively before the next sibling edge. Duplicate edges therefore
consume edge work, cycles terminate, and a self-loop remains positive-length.
Active graph depth starts at one for the selected start object.

Input, expression, predicate-call and graph depth are independent limits and
maximum-usage counters. Entering a frame checks its own limit before the frame
or any work inside it; returning always removes that active frame.

Every charge checks `used == limit` before performing its operation. Zero is a
real limit. Exhaustion records one closed cause—`Limit`, `CounterOverflow` or
`Allocation`—and the affected dimension. A bounded allocation is attempted only
after charging its retained-entry dimension; allocator refusal is attributed to
that dimension without claiming its numeric ceiling was reached. Validation precedes execution, but unavailable
typed placeholders charge their node/entry and remain dormant until their value
is selected. Each report exposes the clamped limits and successful charges made
before its terminal outcome.

## Verification

Rust public-API tests invoke both version-exact entry points and independently
count small fixed inputs for each dimension,
then run zero/no-work, exact, one-short and above-hard requests while holding all
other dimensions sufficient. Tests use ordered duplicate sequences, nested
predicate calls, Unicode text, supported identity comparisons, materializing
queries and cyclic/diamond graphs.
Mutation controls remove individual charge sites and require the exact-boundary
case to fail. A sufficient retry after exhaustion returns the same value with
fresh counters and unchanged inputs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-009-AC-1 | All thirteen counters use the exact units and inclusive hard ceilings in this requirement, including one pair unit plus equal-length UTF-8 byte units for text comparison; a caller may lower each independently, zero remains effective and a request above a hard ceiling is clamped. | Test (TC-137) |
| NFR-009-AC-2 | Each charged dimension admits its independently counted exact case and stops its one-short case before the next operation, retaining only successful usage and no partial completed value. | Test (TC-137) |
| NFR-009-AC-3 | Any checked-counter overflow or allocator refusal reached by the implementation produces its closed typed exhaustion cause assigned to the affected charged dimension and cannot be reclassified from diagnostic text. | Analysis |
| NFR-009-AC-4 | Re-evaluation starts with fresh counters, leaves the borrowed artifact and input view unchanged, and returns the same result and usage when the immutable inputs and sufficient effective limits are equal for both version-exact entry points. | Test (TC-137, TC-142) |

## Dependencies

- [FR-049](../functional/FR-049-admit-composed-evaluation-inputs.md) owns the typed input and report boundary.
- [FR-046](../functional/FR-046-execute-predicates-and-ordered-queries.md) owns predicate/query operator order.
- [FR-047](../functional/FR-047-evaluate-finite-object-reference-graphs.md) owns graph identity and Boolean meaning.
- [NFR-006](NFR-006-bound-native-runtime.md) remains the historical runtime accounting contract; its identity and counters are not reused or silently reinterpreted.
