---
id: TC-421
title: "The package source map carries the wire's source map, and a location resolves or refuses by cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: verifies
---
# TC-421: The package source map carries the wire's source map, and a location resolves or refuses by cause

## Description

Verify ADR-013 O-12 and C-14: the I2 reader's verified outcome carries the
wire's `source_map` as the package source map, keyed by the O-07
occurrence key, and a kernel location tag resolves through it or refuses
with a typed cause. This catches a dropped or truncated region list, a
misspelled role, a relabelled source and a lookup that reports the wrong
cause.

Scope: FR-095-AC-3, FR-095-AC-4.

## Test Procedure

1. Read a verified wire declaring `A` and `B`, `B`'s entry holding two
   regions. Look up each entry's key and each node's occurrences.
2. Build a map with an empty entry, and one with a key mapped twice.
3. Convert a hand-built entry with region start 9, end 3, and one whose node
   id digest is uppercase hex; read each defect's refusal code. Map each of
   IR's six occurrence roles to its spelling and compare it with IR's own
   serialized spelling.
4. Read QSpec's published positive fixtures from
   `$QSPEC_DIR/proposals/checked-package-v2/fixtures/positive-*.json` through
   IR's v2 reader, with each fixture's own locked artifacts as current and
   its required features as supported, and look up every `source_map` entry
   in the converted map.
5. Resolve location tags for a mapped occurrence, a node the map does not
   hold and a mapped node at an unmapped ordinal.
6. Read a wire whose `source_map` entry names a node absent from its graph.

Tag the tests `#[trace("TC-421", "FR-095-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: each key maps to exactly its wire regions in wire order, under the
  wire's authority, identity, revision and digest; each node has exactly its
  one entry.
- Step 2: `NoRegion` and `DuplicateOccurrence`.
- Step 3: `ReversedRegion` and a node-id defect, each `invalid_source_map`;
  each role's spelling equals IR's (`declaration`, `type`, `expression`,
  `anchor`, `claim`, `generated`).
- Step 4: every entry looks up to exactly its regions, and each node's
  occurrence count equals its entry count.
- Step 5: the regions, `UnknownNode` and `UnknownOccurrence`.
- Step 6: refused as `invalid_source_map`.

## Status

Passed locally. `qsl-package/src/checked_v2/tests.rs` backs steps 1, 3, 4
and 6, and `qsl-foundation/src/source/provenance.rs` steps 1, 2 and 5. Step
4 runs under `make conformance`. It uses IR's reader rather than the whole
I2 read because `library`'s identity-preimage check refuses three of the
five fixtures for `identity_projection` order, which IR admits; step 1 covers
the whole read.
