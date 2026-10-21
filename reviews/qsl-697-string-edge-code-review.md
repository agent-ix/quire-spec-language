---
id: "SR-13625"
title: "code-review of QSL-697 standalone qsl-string-edge scanner (PR #1130)"
type: "SpecReview"
analysis: "code-review"
scope: "agent-ix/quire-spec-language@b16ba868c31d637c85ebdfba7ccfbe235bbf606b; PR #1130; git diff origin/main...HEAD (qsl-string-edge/**, xtask/src/string_edge.rs, xtask/src/error.rs, xtask/src/lib.rs, xtask/src/{checked_input,definition_scan,seam_probe,typestate_scan,test_suites}.rs, Cargo.toml, Cargo.lock, xtask/Cargo.toml, README.md); spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/test-cases/TC-162-string-edge-scan-coverage.md; QSL-697"
review_set: "subset"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: references
---

## Summary

Ticket: QSL-697. Code review with the rust-review lane folded in, of PR #1130: the new qsl-string-edge library and command and the thin xtask adapter. The detector move preserves behaviour (reproduced) and every gate passes, but the command report omits the item anchor FR-064 and AC-7 require, a directory link can make one source count under two crate scopes, and a `target/` directory is silently unscanned.

## Verdict

**FAIL**: one high finding (FND-001, command report lacks the item anchor). The PR is not mergeable until FND-001 is fixed; FND-002 and FND-003 are medium and should be fixed in the same round. Clean units: AC-1, AC-2, AC-4, AC-6, AC-8, AC-10 (built binary and Git route) and AC-11 were examined with no defect. The Cargo layout is sound: name qsl-string-edge, edition 2021, publish = false, and a path dependency on qsl-attrs inside the same git repository, so the git install resolves (verified by cargo install --path and by the Git route test against the branch). The root README row and the qsl-string-edge/README.md match the implemented flags. The test_suites.rs entries are exercised by the default-workspace lane (see Validation). No vendoring and no unasked-for ceremony.

## Validation

Gates (own clone at b16ba868c31d637c85ebdfba7ccfbe235bbf606b, own CARGO_TARGET_DIR): cargo fmt --all --check exit 0; cargo clippy --locked --workspace --all-targets -D warnings exit 0; cargo test --locked -p xtask -p qsl-string-edge exit 0 (qsl-string-edge it: 31 passed, 1 ignored; xtask lib 200 passed); root cargo test --locked --test it exit 0 (975 passed, 6 ignored); make string-edge exit 0 (clean); make arch-lint-api-surface-qsl / arch-lint-canonical-encoder / arch-lint-qualified-core exit 0; cargo run --locked -p xtask -- test-suites default-workspace --locked --workspace exit 0 (38 suites ran tests, 18 declared empty, including both new qsl-string-edge entries: src/main.rs tests and src/lib.rs doctests); cargo install --path qsl-string-edge into a temp root succeeded; the ignored Git install/library-dependency test, run with QSL_STRING_EDGE_BRANCH=qsl-697-string-edge, passed. No applicable AssuranceProfile was found under spec/.

## Behaviour-preserving move

Behaviour-preserving move, reproduced: a reviewer probe dumped every raw occurrence (marks ignored, and separately unmarked only) with the origin/main xtask detector and with the new crate. Both detectors over the same tree produced identical lists, on the PR tree (460 raw, 0 unmarked) and on the main tree (457 raw, 0 unmarked). The author comparison of old-on-main against new-on-PR, excluding the scanner own files (4 lines old, 7 new), has 453 lines that match field for field, except 16 xtask/src/test_suites.rs lines shifted by +10 by this PR own edit to that file. A textual diff of the detector shows only intended changes: honor_marks is replaced by a per-occurrence `marked` flag (Found), the Occurrence and AllowListEntry docs are updated, collect_string_consts takes an iterator, and the walk, cfg(test) mod exclusion, allow-list and report move to source.rs, rules.rs and lib.rs. No copy of the detector remains: xtask::impl_self_name and the old xtask detector are deleted, and a search for STRING_EDGE_METHODS, collect_string_consts, PREFIX_GATE_METHODS, record_forced_branch_gating and honor_marks outside qsl-string-edge finds nothing. The adapter passes 15 roots, including qsl-string-edge/src, and EdgeRules::default(). It defines no default rules or re-exports, and no compat shim remains.

