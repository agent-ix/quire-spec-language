---
id: SR-751
title: "QSL-278 gap analysis of PR 495 (FR-106, FR-107, FR-109; TC-464..468)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c7454d7c50d906dbbca511f91ae1ef4696c72f62; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/test-cases/TC-464-snapshots-and-invocations-admit.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md; spec/test-cases/TC-466-s6a-evaluates-state-clauses.md; spec/test-cases/TC-467-s6a-clause-entry-refusals-exhaustion-and-determinism.md; spec/test-cases/TC-468-spine-clause-run-reports-typed-dispositions.md; qsl-replay/src/spine/clause/tests.rs; qsl-semantics/tests/it/state_clauses.rs; qsl-semantics/src/model/observation/ordered_json.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-replay/src/spine/clause.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-465
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-468
    type: reviews
---
## Summary

Ticket: QSL-278. PR: quire-spec-language#495 at c7454d7c.

This analysis maps every TC-464 to TC-468 step or row to a test, by reading
the code only. The PR adds about 30 test functions. Most TC rows have no
test. The `qsl-replay` tests use their own `test/nodes`/`NoCycle` fixture,
not TC-458's ConfigVersion corpus. Their evaluate tests also build
`AdmittedObservations` by hand, so admission is skipped.
`observation.rs`, `document.rs` and `frame.rs` have no unit tests.

Backed: TC-465 row 28, TC-468 step 4 `ghost`, and the
`ProtocolClauseFamily::evaluate` seam (TC-467 step 4, which the `make ci`
seam-probe log confirms). Weakly backed:

- TC-465 row 7: document order differs from alphabetical order, but the code
  is not asserted.
- TC-465 row 26: a different mutation, and the field is not asserted.
- TC-464 step 3: asserts only `is_some`.
- TC-466 step 3a: a different clause, 3 expansions against the TC's 2, and
  the meter log is not isolated to the `reaches` node.
- TC-467 steps 1 to 3.
- TC-468 steps 1 and 2 (partial).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-465 (FR-106-AC-3, AC-4, AC-5, AC-7): about 38 of 42 rows have no test. Missing are rows 1-6, 8-25, 27 and 29-42, including the incomplete and dangling rows (23, 24), which would have caught SR-750 FND-001, and every multi-defect order row. | spec/test-cases/TC-465-admission-refuses-each-input-defect.md; qsl-semantics/tests/it/state_clauses.rs:1363-1480 |
| FND-002 | high | FR-109 Outputs are not implemented. `ClauseRunReport` holds only `source_digest`, `package_id` and `disposition`. It has no model selections, no selection as given, no snapshot or invocation identities and digests, no extraction provenance, no usage (admission work and meter charges), and no `category` or `truth`. `ClauseRunRequest` has no I3 extracted-source input. FR-109-AC-1 (provenance) and AC-5 ("equal reports including usage") cannot pass. | qsl-replay/src/spine/clause.rs:183-190; qsl-replay/src/spine/clause.rs:73-104 |
| FND-003 | medium | TC-466 (FR-107-AC-1..3): the ConfigVersion clauses `ParentOrder`, `NoCycle` and `VersionUnchanged` are never evaluated over admitted corpus documents. Step 2 (`pre(..)` reads and `deref` under `pre`, FR-107-AC-2) has no test. The step 3 `ReachesTarget` probe cases b-e have none either, including denying each of the 5 charges. | qsl-replay/src/spine/clause/tests.rs:376; qsl-replay/src/spine/clause/tests.rs:424 |
| FND-004 | medium | TC-464 (FR-106-AC-1, AC-2, AC-6): steps 1, 2 and 4 (healthy-parent admit, a re-serialized digest that gives an equal value, and an empty working directory) are missing. Step 3 asserts only that pre and post are present. | qsl-semantics/tests/it/state_clauses.rs:1392; qsl-semantics/tests/it/state_clauses.rs:1405 |
| FND-005 | medium | TC-467 and TC-468 gaps. TC-467 step 1 asserts neither `resource_exhausted`/`insufficient-next-charge` nor the default-budget rerun. Step 3 covers one case and compares a bool. Missing from TC-468: step 1 violating-parent (exit 10); step 2 missing-model and `sameIdentity` as a Clause; all of step 3 (dangling, incomplete, exhausted); the step 4 `sameIdentity` cases; and all of step 5, including the FR-100 mapping comparison and the `CheckedInvariant` and `CallFailure::Fault` internal-failure reports. | qsl-replay/src/spine/clause/tests.rs:536; qsl-replay/src/spine/clause/tests.rs:568; qsl-replay/src/spine/clause/tests.rs:632-900 |
| FND-006 | low | Trace tags that do not match. Three name ACs that do not exist: `FR-107-AC-7` (tests.rs:566), `FR-109-AC-6` (tests.rs:846) and `FR-109-AC-7` (tests.rs:877). The frame refusals are tagged TC-464 instead of TC-465 (state_clauses.rs:1390-1465). Check-1.6 tests are tagged FR-106-AC-2 instead of AC-3 (ordered_json.rs:243, tests.rs:911). The `allInstances`/`lookup` guard tests are tagged FR-107-AC-5 (evaluate.rs:2070, 2129). | qsl-replay/src/spine/clause/tests.rs:566; qsl-replay/src/spine/clause/tests.rs:846; qsl-replay/src/spine/clause/tests.rs:877; qsl-semantics/tests/it/state_clauses.rs:1390; qsl-semantics/src/model/observation/ordered_json.rs:243 |

## Verdict

Request changes. FR-106-AC-3..7, FR-107-AC-1..3 and FR-109-AC-1, AC-3 and
AC-5 are not delivered as the spec states. Write the missing TC rows over the
ConfigVersion corpus (they will expose SR-750 FND-001 and FND-005). Add
FR-109's provenance and usage to `ClauseRunReport`. Or split the missing
parts onto a follow-up ticket, with an honest FR Status line for each.
