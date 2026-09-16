---
id: FR-142
title: "Evaluate dimensions, quantities and exact unit conversions"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-044
    type: references
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
  - target: ix://agent-ix/quire-specification/FR-140
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

# FR-142: Evaluate dimensions, quantities and exact unit conversions

## Description

When checking a quantity expression, the semantic kernel SHALL derive its
dimension and unit from declared identities and exact conversion functions.

## Inputs

Dimension vectors, unit identities, exact scale/offset conversions, quantities
and operations.

## Outputs

A completed typed quantity with any explicit conversion loss, or an undefined,
refused or incomplete evaluator outcome. I13 dispositions are not evaluator
outputs.

## Behavior

Addition/comparison require compatible dimensions and an explicit common-unit
conversion. Multiplication/division combine dimension exponents. Affine units
use their selected offset rules and admit conversion/comparison only; complete
V1 has no implicit point-versus-difference type, so arithmetic on a nonzero-
offset quantity refuses. Conversion never occurs implicitly.

## Dimension and conversion algebra

A dimension is a finite sorted map from I04 semantic-node base-dimension
identity to signed integer exponent; zero exponents are removed. Multiplication
adds exponents, division subtracts them and integer power multiplies them.
Quantities compare or add only when these normalized maps are equal.

The unit declarations for one I04 semantic-node dimension identity form a closed directed
graph with exactly one canonical root. The root has no target, scale one and
offset zero. Every other unit names exactly one target unit under the same
semantic-node dimension; each edge means
`target_value = scale × source_value + offset`, where scale is a nonzero exact
rational and offset is an exact rational. The graph is acyclic and every unit
reaches the root. A duplicate/missing root, target cycle, unknown target or
cross-dimension target refuses semantic admission. Composing the edges yields
each unit's exact mapping to the canonical unit; conversion applies the source
mapping then the inverse target mapping.

A nonzero-offset unit is affine: it may be converted or compared but cannot be
added, subtracted, multiplied, divided or raised to a power. A separately
declared zero-offset difference unit supports arithmetic; complete V1 does not
infer a point/difference kind from an operation. Conversion into an exact target
reports no loss; conversion requiring
decimal/finite-domain rounding follows FR-140 and records that loss.

Base-dimension and unit identities are opaque I04 checked-semantic-graph node
keys. A base dimension's `quire.checked-semantic-node/v1` digest is SHA-256 over
RFC 8785 JCS of `{ version: quire.dimension-node/v1, owner,
qualified_declaration, terms }`, where owner is the stable subject projection of
an exact admitted source, DefinitionRef or ModelRef and must join that lock
selection. A unit digest uses `{ version: quire.unit-node/v1, owner,
qualified_declaration, dimension_node_id, target_unit_node_id, scale,
offset }`, with normalized exact rationals. Acyclic unit targets make these keys
recomputable in root-to-leaf order. The strict I04 reader refuses a body/key
mismatch as `invalid_semantic_graph`. A dimension node is self-typed and depends
exactly on the base-dimension keys in its terms. A unit's `semantic_type` equals
`dimension_node_id`; its dependencies are exactly that dimension plus its
non-null target. Contradictory semantic fields refuse even when the retained
preimage and key agree. Equal names, scales or dimension shapes do
not confer identity, while the same exact imported owner/declaration remains
stable across unrelated package edits. Dimension exponents are mathematical signed integers subject to
explicit operation limits; implementations cannot narrow them to a machine
integer. A conversion publishes no partial quantity until canonicalization,
target rounding and target-domain admission all succeed.

Multiplication, division and integer power first convert every zero-offset
operand to its dimension's canonical root, perform exact value arithmetic and
normalize the resulting base-dimension exponents. The result unit is an
evaluator-owned value identity in the `quire.value.compound-unit/v1` domain.
Its JCS preimage is `{ version: quire.value.compound-unit/v1, terms }`, where
`terms` is the ascending list of admitted I04 canonical root-unit node IDs with
their nonzero mathematical integer exponents. The empty list is the
dimensionless unit. This digest is not an I04 checked-semantic node, graph
reference, declaration or source-mapped authority, and evaluation does not
insert it into the checked package. Its closed schema and vectors are selected
by `quire.value.complete/v1`. An explicit later conversion may select a
compatible declared I04 unit; multiplication never guesses one from a matching
name or scale.
All identity reads, traversed edges, exact rational operations, target-domain
checks and result retention use the named points in
`quire.value.accounting/v1`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-142-AC-1 | Exact compatible zero-offset conversion followed by arithmetic produces the declared result unit and value. | Test (TC-187) |
| FR-142-AC-2 | Incompatible dimensions or prohibited affine-unit arithmetic refuses at the operator. | Test (TC-187) |
| FR-142-AC-3 | A lossy conversion returns its exact loss classification or refuses when the target permits no loss. | Test (TC-187) |
| FR-142-AC-4 | Normalized dimension maps remove zero exponents and make multiplication/division inverse for admitted quantities. | Test (TC-187) |
| FR-142-AC-5 | Exact affine conversion composes through the unique canonical root; invalid unit-graph topology, zero scale or affine arithmetic refuses. | Test (TC-187) |
| FR-142-AC-6 | Multiplication, division and integer power produce the exact value in the canonical compound-unit identity, including dimensionless cancellation. | Test (TC-187) |
| FR-142-AC-7 | Exact-bound accounting succeeds and denial of a named next unit charge returns incomplete without a partial quantity or compound identity. | Test (TC-187) |
| FR-142-AC-8 | The published `quire.value.compound-unit/v1` schema and vectors fix the compound-unit identity preimage: each vector is schema-valid, its RFC 8785 JCS SHA-256 equals its recorded identity, and its terms name admitted root-unit node keys with nonzero exponents in strictly ascending key order; each authored stale-key mutation changes the identity and each authored noncanonical mutation, including a `-0` exponent, is refused by exactly its named schema or semantic check. | Test (TC-233) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
