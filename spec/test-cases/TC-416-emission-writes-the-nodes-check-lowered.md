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

It also verifies each node's `dependencies` against FR-093's Node
dependencies rules, and the emission against QSpec's v2 positive fixtures by
the members those fixtures pin. This catches a type annotation listed as a
dependency, a missing `bounded_domain` base or group member, and an operator
or operation spelling that differs from QSpec's.

Scope: FR-093-AC-7, FR-093-AC-9, FR-093-AC-12, FR-093-AC-13, FR-093-CON-2.

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
5. Check and emit the recursive `f` of FR-092 vectors G4 to G6, and read the
   `recursion_group` label and graph position of each member.
6. Check and emit `record Tree { kids: Sequence<Tree>[0, 3]; }`. For each
   node emitted in steps 1, 5 and 6, rebuild its `dependencies` by rules 1
   to 5 of FR-093's Node dependencies with a test-side walker over the
   emitted JSON that calls no `qsl-package` function. Read the
   `dependencies` of E1, F2, E2, T1, L1 and P1 (step 1), of T4 and G4 to G6
   (step 5), and of G7 to G9 (step 6).
7. Under `make conformance`, read `positive-operation-identities.json` and
   `positive-control-operations.json` from the `QSPEC_DIR` checkout. For
   each application node whose `operation.identity` a row of FR-093's
   application table lowers, check and emit a function whose body holds
   that operation, read the emitted package with IR's v2 reader, and
   compare the emitted node with the fixture node on the members FR-093-AC-13
   names.

Tag the tests `#[trace("FR-093-AC-n", "TC-416")]` with the AC each backs.

## Expected Results

- Step 2: every recomputed key equals the node's `node_id`.
- Step 3: no match.
- Step 4: every node has at least one occurrence; `a`'s parameter node has an
  `anchor` occurrence and one `expression` occurrence per read; the scalar
  nodes typing P1's body literals have a `generated` occurrence.
- Step 5: the three members carry the label
  `0b9e8d18320d0ce587699e40ac33a25fd41c4a640226bda4b8b1521edc5e4c50`, their
  graph order is G5, G4, G6 (ordinals 0 to 2), and step 2's recomputation
  from graph order gives G4 to G6.
- Step 6: every emitted list equals its rebuilt list. In ascending digest
  order, E1 lists P2, P1; F2 lists P2, P1, E1; E2 lists L1, F2, P1; T4 lists
  T2; G4 lists G5, P4; G5 lists L1, E11, G6; G6 lists G4, E13; G7 lists G9;
  G8 lists G7; G9 lists G8; T1, L1 and P1 list none.
- Step 7: IR admits each emitted package, and each compared member equals
  the fixture's. The compared members are exactly those FR-093-AC-13
  names.

## Status

Planned; QSL-6 S1b, after QSL-156 A4b merges. `qsl-package/src/emit.rs`
refuses every non-empty graph. Emission needs the edition lock evidence
(ADR-011 §2.4), and step 7's text and IEEE operations need a law from it.
Step 7's admission half also waits on IR's v2 reader reading ADR-013 QC-24's
`value`/`parameter` and function nodes and QC-25's model nodes: IR's closed
`ValueForm` at `c0ba691` has no `parameter`, and it refuses such a package as
`invalid_semantic_graph`.
