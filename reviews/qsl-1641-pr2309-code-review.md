---
id: SR-20261104190827
title: "Code review of PR #2309: QSL-1641 A bytes-only temporal evidence entry (code-review with rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@51570af56 trial-merged with origin/main 0642337ef; git diff origin/main...HEAD (PR #2309, branch qsl-1641-bytes-only-entry): qsl-package/src/bare.rs, qsl-package/src/checked_views/subject.rs, qsl-eval/src/temporal/facts.rs, qsl-eval/src/temporal/evidence/{package,call,work,activation,request}.rs, qsl-eval/src/temporal/{ledger,plan,observed}.rs, qsl-replay/src/spine/clause/temporal.rs; ticket QSL-1641"
review_set: subset
---

## Summary

Ticket: QSL-1641. PR: quire-spec-language#2309, head 51570af56, trial-merged with origin/main 0642337ef (clean merge). Method: code-review with the rust-review lane (rust-style loaded). The diff makes `ClauseFacts` the one clause value the evidence call reads, builds it from the package's bytes on `from_emission`, `from_read` and the new `from_package`, adds `BarePackage`, dedupes shared formula nodes by node id and numbers atoms by first use, and rebinds `LedgerBinding.package` to `package.name()`.

Examined, clean:

- No `unwrap`, `expect`, `panic!`, `unsafe`, `#[allow]` or indexing on a production path in the added lines; every lookup returns `Option` or `Result`.
- `ClauseFormula::of_graph`: the reverse post-order walk is iterative, finds cycles and out-of-range operands as `ReachFault::Malformed`, and lists an operator before its operands; plan root 0 is the root.
- `LedgerBinding.package` as a `String`: the binding is compared and ordered only, and `result_identity` takes its package component from the declaration, so no ledger encoding changes. Nothing else in the workspace reads `.authority` of a binding.
- Cargo.lock bump of quire-contract-model to 401cb8ff: lock line only, `branch = "main"` kept, no `rev=`.
- The cross-check on `from_emission` and `from_read` still compares `formula_graph()` with the source clause (`same_formula`) and reads nothing else from the source.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | qsl-replay binds `LedgerBinding.package` to the source identity string, not `package.name()`, so FR-532 "on every route" and the PR text ("also qsl-replay") are not met there. | qsl-replay/src/spine/clause/temporal.rs:756-766 |
| FND-002 | medium | `bound_atom_indices` zips `reads` with the `holds` entries and silently truncates on a length mismatch; the deleted `instance_bound_indices` returned `None`, and the only test of that branch was rewritten without it. | qsl-eval/src/temporal/evidence/package.rs:464-476, qsl-eval/tests/it/temporal_causes.rs:294-310 |
| FND-003 | low | `finite_window_profile` filters `TemporalProfile::Timed` out, so a lock row naming the timed profile (a profile the evaluator names and refuses with `NotTimedProfile`) refuses `clause-unreadable` `profile-unknown` instead of the temporal refusal the evaluator gives. | qsl-eval/src/temporal/evidence/package.rs:456-458 |
| FND-004 | low | `BarePackage::dependencies` deduplicates by `name()` alone; two distinct admitted packages of one name in a closure collapse to the first. | qsl-package/src/bare.rs:56-67 |
| FND-005 | low | The assertion that the package listing differs from the checked arena (`assert_ne!(subject.formula, ClauseFormula::of_checked(..))`) was deleted and the test renamed rather than restated. | tests/it/temporal_package_route_formula.rs:140-170 |
| FND-006 | low | Public surface grew beyond the need: `ClauseFacts::with_formula` is used only by tests, `FormulaNodeId::from_arena_index` lets any crate mint ids of a checked arena, and `atom_numbers`, `bound_atom_indices` and `TemporalSubjectView::capture_names` became `pub`. | qsl-eval/src/temporal/facts.rs:89-93, qsl-semantics/src/check/temporal_clause.rs:182-188 |
| FND-007 | low | `graph_shape` and `window_of` repeat the closed-ends-and-integral-bounds predicate that `work::shape` and `Window::of` already hold, and `run_window` clones the whole `Plan` per evaluation. | qsl-eval/src/temporal/facts.rs:337-420, qsl-eval/src/temporal/finite_window/run.rs:212-216 |

## Verdict

Not mergeable as written: no high finding, but FND-001 and FND-002 should be fixed in the same round as the test gaps in the gap-analysis review. Gates on the head trial-merged with origin/main: fmt, `cargo check --locked --workspace --all-targets`, clippy `-D warnings` default and `--no-default-features`, arch-lint `qualified-core` and `api-surface --qsl . --qsl-only` all pass. `qsl-package` (645 + doc 52), `qsl-semantics` and `qsl-eval` pass; `qsl-replay` has 19 failures, a strict subset of the 20 known (the one missing on this head is `aggregate_package_accepts_temporal_selection_from_later_source`); root `it` plain (882) and `--all-features` (941) pass.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The replay `LedgerBinding.package` doc comment says "The checked package's name" but the value is the caller's source identity string. The new test `the_replay_binding_names_the_package_the_source_emits` compares the recipe's identity label with `package.name()`; it does not run the replay path, so it pins a coincidence of the recipe, not a rule. | qsl-replay/src/spine/clause/progress.rs:68, tests/it/bare_package.rs:936-955 |

## Dispositions

Round 1, head 003b53ccd (A) trial-merged with origin/main 3800efb02; #2311 head 2db5b5c3a is A plus the identity file.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | FR-532 defines "route" as the three evidence routes (from an `Emission`, from a read-back package, from a bare package); all three bind `package.name()` (test `the_progress_binding_names_the_package_and_no_source_label`). The replay ledger is a separate type no evaluator route reads. The label equals the manifest name only in the recipe; see FND-008 for the doc and test wording. |
| FND-002 | fixed | `bound_atom_indices` returns `None` on a count mismatch; assertions restored for `[true,false]`, `[]`, shared, instance-bound and distinct-after-shared atoms. Removing the guard fails `a_leaf_count_that_differs_from_the_holds_count_maps_no_atoms`. |
| FND-003 | fixed | `finite_window_profile` names the timed profile, so the evaluator gives its own refusal as in-process; unit test pins the closed table and `None` for unknown identities. |
| FND-004 | fixed | `dependencies()` deduplicates by pointer identity; `the_dependency_closure_follows_a_chain_and_lists_a_diamond_once` fails without the recursion. |
| FND-005 | accepted-no-change | The retained `assert_eq!` of the formula to `of_graph(view.formula_graph())` and of its work to the checked formula's work carry the AC-23 claim; the dropped inequality against the arena formula is trivially true once the formula holds a first-use plan. |
| FND-006 | fixed | `atom_numbers` crate-private; `with_formula` and `bound_atom_indices` `doc(hidden)`. `from_arena_index`, `capture_names`, `declaration_key` and `capture_value_types` stay `pub` because `qsl-eval` calls them across the crate boundary; accepted. |
| FND-007 | fixed | `counts_ticks` is the one predicate for the work counters, the plan and the cross-check. The per-evaluation `Plan` clone in `run_window` remains (low, same as the coordinator review's FND-003); no AC depends on it. |
