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

Scope: FR-093-AC-7, FR-093-AC-9, FR-093-AC-12, FR-093-AC-13, FR-093-AC-17, FR-093-AC-18, FR-093-AC-19, FR-093-AC-20, FR-093-AC-21, FR-093-AC-22, FR-093-CON-2.

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
8. Link a graph with imports `test/units` `2` and `test/geometry` `1` of one
   package, emit it and read it back through QSL's I2 read; link a chain
   root -> `test/b` -> `test/c` -> `test/units`; link imports
   `test/\u{1F600}` and `test/\u{FF61}`. Under
   `make conformance`, read QSpec's `dependency-selection-vectors.json`
   (FR-093-AC-16).
9. Check and emit a package holding `t` (step 1) and read the wire's
   `diagnostics.catalog` member.
10. From source text, check and emit `ordered enum Status { READY, DONE }`
    with `before using v(a: Status, b: Status): Boolean pure { a < b }`,
    then with the same function over `a = b`, then with only
    `function t using v(): Boolean pure { true }`; `record P { x: Int[0, 9]; }`
    with only `t`; `record Q { y: Int[0, 9]; }` declared before that `P`,
    with only `t`; and `record R { x: Int[0, 9]; }` beside the post clause
    `VersionUnchanged` over `1 < 2` and a protocol attempt naming it. List each enum member
    node's occurrences and the source text of each `generated` source-map
    region, and read each package back through QSL's I2 read
    (FR-093-AC-18).
11. Classify every kind of IR's `CheckedNodeKind::all()` as a family QSL
    writes (admitted, or one of the two cases of FR-093-AC-19) or as a
    kind QSL never writes, with its reason. Emit a fixture for each family
    QSL writes: the packages of steps 1, 5, 6, 7 and 10, `fadd` over
    `Float32[nearest-even]`, the declared record and tuple, TC-435's and
    TC-442's spine fixtures, TC-442's unit with a `Population<M::Gadget>[3]`
    parameter, a reference to a systems interface, TC-469's ConfigVersion
    unit, TC-440's records, TC-160's `q` and `t`, a unit of quantifiers
    over a set, a bag and an ordered set with a predicate, a decimal
    comparison and record and tuple values, and an equality over the
    recursive `Tree` of step 6. Read each through QSL's I2 read, and decode
    each checked node's (`node_tag`, `semantic_form`) through IR's node
    kinds (FR-093-AC-19).
12. Check and emit `record P { x: Int[0, 9]; }` and the `Tree` of step 6
    under owner (`a`, `u`), a function over a `Reference<M::Order>`
    parameter with `acme/orders` admitted (FR-094-AC-1), and the clause
    functions of FR-094-AC-5. For every emitted node, read its `owner` and
    its `identity_projection` entry's `owner`, and compare them with the
    owner of the node's preimage; recompute each structural key and each
    group label from the wire node alone with a test-side key function that
    calls no `qsl-package` function, and read each package with IR's v2
    reader (FR-093-AC-21).
13. Check and emit `record Point { x: Integer; }` and
    `record List { next?: List; }` under (`agent-ix`, `example-a`), then
    under (`agent-ix`, `example-b`). Compare the two packages' `Point` ids,
    `List` group labels, `List` group member ids, `Integer` ids and
    `package_id`s, and read both with IR's v2 reader (FR-093-AC-22).

Tag the tests `#[trace("FR-093-AC-n", "TC-416")]` with the AC each backs.

## Expected Results

