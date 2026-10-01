---
id: SR-952
title: "QSL-358 slice 1 code review (with rust-review lane and test-oracle check) of PR 565, the quire-semantic-value no_std leaf"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a7df1ff07f56e2b37249841138600c86c1b3d1b5; diff d1c5fa47...a7df1ff0; quire-semantic-value/**; qsl-semantics/src/value/{unit,semantic_node,mod}.rs; qsl-semantics/tests/it/{identity_golden_vectors,quantities}.rs; qsl-eval/src/value/expression/evaluate.rs; tools/arch-lint/{api_surface,metadata,duplicate_revisions,canonical_encoder,main}.rs; xtask/src/{import_graph,string_edge}.rs; tests/it/family_outcome_layering.rs; Cargo.toml; Cargo.lock; Makefile; crate manifests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 1). PR: quire-spec-language#565 at a7df1ff0, diff
`d1c5fa47...a7df1ff0`.

The coder's claims, checked against the code:

- **Seam gone, one minting path.** No `CompoundUnitMint` or `OnceLock` remains
  in `quire-semantic-value` or `qsl-semantics/src/value`. The only
  `quire.value.compound-unit/v1` encoder is `quire_semantic_value::unit::compound_unit_id`.
  `CompoundUnit::id` and `CompoundUnitPreimage::id` both call it. `compound_unit_id`
  is now `pub` where `compound_id` was private, but the kernel's `UnitId::compound`
  is already `pub`, and on main `CompoundUnitPreimage::from_json(..).id()` already
  computed an id from arbitrary bytes. No new authority. Confirmed.
- **No re-export shims.** No `pub use quire_semantic_value` anywhere. The
  `qsl-semantics` `value` re-exports name only its own items
  (`admit_unit_graph`, the three preimage types). Confirmed.
- **Refusal order.** `admit_unit_graph` runs, in order: per-dimension
  `check_terms`, `DuplicateNode`, `retains`; per-unit `check_semantics`
  (unreduced, then `UnitEdge::checked`: zero scale, then non-identity root),
  `DuplicateNode`, `retains`; then `from_checked_nodes` (dimension maps, unit
  dimension resolution, target resolution, cross-dimension, missing/duplicate
  root, cycle); then owner; then stale key. That is main's `UnitGraph::admit`
  order, line for line. Tests pin it: `u11_invalid_topologies_refuse_admission`
  stores the cycle nodes under stale keys and expects `MissingRoot`/`TargetCycle`
  (topology before stale), `stale_keys_and_foreign_owners_refuse_admission`'s
  foreign owner also makes the key stale and expects `OwnerNotSelected` (owner
  before stale), and the generated-graph `zero` mutation expects `ZeroScale` over
  the stale key (per-node before stale). Confirmed.
- **Ids byte-identical.** `a_runtime_compound_unit_carries_the_golden_compound_unit_id`
  admits the golden dimension and unit, builds `metre^2` both from the golden
  preimage's terms and by `result_unit(Multiply, metre, metre)`, and compares both
  to the unchanged `COMPOUND_UNIT_DIGEST` constant. A real oracle. The existing
  `dimension_unit_and_compound_unit_digests_match_their_golden_vectors` also still
  passes through the new path. Confirmed.
- **quire-canonical.** Workspace entry is `branch = "main"`,
  `default-features = false`. One lock entry, `9572a216`. `qsl-semantics` alone
  turns on `std` (for `model::key`'s `WriteSink`, the only `std`-gated item any
  crate uses). Confirmed.
- **`CompoundUnit::dimensionless` deleted.** No caller anywhere. Confirmed.
- **Six quantity items made `pub`.** `has_dimension_of`, `converts_to`
  (`value::declaration`), `UnitScope::get`, `UnitOperation`, `result_unit`,
  `check_comparable` (`check::check`) are each used from `qsl-semantics`.
  Confirmed.
- **Leaf discipline.** `[dependencies]` are exactly `quire-exact`,
  `quire-canonical`, `serde` (no default features, `alloc`) and `thiserror` (no
  default features); TC-390 asserts that set. `#![no_std]` + `extern crate alloc`.
  The gate log shows `cargo build --locked -p quire-semantic-value --target
  thumbv7em-none-eabi` compiling `quire-canonical` and the crate, and
  `make ci` runs it as `quire-semantic-value-no-std`. Confirmed.
- **T-12 allow-list "gap" (coder's note).** Not a hole. `AllowedCaller` already
  carries `crate_src` as well as `module_prefix`, and `module_allowed` requires
  both. A module named `check` in `quire-semantic-value/src` is not
  `qsl-semantics/src`'s `check` and is reported. The T12-E test already plants a
  same-path module in another crate and sees it reported. No change needed.

Rust-review lane: `#![forbid(unsafe_code)]`, `missing_docs` warn, clippy `all`
deny. One `panic!` in `compound_unit_id`, unchanged from main and documented (only
a failed heap reservation reaches it). No integer conversions added.
`UnitGraph::from_checked_nodes` is the one new public constructor, see FND-001.

Gate: the coder's `make ci` log at a7df1ff0 ends `head=a7df1ff0... exit=0` and
shows the four new tests passing. I did not re-run it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `UnitGraph::from_checked_nodes` is a new public constructor that trusts its caller for the per-node checks. `DimensionNode` and `UnitNode` have `pub` fields, so a caller can pass dimension terms that `check_terms` would refuse (a zero exponent, a repeat, unsorted). A zero exponent then lands in `Dimension`, whose own doc says "no exponent is zero". `declared_unit_id` still documents "admission checked that key against its `quire.unit-node/v1` preimage", which is false for a graph built here: no key or owner is checked. This is the constructor RT will call. Give `DimensionNode` private fields and a `checked(terms)` constructor that runs `check_terms`, as `UnitEdge::checked` does for edges; `admit_unit_graph` calls it where it now calls `check_terms`, so the refusal order is unchanged. Reword `declared_unit_id` and `from_checked_nodes` to say key provenance is the caller's (QSL's `admit_unit_graph`). | quire-semantic-value/src/unit.rs:213-234,271-287,304-310 |
| FND-002 | medium | Two copies of the `node-id` JCS shape in one repo. `quire-semantic-value/src/unit.rs` has a private `CanonicalNodeId { digest, domain }` plus a `DigestHex` spelling, and `qsl-semantics/src/value/semantic_node.rs` keeps its own `pub(crate) CanonicalNodeId` with `From<NodeKey>` and `From<WireNodeId>`. Both encode the same preimage member, and node keys and compound ids agree only while both stay identical. SV is the lower layer, so it should own the one type: make SV's `CanonicalNodeId` `pub` with `From<[u8; 32]>` and `From<NodeKey>`, delete the `qsl-semantics` copy, and convert a `WireNodeId` with `CanonicalNodeId::from(*id.as_bytes())` (the orphan rule rules out a `From<WireNodeId>` impl there). | quire-semantic-value/src/unit.rs:506-536; qsl-semantics/src/value/semantic_node.rs:169-191 |
| FND-003 | low | Dropping the `OnceLock` triples the compound-id hashing on the evaluator's quantity path. For each evaluated `*` or `/` of quantities, `evaluate_quantity_unit` calls `unit.id()` (JCS encode + SHA-256), then `UnitScope::form` calls `unit.id()` again, then `UnitTable::insert` calls it a third time when the unit is not the package's. Main hashed once per evaluation. `NodeKind::Quantity` runs once per node per evaluation, so loops and simulation pay it each step. Have `form` take the id the result quantity already carries (`quantity.unit()`), and give `UnitTable` a crate-private insert that takes a known id. | quire-semantic-value/src/quantity.rs:108-112,179-186,326; qsl-eval/src/value/expression/evaluate.rs:1093-1098 |
| FND-004 | low | Two code docs say SV depends on K only: `lib.rs` ("It depends on `quire-exact` only") and arch-lint's `SHARED_LEAVES` doc ("which depends on K only"; the `edge_repo` doc adds "no ecosystem crate"). SV also depends on `quire-canonical`, an agent-ix crate, plus `serde` and `thiserror`. Say it depends on K and ADR-013's one encoder, and on no QSL layer. | quire-semantic-value/src/lib.rs:5-6; tools/arch-lint/metadata.rs:13-24 |

## Verdict

The move is correct. Every coder claim holds when measured, the compound id is
pinned to the golden vector by a real test, refusal order is unchanged and tested,
and the no_std build is a genuine gate. The T-12 allow-list concern is not a hole.
FND-001 and FND-002 should be fixed in this PR. FND-002 is the duplicate the
brief asked about; unifying it in SV is the right call. FND-003 and FND-004 are
low and cheap.

## Dispositions

Round 1, reviewed at 9b75cc5e0fa39a7eacd5959fb884ad1d9a9df311. The branch was rebased onto main 0e3426e9; `git range-diff d1c5fa47..a7df1ff0 0e3426e9..dea56798` shows the slice commits unchanged apart from #561's rename context in `qsl-package/Cargo.toml`, so the fix round is commit 9b75cc5e alone. I did not run the gate; it is running separately.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9b75cc5e: `DimensionNode` and `UnitNode` have private fields and `checked()` constructors (`check_terms`; zero scale then non-identity root, with root-ness taken from the node's own `target`); `UnitEdge::checked` is private. `admit_unit_graph` calls them where it called `check_terms` / `UnitEdge::checked`, so refusal order is unchanged. `from_checked_nodes` and `declared_unit_id` docs state key provenance as the caller's. Two new unit tests in SV pin both constructors' refusal order |
| FND-002 | fixed | 9b75cc5e: one `pub CanonicalNodeId` in `quire_semantic_value::semantic_node` with `From<[u8; 32]>` and `From<NodeKey>`; the `qsl-semantics` copy is deleted and its callers use `CanonicalNodeId::from(*id.as_bytes())`. `DigestHex` spells bytes as `{byte:02x}`, the same as `WireNodeId`'s and `NodeKey`'s `Display`, so preimages are byte-identical |
| FND-003 | fixed | 9b75cc5e: `IdentifiedUnit` carries the id; `evaluate_quantity_unit` returns it, `UnitScope::form` takes it and `UnitTable::insert_identified` stores it without rehashing, so a compound `*` or `/` hashes once per evaluation (check stage: once per node) |
| FND-004 | fixed | 9b75cc5e: `lib.rs` and the `SHARED_LEAVES`/`edge_repo` docs name K, `quire-canonical`, `serde` and `thiserror`, and no QSL layer or IR/RT/CG crate |
