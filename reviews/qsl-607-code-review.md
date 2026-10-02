---
id: SR-1221
title: "Code review of quire-spec-language PR #601: quire:canonical tags and cargo xtask canonical-types (FR-271, FR-272)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e83cc06c14295817b0bbccfc6c13e0c580f7732b; PR #601 diff against origin/main: xtask/src/canonical_types.rs, xtask/src/definition_scan.rs, xtask/src/error.rs, xtask/src/lib.rs, xtask/src/main.rs, xtask/src/typestate_scan.rs, and the 37 `/// quire:canonical` tag sites in qsl-cst, qsl-eval, qsl-foundation, qsl-package, qsl-replay, qsl-semantics, quire-exact and quire-semantic-value"
review_set: subset
---
# Code review of quire-spec-language PR #601

## Summary

Ticket: QSL-607 (slice D2 of QSL-13). The PR adds the `/// quire:canonical`
doc tag on 37 types and `cargo xtask canonical-types [<workspace>]`. Per
ADR-032 R-1 the gate is not in `make ci`. The PR also deletes
`SEMANTIC_VALUE_NAMESAKES` and the tagged names from `definition_scan`'s
per-test lists.

Examined:
- The `packages` reader over `cargo metadata`: shipped target kinds,
  nested-root folding, and ecosystem classification by FR-059's `classify`.
- `FileScan`: module-level definitions, the tag reader, misplaced tags in
  fns, impls, fields and variants, and `pub use` leaves.
- `scan`'s four rules.
- The `definition_scan` deletions.
- The tags: each sits on a `pub` struct, enum or alias, and the run gives no
  `tag` finding.
- Rust lane (rust-review): no `unwrap` on a production path. The `expect` in
  `main` was already there. The new `Error` variants map to `Code::Metadata`
  (exit 2) or `Code::CanonicalTypes` (exit 1) in exhaustive matches. There is
  no `unsafe`.

Run at this head over the workspace: exit 1, 52 findings (42 `identifier`,
10 `re-export`). All 10 `re-export` findings are FND-002.

Team-leader questions:
- **Facade re-exports.** A re-export through another workspace crate is
  reported in one hop. Any `pub use` whose first segment is not the owner
  crate is a finding, which is what DT-2 asks, and `pub use v::Value` is
  tested. Two cases are not followed. The first is a glob `pub use x::*`:
  `use_tree` ignores `UseTree::Glob`, and there is no cross-crate glob in
  shipped code today. The second is FND-002's relative-module root. Following
  globs at crate granularity is small (FND-004), so do it here rather than
  narrowing the ADR prose.
- **`make ci` coverage of the removed names.** This is consistent with
  ADR-032. DT-6 says the `definition_scan` lists "are replaced by DT-1 and
  DT-2", and R-1 runs the replacement on demand until M-6d with "its findings
  until then are the DT-7 work list". R-1 also requires the gate to report
  nothing before it joins `make ci`, so any namesake added in the meantime
  must be resolved first. Keeping the old SV `Location`/`Origin` assertions
  would now contradict the tags, because `quire-exact` owns those types and
  SV's are namesakes. Not a finding.

Gates at this head (worktree-local target dir): `cargo test -p xtask` passed
(98 tests), and clippy `-D warnings` and fmt were clean.

## Verdict

