---
id: SR-9119
title: "Code review of quire-spec-language PR #842: chain-link model identity"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2719844b3826b23067e57f51d811dfed7e36717d; qsl-semantics/src/model/{normalize,links,key,refusal,accounting}.rs, qsl-semantics/src/check/lowering/model.rs, qsl-foundation/src/digest.rs, qsl-semantics/tests/it/model_identity_*.rs, identity_golden_vectors.rs, model_normalization.rs, model_population.rs, model_systems.rs, model_dispatch.rs, qsl-eval/tests/it/model_reference_queries.rs"
review_set: subset
---
# Code review of quire-spec-language PR #842

## Summary

Ticket: QSL-670. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...HEAD` (commits 0e508202c, 1da8213f9, 2719844b3).
Oracle: QSL FR-081 (chain-link section, AC-9 to AC-13), NFR-012, TC-1022 to
TC-1025, and the merged QSpec pair (cited by id only).

Examined:
- Type preimages: qualify fact, then one inherit fact per deduplicated direct
  parent in ascending key order, linked to the parent's identity; hashed in
  topological order (min-heap Kahn pass), parents first. NFR-012 explicitly
  allows a type identity to be hashed in phase 3 before its phase-5
  `normalize.hash` (examined)
- Member preimages: qualify only when the type declares the member; inherit
  facts one per direct parent that holds the member, inputs `[P, m]`, parent
  filled after the parent member is hashed (fixed-width placeholder while
  lengths are charged) (examined)
- Phase 4: redefine and subset facts carry `[r, t]` / `[s, t]` with no path,
  sorted by inputs then rule bytes (redefine before subset);
  `normalize.redefinition-check` `m + r`; `normalize.conflict-check`
  `(c-1) x sum f(o)` with `f(o) = 1 + direct parents`. Recomputed by hand for
  the N06 admitted model (68 + 11 + 14 = 93) and the N12 case (76); both match
  the tests and the value-accounting note that a type's derivation facts are
  its retained facts (examined)
- `cycle_walk` runs only for types the topological pass leaves unplaced;
  explicit stack; `Lineage` (proper ancestors, least path) and `links.rs`
  (`expand_inherit`, `least_path`) are explicit-stack. No recursion
  proportional to chain depth in normalization or the reader; N19 runs the
  10,000-edge chain on a 524,288-byte stack in a child process and passes
  (examined)
- Determinism: all iteration that reaches output is over sorted keys, sorted
  identities or BTreeMaps; HashMaps are lookup-only. N07 reverses node order
  and asserts the oracle view digest (examined)
- Strict reader `Links::new`: refuses an inherit fact with no link, a link on
  another rule, an absent parent, wrong kind, wrong owner, wrong original
  (`MisWiredLink`) and a repeated parent (`DuplicateParent`); both causes
  map to `invalid_model_binding` in `catalog_code` and the refusal tests
  (examined)
- Consumers by signature: `git grep` over the workspace (crates, xtask,
  examples, fuzz, qsl-package, qsl-replay, schemas) finds no `/v1`
  effective-declaration, effective-view, object-universe or inherit/subset/
  redefine rule string outside historical review/ADR prose; `FactInputs` and
  `KeyPath` have no remaining user; the only non-normalize reader of
  `preimage.derivation` is `AdmittedModel::assemble`, which now keeps direct
  supertypes and walks them iteratively (examined)
- Panics in non-test code: new code adds no `unwrap`; indexing in
  `topological_order` and `MemberStore` is bounded by construction (examined)
- qsl-eval L07/L12 and model_population L01: sorting the expectation by the
  reference's own `Ord` (universe, type, object) is legitimately
  order-independent; the full sequence is still pinned, and the dropped
  `last()` check followed only from the old digest order (examined)
- Gate: `/home/peter/dev/worktrees/logs/qsl670c-ci2.log` ends `EXIT 0`,
  written 12:40 after the head commit (11:13); worktree clean at 2719844b3;
  `model_identity_full_size::ten_thousand_edge_chain ... ok` is in it, not
  `#[ignore]`d (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** No wrong identity, digest, charge or refusal, no panic or stack
overflow, no failed AC, vacuous test, circular oracle or broken gate found.

Recorded, not reported:
- medium: `ModelIndex::build` (index.rs `type_fact_counts`) still counts
  every inheritance path for `f(x)`. Production conformance reads the
  view-built index (`completed_view`, retained-fact count), so no charge is
  wrong today, but `EffectiveView::with_operation_bodies` rebuilds its index
  through `ModelIndex::build`, so that view carries the old path count.
- medium: the kernel crate quire-exact (locked d41a1da) still exports
  `EFFECTIVE_ID_DOMAIN` and `UNIVERSE_ID_DOMAIN` as the `/v1` strings and
  documents `EffectiveId`/`UniverseId` as `/v1`. QSL does not read them;
  a consumer that labels with them would emit a `/v1` label on a `/v2`
  digest. Needs a quire-exact change.
- low: `DerivationConflict.paths` stays empty; the expanded paths are only in
  `detail`, and no test asserts them.
- low: `cycle_walk` charges one `normalize.fact` per path extension for a type
  below a cycle, not one per direct edge; only observable as Incomplete vs
  Refused under a tight `derivation_facts` on a package that is refused anyway.
- low: `AdmittedModel::is_subtype` is now a graph walk per call (was a set
  lookup).
- low: `Links` trusts each entry's `effective_id` without rehashing, so a
  tampered view with a link cycle would not terminate in `expand`; the reader
  has no production caller yet.
