---
id: SR-1957108
title: "Code review of quire-spec-language PR #1978: QSL-1377 selected exact identities"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@peter/qsl-1377-selected-exact-identities; PR #1978 diff 8220d6c0a...HEAD; Cargo.toml, Cargo.lock, qsl-inspect/qsl-package/qsl-replay/qsl-semantics manifests, qsl-package/src/{checked_v2.rs,checked_v2/*.rs,emit.rs,emit/tests/*.rs}, qsl-semantics/src/{check/*.rs,model/intake/unit.rs}"
review_set: subset
---
# Code review of quire-spec-language PR #1978 (QSL-1377)

## Summary

Reviewed the Rust implementation and tests for producing model-intake identity rows and joining them to the IR-admitted checked package, plus QSL's graph-local `Index` path encoding/readback changes. Scope is PR #1978 against `8220d6c0a`. The API pin is the requested IR commit `154227dcbcda09f3abf635375777893132fc7686`.

The producer derives record and object rows from the admitted `DomainPackage` and normalized `EffectiveView`, and carries them through `SelectedModels`, `PackageDeclarations`, and `CheckedGraph`. The object/reference integration case verifies projection and the IR's typed missing-witness refusal. The record and inherited-field test only checks rows before they are joined to an IR-admitted package, so the ticket's record-row join remains unverified. The producer also does not emit declared-unit rows from `DomainPackage.units`; the pinned IR API has a declared-unit projection form, so a selected declared unit has no QSL witness.

## Verdict

**FAIL** — clippy fails on the new test-module placement, and the selected identity producer is incomplete for declared units; record and inherited-field rows also lack an end-to-end admission/projection regression.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `selected_exact_identity_tests` is defined before the public `admit_unit` and `admit_unit_with_cancel` items. The repository's `clippy::all` lint rejects this with `clippy::items-after-test-module`; `cargo clippy -p qsl-semantics -p qsl-package --all-targets --all-features -- -D warnings` exits 101 at this location. Move the test module after production items. | qsl-semantics/src/model/intake/unit.rs:570-571; :721-736 |
| FND-002 | medium | `selected_exact_rows` emits only record and object declaration rows and ignores `DomainPackage.units`. Consequently, an admitted selected `scalar_type/unit` cannot receive the declared `UnitId` witness and IR exact projection returns `UnavailableIdentityWitness`, despite the pinned IR API exposing `SelectedExactDeclarationForm::DeclaredUnit` and the QSL producer contract covering selected declaration identities. Derive the declared-unit row from the admitted unit graph or explicitly narrow the producer contract before calling this complete. | qsl-semantics/src/model/intake/unit.rs:477-512; :555-568 |
| FND-003 | medium | The integration test exercises only the object/reference row. The record and inherited-field assertions run `selected_exact_rows` against a normalization fixture but never validate those rows through `CheckedGraph::selected_exact_identities` / IR `from_stage_rows`, nor project a record view containing the inherited field. A wrong structural record ID or field identity can therefore pass this producer test while the real IR join refuses it. Add a checked-package admission and record-projection case for a record with an inherited field. | qsl-semantics/src/model/intake/unit.rs:571-717; qsl-package/src/emit/tests/owner_model.rs:167-206 |

## Coverage

- Rust review inspected the changed manifests, lockfile pin, graph-path conversion/readback callers, model intake row derivation, and test oracles. No workflow files changed.
- `cargo fmt --check`: PASS.
- Focused `cargo test -p qsl-semantics selected_exact_identity_tests --lib`: PASS (1 test).
- Focused `cargo test -p qsl-package selected_exact --lib`: PASS (0 tests matched; this is not substantive test evidence).
- `cargo test -p qsl-package`: FAIL, test process terminated by SIGKILL while running the parallel deep-graph suite; no assertion failure was reported before termination. A serial rerun (`cargo test -p qsl-package -- --test-threads=1`) was interrupted after about three minutes while `emit::tests::deep_bodies::a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack` was running, so it has no final status. The focused `cargo test -p qsl-package selected_model_identity_rows_back_reference_projection_and_missing_rows_refuse --lib` PASS (1 test).
- `cargo test -p qsl-semantics`: FAIL, 723 passed, 1 failed, 1 ignored. The failing test is `check::protocol_clause::tests::graph::a_decision_reads_only_listed_facts_of_its_owner`, asserting expected `missing_declaration/missing-name` but received `ill_typed/operator-ineligible` at `qsl-semantics/src/check/protocol_clause/tests/graph.rs:900`.
- `cargo clippy -p qsl-semantics -p qsl-package --all-targets --all-features -- -D warnings`: FAIL on FND-001.
- Required `cargo check -p qsl-package --no-default-features` and `cargo build -p qsl-package --no-default-features`: both FAIL with E0433 at unchanged `qsl-package/src/evaluation_view.rs:52` (`std::slice::from_ref`), matching the reported main-baseline failure.
- `cargo deny check`: FAIL in license policy on existing AGPL-3.0-or-later dependencies; advisories, bans, and sources passed. No license change in this PR was found.
- No aggregate CI command or manual native workflow dispatch was run.
