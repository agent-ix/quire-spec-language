---
id: SR-2026110216382
title: 'PR #2152 code-review: AC sweep FR-307 reusable semantic libraries (QSL-1491)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@peter/qsl-1491-ac-sweep; git diff origin/main...pr2152 (PR #2152): tests and trace tags only; tickets QSL-1491'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1491. PR: quire-spec-language#2152 (head peter/qsl-1491-ac-sweep). Method: code-review over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSpec-FR-307-AC-4 (QSL-1491) | spec/functional/tooling/FR-307-package-reusable-semantic-libraries.md; tests: qsl-replay/src/spine/dependency_tests.rs; qsl-semantics/tests/it/library_resolution.rs a_duplicate_import_qualifier_is_ambiguous_and_an_unaliased_import_binds_no_name; a_cycle_and_a_library_refusal; l01; l05; l07 | examined |
| QSpec-FR-307-AC-6 (QSL-1491) | spec/functional/tooling/FR-307-package-reusable-semantic-libraries.md; tests: qsl-cst/tests/it/selection.rs; qsl-replay dependency_tests.rs; qsl-semantics library_resolution.rs an_import_names_only_its_library_identity; selection_and_dependency_input_refusals; an_imported_name_whose_signature_is_package_dependent_refuses; an_imported_call_evaluates_in_the_library_and_returns_to_the_caller; conflicting_definition_classifies_to_i2_rule_2; missing_import_classifies_to_i2_rule_1; l01 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Stale trace tags remain on the PR tree: QSpec-FR-307-AC-5 (withdrawn) on an_import_view_lists_each_export_with_its_structural_key, an_export_name_that_is_not_a_qualified_name_is_invalid and a third test, and QSpec-FR-307-AC-3 (no such criterion in the current QSpec FR-307) on l07_migration_is_a_changed_export_set_and_never_relabels_evidence, where this PR adds the new AC-4 tag directly beneath the stale AC-3 one. The PR notes them and leaves them in place. They bind tests to criteria that no longer exist; remove them (binding to nothing, not to a replacement). | qsl-semantics/tests/it/library_resolution.rs:193-196, 234, 242, 274 |
| FND-002 | low | token_offset reconstructs the unit header text ("language ... profile v = ...;\n") as a string literal duplicating the one parse_import builds. If parse_import's header changes, the expected offset drifts and the test reports a spurious failure instead of tracking it. Derive the offset from the same source string the parse used. | qsl-cst/tests/it/selection.rs:184-188 |
| FND-003 | low | The AC-4 tag on l07 stands for "conflicting diamond": l07 asserts ConflictingDefinition when two packages are supplied under one identity (invalid_package/conflicting-definition), which is the AC-6 clause. No test builds a diamond whose two branches reach different packages. Because resolution is by identity the two are probably the same code path, so this is a judgement call. | qsl-semantics/tests/it/library_resolution.rs:193-232 |

## Verdict

3 finding(s). Every added or changed test was read against the QSpec acceptance-criterion text. Gates on the trial merge: cargo test --all-features -p qsl-cst -p qsl-replay -p qsl-semantics all pass; workspace check, clippy x2, arch-lint rc=0. Ticket closability per ticket:

- QSL-1491: YES against its exit criterion (AC-4 and AC-6 both tagged and passing on the trial merge, and FR-307 has no other untagged live AC); remove the stale AC-3/AC-5 tags, non-blocking.
