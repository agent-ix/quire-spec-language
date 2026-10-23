---
id: SR-12367
title: "Gap analysis of QSL-596 part 1 against FR-280, FR-281, FR-286, FR-288, TC-761, TC-762, TC-763 (PR #1362)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d5d3c48f4029cd6ecfd3499bfcede27d4e2607ac; diff 19243dd26..d5d3c48f4 only (#1299 files excluded); spec/functional/FR-280 (AC-1, AC-2, AC-5), FR-281 (AC-1..AC-7), FR-286 (AC-4, context), FR-288 (context), spec/test-cases/TC-761, TC-762, TC-763; qsl-analyze/src/analyze.rs, analyze/items.rs, model_check/manifest.rs, qsl-analyze/tests/it/analyze.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-762
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-763
    type: reviews
---
# SR-12367: Gap analysis of QSL-596 part 1 against FR-280, FR-281, FR-286, FR-288, TC-761, TC-762, TC-763 (PR #1362)

## Summary

Ticket: QSL-596, part 1. Planless. Plan completion: not assessed. Only the diff 19243dd26..d5d3c48f4 was reviewed; #1299's files are excluded. Traceability comes from the trace tags in `qsl-analyze/tests/it/analyze.rs`. All 9 tagged tests were run and pass.

How each test binds to its criterion:

- **FR-281-AC-2:** partial. See FND-001.
- **FR-281-AC-3:** bound for the EN-1 halves (an EN-1 proved item, an EN-1 refuted item, and an unsupported item that names `monitorability`), with request order asserted. The EN-5 item is blocked.
- **FR-281-AC-4 and AC-5:** they bind the settlement arms only. See FND-003.
- **FR-281-AC-6:** bound over EN-1, `max_states` 1, giving `LimitReached{MaxStates, 1}`. The cancel case is an extra check. EN-5 is blocked.
- **FR-281-AC-7:** the settlement and the outcome mapping are bound. The real replay is injected.
- **FR-280-AC-1:** bound for EN-1.
- **FR-280-AC-2:** mis-tagged. See FND-002.
- **FR-280-AC-5:** the `analyze` half is bound.

How each gap and choice the coder recorded was judged:

- **EN-1 manifest domains `{population, integer}` and fairness `{weak, strong}`:** OK. It is a spec gap for draco (FND-006).
- **More than one candidate refused (`Ambiguous`):** a spec gap (FND-004). It is acceptable for now, but it breaks the one-record-per-item rule for the whole request.
- **EN-1 `ModelCheckRefusal` and FR-192 `UnsettledRefusal` returned as `Unsettled`:** a spec gap (FND-004). Not inventing a verdict is the right choice.
- **EN-5 parts blocked (FR-281-AC-1, the EN-5 halves of AC-3 and AC-6):** OK, truly blocked. EN-5 has no solver or FR-201 checker yet (QSL-574/575/576 are open). These ACs stay open.
- **FR-281-AC-7 replay injected:** OK as a recorded block. `ModelTraceRefusal::UnknownTemporalClause` exists (FR-337). AC-7 stays open.
- **Part 2 (FR-292, TC-776):** not started. It is QSL-596's remaining scope.

## Verdict

**Gap analysis: no high finding.** There are three mediums: FND-001, FND-002 and FND-003. The tests are real EN-1 and EN-4 runs over actual models. The EN-4 equality in FR-280-AC-5 holds by construction, because `analyze` calls `serve_request`. It is still not a tautology: the test asserts the routed backend id, the request index, a deterministic seeded re-run, `measured`/`rejected` and three witnesses.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-281-AC-2 asks for an EN-1 item claiming *an invariant that one transition breaks*. The tagged test runs `ModelCheckItem::DeadlockFreedom` on the Counter subject. The AC-3 refuted item is the same deadlock case. If `replay_model_trace` cannot yet replay an authored-clause counterexample (FR-337), then AC-2 is blocked for the same reason as AC-7, and the PR's gap list leaves it out. Either test an invariant clause, or record AC-2 as blocked and keep it open. | qsl-analyze/tests/it/analyze.rs:84-118 |
| FND-002 | medium | The EN-4 `Service` test carries the FR-280-AC-2 tag. AC-2 is "for each TC-762 item", which means the EN-1 and EN-5 items. The EN-4 `Service` item is TC-761 step 4 (AC-5). There is no EN-1 provider entry, so FR-280's entry ("routed to a QSL engine") and AC-2 are not met for EN-1. Fix: add the EN-1 entry (or one generic entry that runs `analyze`, which FR-280 Behavior 5 asks for), test equality for the EN-1 TC-762 items, and retag. | qsl-analyze/tests/it/analyze.rs:486-529 |
| FND-003 | medium | The FR-281-AC-4 and AC-5 tests inject their outcomes. The "checker" is a closure that returns `Err(BadState)` or `Ok(())`, and the replay outcomes `NotReproduced(..)` and `Refused(UnknownParameter(..))` are constructed directly. No certificate member is altered, no FR-282 checker runs, and no replay runs. They test only the four match arms, not TC-763 steps 1 and 3 as written. This follows from SR-12366 FND-002: once `analyze` owns the checker and the replay, the tests should alter a real certificate and replay a real counterexample. | qsl-analyze/tests/it/analyze.rs:174-294 |
| FND-004 | low | Spec gap for draco. FR-281 Behavior 1 ("exactly one terminal record for each requested item") gives no record for (a) an item with more than one candidate. The code refuses the whole request (`AnalyzeRefusal::Ambiguous`), so every other item loses its record too. It also gives none for (b) an engine refusal with no FR-331 record (EN-1 `ModelCheckRefusal`, FR-192 `UnsettledRefusal`), which the code returns as `Err(Unsettled)`. FR-281 or FR-290 should say what each settles. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md; qsl-analyze/src/analyze.rs:268-306 |
| FND-005 | low | Spec gap for draco. FR-281 asks for "a typed cause that names the claim kind", but FR-286 and the QSpec FR-331 cause vocabulary define no wire spelling for it. The code invents the cause `unsupported-claim-kind` and the outcome member `claim_kind`. Specify them, or name the existing cause to use. | qsl-replay/src/proof_result.rs:449-455; qsl-replay/src/outcome.rs:251-254 |
| FND-006 | low | Spec gap for draco. FR-280 Behavior 4 spells out EN-4's manifest but not EN-1's (pair, `domains`, `fairness_kinds`). The coder chose (`temporal-satisfaction`, `bounded`), `{population, integer}` and `{weak, strong}`. That choice is consistent with EN-1's code and acceptable. FR-280 should state it. | qsl-analyze/src/model_check/manifest.rs:26-39 |
