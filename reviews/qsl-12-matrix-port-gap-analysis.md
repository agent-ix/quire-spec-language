---
id: SR-20261110040709820012
title: "QSL-12 matrix port gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-12; PR #2649; qsl-conformance, changed analyze/monitor/replay producers, minimal xtask inventory"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-566
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
---

## Summary

Independent scoped gap-analysis of the frozen lifecycle matrix port. 1 actionable findings; no broad repository qualification is claimed.

## Verdict

FAIL for this candidate until the recorded findings are resolved.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | every_case_gives_its_expected_stable_document carries FR-565-AC-2 while asserting explicit qsl-conformance/lifecycle labels; the current criterion requires quire plus the physical file name. The binding is false until the stale owning criterion is aligned to the already-published explicit-label contract. | qsl-conformance/tests/it/matrix_shape.rs:63,113-116 |

## Coverage

Frozen reviewed source: `f685982174026cb6be873378c63b8d88432206f9`, base `dd627eb4ef539a6bee158f2f8ea3302439e98054`. Ticket QSL-12, PR #2649. All 52 changed paths inspected, including local fixture bytes, manifests, minimal suite inventory/scanner additions and affected callers. No applicable AssuranceProfile found. No source edits, Cargo, full suite, Kani, proof jobs or peer-branch modifications.

Focused execution evidence read from the coder logs: matrix 15, disposition 1, replay outcome 24, marker 1 and explicit clause-null 1 passed; four-crate all-target Clippy and fmt passed. These are supplied focused results, not independent reruns or complete qualification. Root retains the final affected gate and quiet direct metadata guard.

FR-564-AC-4/6 counterexamples, FR-566-AC-1 package-byte equality and all FR-567 qualification work are explicitly excluded/open. Their absence is not hidden by a tag or a passing row. Native CEX ownership remains p3b. No plan was supplied. Plan completion: not assessed.

Reconciliation: scoped manual acceptance-criterion/attribute/assertion inspection. `quoin matrix --repo . --json` was attempted but produced no result and was canceled after more than two minutes to keep this scoped PR review bounded; no computed matrix counts or repository-wide matrix PASS are claimed. No handwritten TestMatrix was created. The source trace bindings below are an audit receipt, not a replacement matrix.

The typed flattened cause representation was inspected against FR-286 and the QSpec unsupported diagnostic branch: unsupported disposition omits cause, while existing clause null and normal terminal causes remain present. Current callers preserve backend absence for dispositions. No changed constructor user was found in the reviewed external checkout search. The supplied registry is deliberately ignored for manually negotiated matrix items, as permitted by FR-566; no inference about unseen Kani/parity integration is made.

## Examined Scope

- `FR-564-AC-1` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Each of `check`'s six applicable cells returns its kind: success exit 0; refusal `ill_typed` exit 20; unsupported `unsupported_construct`/`declaration-form` exit 21; timeout `StageFailure::Cancelled(Deadline)` exit 22; cancellation `StageFailure::Cancelled(Requested)` exit 22; incomplete `LimitExceeded` naming `s3.nodes` exit 22.

- `FR-564-AC-2` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Each of `package`'s five applicable cells returns its kind: success exit 0; unsupported `unsupported_construct` exit 21; timeout `Cancelled(Deadline)`, cancellation `Cancelled(Requested)` and incomplete `LimitExceeded` naming the `package` limit field, each exit 22.

- `FR-564-AC-3` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Each of `execute`'s seven applicable cells returns its kind: `seven` completes 7, exit 0; `id(12)` `CallFailure::Input`, exit 20; `jit` not supplied `CallFailure::Unsupported` cause `NotSupplied`, exit 21; `CallFailure::Cancelled(Deadline)` and `CallFailure::Cancelled(Requested)`, exit 22; `Incomplete` naming `work_units`, exit 22; the fault-injecting backend's `CallFailure::Fault`, exit 30, no value.

- `FR-564-AC-4` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Each of `analyze`'s applicable cells returns its kind: `proved` naming `en-5`, exit 0; `refuted` naming `en-1` with its one `ItemCounterexample`, exit 10; `StageFailure::Refused(AnalyzeRefusal::Binding)` exit 20; `unsupported` record carrying the `unsupported_projection`/`unsupported-requested-capability` diagnostic and no `cause`, exit 21; `incomplete` `cancelled{source: deadline}` and `cancelled{source: requested}` for the EN-4 item, a routed item whose engine would run, exit 22; `incomplete` with `LimitReached{max_states}`, exit 22.

