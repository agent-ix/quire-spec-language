---
id: SR-1953002
title: "Gap analysis of PR #1953 (QSL-1340): FR-411-AC-5 deadlock verdict join"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1953 git diff origin/main...2282c6a49 (FR-411-AC-5, TC-1142 step 5; qsl-analyze/src/analyze/items.rs; qsl-analyze/tests/it/derived_request.rs); ticket QSL-1340"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-411
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1142
  type: reviews
---
# SR-1953002: gap analysis of PR #1953 (QSL-1340)

## Summary

Ticket: QSL-1340. Plan completion: not assessed. The one new test, `a_deadlock_verdict_is_joined_on_the_terminal_declaration`, is tagged `#[trace("TC-1142", "FR-411-AC-5")]` and binds correctly. I mutated the head to measure what it kills. With both mutations below applied together, the 129 qsl-analyze `it` tests matching derived_request/deadlock_item/zone_ still passed. The mutations were (a) dropping the run package from `derived_model_item`'s join and (b) deleting the EN-6 join in `derived_zone_item`. The coder's own mutation of the replay wrapper is killed, by the last assertion of the test.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-411-AC-5 | spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md:144 | examined |
| TC-1142 step 5 | spec/test-cases/TC-1142-a-model-terminal-declaration-emits-a-v2-terminal-node.md:38-46 | examined |
| a_deadlock_verdict_is_joined_on_the_terminal_declaration | qsl-analyze/tests/it/derived_request.rs:631-741 | examined |
| derived_model_item / derived_zone_item joins | qsl-analyze/src/analyze/items.rs:404-411,528-531 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The EN-6 join is untested. Deleting the whole `if kind == ItemKind::DeadlockFreedom { join_terminals(...); joined_replay(...) }` block from `derived_zone_item` leaves every test green. FR-411's join section names the EN-6 adapter, so it needs a timed case: a certificate over the other declaration refuses, and a reproduced replay under the other declaration settles inconclusive. Once SR-1953001 FND-001 is fixed, a run-only mismatch over a symmetry-reduced run should refuse too. | qsl-analyze/src/analyze/items.rs:528-531 |
| FND-002 | medium | The run-package comparison in `derived_model_item` is not killed. The test covers run plus certificate both mismatched, and certificate alone mismatched (`mixed.certificate = none.request()`), but never run alone. Removing `item.request.subject.simulation.package` from the join array leaves the test green. Add the reverse `mixed` case (when-certificate, none-run). | qsl-analyze/src/analyze/items.rs:404-410; qsl-analyze/tests/it/derived_request.rs:698-705 |
| FND-003 | low | Every key-mismatch assertion compares `Some(key)` with `None`. No assertion shows that two different `when` predicates give two different keys, and that is the realistic edit and the premise of the join. A reviewer probe confirmed they differ (`self.value = 3` vs `= 2`), so this is a missing assertion, not a defect. | qsl-analyze/tests/it/derived_request.rs:681-686,724-738 |

## Verdict

**Changes requested (two medium).** FR-411-AC-5's EN-1 halves are backed: obligation equality, run+certificate refusal, certificate-only refusal, the reverse direction, `proved`/`refuted` on matching packages, and the replay refusal. The test fails on origin/main because `DerivedItemMismatch::TerminalDeclaration` and `TerminalMismatch` do not exist there. There is no stub or tautology. The tests that only had a field added (dense_route.rs, zone_replay.rs) are construction updates.

## Dispositions

Round 1, fix commit `cfe66261f`. Each mutation below was applied alone and `cargo test -p qsl-analyze --test it -- derived_request` re-run.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | New EN-6 test: reduced-run mismatch, certificate-only mismatch, matching reduced proof. Deleting the EN-6 terminal join fails `a_timed_deadlock_verdict_is_joined_on_the_run_and_check_packages`, as does dropping the run package from it. |
| FND-002 | fixed | Run-only case (`none` run, `when` certificate) added; dropping the run package from `derived_model_item`'s join fails `a_deadlock_verdict_is_joined_on_the_terminal_declaration`. |
| FND-003 | fixed | Asserts the `value = 3` and `value = 2` keys differ, and a run and check over the `value = 2` package refuse naming both keys. |
