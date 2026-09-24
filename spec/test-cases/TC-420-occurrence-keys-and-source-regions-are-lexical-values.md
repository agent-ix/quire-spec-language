---
id: TC-420
title: "Occurrence keys and source regions are lexical values, and every checked node has an occurrence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: verifies
---
# TC-420: Occurrence keys and source regions are lexical values, and every checked node has an occurrence

## Description

Verify ADR-013 O-07's occurrence key and region, and O-12's region
equality. This catches a region equality that compares the authority or
revision labels, a `RawSourceRef` that admits a non-source digest, and a
lowering that leaves a node with no source occurrence.

Scope: FR-095-AC-1, FR-095-AC-2.

## Test Procedure

1. Compare occurrence keys that differ only in node id, only in role and only
   in ordinal, and two with equal members.
2. Build a `RawSourceRef` with an empty authority, an empty identity and a
   `quire.definition.bytes/v1` digest; a `Revision` with an empty namespace
   and an empty value; a `SourceRegion` with start 5, end 4 and with start 5,
   end 5.
3. Compare a region (digest `d`, 4, 9) with one of the same members under
   another authority, identity and revision, and with regions changing only
   the digest, only the start and only the end.
4. Check a package holding `helper(flag: Boolean): Boolean { flag }` and
   `caller(): Boolean { if true then helper(false) else false }`, and look up
   an occurrence for every node of its semantic graph.

Tag the tests `#[trace("TC-420", "FR-095-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: only the key with equal members is equal.
- Step 2: each malformed member refuses with its own cause; the point region
  is admitted.
- Step 3: the relabelled region is equal and orders equal; the other three
  are unequal.
- Step 4: at least five nodes, each with an occurrence.

## Status

Passed locally. `qsl-foundation/src/source/provenance.rs` backs steps 1 to
3 and `qsl-semantics/src/check/mod.rs`
(`every_lowered_node_has_a_source_occurrence`) step 4.
