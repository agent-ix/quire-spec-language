---
id: "SR-13626"
title: "gap-analysis of QSL-697 standalone qsl-string-edge scanner (PR #1130)"
type: "SpecReview"
analysis: "gap-analysis"
scope: "agent-ix/quire-spec-language@b16ba868c31d637c85ebdfba7ccfbe235bbf606b; PR #1130; git diff origin/main...HEAD (qsl-string-edge/**, xtask/src/string_edge.rs, xtask/src/error.rs, xtask/src/lib.rs, xtask/src/{checked_input,definition_scan,seam_probe,typestate_scan,test_suites}.rs, Cargo.toml, Cargo.lock, xtask/Cargo.toml, README.md); spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/test-cases/TC-162-string-edge-scan-coverage.md; QSL-697"
review_set: "subset"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: references
---

## Summary

Ticket: QSL-697. Planless gap analysis of FR-064 (AC-1..AC-11) against the computed matrix, the tagged tests and the qsl-string-edge and xtask sources at PR #1130. All 11 criteria compute `tagged` (AC-10 partly through an ignored test). The gaps are an untested binary-item half of AC-7, two tagged tests that pin behaviour FR-064 does not allow, and one unowned public function.

## Verdict

**CONDITIONAL**: no untagged criterion and no high finding. FND-001 and FND-002 are medium and tie to code-review SR-13625 FND-001..003, so the PR is not mergeable until those are fixed. Semantic review (intent/test/code) was performed only as far as the mutation campaign and the reviewer probes reach; the full opt-in semantic pass was not run.

## Coverage
Plan completion: not assessed

Matrix: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2) completed. FR-064: 11 of 11 criteria `tagged`; AC-10 binders include consumer.rs:102 (ignored). TC-162 is the verifying TC. Reverse gap: the qsl-string-edge modules detector/source/rules/error/main are all owned by FR-064; impl_self_name is not (FND-004). Stubs: none (no todo!/unimplemented!/TODO). 
Mutation testing (43 mutants on a copy, cargo test -p qsl-string-edge plus -p xtask for adapter mutants): 41 killed, 2 survived. M10 (absolute entry outside the base accepted via unwrap_or) and M10b (RootDir component ignored) are equivalent: each guard alone refuses every absolute path the other admits, so no test can separate them. The killed mutants cover the branch-gating refusal, the refusal over marked items, marks honoured, allow-list application, sort, exit codes 0/1/2 (binary and adapter), the trimmed and empty reason, `..` and absolute entry escapes, deny_unknown_fields on both structs, a missing allow_list, out-of-base, duplicate and both overlap orders of roots, per-root no-production, escaping links, cycles, dangling .rs, inside links, the tests/ skip, cfg(test) mod files, module-root stems, branch classification (condition and forced), has_cfg_test, rules relative to the base, per-crate constants, the branch tag in render, the adapter root list, violation propagation, a duplicate --base-dir, the impl-fn marker, `/` separators and the canonical base. Log: rv1130/mut.log.

Gates (own clone at b16ba868c31d637c85ebdfba7ccfbe235bbf606b, own CARGO_TARGET_DIR): cargo fmt --all --check exit 0; cargo clippy --locked --workspace --all-targets -D warnings exit 0; cargo test --locked -p xtask -p qsl-string-edge exit 0 (qsl-string-edge it: 31 passed, 1 ignored; xtask lib 200 passed); root cargo test --locked --test it exit 0 (975 passed, 6 ignored); make string-edge exit 0 (clean); make arch-lint-api-surface-qsl / arch-lint-canonical-encoder / arch-lint-qualified-core exit 0; cargo run --locked -p xtask -- test-suites default-workspace --locked --workspace exit 0 (38 suites ran tests, 18 declared empty, including both new qsl-string-edge entries: src/main.rs tests and src/lib.rs doctests); cargo install --path qsl-string-edge into a temp root succeeded; the ignored Git install/library-dependency test, run with QSL_STRING_EDGE_BRANCH=qsl-697-string-edge, passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The binary half of AC-7 is only partly verified. No test compares the item of a binary occurrence with the library value, because the command prints no item (SR-13625 FND-001). The tagged command tests assert file, line and branch tag only. The TC-162 step 11 expectation of item `accepted` has no binder. | qsl-string-edge/tests/it/command.rs:32-101 |
| FND-002 | medium | Two tagged tests assert behaviour FR-064 does not allow. test_code_is_not_scanned (AC-3) requires src/target/generated.rs to be skipped by directory name. a_directory_link_cycle_is_an_error_and_an_inside_link_is_followed (AC-9) requires a link to another tree to be followed under the link path. The matrix counts both criteria as tagged while the evidence protects SR-13625 FND-003 and FND-002. | qsl-string-edge/tests/it/library.rs:149; qsl-string-edge/tests/it/links.rs:58-64 |
| FND-003 | low | The Git-install and Git-library-dependency half of AC-10 is backed only by an #[ignore]d test that no default lane runs, and against main it can only pass after merge. The reviewer ran it against the PR branch with QSL_STRING_EDGE_BRANCH=qsl-697-string-edge and it passed (1 passed, 26.19s). FR-064 still requires the acceptance run of the documented `--branch main` route once merged, and nothing in the repo records that run. | qsl-string-edge/tests/it/consumer.rs:99-188 |
| FND-004 | low | Reverse gap: the public function impl_self_name in the scanner crate has no owning requirement. FR-064 defines the consumer API as the request, report, typed error and rules. The function serves only other xtask scans (see SR-13625 FND-004). | qsl-string-edge/src/lib.rs:130-155 |

## Dispositions

Round 1, reviewed at 19bb1d94489baf1cfbbb8d742da6bf4599a06e4a (delta 930274142 plus main merge).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after: command.rs the_binary_and_the_library_report_the_same_item: assert_eq!(stdout(&output), report.render()); stdout contains "src/lib.rs:3 in T::kind (branch-gating)". |
| FND-002 | fixed | after: test_code_is_not_scanned no longer requires a target/ skip; directories_named_target_are_production_source asserts src/target/mod.rs is reported. The link test now asserts LinkIntoRoot for links into a selected root, and following only for a directory elsewhere in the base (residual shape: SR-13625 FND-009). |
| FND-003 | deferred | The post-merge run of the ignored Git-install test against --branch main is recorded in the PR body as a follow-up; it cannot pass before merge. The branch run passed. |
| FND-004 | accepted-no-change | Same decision as SR-13625 FND-004: a shared helper used by four xtask scans, kept public to avoid a second copy. |
