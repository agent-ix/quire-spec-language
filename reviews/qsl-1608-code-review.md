---
id: SR-2263001
title: "Code review (rust-review lane) - QSL-1608 package route for temporal evidence, PR 2263"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@755f23585; PR #2263: qsl-eval/src/temporal/evidence/{package,work,call,counters,request}.rs; qsl-package/src/{emit,checked_v2,lib}.rs, emit/monitor_plan.rs, checked_views/{subject,predicate,mapping}.rs; tests/it/temporal_package_route.rs; qsl-eval/tests/it/temporal_causes.rs; qsl-package/src/emit/tests/temporal_package_route.rs"
review_set: subset
---

## Summary

Ticket: QSL-1608. Reviewed head 755f23585 (the coder's merge of origin/main 13c7edf0f, QSL-1463 #2257, into the branch; earlier heads e91e56e71 and 14558b343 were reviewed first, and the production code of the two review targets is unchanged between 14558b343 and 755f23585 apart from the monitor_plan carry). Trial merge is a fast-forward: the head already contains origin/main.

Gates, run in the clone with qsl-slot and qsl-timeout, QSPEC_DIR=/Users/peter/dev/quire-specification (HEAD ad180f162 at the first run):

- (all gates below re-run at 755f23585 after the merge of #2257; results equal or better)
- cargo fmt --all -- --check: exit 0
- cargo check --locked --workspace --all-targets: exit 0
- cargo clippy --locked --workspace --all-targets -D warnings: exit 0 (default features) and exit 0 (--no-default-features)
- arch-lint qualified-core --qsl . and api-surface --qsl . --qsl-only: exit 0, all PASS
- qsl-replay --no-fail-fast: 14 lib + 4 dispatch_link_compile + 1 doctest = the 19 known names (13 layers::equality + frame_occurrence_region, 4 dispatch_link_compile, replay_observed_temporal)
- qsl-package lib: at 14558b343 611 passed, 7 failed; at 755f23585 618 passed, 6 failed (the protocol_view one passes after #2257). Known names: a_node_a_state_clause_places_does_not_move_to_a_type_name, a_valid_attempt_does_not_let_garbage_content_through, an_attempt_and_its_clause_share_one_frame_node_through_emission, an_inherited_operation_binds_its_declaring_types_anchor_and_frame, an_operation_named_only_by_an_attempt_emits_its_anchor_frame_and_record, anchor_occurrence_ordinals_follow_source_order, protocol_view::an_unknown_or_non_clause_node_refuses_without_a_partial_result); all integration tests and 49 doctests green
- qsl-eval: 182 unit, 1054 it (7 ignored), 43 doctests, 0 failed at 755f23585 (1050 it at 14558b343)
- root `it`: plain 878 passed 0 failed; with --all-features 920 passed 0 failed at 755f23585 (877 and 919 at 14558b343) (the temporal_package_route module is gated by feature checked-handoff, so the plain run does not execute it; family_outcome_layering passes in both, so the root dev-dependency on qsl-package with test-support is allowed)

Production code reviewed for unwrap, expect, panic, allow: none added. The test-support hook Emission::with_package_bytes only replaces the package bytes, which still pass the locked reader; it is compiled only under the feature that the root dev-dependency enables, so it cannot reach the shipped library, and a twin built with it fails EvidencePackage::from_emission by design (rows are stale), which the twin test respects by using only the read-back route.

Mutation probes, each applied to a scratch edit of the clone and reverted (root route tests with --all-features, qsl-package route tests, whole qsl-eval):

- clock_of in package.rs pairs by declaration only (first row): killed by the twin test.
- view clock_of in subject.rs pairs by declaration only: killed by the qsl-package twin test (not by the root tests, which use the package's own clock_of).
- occurrence selector dropped in call.rs resolve: killed by the twin tests (root and qsl-eval).
- occurrence selector dropped in EvidencePackage::subject only: survived everything (FND-003).
- instance-bound index off by one: killed by three tests.
- instance-bound capture-name match disabled: killed by two tests.
- instance-bound trigger-binder branch removed: survived (see the gap analysis, SR-2263002 FND-001).
- ClauseFormula::of_graph replaced by of_checked in subject(): survived everything (FND-001).
- package graph truncated to its first node: killed, only through the work counters and readability (FND-001).
- clause position off by one in subject(): killed.
- LedgerBinding.occurrence of the package subject set to 0: survived (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-515 "One evaluator formula" says the evaluator's formula is built from formula_graph() and from no other data, and FR-515-AC-23 and the module docs say both routes give the evaluator the same formula by construction. The call reads the package formula only for ClauseWork (assess and run_instance via formula.work()); the truth evaluation is evaluate_finite_window_at over subject.clause, the in-memory CheckedTemporalClause taken from the source graph, and the instance-bound atom indices are computed by walking checked.formula(). Nothing compares the package's formula_graph with that clause beyond the holds count, so a package whose graph disagrees with the in-memory clause is evaluated silently on the in-memory one. Mutation replacing ClauseFormula::of_graph by of_checked in EvidencePackage::subject passes every test of qsl-eval, qsl-package and the root route tests; the only effect of the graph is on work counters and clause-unreadable. Recommendation: in EvidencePackage::subject compare the package graph with the checked formula by a pre-order walk (operation identity, interval, operand positions, holds count) and refuse clause-unreadable on any difference, with a test that edits one operation or interval of the package side; or restate FR-515 and AC-23 to say the graph measures work and reads the atoms while the clause is the in-memory one, and add the cross-check as a requirement. | qsl-eval/src/temporal/evidence/package.rs:290-310 |
| FND-002 | medium | Pairing of the in-memory clause is unchecked on both routes. EvidencePackage::of_read pairs a name row to the first graph temporal clause with the same name (position over names, no uniqueness check), and from_emission checks that each emission reference matches a name row by declaration and occurrence but not that the clause at found.clause carries the row's name. Two clauses of one name, or an emission whose clause position names another clause, evaluate the wrong clause without a refusal. No test has two graph clauses with one name. FR-515 states the pairing rule; it does not state what happens when it is ambiguous. | qsl-eval/src/temporal/evidence/package.rs:70-90 |
| FND-003 | low | EvidencePackage::subject selects the occurrence itself (select_occurrence, then the clause lookup and the LedgerBinding occurrence) and evaluate_temporal_evidence selects it again in call.rs resolve. Dropping the selector in subject() and setting the subject's progress occurrence to 0 both survive every test, because the second selection decides the clock row and the result. The two selection sites can disagree without any test noticing; keep one. | qsl-eval/src/temporal/evidence/package.rs:262-300 |

## Verdict

Not mergeable until FND-001 is resolved (code cross-check, or the spec restated and the claim in the module docs removed). FND-002 and FND-003 can be fixed in the same round. The key-based pairing of clock rows and occurrences is correct and tested: pairing by position or by declaration alone fails the twin tests, and the twin package is admitted by the locked reader. The carry of the new TemporalRows through carry_rows after #2257 re-keys declarations of clauses and bindings, destructures each row without a rest pattern, and is covered by the extended retained_rows tests; a missed carry would fail closed with EmissionMismatch.
