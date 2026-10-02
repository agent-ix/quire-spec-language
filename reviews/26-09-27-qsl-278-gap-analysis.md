---
id: SR-751
title: "QSL-278 gap analysis of PR 495 (FR-106, FR-107, FR-109; TC-464..468)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/test-cases/TC-464-snapshots-and-invocations-admit.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md; spec/test-cases/TC-466-s6a-evaluates-state-clauses.md; spec/test-cases/TC-467-s6a-clause-entry-refusals-exhaustion-and-determinism.md; spec/test-cases/TC-468-spine-clause-run-reports-typed-dispositions.md; qsl-replay/src/spine/clause/tests.rs; qsl-semantics/tests/it/state_clauses.rs; qsl-semantics/src/model/observation/ordered_json.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-replay/src/spine/clause.rs"
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

Ticket: QSL-278. PR: quire-spec-language#495.

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
| FND-007 | medium | Round 2: the TC-465 row 25 test (`tc465_row25_an_incomplete_population_no_reference_names_still_admits`) asserts `Refused wrong-role-mapping`, where TC-465 says row 25 admits. It runs on the base package, which does not declare `archive`. The `tc465_document_with_archive_population` variant does exist, and I ran row 25 against it: it admits. The test name claims an outcome the test never asserts. | qsl-semantics/tests/it/state_clauses.rs:2885-2907 |
| FND-008 | low | Round 2: the TC-468 step 5 `CallFailure::Fault` half builds `ClauseDisposition::EvaluateFault` by hand and asserts its accessors, so it never runs `run_clause`'s `CallFailure::Fault` arm (clause.rs). That half is close to tautological. | qsl-replay/src/spine/clause/tests.rs:2148-2161 |

## Verdict

Request changes. FR-106-AC-3..7, FR-107-AC-1..3 and FR-109-AC-1, AC-3 and
AC-5 are not delivered as the spec states. Write the missing TC rows over the
ConfigVersion corpus (they will expose SR-750 FND-001 and FND-005). Add
FR-109's provenance and usage to `ClauseRunReport`. Or split the missing
parts onto a follow-up ticket, with an honest FR Status line for each.

## Dispositions

Round 2. I checked the test inventory against the
code at the PR head. The caller's qsl-278-ci-r10.log shows the tests passing
(120 `tc465_row` result lines, exit 0).

| ID | Disposition |
| --- | --- |
| FND-001 | open (mostly fixed). Rows 1-19, 21-29 and 31-42 now have tests tagged `TC-465`. Rows 39-41 follow the coordinator's rulings: `self` resolves by conformance, and rows 40/41 are amended in TC-465. Rows 20 and 30 are deferred to QSL-289 and marked Unverified in TC-465, per the coordinator's ruling. Still open: the row 25 test asserts the wrong outcome (new FND-007). |
| FND-002 | open (partly fixed). Added: `category()`, `truth()`, `provenance` (model selections, the selection, documents) and `usage` (the evaluation meter's admission count and per-kind consumption). Still open against FR-109 Outputs: (a) for an `Invocation`, `provenance.documents` holds only the invocation and never its `pre`/`post` snapshots, which admission read (clause.rs:472, 484). It also lists the selected document after an admission failure that happened before that document was read, such as check 2. (b) `usage.admission_consumed` is always empty (clause.rs:268), so "the admission work" is not reported. (c) There is no I3 extracted-source input and no extraction provenance. |
| FND-003 | deferred: QSL-279. Step 1 `ParentOrder` and `NoCycle` run over admitted snapshots. `VersionUnchanged`, step 2 and step 3 are marked Pending QSL-279 in TC-466. I confirmed the dependency is real. In a throwaway worktree, adding any clause on `Config::ConfigVersion::attemptUpdate` to the fixture unit (`post VersionUnchanged ...`, `post Trivial ... { true }` or `pre TrivialPre ... { true }`) makes `spine::compile` fail with `Emit(UnlocatedOccurrence { role: Generated, ordinal: 0 })`, whatever the clause body. So no pre- or postcondition can reach `evaluate_clause` through the spine until FR-105/QSL-279 emission lands (qsl-package/src/emit.rs:35-42, 142). |
| FND-004 | fixed: TC-464 steps 1-4, with step 3 asserting distinct pre/post, `self`, the result, and empty parameters, created and deleted (state_clauses.rs:1491-1640). |
| FND-005 | open (mostly fixed). TC-467 steps 1 and 3 and TC-468 steps 1-4 are now covered. Still open: TC-468 step 5's comparison, for each FR-100-AC-9 outcome other than `Completed`, of `run_clause`'s `outcome`/exit code against FR-100's own mapping. No test does this. Only the internal-failure pair is covered, and half of that is constructed (new FND-008). |
| FND-006 | fixed: no tag names a nonexistent AC any more, and the frame tests are retagged TC-465. |

### Round 3

| ID | Disposition |
| --- | --- |
| FND-001 | fixed, with rows 20 and 30 deferred to QSL-289 per the coordinator's ruling. The row 25 test now admits over the real `archive` population. |
| FND-002 | open, narrowed to I3. Fixed: after a successful admission, provenance lists the invocation plus its pre and post snapshots (clause.rs:516-522), and `usage.admission_consumed` is a real `AdmissionUsage` of bytes, depth, objects and values. After a failed admission, provenance still lists only the documents the selection named (clause.rs:503), which is minor. Still open: FR-109 does require I3. Inputs (FR-109:49-51) make "an I3 extracted source ... with its original document identity" an alternative to source bytes, and Outputs (FR-109:81-82) require "the extraction's original identity and digest when I3 was used". No AC exercises it, and `spine::compile`/FR-100 have no I3 path, so the disclosure in clause.rs:224-236 is accurate. It needs three things: an optional `qsl-source` dependency behind a `quire-extraction` feature in `qsl-replay` (the root crate already wires it, Cargo.toml:24, src/command/extraction.rs:194); a `ClauseRunRequest` source enum of bytes-with-labels or extracted-with-original-identity that feeds the extracted body to `compile`; and a provenance field for the original identity and digest. This PR owns it unless the coordinator defers it by ruling to a named follow-up ticket (none exists yet; file one) and FR-109's Status states the gap. |
| FND-005 | fixed: each general outcome kind runs through `convert_outcome` and is checked for category, truth and exit code against FR-100's values. FR-100's own exit mapping lives in the root crate (src/command/output.rs), which `qsl-replay` cannot call, so fixed expected values are the right form. |
| FND-007 | fixed. |
| FND-008 | accepted-no-change. The citation holds. `evaluate_clause`'s three `CallFailure::Fault` constructions (qsl-eval/src/value/expression/mod.rs:455-458, 473-475, 484-486) are unreachable through `run_clause`: admission always sets `pre` and `post` for an invocation, refuses a `null` result where `binds_result()`, and admits every declared parameter (check 10). The `CallFailure::Fault` to `EvaluateFault` mapping is inline in `run_clause`, so only the constructed form can reach it without a real clause. Optionally, extract that mapping into a function and feed it a real `CallFailure::Fault` from `evaluate_clause` over hand-built observations (its fields are `pub`). |

### Round 4

| ID | Disposition |
| --- | --- |
| FND-002 | deferred: QSL-295 (filed, blocked by QSL-278), per the coordinator's ruling. The FR-109 Status section records that the I3 extracted-source input and its provenance fields are pending QSL-295. Provenance and admission usage were fixed in round 3. |