## Mutation testing

Mutation testing (43 mutants on a copy, cargo test -p qsl-string-edge plus -p xtask for adapter mutants): 41 killed, 2 survived. M10 (absolute entry outside the base accepted via unwrap_or) and M10b (RootDir component ignored) are equivalent: each guard alone refuses every absolute path the other admits, so no test can separate them. The killed mutants cover the branch-gating refusal, the refusal over marked items, marks honoured, allow-list application, sort, exit codes 0/1/2 (binary and adapter), the trimmed and empty reason, `..` and absolute entry escapes, deny_unknown_fields on both structs, a missing allow_list, out-of-base, duplicate and both overlap orders of roots, per-root no-production, escaping links, cycles, dangling .rs, inside links, the tests/ skip, cfg(test) mod files, module-root stems, branch classification (condition and forced), has_cfg_test, rules relative to the base, per-crate constants, the branch tag in render, the adapter root list, violation propagation, a duplicate --base-dir, the impl-fn marker, `/` separators and the canonical base. Log: rv1130/mut.log.

## Judgment calls

Author judgment calls: (1) a report sorted by file, line and item across roots matches FR-064 and is what the spec asks for. (2) xtask exits 2 for parse and refused-entry failures, which matches AC-8/AC-9 (a rule error is a failure, not a violation). (3) Checking per root for production Rust files is a sound reading, because a root without them would otherwise be silently clean. (4) Checking branch gating over marked items is required by AC-8. (5) Treating a whitespace-only reason as empty is a reasonable reading of non-empty. (6) Requiring an absolute entry to sit under the canonical base is refused on a symlinked base spelling (FND-008). (7) Links: escaping the base is an error, a cycle back into an ancestor is an error, a dangling .rs link is a read error and other dangling links are ignored, all sound. Following a link into another directory inside the base is not (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The command report omits the enclosing item anchor: Report::render prints only `file:line (branch-gating)`. FR-064 requires reports to identify file, one-based line and the enclosing item anchor, and AC-7 requires the library and binary to agree on item. TC-162 step 11 expects `src/lib.rs:1 in item accepted`. A consumer also cannot learn from the command the anchor an allow-list entry needs. The tests pin the item-less text (tests/it/command.rs:76-78, tests/it/consumer.rs:54-57, xtask/src/string_edge.rs:265-269). | qsl-string-edge/src/lib.rs:59-79 |
| FND-002 | medium | walk follows a directory link inside the base into another crate source tree. Reproduced: with roots a/src (holding linked -> ../../b/src) and b/src, the command reports the same occurrence twice (a/src/linked/lib.rs:2 and b/src/lib.rs:2, exit 1) and scans b files in a constant scope. FR-064 rejects duplicate or overlapping roots so the scan cannot confuse crate scopes or report an occurrence twice; a link bypasses that rule. tests/it/links.rs:58-64 pins following as intended. Refuse a link whose target lies in or contains another selected root (or any link that leaves its own root). | qsl-string-edge/src/source.rs:140-164 |
| FND-003 | medium | Any directory named `target` or `.git` under a crate root is skipped by its name. Reproduced: c/src/lib.rs declaring `mod target;` with c/src/target/mod.rs holding `v == "x"` scans clean, exit 0. FR-064 excludes only #[cfg(test)] code, files declared by #[cfg(test)] mod x; and tests/ directories, and says a file is never out of scope by its name alone. The README does not document the skip, and tests/it/library.rs:149 asserts it. The skip comes from the old xtask walk (origin/main xtask/src/string_edge.rs:453), but it is now the consumer contract. | qsl-string-edge/src/source.rs:156 |
| FND-004 | low | impl_self_name is an xtask helper with no part in the scan request. It is now public API of the reusable scanner, which exposes syn::Type, so syn becomes part of every consumer API. Its docs carry QSL PR-review prose. xtask checked_input, seam_probe, typestate_scan and definition_scan now import it from the scanner crate. Hide it with #[doc(hidden)] or pub(crate) and give QSL its own home, or document it as consumer API. | qsl-string-edge/src/lib.rs:130-155 |
| FND-005 | low | The command case meant to cover an out-of-base root passes `--crate-root ../escape`, which does not exist. It therefore exercises RootUnresolved, and the needle `escape` matches the path in that message. Command exit 2 for an existing out-of-base root, an overlapping root and a non-directory root is not exercised. The library tests do cover those variants, and main maps every error to 2. | qsl-string-edge/tests/it/command.rs:226-235 |
| FND-006 | low | AllowListGatesABranch names `file:item` but not the line of the branch-gating occurrence that caused the refusal. AC-5 says the rejection names its file and line, and the line is already available in `found` at lib.rs:98-110. | qsl-string-edge/src/error.rs:136-146 |
| FND-007 | low | NoProductionSources documents its path as "the root as the caller named it", but load_root fills it with the canonicalised root. A consumer passing `--crate-root src` sees /private/var/... in the message. The other root variants do carry the caller spelling. | qsl-string-edge/src/error.rs:62-68; qsl-string-edge/src/source.rs:100-104 |
| FND-008 | low | An absolute allow-list `file` is stripped against the canonical base only. Reproduced on macOS: `--base-dir /tmp/.../ws` is accepted, but the entry `/tmp/.../ws/d/src/lib.rs` is refused as outside the base, exit 2. The same file spelled under /private/tmp is accepted. FR-064 says absolute inputs are accepted. Canonicalise the entry parent before stripping, or strip against both spellings of the base. | qsl-string-edge/src/rules.rs:81-83 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | LinkIntoRoot only refuses a link whose target lies inside a selected root. A link to a directory that contains a selected root is still followed. Reproduced at 19bb1d944: with roots a/src (holding linkb -> ../../b) and b/src, the same occurrence is reported twice (a/src/linkb/src/lib.rs:1 and b/src/lib.rs:1, exit 1), and b is scanned in a constant scope. Fix: also refuse when `root.starts_with(&target)`, and add a test for that shape. | qsl-string-edge/src/source.rs:151-157 |

## Dispositions

Round 1, reviewed at 19bb1d94489baf1cfbbb8d742da6bf4599a06e4a (delta 930274142 plus main merge).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after: Report::render: "  {}:{} in {}{}\n", occurrence.file, occurrence.line, occurrence.item, branch tag; command stdout == render() and names `T::kind` (tests/it/command.rs the_binary_and_the_library_report_the_same_item; mutant R1 killed). |
| FND-002 | fixed | after: if roots.iter().any(/root/ target.starts_with(root)) && !ancestors.contains(&target) { return Err(Error::LinkIntoRoot { path }); } (source.rs; links.rs a_link_into_a_selected_root_is_refused; mutants R2, R3, R3b killed). The residual shape is FND-009. |
| FND-003 | fixed | after: if is_dir { if name == "tests" { continue; } ... (README: no other directory is skipped by name; library.rs directories_named_target_are_production_source; mutant R4 killed). |
| FND-004 | accepted-no-change | The helper is shared by four xtask scans, and the alternative is a second copy; the PR body records it as a shared helper rather than specified consumer API. Accepted for this PR. |
| FND-005 | fixed | after: command.rs outside_overlapping_and_non_directory_roots_exit_two: an existing out-of-base root, overlapping roots and a non-directory root each exit 2 with an empty stdout. |
| FND-006 | fixed | after: "string-edge: allow-list entry {file}:{line} ({item}) gates a branch ..."; line: gating.occurrence.line (mutant R5 killed). |
| FND-007 | fixed | after: NoProductionSources { /// The root, resolved against the base directory. path: PathBuf } |
| FND-008 | fixed | after: path.strip_prefix(base).ok().or_else(// path.strip_prefix(given?).ok())? with given = std::path::absolute(given_base); library.rs an_absolute_entry_may_use_the_base_as_given (symlinked alias); mutants R6 and R7 killed. |
| FND-009 | still-open | New in this round: a link to a directory containing a selected root still double-reports (see New findings). |