- Step 2: every recomputed key equals the node's `node_id`.
- Step 3: no match.
- Step 4: every node has at least one occurrence; `a`'s parameter node has an
  `expression` occurrence over its binder site (QSpec FR-341-AC-10) and one
  `expression` occurrence per read; the scalar nodes typing P1's body
  literals have a `generated` occurrence.
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
- Step 8: the lock and the identity preimage carry the same two entries,
  `test/geometry` then `test/units`, each with the package's `package_id`;
  the package reads back Verified and its `package_id` differs from the
  unlinked graph's; the chain's closure is `test/b`, `test/c`, `test/units`,
  with paths `[b]`, `[b, c]` and `[b, c, units]`; `test/\u{FF61}` is
  written before `test/\u{1F600}` and reads back Verified. The
  vectors' `package_id` recomputes, each entry mutation refuses at its entry
  and each order vector gets its recorded outcome and locus.
- Step 9: `diagnostics.catalog` is exactly authority `agent-ix` and
  identity `quire.native.diagnostics/v1`, with no other member, and IR
  admits the package (FR-093-AC-17). The lock's `edition` and each
  `definition_selections` row are exactly the catalog row's `authority` and
  `identity`, and a `Float64` addition's `ieee_profile` law `definition` is
  the same two-member row and a member of `definition_selections`
  (FR-093-AC-20).
- Step 10: every package emits with nothing omitted. Each member node's
  occurrences are exactly one `generated` occurrence, ordinal 0, whose
  region text is `a < b`, `a = b`, or `Status` when no function names the
  enum; the `Int[0, 9]` node's region text is `P`, with or without `Q`;
  with `R` and `VersionUnchanged`, the `Integer` node's only occurrence is
  `generated`, region text `1 < 2`. Each function-only package reads back
  Verified (FR-093-AC-18).
- Step 11: every IR node kind is classified exactly once, and every family
  a fixture emits is classified as written by QSL. IR admits every fixture
  at its emitted `package_id` with a node of each of its rows' families,
  the `Tree` equality's `quire.op.structural.eq` node included.
  TC-160's `q` and `t`, over a `metre` unit the fixture unit's own source
  declares, give the `dimension`, `unit` and `compound_unit` rows. No node is
  omitted (FR-093-AC-19).
- Step 12: exactly the declared structural nodes (`P`, `Tree`), M1 and the
  clause functions carry `owner`, each equal to its preimage's owner and to
  its projection entry's; no other node carries one; every recomputed key
  equals its `node_id`, the `Tree` label recomputes, and IR admits each
  package (FR-093-AC-21).
- Step 13: the `Point` ids differ, the `List` labels and member ids differ,
  the `Integer` id is equal, the `package_id`s differ, and IR admits both
  (FR-093-AC-22).

## Status

Steps 1 to 6 implemented in `qsl-package/src/emit/tests.rs`.
Step 8 implemented (`emit/tests.rs` and
`checked_v2::tests::conformance_dependency_selection_vectors`).
Step 9 implemented (`the_lock_selects_the_catalog_definitions`).
Step 10 implemented in `qsl-package/src/emit/tests.rs`
(`enum_members_no_literal_names_are_placed_under_an_ordered_comparison`,
`an_ordered_comparison_of_enum_parameters_reads_back_verified`,
`enum_members_no_literal_names_are_placed_under_an_equality`,
`types_no_function_names_are_placed_at_their_declared_names`,
`a_node_two_unnamed_types_share_is_placed_at_the_least_name`,
`a_node_a_state_clause_places_does_not_move_to_a_type_name`).
Step 11 implemented in `qsl-package/src/emit/tests/admission_corpus.rs`
(`every_emitted_node_family_is_admitted_at_its_package_id`).
Step 7 is implemented in `qsl-package/src/emit/tests/golden.rs`
(`conformance_emitted_application_nodes_match_qspec_positive_fixtures`, tagged
`FR-093-AC-13`, run by `make conformance`): 13 fixture application nodes are
compared, and IR's v2 reader admits each emitted package. It compares every
member including `mode` (`float64.add` under `toward-zero` and `nearest-even`)
and skips, by name, the fixture identities no row lowers in a
function body.

Steps 12 and 13 are not implemented yet (QSL-638): the emitter writes no
`owner` member.