- `FR-564-AC-5` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Each of `monitor`'s seven applicable cells returns its kind: `tested` exit 0; violation exit 10; the admission refusal exit 20; unsupported `missing-fairness-premise` exit 21; `StageFailure::Cancelled(Deadline)` and `StageFailure::Cancelled(Requested)`, exit 22; `Incomplete` naming `work_units`, exit 22.

- `FR-564-AC-6` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Each of the applicable `prove` (QSL half) cells yields one FR-286 item of its kind: through `qsl_analyze::provider::run`, `proved` exit 0, `refuted` exit 10 with its counterexample, `unsupported` with `unsupported_projection`/`unbounded-extent` exit 21, `incomplete` naming `max_states` exit 22; the binding refusal exit 20; `incomplete` `timed-out` and `incomplete` cancelled, exit 22 (FR-291-AC-2's records); `failed` with `tool-failure{crash}`, exit 30.

- `FR-564-AC-7` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Every outcome of AC-1 to AC-5 serializes to an FR-286 document whose `operation` names its verb and whose `category` is its kind's, and every AC-6 item serializes to an FR-286 `items` entry whose `category` is its kind's; serializing each twice gives equal bytes, and each exit code is the one FR-285's function returns for that category.

- `FR-565-AC-1` — role: examined; `spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md`; excerpt: `lifecycle_matrix()` returns one cell per (verb, kind) pair in FR-564's verb and kind order: `Case` cells exactly at FR-564's applicable cells and `NotApplicable` cells exactly at its n/a cells, each `NotApplicable` with a non-empty reason.

- `FR-565-AC-2` — role: examined; `spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md`; excerpt: For every `Case`, `run_case` returns `Ok` with a typed document (or, for `prove`, an item) whose category and exit code equal the case's expected category and exit code, two `run_case` calls give equal `OutcomeDocument` values, and every source identity in the document is `SourceIdentity::new("quire", <the case's file name>)`. A case with an input the test removes returns `Err(CaseError)` whose `stage` is `CaseStage::Read`, the `Copy` stage value, and no `CaseOutcome`.

- `FR-565-AC-3` — role: examined; `spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md`; excerpt: Run with the process's working directory set to an empty temporary directory, every `Case` gives the same `OutcomeDocument` as AC-2.

- `FR-566-AC-1` — role: examined; `spec/functional/FR-566-keep-qsl-outcomes-unchanged-by-a-registered-process-plugin.md`; excerpt: For every `Case` of `lifecycle_matrix()`, `run_case_with` over configurations 1, 2 and 3 returns `Ok` with equal `OutcomeDocument` (or item) values and equal exit codes; FR-275-AC-1's `package` bytes are byte-equal in the three. As a control that configuration 3 is live, FR-281-AC-14's item settled `Supported(qsl.test-plugin)` runs on the test plugin provider in configuration 3 and its record names `qsl.test-plugin` with certification `trusted`.

- `FR-566-AC-2` — role: examined; `spec/functional/FR-566-keep-qsl-outcomes-unchanged-by-a-registered-process-plugin.md`; excerpt: For FR-281-AC-1's item, the candidate set in configuration 3 equals configuration 2's; for an item of a kind the test plugin advertises, configuration 3's candidate set is configuration 2's plus `qsl.test-plugin`, and registering the plugin's descriptor before or after the linked ones gives equal candidate sets.

- `FR-281-AC-11` — role: examined; `spec/functional/FR-281-analyze-claims-with-in-process-engines.md`; excerpt: With test engines that each refuse a `Supported` item, `analyze` settles: an `Admission` refusal `declined` with cause `invalid-input` and its diagnostic, and an incomplete admission `declined` with `incomplete-input`; a `RequiresBound` refusal `unsupported` with `unsupported_projection`/`unbounded-extent` whose payload names the item's kind, the engine's `BackendId` and the modes its manifest advertises for that kind; a `Symmetry` refusal `declined`, `invalid-input`, `invalid_runtime_input`/`invalid-value`; a `NotStatistical` refusal `invalid-request` with `invalid_capability`/`not-statistica

- `FR-286-AC-3` — role: examined; `spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md`; excerpt: An `analyze` outcome of three items serializes with three `items` entries in request order, each holding its terminal record and category. Serializing any outcome twice gives equal bytes.

