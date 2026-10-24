---
id: SR-12165
title: "Code review of quire-spec-language PR #1005: library chain memory and time"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d191db7c102846b24c9a92121ebe59e650885b2e; PR #1005 diff against origin/main: qsl-package/src/checked.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1005

## Summary

Ticket: QSL-875. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (3 files, +148/-28, five commits).
Read-only: nothing was built or run, at the dispatcher's instruction. The
coder reports qsl-package, qsl-replay, xtask, the chain test, clippy
`-D warnings` (all features) and fmt green, and measures the chain test at
16.55 GB / 86 s before and 1.25 GB / 34-37 s after; none of this was re-run
here.

Checked and clean:

- Stale memo (ADR-013 O-02). `CheckedPackage` is not `Clone` (it derives
  `Debug` only) and has no `Default`. Its fields are private to
  `checked.rs`, and every constructor (`link`, `link_with`, the test-only
  `dependency_chain`) starts the memo empty with `OnceLock::new()`. The only
  `&mut self` methods are `bind_abstraction`, which resets the memo, and
  `Drop`. The emission does not read the abstraction relation (`emit.rs`
  never names it), so the reset is conservative, not load-bearing. A
  package reached through `Arc` cannot be mutated while a dependent holds
  it. `CheckedGraph` keeps no interior state the emission depends on:
  `memoized_regions` is a pure function of the graph. `remember_emission` is
  `pub(crate)` and called from one site, `emit_checked_with_cancel`, after a
  successful emit. A refusal or a cancellation stores nothing. A race between
  two threads stores one of two identical emissions. So a memoised id is
  always the emitter's own output for exactly this content.
- Other emit entry points. `emit_package` (caller regions),
  `emit_package_with_fault`, `emit_package_decoding` and
  `emit_checked_with_fault` neither read nor write the memo, so a
  caller-region or fault-injected emission can never be served as
  `emit_checked`'s.
- `recomputed_package_id` reads the memo, else emits, and applies the same
  `omitted` refusal to both. `LinkRefusal::DependencyEmission` cannot change:
  a refused emit is never memoised.
- Diamond rule (FR-307). `unify` compares `package_id` only, before and
  after, so no selection unifies or refuses differently. The first path to
  reach an identity is still the one kept.
- Path rebuild. `path(identity)` walks `Via { first, package }` to
  `package.selections[identity]` and stops at `Direct`, pushing `identity`
  last. This is the old construction `[import.identity] ++
  import.package.selections[identity].path`, unfolded, so the displayed
  `ConflictingDefinition` paths are identical and complete. The `Display`
  impl calls `path(.identity)` with the refusal's own identity.
- Wire and identity preimage. `emit.rs::dependency_selections` writes
  `identity` and `package_id` only. The path was never on the wire or in the
  lock or preimage, so no package id, lock or preimage comparison changes.
  `read_import_view` and `checked_v2.rs` are untouched; `checked_v2.rs` reads
  only `package_id` from a selection.
- Drop (ADR-030 D-1). `release_into` moves both `dependencies` and every
  selection's `Via` `Arc` onto the explicit stack, and the re-entered
  `Drop` of an unwrapped package finds empty maps. A `Via` package was
  linked before its dependent, so no `Arc` cycle can form. `Emission` holds
  no `CheckedPackage` (`own_evidence` adds features only), so the memo adds
  no cycle either. A `ResolvedDependency` cloned into a `LinkRefusal` drops
  its `Arc` through the same iterative `Drop`.
- Cancel polling (FR-276). `emit_checked_with_cancel` always emits and
  polls, so `a_cancel_inside_package_stops_within_one_charge` (which runs
  `package` twice over one `CheckedPackage`) sees the same charges.
  `emit_checked` and `link_with` used `Cancel::new()`, a handle nobody
  cancels, before this PR too, so serving them from the memo removes no
  poll that a caller's handle could see.
- Test changes. The three edits in `emit/tests.rs` replace `.path` with
  `.path(identity)` using the selection's own map key or refusal identity,
  and compare against the same expected vectors. They are semantically
  identical. No `#[trace]` tag is added, removed or moved.
- The memory and time claims are plausible. The old paths held about n^3/6
  `LibraryName`s (about 1.7e8 at n = 1,000). The new closure holds O(n^2)
  small entries plus each package's own emission. The time saved is the
  second emit of every library in its dependent's `link_with`.
- No unrelated change, no compat layer, no new dependency, no `unsafe`, no
  panic surface. `let _ = self.emission.set(..)` correctly ignores a lost
  race between equal values.

## Verdict

PASS. The change keeps every package id, refusal and identity. Four low
findings are recorded below. They do not block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ResolvedDependency`'s derived `PartialEq` now compares `package_id` plus the first hop (`DependencyPath::eq`), so `a == b` no longer implies `a.path(id) == b.path(id)` for this public type. No production code compares selections. `tests/support/front_end.rs` still compares the whole path by induction because it walks every dependency pair. Document on `ResolvedDependency` that equality covers the first hop only, or drop path from its equality. | qsl-package/src/checked.rs:166-170; qsl-package/src/checked.rs:209-221 |
| FND-002 | low | `ResolvedDependency::path` takes the identity as a separate argument. A caller that passes a different identity gets a wrong path, and the `None => return path` arm, unreachable for a well-formed closure, would silently return a path without its last element. Use `expect`/`debug_assert` on that arm, or keep the identity in the type. | qsl-package/src/checked.rs:181-193 |
| FND-003 | low | `remember_emission` stores a clone of the `Emission`, so every emission a caller keeps (for example lifecycle `emit_unit`'s) holds its bytes twice, and a memo hit in `emit_checked` clones them again. `CheckedPackage`'s derived `Debug` now also prints the memoised bytes. Holding `Arc<Emission>` would avoid both copies. | qsl-package/src/checked.rs:157-161; qsl-package/src/checked.rs:464-466; qsl-package/src/emit.rs:1003-1005 |
| FND-004 | low | `recomputed_package_id`'s doc still says "recomputed by emitting its package". It now reads the emitter's memo when there is one. Say so, so that the ADR-015 D-2 reading ("every `package_id` is recomputed") is explicit: emitter-computed, never caller-supplied. | qsl-package/src/checked.rs:501 |
