---
id: SR-12315
title: 'Code review (Rust lane) of QSL PR #1203: S4 emits the FR-322 population reference (QSL-869, FR-391)'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@273dee611; git diff origin/main...273dee611 (PR #1203, branch qsl-869b-impl): qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/node_key/mod.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/population_bound.rs, qsl-package/src/emit/tests/population_reference.rs, qsl-eval/tests/it/named_population.rs, qsl-analyze/tests/it/model_check_safety.rs; ticket QSL-869'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-391
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1076
  type: reviews
---

## Summary

Ticket: QSL-869. PR quire-spec-language#1203, branch qsl-869b-impl, head 273dee611, one commit on top of eb0f8b382. The review covers `git diff origin/main...HEAD` only: 9 files, +222/-97. The rust-review lane is folded into this file. Origin/main is 4 commits ahead; none touches these files and the merge has no conflicts.

The change: `NodeKind::NamedPopulation` now lowers to a `reference` to a `relation`/`population` model node built by `Lowering::population_reference_node`, which is `model_node_content(owner, NodeTag::Relation, "population")` with the `ModelOwner` taken from the admitted selection. `LeafTerm::PopulationReference`, its `PreimageLeaf` variant, `EmitRefusal::PopulationReferenceNotEmitted` and the S4 scan are deleted.

Measured (all builds through `locked-build.sh`, one target dir, deleted afterwards):

- **Key identity.** A temporary unit test keyed `ModelOwner("acme/runs", "ix://acme/runs/counters")` and `.../archive` through QSL's own `node_key(model_node_content(..., Relation, "population"))`. The preimage bytes are QSpec FR-322's closed `ModelDeclarationNode` preimage byte for byte, and the keys equal P7 (`15ed74b1...`) and P8 (`cc575f3d...`) of `model-owner-form-vectors.json` on QSpec main bb6d08ae.
- **Reader admission.** A temporary qsl-package test emitted four units over the TC-1076 fixture (`qsl-semantics/tests/support/named_population.rs`) and read each through IR's `quire_contract_model::read_checked_package` and QSL's `read_v2`: the FR-391-AC-6 unit (`pre A`/`B`/`C` on `M::Widget::grow`) gives 2 population nodes; `lookup` plus `allInstances` on `counters` gives 1 node; the domain form `forall(o in M::counters : ...)` and the `holds` atom each give 1 node. All four are `AdmittedV2` and `Read::Verified`.
- **Mutations.** M1, owner node varied per read (breaks sharing): 4 tests fail (both qsl-package `population_reference` tests, qsl-eval `a_named_population_is_emitted_once_per_declared_population_read` and `a_holds_atom_naming_a_population_is_emitted`). M2, every population keyed to one owner node (merges A and B): the same 4 fail. Both reverted.
- **Gates on the PR head.** The PR's tests pass: qsl-package `emit::tests::population*` and `emit_checked_never_returns_incomplete_across_the_fixture_set` 12/12, qsl-eval `named_population` 8/8, qsl-semantics `named_population` 7/7. `make arch-lint-api-surface-qsl` PASS. `make string-edge` PASS. `cargo clippy -p qsl-semantics -p qsl-package -p qsl-eval -p qsl-analyze --all-targets --all-features -- -D warnings` exits 0. `cargo fmt --all -- --check` FAILS (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `cargo fmt --all -- --check` fails on three hunks: the `qsl_package` import order in the qsl-eval test, the `push(archive)` chain at population_reference.rs:18, and a hunk at population_reference.rs:141. `make ci` runs this check in `ci-default-features` (Makefile:211), so the head fails the merge gate. Fix: run `cargo fmt --all`. | qsl-eval/tests/it/named_population.rs:9-13; qsl-package/src/emit/tests/population_reference.rs:18,141 |
| FND-002 | low | Two builders now make the same `relation`/`population` node. `abstraction::population_node` (FR-451 `populations` entries) takes the owner from `declaration.package`. The new `population_reference_node` takes it from the matching `model.selection.identity`. The two agree today, because the lookup matches on equality, so a population read by a clause and named in an abstraction relation is one node. But nothing ties the two builders together, and FR-322 requires one key. Fix: one shared content helper for the population node, called from both. | qsl-semantics/src/check/lowering/abstraction.rs:51-66; qsl-semantics/src/check/lowering/model.rs:956-970 |

## Verdict

**Mergeable after FND-001 (`cargo fmt --all`). There is no logic defect.** FND-002 is a low cleanup.

What is sound:

- **Keying.** The node is keyed by the declaration key alone: package identity, then node. No member type, no maximum and no clause enters it. The emitted owner is `{kind: "model", identity: <selection identity>, node: <population IR identity>}`, as FR-322 "Node" and "Declaration key" state, and the key reproduces P7 and P8.
- **Sharing.** `insert_node` dedupes by content hash plus `holds`, so every read of one population is one node. `A` and `C` collapse into one `all_instances` application, and `B` is a second one. The authored maximum stays in the application's `result_type`: the bound test asserts the node key is unchanged and the result type differs.
- **Dependencies.** The population node is a `LeafTerm::Reference`, so `BodyNames` lists it in the application's `dependencies`, and the test asserts this for every application. The node's own `dependencies` are empty.
- **Removing the refusal.** No code path still assumes it. A grep for `PopulationReference` and `PopulationReferenceNotEmitted` over the workspace finds only two old review files. qsl-eval evaluates the checked tree (`NodeKind::AllInstances`/`NamedPopulation`), not the lowered leaf, so evaluation does not change. Replay recompiles through the same lowering. The kani tree names neither removed item.
- **Public API.** The two loud breaks (`EmitRefusal::PopulationReferenceNotEmitted`, `LeafTerm::PopulationReference`) are named in the PR body. The variants are deleted outright, with no shim.
- **Panic surface.** No `unwrap`, `expect` or indexing was added to production code. `population_reference_node` propagates `model_owner`'s typed `KeyFault` refusals.

## Dispositions

Disposition pass 1 at head b8d4723ef (fix commit 50605a374, plus a merge of main). Each row was checked by measurement.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 50605a374: `cargo fmt --all -- --check` exits 0 at b8d4723ef |
| FND-002 | fixed | 50605a374: `population_node_content(owner)` in lowering/model.rs is the one builder, and `Lowering::population_reference_node` and `abstraction::population_node` both call it. A probe re-keyed P7/P8 through it and they are unchanged |
