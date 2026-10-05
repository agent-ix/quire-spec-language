---
id: SR-1298
title: "Code review of PR #632 (IR-582: depend on the extracted quire-exact and quire-semantic-value repositories)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f40729a9a9adafaeb233fadcb938245f2861776c; Cargo.toml, Cargo.lock, deny.toml, Makefile, */Cargo.toml, tests/it/family_outcome_layering.rs, tests/it/layer_crate_reexports.rs, tools/arch-lint/{api_surface,canonical_encoder,graph,main,metadata,qualified_core}.rs, xtask/src/{canonical_types,definition_scan,string_edge,typestate_scan}.rs, quire-exact/**, quire-semantic-value/** (deleted)"
review_set: subset
---

## Summary

Ticket: IR-582. Code review with the rust-review lane over
`git diff origin/main...HEAD` (merge base 47dd209e7; one commit behind main
02530e7a5).

Checked:

- Absence: `git ls-tree -r HEAD --name-only | grep -E '^quire-(exact|semantic-value)/'`
  is empty. No `path =` dependency on either crate remains in any
  `Cargo.toml`; every consumer uses `{ workspace = true }` over the two
  `[workspace.dependencies]` git entries at `branch = "main"`.
- One copy each: `Cargo.lock` has exactly one `quire-exact` package
  (git source, 1098e44e) and one `quire-semantic-value` (787d87bb), and
  `quire-semantic-value`'s dependency entry names the same `quire-exact`.
  The new `deny.toml` `deny-multiple-versions` entries keep it that way;
  they follow the existing `quire-canonical` entry and pass the value test
  (a second copy would split `Value`/`NodeKey` across two types).
- #631 (main 02530e7a5) touches qsl-replay/qsl-route only, names no
  deleted path, and its new import `quire_semantic_value::location::Origin`
  exists as `pub enum Origin` in quire-semantic-value main
  (`src/location.rs:20`).
- No compatibility layer, shim or re-export of the old in-tree crates.
- `metadata.rs`: `SHARED_LEAVES`/`edge_repo` deleted; `classify` returns
  `None` for an own-repository source, so FB-05/FB-11 edge extraction is
  unchanged. Good deletion.
- `qualified_core.rs`: `CORE_CRATES` drops the leaves. The direction check
  walks each core crate's resolved closure, so the leaves' own dependencies
  are still walked through `qsl-foundation`/`qsl-semantics`.
- Makefile: the two no_std targets and the `-p quire-exact` lane move to
  the new repositories (`build-no-std` is in each repo's `make ci`).
- `layer_crate_reexports`: counting the leaves keeps the root crate from
  becoming a second public path to kernel items, the same check it applied
  while they were path dependencies. It does something real; keep it.
  `canonical-types`' re-export rule does not cover this, because it skips a
  `pub use quire_exact::X` rooted at the owner crate.
- `xtask canonical-types` `SHARED_LEAVES`: `scan` falls back to the
  absolute `src_path` (`strip_prefix(...).unwrap_or(root)`), so it reads the
  leaves' source from the cargo git checkout. Without the list the leaves
  classify as nothing and their 15 `quire:canonical` types drop out of the
  canonical set, so QSL could add a copy of `Value` or `NodeKey` without a
  finding. The scan does something real (see FND-001 for the problems).
- Gate: `~/dev/worktrees/logs/qsl-ir582-make-ci.log` ends `exit=0`, run after
  the HEAD commit. A focused `cargo xtask canonical-types` run was added
  (canonical-types is not in `make ci`): it reads the leaves from
  `~/.cargo/git/checkouts/quire-exact-*/1098e44/src` and
  `quire-semantic-value-*/787d87b/src`, and reports 30 `identifier` findings.
  Most are known QSL debt from before this PR (FR-272: the gate stays on
  demand until it reports nothing). One of them, `Origin` at
  `quire-semantic-value/.../src/location.rs:20` against quire-exact's
  `src/location.rs:69`, sits entirely inside the two other repositories.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `canonical_types` `SHARED_LEAVES` treats two other repositories' crates as QSL workspace members in a QSL run (so QSL's gate applies `identifier`/`tag` rules inside quire-exact and quire-semantic-value source, which QSL cannot fix) and labels them `quire-spec-language` in any other run. It should classify each leaf as its own ecosystem repository and drop `in_qsl_workspace` | xtask/src/canonical_types.rs:55-58,104-128 |
| FND-002 | low | No test exercises the new leaf branch: the TC-747 fixture still sources `quire-exact` from `QSL_SOURCE`, which `classify` already maps to QSL, and the real-workspace `ValueType` assertion would pass under either label | xtask/src/canonical_types.rs:1144-1170 |
| FND-003 | low | `layer_crate_reexports.rs` module doc still says the layer set is "every workspace crate the root crate names as a `path` dependency"; only the fn doc was updated | tests/it/layer_crate_reexports.rs:8-15 |
| FND-004 | low | Shipped comments cite files and line ranges in the old in-tree path (`quire-exact/src/accounting.rs:518-529`, `quire-exact/src/integer.rs`, `quire-exact/src/accounting.rs`); those files now live in another repository and the line ranges will drift. Name the item (`quire_exact::Meter`, `quire_exact::integer::Integer`) instead | qsl-replay/src/spine/clause.rs:391; qsl-semantics/src/check/family.rs:255; qsl-semantics/src/value/mod.rs:136 |
| FND-005 | low | Rewrapped doc comments run past the 100-column line width (joined lines left unwrapped) | tests/it/family_outcome_layering.rs:243,247; tools/arch-lint/api_surface.rs:1248,1250 |

## Verdict

Changes requested (no high). The focused canonical-types run backs FND-001:
QSL's gate reports a namesake inside quire-semantic-value against quire-exact,
which only those repositories can fix. The extraction is complete and verified by
absence, the lock holds one copy of each crate, there is no compatibility
layer, and every deleted scan root is either covered elsewhere or recorded in
the gap-analysis SR. FND-001 is the one design issue. Keep the canonical-types
scan of the leaves in QSL: the gate exists to scan ecosystem dependencies,
and the leaves cannot scan their consumers. But label each leaf as its own
ecosystem repository, not as QSL's workspace and not as `quire-spec-language`.
Under FR-272's own DT-5 rule a type owned by another repository is held by the
`copy` rule, as IR's types already are. The leaves' internal one-definition
check then belongs in their own repositories.

## Dispositions

Round 1, reviewed at c989d92266f8025c27bfdbe0c65bcc54a331f9e8 (rebased on
main 02530e7a5). Absence, one-copy lock and no `path =` dependency re-checked
at this head.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e36d68e99: each leaf is `Repository::Ecosystem(<crate>)` wherever it is sourced from; `in_qsl_workspace` deleted |
| FND-002 | fixed | e36d68e99: `tc_747_shared_leaves_are_their_own_ecosystem_repositories` sources quire-exact from its own repository URL and quire-semantic-value from a path, asserts both labels, and checks `copy` versus no finding; TC-747's backend fixture now uses `EXACT_SOURCE` |
| FND-003 | fixed | c989d9226: the module doc names the shared leaves beside the path dependencies |
| FND-004 | fixed | e36d68e99: comments name `quire_exact::Meter`, `quire_exact::integer::Integer` and `quire_exact`'s accounting types |
| FND-005 | fixed | e36d68e99: the joined lines are rewrapped under 100 columns; the remaining long lines (api_surface.rs:1251, family_outcome_layering.rs:262) were already on main |
