---
id: TC-416
title: "The v2 emission arm writes the nodes check lowered, and each emitted node recomputes to its node id"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
---
# TC-416: The v2 emission arm writes the nodes check lowered, and each emitted node recomputes to its node id

## Description

Verify the FR-093 ownership split: `check` lowers and keys, and the layer-4
`package` emission arm only serializes. Every emitted node's key, recomputed
from the node as written, equals its `node_id`, and `package` builds no body
term and calls no key function.

This catches a second lowering in `package` that drifts from the one `check`
hashed, which a reader would refuse as `stale-node-key`.

Scope: FR-093-AC-7, FR-093-AC-9, FR-093-CON-2.

## Test Procedure

1. Check and emit a package holding `both`, `nb`, `h`, `f` and `t`
   (FR-093-AC-1, AC-2) under owner (`a`, `u`).
2. For each emitted node, rebuild its preimage from the wire node (FR-322's
   application-node rule when its body holds an application, else FR-092's
   structural-node rule) and hash it.
3. Scan the `package` crate's non-test code for a node body term
   constructor, a node-identity preimage builder and a `NodeKey`
   constructor call.
4. List every emitted node's occurrences.

Tag the tests `#[trace("FR-093-AC-n", "TC-416")]` with the AC each backs.

## Expected Results

- Step 2: every recomputed key equals the node's `node_id`.
- Step 3: no match.
- Step 4: every node has at least one occurrence; `a`'s parameter node has an
  `anchor` occurrence and one `expression` occurrence per read; the scalar
  nodes typing P1's body literals have a `generated` occurrence.

## Status

Planned; QSL-6 S1b, after QSL-156 A4b. Emission needs the edition lock
evidence (ADR-011 §2.4).
