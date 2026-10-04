---
id: SR-1288
title: "Code review of quire-spec-language PR #628: extract quire-walk to agent-ix/quire-walk"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4c418ede82df74fb1589f9367e16a13fa5a99c73; PR #628 diff against origin/main: Cargo.toml, Cargo.lock, Makefile, qsl-forms/Cargo.toml, qsl-semantics/Cargo.toml, quire-walk/** (deleted), tests/it/{main,family_outcome_layering,quire_walk_leaf}.rs, tools/arch-lint/{graph,metadata,qualified_core}.rs, xtask/src/string_edge.rs"
review_set: subset
---
# Code review of quire-spec-language PR #628

## Summary

Ticket: none resolvable from the branch (task/quire-walk-extract); review slug qsl-628. PR: quire-spec-language#628.

The PR deletes the in-workspace quire-walk crate and depends on agent-ix/quire-walk
as a workspace git dependency at branch main. qsl-forms and qsl-semantics take it
through `{ workspace = true }`.

Checked and clean:
- No `quire-walk/` directory, no `../quire-walk` path, no `quire-walk-no-std`
  target and no `quire-walk/src` crate root is left. `git grep` finds only spec
  prose and older review files that describe history.
- Cargo.lock has exactly one quire-walk package, sourced from
  `git+https://github.com/agent-ix/quire-walk?branch=main#89d05df2bcd6740c6f94f79bf6f3b5b007cf22a1`.
- The Makefile drops `quire-walk-no-std` from `.PHONY`, the target and `ci`.
  Nothing else named the target.
- `CORE_CRATES` drops quire-walk (11 entries). `check_direction` still walks the
  core crates' dependency graph through the external quire-walk node, and
  `core_offence` returns None for it (not a member, not above core, not a
  backend or frontend). The qualified-core fixture adds quire-walk as a git,
  non-member package, which matches the live graph.
- `family_outcome_layering`: qsl-forms still asserts its exact four
  `[dependencies]`, quire-walk included. qsl-semantics drops quire-walk from its
  workspace-crate list, which is right now that it is not a workspace member.
- `cargo test --locked -p arch-lint`: 84 + 9 passed. The supplied make ci log
  shows quire-walk compiled from 89d05df2 and the layering test passing.

## Verdict

Approve with one medium finding. The extraction is complete and there is no
leftover copy. The `SHARED_LEAVES` entry for quire-walk is now dead code, and the
test that claims to cover it cannot fail if the entry is removed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `SHARED_LEAVES` still lists `quire-walk`, but with the real source (`git+https://github.com/agent-ix/quire-walk`) `classify` already returns None: neither the name nor the source contains quire-spec-language, quire-contract-ir, -runtime or -codegen. `edge_repo`'s exemption never fires for it. `tc_arch_lint_metadata_009` now uses that same `walk_git` source, so its first half (no edge for IR, RT or CG to quire-walk) passes with or without the `SHARED_LEAVES` entry. The test asserts the right behaviour, but it gives no evidence for the entry, and the doc comment on `edge_repo` and the const's own doc imply the entry is what admits the edge. Delete `quire-walk` from `SHARED_LEAVES` (make it `[&str; 2]`), restate test 009 as "a crate outside the four ADR-011 repositories contributes no edge, for every backend", and drop the matching `SHARED_LEAVES` claims from FR-356 Behavior 3, FR-356-AC-1 and ADR-011 §6.1 (see SR-1290 FND-001). | tools/arch-lint/metadata.rs:21; tools/arch-lint/metadata.rs:30-36; tools/arch-lint/metadata.rs:357-393; tools/arch-lint/graph.rs:54-70 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 944ee8c14646469d3f62ed5e274ec06817c7bfc8 |
