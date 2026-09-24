---
id: NFR-011
title: "Bound value checking work"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: constrains
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: depends_on
---
# NFR-011: Bound value checking work

## Statement

If the next checking charge would exceed a selected checking ceiling, then the
checker shall refuse with resource_exhausted, naming the ceiling's kind and its
selected bound, without performing that charge.

## Scope

One `PackageDeclarations::check` call under one `CheckingLimits`, which
applies all four ceilings. One standalone expression check
(`CheckedGraph::check_expression` and its clause variants) applies the node
and nesting ceilings of its `CheckingLimits`; it encodes no declaration and
charges no work. The ceilings are inclusive unsigned counts supplied by the
caller. The values below are the default ceilings a caller gets when it
configures none. A caller may set each ceiling independently, above or below
its default, and the ceiling is used as given. The one exception is nesting
depth, which cannot exceed its default of 128. The ceilings a check ran
under are recorded with its result (`CheckedGraph::effective_limits`,
`CheckedExpression::effective_limits`). An implementation ceiling is not a
domain bound (NFR-001).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Checking node ceiling | At most the selected ceiling (default 100000 units) per checked package | 100000 units by default | negative-abuse-testing |
| Checking nesting depth | At most the selected ceiling (default 128 levels, also the maximum) per expression | 128 levels | negative-abuse-testing |
| Declaration preimage bytes | At most the selected ceiling (default 16777216 bytes) per declaration | 16777216 bytes by default | negative-abuse-testing |
| Checking work | At most the selected ceiling (default 1000000 work units) per checked package | 1000000 units by default | negative-abuse-testing |

## Counter definitions

- **Node unit.** Typing charges one unit for each expression node it types.
  The count covers the whole package, not each declaration (FR-062-AC-11).
  Lowering charges one unit for each text leaf and each recursion leaf that
  FR-093's text-leaf walk appends.
- **Nesting level.** One level is one nested expression that typing enters.
  The same count bounds the depth of a text-leaf walk.
- **Preimage byte.** FR-062's length-prefixed encoding of one parsed
  declaration: an eight-byte prefix and the bytes for each written string,
  eight bytes for each number and one byte for each flag.
- **Work unit.** Each declaration charges its preimage write count. Lowering
  charges one unit for each node it builds and each composite a text-leaf
  walk enters. Keying a recursion group charges the group's key work.

## Default derivation

- **Nodes, 100000.** Twice NFR-001's default syntax-node ceiling of 50000.
  A package checked from one source unit types at most as many expression
  nodes as that unit has syntax nodes. Typing can therefore use one half of
  the ceiling, and the other half is left for FR-093's leaves.
- **Nesting depth, 128.** The largest depth that keeps each recursive
  checking pass within the host stack (`MAX_CHECKING_DEPTH`).
- **Preimage bytes, 16777216.** NFR-007's default package byte ceiling,
  sixteen times NFR-001's default source ceiling. A declaration's encoding
  writes each of its source strings once, plus a tag and an eight-byte
  length prefix for each node. Its size therefore grows linearly with the
  declaration's source bytes and nodes, both of which NFR-001 bounds.
- **Work, 1000000.** Ten work units for each default node unit. The largest
  recorded checker input is an 8000-function call chain. It uses 16000
  nodes and charges 160000 work units.

Every existing fixture and conformance vector checks within these defaults.
The largest recorded input uses 16% of the node ceiling and 16% of the work
ceiling.

## Cost at the defaults

Width, not depth, is the risk. FR-093's leaf list over `n` mutually
referencing records that can all reach a text type holds about `(n - 1)!`
leaves. Each leaf costs one node unit, so the node ceiling, not `(n - 1)!`,
bounds the walk. The walk holds at most one leaf per node unit, and each
leaf's path is at most twice the depth ceiling in segments, so its memory
grows at most linearly with `n` before it refuses.

`qsl-bench/BASELINE.md` records the measured curve. At the defaults the
8-record cluster checks, and every cluster from 9 to 12 records refuses on
the node ceiling in about 0.1 s, with peak RSS under 100 MB.

## Verification

TC-420: at the default ceilings a nine-record Text-reachable cluster refuses,
naming the node ceiling and its default bound. A checked package and a
checked expression record the default ceilings, and record a caller's
ceilings as given, above or below the defaults. The refusal at a
caller-selected bound is covered for nodes by TC-381, for nesting depth by
FR-062-AC-7, and for preimage bytes and work by FR-062-AC-5 (TC-160).

## Dependencies

- [FR-062](../functional/FR-062-implement-checked-family-contract.md)
  defines the checking limits and their refusals.
- [FR-093](../functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  defines the text-leaf walk the node ceiling bounds.
- [NFR-001](NFR-001-bound-syntax-work.md) sets the syntax ceilings the
  defaults derive from.
- [NFR-007](NFR-007-bound-native-packages.md) sets the package byte ceiling
  the preimage default reuses.
