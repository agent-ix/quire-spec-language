---
id: SR-16276
title: "IR-586 S2 PR 1384 code review: repoint DeclarationKey and AdmittedIntegerDivision to quire-checked"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ae001482e28a98797037954c417fdaebc50bd6ca; diff ir-588-s4-qsl...ir-586-s2-qsl (201 files): Cargo.toml, Cargo.lock, */Cargo.toml, qsl-semantics/src/model/key.rs, qsl-semantics/src/value/definition.rs, qsl-semantics/src/value/mod.rs, qsl-replay/src/lib.rs, src/linking/composed/models.rs, and the import-only repoints in qsl-semantics, qsl-eval, qsl-package, qsl-replay, qsl-analyze, qsl-bench, root src and tests"
review_set: subset
---

## Summary

Ticket: IR-586. PR 1384 is stacked on PR 1382 (IR-588 S4). Only this PR's
own diff was reviewed. The rust-review lane is folded in.

The PR deletes `DeclarationKey` from `qsl_semantics::model::key` and
`AdmittedIntegerDivision` from `qsl_semantics::value::definition`. It
repoints every caller to `quire_checked::key` and
`quire_checked::integer_division`. It adds `quire-checked`, sourced from
quire-eval branch `ir-586-s2-eval`, as a dependency.

## Verdict

CONDITIONAL. The repoint is mechanical and correct:

- A sweep of every `.rs` hunk that is not an import or path change found
  only the `DeclarationKey::wire` change and one rustfmt reflow.
  `declaration_key_wire` builds the same `{package, node, digest_domain}`
  view, and it is `pub(super)` like the method it replaces.
- `identity` and `validate_derivation` stay in `model`.
  `admit_integer_division` and `divide` keep their signatures.
- The quire-checked type has the same derives and the same field order.
  `fixture` builds the same value.
- No QSL crate keeps a definition, alias or `pub use` of either type at a
  `qsl_semantics::` path.
- `qsl-replay`'s FB-05 facade `pub use` now points at `quire_checked`, so
  CG's `qsl_replay::DeclarationKey` still resolves.
- The root `linking::DeclarationKey` is a separate enum (`Type`, `Value`,
  `Scalar`, `Field`, `Variant`, `Operation`). It is the FR-273 namesake, not
  a copy.
- `src/linking/composed/models.rs` keeps its private
  `as DomainKey` import alias, which is not a re-export.
- Only `qsl-semantics` names `construct`. `test-support` appears only in
  `[dev-dependencies]` tables.
- Cargo.lock adds only the `quire-checked` package and its edges.

External consumers: quire-driver imports only `qsl_semantics::model::key::hex`,
which is unchanged. CG, RT and IR do not name either type. A GitHub code
search of agent-ix outside QSL found no hits. The breakage list is therefore
empty in practice.

The open findings are guard regressions, not wrong behaviour. Merge
condition: fix FND-001 in this PR, or land T12-G and the TC-390 `construct`
table check before any other crate could call `construct`, under a named
ticket.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Construction authority for `AdmittedIntegerDivision` is no longer enforced in QSL. Before this PR, the private field limited construction to `value::definition` at compile time. Now `qsl-semantics` enables `construct`, and feature unification turns it on for every workspace crate. So `qsl-eval`, `qsl-package` or `qsl-replay` can call `quire_checked::construct::admitted_integer_division` and compile. Nothing replaces the guard: FR-060 T12-G and FR-400-AC-1's TC-390 `construct` table check are absent, and the PR body lists them as not done. Fix: add T12-G (callers limited to `check`, `library`, `value::definition`) and the TC-390 manifest check in this PR. | qsl-semantics/src/value/definition.rs:697; qsl-semantics/Cargo.toml:32 | correct-requirement-no-evidence |
| FND-002 | medium | `DeclarationKey` drops out of the FR-271/FR-272 canonical-types gate. `canonical_types::packages` scans workspace members, `SHARED_LEAVES` (`quire-exact`, `quire-semantic-value`) and whatever `graph::classify` maps to QSL, IR, RT or CG. `quire-checked` (source `quire-eval`) matches none of these. The `quire:canonical` tag on `DeclarationKey` is therefore never read, and a QSL crate could re-add a `DeclarationKey { package, node }` copy without a DT-2 or DT-3 finding. Fix: add `quire-checked` to `SHARED_LEAVES`, or classify the quire-eval repository. | xtask/src/canonical_types.rs:61; tools/arch-lint/graph.rs:54 | correct-requirement-no-evidence |
| FND-003 | medium | The `test-support` guard does not cover `quire-checked`. The new dev-dependency entries sit under comments saying the root crate's `no_shipped_dependency_enables_test_support` refuses a normal dependency that enables `test-support`. That test only checks dependencies in `workspace_crates`, so `quire-checked = { features = ["test-support"] }` in a normal table would pass. FR-400-AC-9's QSL clause has no test. Fix: extend the check to `quire-checked` (or to every dependency that declares a `test-support` feature), with a fixture case. | tests/it/family_outcome_layering.rs:639-645; qsl-eval/Cargo.toml:64 | correct-requirement-no-evidence |
| FND-004 | low | Several `quire-checked` lines are misplaced in the manifests. In the root `Cargo.toml`, `qsl-eval`, `qsl-package`, `qsl-replay` and `qsl-semantics`, the new normal-dependency line sits between the "layer SV" comment and `quire-semantic-value`, so that comment now appears to describe `quire-checked`. In `qsl-semantics/Cargo.toml`, the dev-dependency has no comment and sits after a blank line, directly before `[[test]]`. | Cargo.toml:61-63; qsl-eval/Cargo.toml:23-24; qsl-package/Cargo.toml:52-54; qsl-replay/Cargo.toml:34-36; qsl-semantics/Cargo.toml:31-32,71 | implementation-bug-despite-evidence |
| FND-005 | low | Spec text still names the old path. FR-273's facade row says "the `pub use` names `qsl_semantics::model::key::DeclarationKey`". ADR-013 O-03 still names `model::key::DeclarationKey` as the public type. Neither is updated, although FR-400 says each move repoints every caller at once. | spec/functional/FR-273-resolve-each-canonical-type-namesake.md:99,111; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:169 | wrong-requirement |