- `FR-286-AC-10` — role: examined; `spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md`; excerpt: Over FR-283-AC-1's clause run `always p` violated at position 0, `from_clause_run` gives an item with `result` `refuted`, category violation, `clause`, `position` 0, a `counterexample` with a `prefix` of three identities, an empty `loop` (`[]`, the finite spelling of FR-330 and QSpec FR-364), `trace_position` 0, a `decisive` beside the counterexample with `position` 2, no `quantifier` and no `index`, `element` `false` and the empty `value_path`, `cause` null and a `witness` member exactly when the run's `ClauseRunReport::witness` holds a record (FR-266 gives this temporal run the basis `closed

- `FR-286-AC-11` — role: examined; `spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md`; excerpt: Over FR-283-AC-5's clause run, `from_clause_run` gives an item with `result` `refuted`, category violation, `position` 1, a `counterexample` with `trace_position` 1, cause `undefined-evaluation{where, cause}` with `cause` `division-by-zero`, no `witness` and no `undefined` label.

- `FR-286-AC-12` — role: examined; `spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md`; excerpt: Over FR-283-AC-11's clause run, `from_clause_run` gives an item with `result` `undefined`, category undefined, cause `insufficient-data`, `position` 0, `count` 1 and `min_count` 2, and neither `counterexample` nor `witness`; its document exits 10.

- `FR-287` — role: context_only; `spec/functional/FR-287-reach-qsl-through-the-driver-cli.md`; excerpt: The user binary is `quire`, built from the driver repository (ADR-029 CB-1).
QSL builds no user binary. Its only executable targets are the developer
tools, the packages that run against source trees and measurements rather
than serve a user and that no QSL library's build depends on: `xtask`, the
packages whose manifest directory is under `tools/`, `qsl-string-edge` (the
standalone string-edge scanner FR-064 requires as a binary) and `qsl-bench`
(whose `qsl-bench-probe` binary MP-006's probe and the Makefile's
`BENCH_PROBE` run). The request
handling FR-027 and FR-100 specify is a QSL library

- `FR-567` — role: context_only; `spec/functional/FR-567-qualify-qsl-on-the-final-path-the-driver-calls.md`; excerpt: The final path is the path a user's run takes: a `quire` verb in the driver
calls a QSL library operation (FR-287), which runs on the interpreter or on
a non-interpreter backend the build links (FR-296). The `qualify` gate's
verdict SHALL cover that path, not only the entry points the corpus runner
calls: the gate (FR-353) runs three parts in every run and reports
`qualified` only when all three pass.

- `FR-565` — role: examined; `spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md`; excerpt: - A `MatrixCase` holds its verb and kind, every input byte the case needs
  with the identity it is read under (sources, domain packages, libraries,
  traces, snapshots), the operation's request parameters and limits, the
  cancellation plan (none, or cancelled before the call with `Requested` or
  `Deadline`), and the expected category and exit code. Its bytes are the
  crate's own data, and `run_case` reads no file.


## Dispositions

Round 1 reviewed exact source `5f2f92bfa77e24802615b8b9cd2ef3827949965b`. Initial Findings, Summary and FAIL verdict above are preserved as historical review evidence.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-565 source-identity Description, Behavior and AC-2 now name explicit authored authority/identity separately from physical file names. TC-1394 procedure and expected result agree with the existing FR-287 contract. |

## Latest Verdict

PASS for the scoped gap-analysis after disposition round 1. All original findings are fixed; no new findings. Final affected aggregate gate remains with the lead. This is not QSL-12 closure or whole-repository qualification.

## Disposition Evidence

Only four source/spec paths changed after the initial reviewed head: run.rs, execute_row.rs, FR-565 and TC-1394. Both execution paths share the unmodified full ScalarLimits value. The non-work regression uses actual arithmetic so it reaches IntegerBits charging; its supplied RED/GREEN logs show the old implementation failing exit 0 versus expected 22, then the fixed matrix target passing all 16 tests. The initial literal-only failed attempt is explicitly excluded. Supplied final fmt and four-crate test-inclusive Clippy pass. Replay/analyze/monitor producers are unchanged from the original reviewed source and their focused evidence remains applicable. No Cargo, proof, full suite or second matrix run was performed by the reviewer.

The FR-565-AC-2 binding on matrix_shape::every_case_gives_its_expected_stable_document is now correct against the amended criterion; the separate removed-input control covers the read failure arm. No new coverage is claimed for the intentionally open native counterexample, package-byte or qualification criteria.
