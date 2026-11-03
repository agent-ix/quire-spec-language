---
id: SR-1493003
title: "gap-analysis of QSL-1493 FR-501-AC-1..12 and FR-330-AC-9 finite-window part (quire-spec-language PR 2165)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-501-*.md; spec/functional/FR-330-*.md; spec/test-cases/TC-1295-*.md; spec/functional/FR-285-*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
---

# Gap analysis of QSL-1493 finite-window evaluation (quire-spec-language PR 2165)

## Summary

Ticket: QSL-1493. Head 7826aa7f2a73. Planless; criteria read from main. Plan completion: not assessed.
FR-501-AC-13..17 are declared out of this PR (stacked follow-up) and are not counted as findings;
note the ticket text names TC-1295 rows 1-26 and the PR's tests carry tags for AC-1..12 and
FR-330-AC-9 only.

Per criterion: AC-1, 2, 3, 5, 6, 10, 12 are implemented and asserted with values on the closed
and open cases. AC-4 asserts missing/pending/true but reports the atom as "holds-node@1" not `p`
and models a conflicting valuation as two positions at one stamp. AC-7 is asserted with the same
fixtures (refusal position 2) but the refusal is broader than FR-501 (see SR-1493002 FND-005).
AC-8 asserts only the activation-0 `once` case. AC-9 clause-level cases pass only on the one
clause shape the override recognises (SR-1493002 FND-002); TCV vectors are hand-copied, not
read from QSPEC_DIR, and TCV-02/05 are absent. AC-11 asserts category labels and exits on
TemporalVerdict values; FR-285-AC-7 exit 11 is not reached by category().trace_exit_code().
FR-330-AC-9: Timestamped/profile shape mismatch refusal is asserted; the ledger and watermark
`contradicted_dimension` sentence is not in this PR (separate ticket QSL-1494 per the planner).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-501-AC-8 and TC-1295 row 19 are green only for the one fixture (`once[0,4]`, stamps 3,4, false/false, activation at position 0). The implementation treats every Violated verdict of a past-operator clause as MissingHistory (SR-1493002 FND-001); no test pins the rule's exception (a definite false from present history, or an activation whose boundary lies inside the trace), and the row-19 sentence that position 1's activation is also Indeterminate is not asserted. | qsl-replay/src/spine/clause/tests/temporal.rs:1784 | correct-requirement-no-evidence |
| FND-002 | high | FR-501-AC-9 and AC-4 conflicting-valuation coverage is shape-specific: tests use only `implies eventually[0,3000]` (straddle) and root `eventually[0,5]` (conflict), exactly the two shapes the replay overrides detect. There is no test of a straddle or conflict under another clause shape, so the passing tests do not show the verdict classes are evaluator semantics. | qsl-replay/src/spine/clause/tests/temporal.rs:1490,1582 | correct-requirement-no-evidence |
| FND-003 | medium | TC-1295 row 21 requires the QSPEC_DIR tick-cover-vectors.json cases (TCV-02, 05, 09..15) to be run through within_bound and definitely_before; the tests hard-code a subset and never read the file, TCV-02 and TCV-05 are not asserted. | qsl-replay/src/spine/clause/tests/temporal.rs:1490-1518 | correct-requirement-no-evidence |
| FND-004 | medium | FR-285-AC-7 requires an Indeterminate clause run to exit 11 and keep its label and reason in machine output. The QSL-local category label and exit_code() exist, but the run path (category() -> Inconclusive, trace exit 0; monitor path outcome.rs:1215) does not use them and nothing calls temporal_batch_exit_code. No ticket owns adding Indeterminate to quire-semantic-value Category, which is what lets the local copy be deleted. | qsl-replay/src/spine/clause.rs | missing-requirement |
| FND-005 | medium | The ticket's "remove the ProfileNotEvaluated refusal for this profile (qsl-eval/src/temporal/engine.rs)" is not done: engine.rs:184 still refuses the profile; a parallel entry bypasses it. FR-330-AC-9's "never ProfileNotEvaluated" is met only on the spine route. | qsl-eval/src/temporal/engine.rs:184 | implementation-bug-despite-evidence |
| FND-006 | low | FR-501-AC-12 has a meter-6 / meter-7 assertion but the sibling-activation sentence (a verdict settled before the denied charge is kept) and the run-twice equality of AC-1..11 are not asserted. | qsl-replay/src/spine/clause/tests/temporal.rs:1637 | correct-requirement-no-evidence |

## Verdict

Gaps found in verification strength for AC-4/8/9 and a ticket-stated removal not done. Code
defects behind them are in SR-1493002.

## Dispositions

Round 1 against stacked PR 2169 head 0d9e923a61f1 (effective diff origin/main...head, main ceecfab74). Gates: workspace check and clippy -D warnings clean (default and --no-default-features); qsl-eval and qsl-replay tests green; qsl-semantics --all-features: 4 failures identical on origin/main (control run), none new; arch-lint qualified-core PASS.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | As SR-1493002 FND-001: historically/once/since and the activation at position 1 are asserted (finite_window.rs:1245-1265 and spine tests); mutation history-off is killed. |
| FND-002 | fixed | Straddle and conflict are asserted under other clause shapes through the generic lift (possibility-lift and cover tests); mutations straddle-sure and atom-classes-off are killed. |
| FND-003 | fixed | TCV-01..15 are read from QSPEC_DIR and asserted by id in both crates. |
| FND-004 | deferred: FR-285-AC-7 machine output tracked by SR-1493002 FND-012 | label_and_exit is the single mapping read by run_clause and the ranking (FR-285-AC-10/12, spec interim mapping). FR-285-AC-7's machine-output sentence is not met: write_clause_run_result refuses every temporal disposition. See SR-1493002 FND-012; no ticket yet. |
| FND-005 | fixed | engine.rs no longer refuses the profile as ProfileNotEvaluated (TraceCarriesNoStamps for a stampless trace). |
| FND-006 | fixed | a_settled_sibling_keeps_its_verdict_when_a_charge_is_denied (finite_window.rs:783-795) and the run-twice equality tests (spine temporal tests around 1137) assert the sentences; AC-13..25 are tagged and asserted. |

Round 2 against stacked PR 2169 head 28a7a3e2e0b5. No gap finding changed; the FND-004 machine-output half stays with SR-1493002 FND-012 (deferred, ticket filed by the code lead). The dispositions above are current.