Changes requested. FND-001 is medium (team-leader ruling), FND-002 and
FND-003 are medium, and FND-004 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `tools/arch-lint/graph.rs` is pulled into xtask with `#[path = "../../tools/arch-lint/graph.rs"] mod graph;` under `#[allow(dead_code)]`. That compiles arch-lint's module, and its TC-156 tests, a second time inside xtask. Team-leader ruling: not accepted. Replace it with an arch-lint lib target that exports `graph::classify` and `Repo` once #599 (LC6) merges, and depend on it from xtask. | xtask/src/canonical_types.rs:45-50 |
| FND-002 | medium | The `re-export` rule takes a 2018-edition relative path's first segment (a local module name) as a crate root. `is_local_root` knows only `""`/`crate`/`self`/`super`. So a crate re-exporting its own namesake from a child module, `pub use input::Value` in `src/state/mod.rs`, gets a `re-export` finding as well as the namesake's `identifier` finding. All 10 `re-export` findings in today's run are of this kind: `src/state/mod.rs:14-15`, `src/temporal.rs:43` x2, `src/protocol_artifact/mod.rs:38`, `src/protocol_artifact/v2/mod.rs:11`, `src/protocol_artifact/v3/mod.rs:11`, `qsl-foundation/src/diagnostic.rs:10`, `qsl-eval/src/simulation/mod.rs:27` and `qsl-replay/src/lib.rs:58`. Each is a mislabelled duplicate in the DT-7 work list. Fix: treat a root as a crate only when it is a scanned package's `crate_name` (or another extern crate). Treat any other root as local, as the code's own comment intends. Add a fixture with `mod m; pub use m::Value;`. | xtask/src/canonical_types.rs:686-708; xtask/src/canonical_types.rs:715-718 |
| FND-003 | medium | In a DT-5 backend run, the `re-export` rule fires on the backend's own members for a canonical type owned by an ecosystem crate. The only skip is `repository != Workspace` on the re-exporting package. So a backend's `pub use qsl_replay::Integer`, which goes through QSL's FB-05 facade, is reported as a re-export not through `quire_exact`. ADR-032 scopes DT-2 to "inside the owning repository", and DT-5 applies only DT-3 to a backend's own code. Fix: skip the rule when the owner's repository is not `Workspace`, and add the facade re-export to the TC-747 backend fixture as a no-finding case. | xtask/src/canonical_types.rs:686-698 |
| FND-004 | low | A glob re-export is never seen (`syn::UseTree::Glob(_) => {}`), so `pub use v::*` from a non-owner workspace crate that re-exports a canonical type escapes DT-2. There is none in shipped code today. Follow it at crate granularity: record non-local glob roots, and report one when the root is a workspace package other than the owner that defines or `pub use`s that canonical identifier. Add a fixture. | xtask/src/canonical_types.rs:393 |

## Dispositions

Round 1, reviewed at fba9bed8eb5b4ba2464314389a17bf31c8614d6d (rebased onto main 13adbf59).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | fba9bed8e: `tools/arch-lint` has a `[lib] name = "arch_lint"` (`lib.rs`: `pub mod graph;`). The binary uses `use arch_lint::graph;`, xtask depends on `arch-lint` and imports `arch_lint::graph`, and the `#[path]` include and its `#[allow(dead_code)]` are deleted. graph's tests run once, in the lib (9 tests). `crate::graph::{classify, Repo}` in qualified_core.rs, metadata.rs and duplicate_revisions.rs still resolves. Merged with open #609 (task/594-core-ci, acd965d35) in a scratch worktree: `cargo test -p arch-lint` passed and `arch-lint qualified-core --qsl .` passed. |
| FND-002 | fixed | 8d124c9b9: a `pub use` root counts as a crate only when it is a scanned package's `crate_name`, and every other root is local. The fixture `mod m; pub use m::Value;` gives only the `identifier` finding. The workspace run at this head gives 42 `identifier` findings and 0 `re-export` (it was 10). |
| FND-003 | fixed | 8d124c9b9: the re-export rule skips a canonical type whose owner is not a workspace member. The TC-747 backend fixture adds a `qsl-replay` dependency with `pub use quire_exact::Value;`, and the backend's `pub use qsl_replay::Value as Kernel;` gives no finding. FR-272 Behavior and AC-3 state it. |
| FND-004 | fixed | 8d124c9b9: `UseTree::Glob` is recorded. `pub use v::*` from a workspace crate other than the owner, which defines or re-exports a canonical identifier by name, is a `re-export` finding. The TC-747 fixture `pub use v::*;` asserts it, and FR-272-AC-1 lists it. |