## New findings (disposition pass 1)

Re-review at `182875cb5810b62451c4be2643bcda0e8893810b` (diff `origin/main...HEAD`, base `f4a16c103`).

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | medium | T12-G matches only the literal token run `quire_checked::construct`. Its own `scope_note` says a crate alias (`use quire_checked as qc;`), a nested use tree (`use quire_checked::{construct::admitted_integer_division};`) and a glob are not matched. FR-060 says "a crate alias or glob import of the module counts as a reference". `construct` is unified on for every workspace crate, so `qsl-eval` can build an `AdmittedIntegerDivision` through one of those spellings and pass both T12-G and TC-390. Fix: resolve references with the existing `CrateAliases`/`ModuleReferences` visitors (the T12-A `forbidden_modules` path), checked against `allowed_callers`, and add fixture cases for the alias, group and glob spellings. | tools/arch-lint/api_surface.rs:416,435-440; spec/functional/FR-060-check-qsl-api-surface-boundary.md:103 | implementation-bug-despite-evidence |
| FND-007 | low | The T12-G allow-list works per module, not per constructor. `library` and `value::definition` may call any `construct` function. FR-060's T12-G row and FR-400 rule 3 allow `library` only for the `PackageId` constructor and `value::definition` only for `AdmittedIntegerDivision`. Today `construct` has one function, so nothing is wrong yet. The gap opens as X-13 adds constructors. | tools/arch-lint/api_surface.rs:420-432; spec/functional/FR-060-check-qsl-api-surface-boundary.md:92 | implementation-bug-despite-evidence |
| FND-008 | low | The three new QSL `compile_fail` doctests carry no error code, so any compile error passes them. The upstream `quire-checked` blocks pin `E0451` and `E0433`. Pin `compile_fail,E0603` (`model::key` still holds a private `use` of the type) and `compile_fail,E0432` for the two `value` paths, so the block fails only because the path does not resolve. | qsl-semantics/src/model/key.rs:16; qsl-semantics/src/value/definition.rs:14,18 | implementation-bug-despite-evidence |

## Dispositions

Disposition pass 1, checked against the fix commits `c026ca16f`, `8c21ae8e8`, `98d219157`, `c8ec6d833` and `3181c7cdb` at head `182875cb5`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | T12-G is in `tools/arch-lint/api_surface.rs` RULES, with fixture test `tc_arch_lint_api_surface_029` and live-tree test `_030`. The TC-390 `construct` table check is `only_qsl_semantics_enables_the_construct_feature`, with a fixture case. The alias and group-import gap is new finding FND-006. |
| FND-002 | fixed | `SHARED_LEAVES` now includes `quire-checked` (`xtask/src/canonical_types.rs:61`). `tc_747_quire_checked_declaration_key_copy_is_flagged` covers the copy case. |
| FND-003 | fixed | `no_shipped_dependency_enables_test_support` pushes `quire-checked` into the guarded set, and the matcher row `quire-checked/test-support` was added. |
| FND-004 | still-open | Fixed in `qsl-eval`, `qsl-package`, `qsl-replay` and `qsl-semantics`. Still misplaced in three places. Root `Cargo.toml:61-63`: `quire-checked` still sits under the "layer SV" comment. Root `Cargo.toml:107-117`: the new FR-400-AC-9 comment is stacked above the `qsl-semantics` comment, so that comment now heads the `quire-checked` line. `qsl-analyze/Cargo.toml:29` and `qsl-bench/Cargo.toml:25` have no layer-C comment. |
| FND-005 | fixed | Fixed in FR-273 rows 36 and 48, FR-273-AC-48, and ADR-013 O-03 (Public type, the `linking::DeclarationKey` note and DA-01). |
