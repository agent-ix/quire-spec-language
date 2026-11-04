---
id: SR-2263002
title: "Gap analysis of quire-spec-language PR 2263: FR-337-AC-23, FR-515-AC-13 and AC-18..23, FR-516-AC-18, AC-29..33 and AC-36..44 against the tagged tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@755f23585; PR #2263 (QSL-1608): FR-337-AC-23, FR-515-AC-13, FR-515-AC-18..23, FR-516-AC-18, FR-516-AC-29..33, FR-516-AC-36..44 against tests/it/temporal_package_route.rs, qsl-eval/tests/it/{temporal_causes,temporal_rulings,temporal_activation}.rs, qsl-package/src/emit/tests/{temporal_package_route,checked_views}.rs"
review_set: subset
---

## Summary

Ticket: QSL-1608. Planless gap analysis of the PR diff; plan completion: not assessed. The spec read is the merged text on origin/main (a5939662a, 8a68008c4, 498994b3f), not the coder's summary. Tests read for what they assert; eleven mutations run (listed in SR-2263001).

Mapping, AC to tagged test, all present and passing at 755f23585:

- FR-337-AC-23: qsl-package emit/tests/temporal_package_route.rs (TC-1326).
- FR-515-AC-13: emit/tests/checked_views.rs. AC-18, AC-21: tests/it/temporal_package_route.rs both_routes_hold_the_emissions_rows_and_ids (rows equal, ids equal the name rows). AC-19: each_clause_pairs_with_its_own_clock_row and the qsl-package test. AC-20: twins test (ordinals 7,3; rows 3,7; read-back only; killed by pairing by declaration and by dropping the selector). AC-22: instance_bound_atoms_are_derived_and_equal_over_both_routes (index shift under formula swap verified, killed by index and capture-name mutations). AC-23: both_routes_give_the_evaluator_the_same_formula and the unreadable-graph test.
- FR-516-AC-18 and AC-29: the call test compares Debug text, result_identity and the charge log between the two routes over R's three clauses. Its replaced-input checks (declaration, instance-bound atom, clock row) mutate EvidenceSubject fields after construction, so they prove the evaluator reads those inputs, not that the routes derive them; the route-derived parts are compared by the rows and the subject-equality tests. AC-30, AC-32, AC-33: present. AC-36, AC-38 to AC-44: present (AC-38, AC-42, AC-43, AC-44 in temporal_rulings.rs).

Coder's seven issues, judged against the merged text:

1. FR-515-AC-22 fourth atom (trigger binder and a capture): spec defect. The checker refuses a holds atom that names the on-each trigger binder (reproduced, MissingName q), so the AC's vector cannot be authored and the trigger-binder branch of instance_bound_atoms is dead code in practice; removing it passes every test (FND-001).
2. progress-ledger-limit: not an issue on the merged spec. The FR-516 cause table does not list it, the Outputs row (incomplete) and AC-38 specify an Incomplete(LimitReached) result, and the code and test do exactly that. Accepted, no change.
3. FR-516-AC-37 timestamped-event assertion: spec defect, see FND-002.
4. CheckedPackageEvaluation: the spec names a type that does not exist; the call is EvidencePackage (FND-004).
5. clause-root-unresolved: spec gap, see FND-003.
6. Which clauses the ACs use: accepted. AC-18 reads three clauses (Due, DueSample, DueTimestamp) and the test runs exactly those; AC-23 reads each clause of R and the test iterates all four named clauses including DueLate.
7. The in-memory clause is not cross-checked: code finding in SR-2263001 FND-001. The pairing of the in-memory clause by TemporalClauseRef.clause position or by name is what FR-515 "Clause and row by key" states for the clause; it is not a position pairing of clock rows or occurrences, which are keyed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-515-AC-22's vector (an atom reading the trigger binder and a capture, index 3) is untestable: the checker refuses an atom that reads the on-each trigger binder (reproduced in a scratch test: `on each (q: Config::ConfigVersion) when (q.versionNumber >= 2) { always[0,5] (holds(q.versionNumber >= 1)) }` fails S1 to S4 with MissingName("q")), and the test builds index 3 from over and a capture instead. The trigger-binder branch of instance_bound_atoms is exercised by no test: removing it passes every suite. Spec lane: restate AC-22 with an authorable vector (and FR-515's trigger-binder sentence), or state that no atom can read the trigger binder and delete the branch. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:242 |
| FND-002 | medium | FR-516-AC-37 ends "an assertion on a timestamped-event clause returns progress-inconsistent too", and the table cause says "an assertion on a clause whose profile is neither". The timestamped-event path of the call admits completeness and watermark assertions (FR-516-AC-8 and AC-42 evaluate timestamped traces under them), and no test covers the AC's third clause. The text conflicts with FR-501 and FR-504, so this needs a spec ruling before the code or a test can follow. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:352 |
| FND-003 | medium | The call refuses an unknown clause with cause clause-root-unresolved (reused from FR-337), but the FR-516 table says "No other cause exists" and lists no cause for a clause not held by the package or not a temporal clause (the Refusal paragraph names the case without a cause). Add the row to the table and to AC-41's universe. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:127-150 |
| FND-004 | low | FR-515 "The in-process package", FR-516 Entry and AC-18/29 name CheckedPackageEvaluation, and FR-515-AC-18 says temporal_subject runs over it. The type is qsl_eval's EvidencePackage (the trait of that name cannot be implemented for CheckedPackage). Restate the spec with the real type and entry. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:169 |
| FND-005 | low | FR-516-AC-41 asks each cause of the table to refuse with an empty charge log and unchanged ledgers, with an unmet twin that is evaluated. The new loop omits activation-mapping-mismatch (its existing test, temporal_activation.rs control_must_map_to_the_clause, asserts neither charges nor ledgers) and asserts no ledger in any row. | qsl-eval/tests/it/temporal_causes.rs:289-460 |

## Verdict

Findings are spec gaps (FND-001 to FND-004, for the spec lane) and one test gap (FND-005); none shows a wrong evaluation result. Combined with SR-2263001 FND-001 (high) the PR is not mergeable as is.
