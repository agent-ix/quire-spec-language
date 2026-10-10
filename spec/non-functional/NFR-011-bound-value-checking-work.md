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
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
---
# NFR-011: Bound value checking work

## Statement

If the next checking charge would exceed a selected checking ceiling, then the
checker shall stop with a stage limit, reported as `stage_limit_exceeded`
with the ceiling's kind and its selected bound, without performing that
charge. The ceilings are stage limits, not a caller work budget
(`quire.native.diagnostics/v1` `stage_limit_exceeded` row; [FR-096](../functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)).

## Scope

One `PackageDeclarations::check` call under one `CheckingLimits`, which
applies all three ceilings. One standalone expression check
(`CheckedGraph::check_expression` and its clause variants) applies the node
ceiling of its `CheckingLimits`; it encodes no declaration and charges no
work. The ceilings are inclusive unsigned counts supplied by the
caller. The values below are the default ceilings a caller gets when it
configures none. A caller may set each ceiling independently, above or below
its default, through its setting (`s3.nodes`, `s3.input_bytes`,
`s3.work_units`;
[FR-255](../functional/FR-255-name-the-setting-that-raises-a-reached-limit.md)),
and the ceiling is used as given. Every checking walk runs over an explicit
heap stack whose growth these ceilings charge, so an expression's or a
type's depth is bounded only by them
([FR-258](../functional/FR-258-check-and-lower-expressions-at-any-depth.md),
ADR-030). The ceilings a check ran
under are recorded with its result (`CheckedGraph::effective_limits`,
`CheckedExpression::effective_limits`). An implementation ceiling is not a
domain bound (NFR-001).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Checking node ceiling | At most the selected ceiling (default 100000 units) per checked package | 100000 units by default | negative-abuse-testing |
| Declaration preimage bytes | At most the selected ceiling (default 16777216 bytes) per declaration | 16777216 bytes by default | negative-abuse-testing |
| Checking work | At most the selected ceiling (default 16777216 work units) per checked package | 16777216 units by default | negative-abuse-testing |

## Counter definitions

- **Node unit.** Typing charges one unit for each expression node it types.
  The count covers the whole package, not each declaration (FR-062-AC-11).
  Lowering charges one unit for each text leaf and each recursion leaf that
  FR-093's text-leaf walk appends. Typed nodes and leaves share the one
  budget.
- **Preimage byte.** FR-062's length-prefixed encoding of one parsed
  declaration: an eight-byte prefix and the bytes for each written string,
  eight bytes for each number and one byte for each flag.
- **Work unit.** Each declaration charges one unit per preimage write.
  Lowering charges one unit for each node it builds and each composite a
  text-leaf walk enters. Keying a recursion group charges the group's key
  work. Each leaf a text-leaf walk appends charges its key bytes: the
  length of each segment's key spelling (`field:<name>`, `member:<name>`,
  `position:<n>`, `inner`) along its path, plus `recursion:<d>` for a
  recursion leaf. Union entry is a composite charge under the same rule;
  each member segment contributes its literal UTF-8 byte length.

## Default derivation

- **Nodes, 100000.** Twice NFR-001's default syntax-node ceiling of 50000.
  A package checked from one source unit types at most as many expression
  nodes as that unit has syntax nodes, so the factor of two leaves room for
  at least as many FR-093 leaves as typed nodes. The checker does not split
  the budget; typed nodes and leaves draw on it in the order they are
  charged.
- **Preimage bytes, 16777216.** NFR-007's default package byte ceiling,
  sixteen times NFR-001's default source ceiling. A declaration's preimage
  is not linear in its source: each parameter or result typed with an enum
  writes one string per case, about 72 bytes each, so an enum reference of
  one token can write kilobytes. The ceiling bounds one declaration's
  preimage whatever its source length.
- **Work, 16777216.** The preimage byte ceiling divided by the fewest bytes
  one charged write produces (one, for a flag). A declaration charges one
  work unit per write, and its preimage bytes are checked before those
  writes are charged. A declaration's writes therefore never reach the work
  ceiling before its preimage reaches the byte ceiling: the byte ceiling
  binds first.

The work ceiling is cumulative over the package, so it can bind before any
one declaration reaches the byte ceiling. It does so only when the
package's declarations together write more than 16777216 times. For
example, 4000 functions each with one parameter over a 250-case enum write
1068000 times, and the package checks. A package whose enum references write
cases times references above that total, such as a 20000-case enum
referenced by 1000 parameters, refuses on work.

The same ceiling bounds FR-093's leaf output. The leaf count is at most the
node ceiling, and the leaves' key bytes together are at most the work
ceiling.

Every existing fixture and conformance vector checks within these defaults.
The largest recorded checker input by node units is FR-093's eight-record
cluster (95904 units, 96% of the node ceiling). The largest by work is an
8000-function call chain (160000 units, 1% of the work ceiling).

## Cost at the defaults

Width, not depth, is the risk. FR-093's leaf list over `n` mutually
referencing records that can all reach a text type holds about `(n - 1)!`
leaves. Long field names or deep paths make each leaf's key long. The walk
shares every path prefix between the leaves under it, and it materializes
no leaf path until the walk completes. A refused walk therefore holds its
path tree, not one path per leaf.

The enforced bound is on counts: at most 100000 leaves, and at most
16777216 key bytes across all of them. Memory follows from these counts.
`qsl-bench/BASELINE.md` records the measurements behind the figures below:

- Every Text-reachable cluster from 9 to 12 records refuses on the node
  ceiling in about 15 ms, with peak RSS about 18 MB.
- The review's deep-and-wide shape is a 46-record chain into a
  17-level binary tree, with 65536 leaves. With 1-, 64- and 256-byte field
  names it refuses on the work ceiling in at most 17 ms, with peak RSS at
  most 11.5 MB.
- The largest admitted packages sit near a ceiling:
  - The eight-record cluster (95904 leaves) peaks at 327 MB.
  - A 4-record chain into the same tree charges 16.2 million key bytes and
    peaks at 674 MB.
- The measured rate is about 41 bytes of memory per charged key byte, plus
  about 3.4 KB per leaf. From that rate, memory at the defaults is
  estimated at about 1 GB. This is an estimate, not a bound the limits
  enforce.

## Verification

TC-423 covers the defaults:

- At the default ceilings, a nine-record Text-reachable cluster refuses,
  naming the node ceiling and its default bound.
- The review's long-path shape refuses, naming the work ceiling and its
  default bound.
- A package of 4000 enum-parameter functions checks at the defaults.
- With the work ceiling at its default ratio to the byte ceiling, a
  declaration past the byte ceiling refuses on bytes, not work.
- `CheckingLimits::new` keeps the default byte and work ceilings.
- A checked package and a checked expression record the default ceilings,
  and record a caller's ceilings as given, above or below the defaults.

The refusal at a caller-selected bound is covered for nodes by TC-381 and
FR-062-AC-7, and for preimage bytes and work by FR-062-AC-5 (TC-160). Deep
inputs that fit the ceilings, and the setting each refusal names, are
covered by FR-258 (TC-725 to TC-727).

## Dependencies

- [FR-062](../functional/FR-062-implement-checked-family-contract.md)
  defines the checking limits and their refusals.
- [FR-093](../functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  defines the text-leaf walk the node ceiling bounds.
- [NFR-001](NFR-001-bound-syntax-work.md) sets the syntax ceilings the
  defaults derive from.
- [NFR-007](NFR-007-bound-native-packages.md) sets the package byte ceiling
  the preimage default reuses.
