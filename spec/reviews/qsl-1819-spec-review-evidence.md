---
id: SR-20261109172912000003
title: "Evidence review of the L5 engine-gap spec batch"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; QSL-1819 (with QSL-1820); branch spec/l5-engine-gaps @ 9514a0e41 vs origin/main db995d817; ADR-029; FR-195; FR-201; FR-280; FR-281; FR-282; FR-286; FR-564; FR-565; FR-567; TC-761; TC-762; TC-763; TC-770; TC-1391; TC-1393; TC-1394"
review_set: subset
---

## Summary

Ticket: QSL-1819 (with QSL-1820). Checks whether each new or edited AC can fail and whether its TC step exercises it: FR-201-AC-8, FR-280-AC-2/5/8/9, FR-281-AC-1/2/6/11/20/21/22, FR-286-AC-14, FR-564-AC-4/6/8, FR-565-AC-1, FR-567-AC-6, and the TC procedures.

## Verdict

Not mergeable until FND-001 and FND-002 are fixed. Clean: FR-280-AC-8 (descriptor, en-5, routing exact to en-5 and statistical to en-4) can fail and is checked by TC-761 step 7. FR-281-AC-6 (a) and (b) can each fail and are checked by TC-763 step 4. FR-286-AC-14 has a positive case, a removed-counterexample case and a proved case. FR-565-AC-1 and TC-1394 step 1 now compare variants per cell without a rotting count.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The dagger clauses are conditional on build state ('once it is a Case', 'a dagger cell whose route exists'). If every dagger cell stays PendingEngine indefinitely, AC-4, AC-6, AC-8, FR-565-AC-1 and FR-567-AC-6 all pass, which is exactly the broken state this batch is meant to remove. Reasons 5 and 6 describe the current build ('has no byte-driven route'), not target design, and the table does not say which dagger cell carries reason 5 and which carries 6 (the p3-matrix code uses its own per-cell strings). Target design: these cells are Cases with the inputs the table names, and the ACs state their results unconditionally. The pending state belongs in QSL-1819/1820, not in the spec. | FR-564:83-88, 126-131, 184, 186, 188 |
| FND-002 | medium | AC-21's UnnamedClause case ('a clause named by a non-identifier') has no TC step: TC-762 step 8 runs only the two UnknownClaim cases and the positive key. The input is also unclear. On main, UnnamedClause fires when the clause name is not an ASCII `Identifier` (qsl-analyze/src/analyze/clause_item.rs:319). The code's own test notes (tests/it/clause_item.rs:229-231) that it is unknown whether S1 admits such a name. If S1 never admits one, the case cannot be built. Either pin the input (for example, a non-ASCII clause name that S1 admits) and add it to TC-762 step 8, or drop UnnamedClause from the AC and the refusal set. | FR-281:536; TC-762:28, 41 |
| FND-003 | low | 'bytes of a different source' under the right label is not pinned, and the prose (173-178) claims that bytes other than the checked ones always make the counterexample fail to replay. That is not so for a semantics-preserving change, such as a comment edit, where a refuted result is correct. Pin the substitute, for example the Counter unit with P's bound changed to `< 5` so that the counterexample does not reproduce, and soften the prose to say 'may then fail to replay'. | FR-281:173-178, 535 |
| FND-004 | low | 'is refused as an internal fault' names no observable value: from_analyze's return type and its error variant or catalog code are not given. Name the result, for example Err(OutcomeError::InternalFault) or FR-285's internal failure with its code, so that the test can assert it. | FR-286:186-187, 273 |

## Coverage

- FR-201-AC-8 (examined)
- FR-280-AC-2 (examined)
- FR-280-AC-5 (examined)
- FR-280-AC-8 (examined)
- FR-280-AC-9 (examined)
- FR-281-AC-1 (examined)
- FR-281-AC-2 (examined)
- FR-281-AC-6 (examined)
- FR-281-AC-11 (examined)
- FR-281-AC-20 (examined)
- FR-281-AC-21 (examined)
- FR-281-AC-22 (examined)
- FR-286-AC-14 (examined)
- FR-564-AC-4 (examined)
- FR-564-AC-6 (examined)
- FR-564-AC-8 (examined)
- FR-565-AC-1 (examined)
- FR-567-AC-6 (examined)
- TC-761 (examined)
- TC-762 (examined)
- TC-763 (examined)
- TC-770 (examined)
- TC-1391 (examined)
- TC-1393 (examined)
- TC-1394 (examined)

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-201-AC-8 runs 'over §15.4's Link model' every certificate of AC-1 to AC-5. Only AC-1's certificate is a Link certificate: AC-2's LongRun is §15.3's, AC-3's FairTerminates is §15.6's Coin, AC-2's dyadic Lower is §15.1's and AC-5's is the Opt model. As written, the AC cannot be built for those. It also asks for 'the verdict that AC gives the wire form', but none of those models emits, so no wire-form verdict exists to compare against. Reword: 'over each AC's own model, the checked-subject form gives the verdict, rule and locus that AC states'. | FR-201:197-203 |
| FND-006 | low | The rewritten FR-567-AC-6 (an analyze/violation case expecting exit 0 gives not-qualified) is the same mechanism as AC-2 (a check/refusal case expecting exit 21), applied to another cell. TC-1396's scope is AC-1 to AC-5 and it has no step for AC-6, so AC-6 is untested. Delete AC-6, since AC-2 covers the rule, or add it to TC-1396's scope with a step. | FR-567:103, 107; TC-1396:16 |

## Dispositions

Round 1, head b8ff9646e.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Dagger and PendingEngine removed from FR-564, FR-565 and FR-567. AC-4, AC-6 and FR-565-AC-1 are unconditional. TC-1391, TC-1393 and TC-1394 match, and no PendingEngine or dagger is left in spec/. The p3-matrix code still has PendingEngine and must follow. |
| FND-002 | fixed | UnnamedClause dropped. That is acceptable: the lexer admits only ASCII identifiers (qsl-cst/src/cst.rs:25), so the case cannot be built. ClauseItemRefusal::UnnamedClause on main (qsl-analyze/src/analyze/clause_item.rs:86) becomes unreachable and the coder should remove it. |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
