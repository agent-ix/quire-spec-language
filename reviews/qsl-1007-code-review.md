---
id: SR-12350
title: "code review and Rust review of QSL-1007 derived identity projection (QSL PR #1268)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@785c72b580d0734d9909437fa6d4058df88ba241; qsl-package/src/checked_v2/tests.rs"
review_set: subset
---
# SR-12350: code review and Rust review of QSL-1007 derived identity projection (QSL PR #1268)

## Summary

Ticket: QSL-1007 (STD-288 code half). The PR changes one file, the test module
`qsl-package/src/checked_v2/tests.rs`. It adds three helpers:

- `with_derived_projection` builds the wire for IR's reader.
- `derived_package_id` gives the package id.
- `fixture_wire_bytes` gives the canonical bytes.

All three derive `identity_projection` from `semantic_graph.nodes`. The rust-review lane is folded
into this file. The PR touches no production code.

The derivation matches FR-322 exactly. It takes every node in graph order, removes only
`occurrences`, and does not sort. It reuses `projection_of`, the helper the envelope builder already
uses. IR's reader rebuilds the projection with `CheckedNodeProjectionV2::from` and compares it member
by member, so it serves as an independent oracle.

Mutation probes run by the reviewer on this head, with `QSPEC_DIR` at QSpec #371 `ff52d3e8`:

- Keep `occurrences` in the projection: the I2 read refuses `UnknownMember` at
  `/identity_preimage/identity_projection/0/occurrences`.
- Reverse the projection order: refused `InvalidPackage` at `.../identity_projection/0/owner`.

Both probes fail `conformance_i2_read_over_qspec_checked_package_v2_fixtures` and
`conformance_dependency_selection_vectors`.

Recomputing `package_id` for a graph mutation is needed, and it hides nothing. Without the
recompute, `unknown-semantic-node-tag` refuses `StaleDependency`/`content-mismatch` at
`/package_id/digest`, and the mutation's own cause is never reached. With the recompute, the
mutation is the only defect on the wire, so its expected code is what the test proves. The three
mutations that do not touch the graph keep the published id.

Installing the projection on the test wire is a legitimate temporary shim, not a mask. IR's model
still has `identity_projection` as a required `deny_unknown_fields` member
(`quire-contract-model` `checked_package/v2/mod.rs:318`), and IR-975 removes it. The shim takes the
projection only from the nodes, never from a shipped copy, and IR checks it against its own
derivation. Positive admission still proves that the published `package_id.digest` equals the JCS
SHA-256 of the selection object plus the derived projection, because IR compares
`package_id.digest` before it validates the graph.

Runs with `QSPEC_DIR` at #371:

- `cargo test -p qsl-package --lib`: 232 passed.
- The conformance subset (6 tests): passed. 12 positive fixtures (11 in non-ascending node order),
  6 adverse mutations, 11 + 3 dependency-selection vectors, 37 frame vectors and 183 source-map
  entries.

The same subset also passes against QSpec `main` `5ee0071f`, because the derived copy overwrites
the shipped projection. So the PR does not depend on which of the two merges first.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `with_derived_projection` silently returns the envelope unchanged when `semantic_graph.nodes` is not an array or `identity_preimage` is not an object. The wire then has no projection, so IR refuses it for the missing member, not for the defect under test. `projection_of` in the same helper chain uses `expect("node object")`. Use `expect` here too, so a malformed test input panics in the harness. | qsl-package/src/checked_v2/tests.rs:1367-1378 |
| FND-002 | low | The rewritten doc comment of `conformance_i2_read_over_qspec_checked_package_v2_fixtures` still says "Three of the five fixtures". Measured on #371, it is 11 of 12. The rewritten line is also past the module's comment width. | qsl-package/src/checked_v2/tests.rs:1567-1570 |

## Verdict

Two low findings. Neither blocks the merge. The derivation, the shim and the recomputed mutation ids
are correct, and the probes above show the tests can fail. The reviewer ran
`cargo clippy -p qsl-package --lib --tests -- -D warnings` and `cargo fmt --check -p qsl-package`,
and both are clean.
