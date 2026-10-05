---
id: SR-1336
title: "Code review of quire-spec-language PR #641: B4 deletes every depth limit kind; ancestor and family steps are edge counts; FR-099 dependency limits"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@fc90c46a13308f63d21d28549dd9b30e9d745087; git diff origin/main...HEAD (PR #641, local head not yet pushed): qsl-foundation/src/diagnostic/stage.rs, qsl-package/src/checked.rs, qsl-package/src/checked_v2.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/library/bundle.rs, qsl-semantics/src/model/{accounting,dispatch,index,normalize,population}.rs, qsl-semantics/src/value/environment_stage.rs, src/command/extraction.rs, xtask/src/typestate_scan.rs, and their tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: reviews
---
# Code review of quire-spec-language PR #641

## Summary

Ticket: QSL-485 (B4). Reviewed local head fc90c46a1 (not yet pushed). The Rust
lane (rust-review) is folded into this file. No build was run; the coder's gate
is queued. Not flagged, as instructed: the qsv pin bump that follows, and the
native-v1 depth limits in src/{package,linking,checking,lowering,native_model}.rs
(the M6C-S lane). The intake `IntakeLimit::NestingDepth` and observation
`nesting_depth` belong to ADR-030 D-4.6 and D-4.10 (FR-261), not to B4's FR
set, so they are not flagged here.

Checked against the code:

- **Depth kinds deleted.** `LimitKind::NestingDepth` and its
  `nesting-depth-exceeded` cause, `PackageLimits::depth`,
  `PackageLimitKind::Depth`, `ResolutionCause::StageLimit`,
  `PackageError::ResourceLimit::actual`, `ImportRefusal::DepthLimit` and
  `DependencyLimits::depth` are gone, with no shim. A grep of the qsl-*
  crates finds no other depth limit kind in B4's scope.
- **ancestor_steps (FR-082).** `RecordIndex::ancestry` counts one edge for
  each `generals` entry of each expanded type, and each type is expanded once,
  so the total is the closure's edge count. The ordinal is the edge that first
  names `t`. This matches the `walked` oracle, which charges each edge before
  following it. `normalize::ancestor_paths` charges each distinct
  `(specific, ancestor)` edge once per root walk, however many paths reach it.
  An uninterned `s` follows no edge and answers `false` at any bound, which is
  correct. The type environment maps `AncestorSteps` to `EdgeCount`, and the
  lifecycle names `environment.ancestor_steps`.
- **family_steps (FR-083).** `FamilySteps` is one count per link, keyed by
  the redefining member. A member has one `redefines`, so the key is the edge.
  `build_family` (charged as `operation.key`), `ancestor_closure` and
  `effective_terms` (charged as `current`) key the same edge the same way. A
  link that follows an edge more than once pays for it once, and the edges
  above `original` are charged on top of it, as FR-083 says ("all those walks
  together").
- **Iterative resolution.** `check_unit` drives an explicit `Vec<Library>`
  stack of frames. `select_import` handles cycle, reuse and selection, and
  charges the limits. `begin_library` handles S1 and I1, and `finish_unit`
  handles S3 and S4. The view is read in the parent frame at `parent.next`
  before `push_import` advances it, so the region is the right import.
  `unwind` wraps a refusal under every library on the stack, innermost first,
  which matches the old recursive `wrap` chain. `self.active` still feeds
  cycle detection.
- **Iterative closure read.** `admit_closure` is a post-order walk on an
  explicit stack. A dependency is pushed above its parent's "expanded"
  marker, so it is admitted first. `read` calls `supply` on a package whose
  dependencies are already admitted, so native recursion is at most 2 deep.
  `hold_closure` uses a worklist. `Drop for CheckedPackage` drains
  dependencies through `Arc::try_unwrap` on a worklist. `CheckedPackageV2`
  (IR @3be6ff7) holds no dependency packages, so its drop does not recurse.
- **FR-099 limits.** `libraries` = `compiled + active + 1`, charged before
  the compile starts. `import_edges` is charged once for every import of
  every compiled unit, before cycle and selection. Each unit compiles once,
  so each edge is charged once. `source_bytes` sums the library sources only.
  Every refusal is `ImportRefusal::Limit(LimitExceeded)` named with its
  `LimitsField`, `stage_limit_exceeded`, at the import's identity span, stage
  `intake`, and wrapped as `Dependency` per FR-099. `Code::StageLimitExceeded`
  is `is_incomplete()`, which is exit 22. `limit_of` turns it into
  `StageFailure::Limit`.

## Verdict

Approve once the findings are fixed. No correctness defect was found. The edge
counting, the iterative resolution and closure read, and FR-099's limits and
refusals are right. The findings are about test strength and one duplicated
constant.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_dependency_chain_of_any_length_compiles_on_a_small_stack` claims that resolution, the closure read and the drop all run on a constant stack, but at 1,000 links on 512 KiB it may not catch a regression in the last two. A recursive derived drop or a recursive `hold_closure` costs a few hundred bytes per level, and 1,000 levels can fit in 512 KiB. Only the old `compile_library` recursion, with its heavy frames, is likely to have overflowed. Fix: run the chain at 4,096 (the `dependency.libraries` default), or add direct tests that drop a 100,000-long `CheckedPackage` chain and run `hold_closure` over it on a 128 KiB thread. | qsl-replay/src/spine/dependency_tests.rs:529-545; qsl-package/src/checked.rs:221-235; qsl-package/src/checked_v2.rs:836-849 |
| FND-002 | low | The refusal half of `the_checked_bridge_counts_every_walk_against_one_family_steps` does not show that the count is shared. The winner `op20`'s precondition walk alone follows all 20 edges, so `family_steps = 19` would refuse even if each walk had its own count. The doc's "refused ... even though the family walk alone fits" suggests otherwise. Only the admit-at-20 half discriminates, and it rules out double counting. Fix: use a fixture where the union of the walks' edges exceeds the bound but no single walk does, such as two branches below the root, or drop the claim. | qsl-semantics/tests/it/model_dispatch.rs:717-783 |
| FND-003 | low | The `ancestor_steps` defaults are now the literal `16_777_216` in `ModelNormalizationLimits` and `PopulationAdmissionLimits`, where before they read qsv's `DEFAULT_ANCESTOR_STEPS`. The same bound is now stated in three places. `check_and_evaluation_share_one_default_and_one_edge_count` guards that they agree. Fix: once the pin bumps, go back to `quire_semantic_value::declaration::DEFAULT_ANCESTOR_STEPS`. | qsl-semantics/src/model/accounting.rs:62; qsl-semantics/src/model/population.rs:203 |
| FND-004 | low | The `walked` test oracle is documented as "the per-call walk `conformance::type_conforms` ran before the index, kept verbatim". This PR rewrote it, together with `RecordIndex::conforms`, to charge per edge, so it is no longer verbatim. Both sides of the comparison changed in one commit. Fix: describe it as the reference edge-charging walk of FR-082. | qsl-semantics/src/model/index.rs:763-764 |
