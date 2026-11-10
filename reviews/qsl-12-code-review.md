---
id: SR-(commit)
title: "Code and Rust review of l5/p3-matrix (Phase 3 PR 1: qsl-conformance lifecycle matrix)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language(commit); diff origin/l5/integration((commit))...HEAD: qsl-conformance/**, qsl-analyze/src/{analyze,monitor}.rs, qsl-analyze/tests/it/analyze.rs, qsl-replay/src/outcome.rs, docs/outcome-parity.md, Cargo.toml"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-566
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
---

## Summary

Ticket: QSL-12. Branch `l5/p3-matrix` at `(commit)`, no PR yet, reviewed against spec on `origin/main` `(commit)` (includes #2585, #2586, #2587). Code review with the Rust lane (rust-style loaded). No cargo builds were run (shared build lane busy); findings are from reading the diff and the code it calls.

The PR is a stated partial: 26 of FR-564's 40 applicable cells are built. The execute row (TC-1390), the analyze success/violation/incomplete cells and the prove success/violation/unsupported/incomplete cells are not built, and FR-567-AC-5 is out of scope. The lead accepted these gaps; this review checks only that the code and docs describe them honestly.

## Verdict

**PASS WITH FINDINGS** (mergeable into `l5/integration` as a stated partial once FND-001 is fixed; the rest are medium or low).

The built cells call the real library operations: `check`/`package` through the spine, `analyze` refusal, timeout and cancellation through `analyze()`, `monitor` through `monitor()`, and prove refusal through `provider::run`. Each test asserts the cause that distinguishes its kind (catalog code, cancel source, limit name), not only the exit code, so the tests can fail. The incomplete cells are exactly one below the counter: running the same case at counter+1 succeeds (`front_end_rows.rs:142-169`).

Building the prove timeout, cancellation and tool-failure cells directly as FR-291 records matches the spec as written. FR-564 defines those cells as "FR-291-AC-2's incomplete TimedOut record" and "FR-291-AC-1's failed tool-failure{crash} record". The plugin run belongs to the driver, and QSL has no settlement-to-record function beyond the record constructors. These cells add little signal (they test serialization, category and exit code), but they do not fabricate a verdict.

Running the analyze timeout and cancellation cells over an EN-4 service claim instead of FR-281-AC-1's EN-5 item is an acceptable deviation. Both causes come from a pre-cancelled handle, and `analyze` checks the handle before any item runs, whatever the engine. The deviation should be stated where the cells are documented.

The `DeadlineSupply` change matches FR-281-AC-12. `AnalyzeLimits` has no `Default`, there is a compile_fail doctest plus a compiling control, `analyze` reads no clock, and no caller in the repo used `deadline_ms`. The deleted `wait_for_deadline` unit tests guarded the timer thread that FR-276/FR-281 remove, so they are obsolete. The replacement test would fail on the old code: with `Milliseconds(1)` the old guard cancelled the items. It also checks that a pre-cancelled `Deadline` handle settles every item `cancelled{source: deadline}`. Consequence outside this repo: the driver must now arm the timer itself.

No ceremony was found: no inventory, digest or count checks. No manifest is vendored: the case fixtures are synthetic and byte-distinct from every other tracked file, and the shared FR-275-AC-1 source is pulled in with `include_bytes!` of `tests/fixtures`. The library code has no panic paths and no unbounded work. The only widened API is `terminal_records`, `analyze_document` and the `document()` methods, which the conformance crate needs.

## Scope

- FR-564-AC-1, AC-2, AC-5: examined, row tests complete and failable.
- FR-564-AC-4, AC-6: examined, partial (see FND-001).
- FR-564-AC-7: examined; exercised by `every_case_gives_its_expected_stable_document`.
- FR-565-AC-1: examined, partial (see FND-001). FR-565-AC-2: examined (FND-004). FR-565-AC-3: examined, clean.
- FR-566-AC-1: examined (FND-003). FR-566-AC-2: examined, clean.
- FR-281-AC-12: examined, clean. FR-281-AC-3 / AC-10: examined (FND-002).
- FR-291-AC-1, AC-2: context_only, for the prove plugin cells.
- `qsl-replay/src/outcome.rs` `OutcomeDocument::from_stopped`: examined, clean.
- `qsl-analyze/src/monitor.rs` `MonitorRefusal::document`: examined, clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Partial coverage is presented as complete. The trace tags claim FR-565-AC-1 (48 cells, Verb includes Execute), FR-564-AC-4 (7 analyze cells) and FR-564-AC-6 (8 prove cells), but the tagged tests cover 33 cells, no Execute verb, 4 of 7 and 4 of 8. The computed matrix will report these ACs as covered. lib.rs and docs/outcome-parity.md also say lifecycle_matrix returns one cell per (verb, kind) pair, which is false. | qsl-conformance/tests/it/matrix_shape.rs:186-217, qsl-conformance/tests/it/analyze_row.rs:10, qsl-conformance/tests/it/prove_row.rs:623, qsl-conformance/src/lib.rs:4-6, docs/outcome-parity.md:9-11 |
| FND-002 | medium | The analyze unsupported cell does not run the analyze verb. It hand-assembles analyze_items, terminal_records and analyze_document. The resulting FR-286 document has cause backend-absent and no unsupported_projection/unsupported-requested-capability diagnostic naming the kind: terminal_records keeps only the CatalogCode and drops Payload::Kind, and OutcomeItem has no diagnostic member. FR-281-AC-3/AC-10 call for that diagnostic and no cause. The row test asserts only result=unsupported, so it would not catch this. The defect predates this PR, but the PR publishes the result as the driver's parity oracle. | qsl-conformance/src/run.rs:373-381, qsl-analyze/src/analyze.rs:1051-1080, qsl-conformance/tests/it/analyze_row.rs:24-26 |
| FND-003 | low | FR-566-AC-1's clause "FR-275-AC-1's package bytes are byte-equal in the three" is not tested: the package document carries no bytes and the test compares only typed documents. run_case_with also discards the registry (`let _ = registry`), so the config 1 vs 2 comparison cannot fail. The spec permits this, but it should be stated in the test. | qsl-conformance/tests/it/plugin_invariance.rs:476-494, qsl-conformance/src/run.rs:114-116 |
| FND-004 | low | The FR-565-AC-2 source-identity check only asserts that the identity ends with `.native`. It never compares it to the case's file name, and it passes vacuously on documents with no source object. | qsl-conformance/tests/it/matrix_shape.rs:260-283 |
| FND-005 | low | The API drifts from the FR-565 signature. run_case/run_case_with return Result<CaseOutcome, CaseError>, while the spec gives `-> CaseOutcome`, and CaseOutcome omits the typed library outcome FR-565 lists. CaseError carries a `stage: &'static str` with no Copy code enum, and the monitor document error is mislabelled Prerequisite{stage:"monitor"} although it comes from the operation under test. The Result is the better design: amend the spec, not the code. | qsl-conformance/src/run.rs:47-50, qsl-conformance/src/run.rs:79-88, qsl-conformance/src/run.rs:98, qsl-conformance/src/run.rs:425 |

## New findings (disposition pass 1)

Re-reviewed at `(commit)` (`git diff (commit)..(commit)`, rounds 1-3, merged `(commit)` with spec #2587/#2588). Checked against the amended FR-281/FR-286/FR-564/FR-565 and QSpec FR-331 (`origin/main` `dfcc85bc`, lines 621-623, 677-685, 865-867).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The new FR-286 item diagnostic is spelled `{code, cause, payload}`. QSpec FR-331 fixes it as `{code, cause, detail}`, with `detail` keyed by FR-272 payload names, and QSL's own FR-331 reader reads `detail`. The parity oracle therefore publishes a wire member no FR-331 reader accepts. | qsl-replay/src/outcome.rs:625-630, qsl-replay/src/planner/entry.rs:833-856 |
| FND-007 | medium | Only a `Payload::Kind` payload reaches the item. RequiresBound's `Unbounded{kind, candidate, modes}`, which the FR-281 table requires, and invalid-request's `Inconsistent`, `Backend` and `Inapplicable` payloads are dropped: `diagnostic_kind` is `None` and the item writes no payload. Amended FR-286 says the diagnostic carries code, cause and payload. No test covers a RequiresBound or invalid-request item. | qsl-analyze/src/analyze.rs:1056-1063, qsl-replay/src/outcome.rs:1043-1056 |
| FND-008 | medium | A declined invalid-request disposition item now writes no `cause`. QSpec FR-331's results table gives `declined` both `cause` and `diagnostic`, and QSL's planner reader requires both. "Exactly one of cause and diagnostic" holds only for `unsupported`. The guard also keys on `DeclineCode::Qsl(Code::InvalidRequest)` plus a diagnostic being present, not on a typed disposition marker, so any later non-disposition declined record with that code would lose its cause too. | qsl-replay/src/outcome.rs:907-911, qsl-replay/src/planner/entry.rs:314-317 |
| FND-009 | medium | `CaseError`/`CaseStage` do not match amended FR-565. The spec has `CaseError{stage}` with stages Read, Check, Package, Execute, Analyze, Monitor, Prove, Document; the code has an enum with Prerequisite/Operation and Engines, Parse, Packages, ... stages. FR-565-AC-2's new clause, that a case with an input removed returns `Err` with `CaseStage::Read`, cannot be met and has no test. | qsl-conformance/src/run.rs:78-121 |
| FND-010 | low | `UnavailabilityCause::Disposition.as_str()` returns `"disposition"`, a spelling outside FR-331's closed `unsupported` causes. Nothing writes it today, but `as_str` is public and the variant shares the cause classification. | qsl-replay/src/proof_result.rs:742 |
| FND-011 | low | Spec text, not code: amended FR-564's analyze timeout and cancellation cells name `Supported(qsl.statistical)`, but the EN-4 manifest's BackendId is `en-4`. The code is right and the FR-564 table should say `en-4`. | qsl-conformance/src/run.rs:329, qsl-analyze/src/statistical/manifest.rs:25 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The tags on FR-565-AC-1, FR-564-AC-4 and FR-564-AC-6 were reduced to the TC id, and lib.rs and docs/outcome-parity.md now name the pending cells and the missing execute verb. #2588 then reduced AC-4/AC-6 to the built cells, so their tags should come back (gap FND-003). |
| FND-002 | fixed | `terminal_records` no longer writes BackendAbsent. The item carries the unsupported-requested-capability diagnostic naming the kind and no cause, and TC-1391 asserts it. Bypassing `analyze()` is stated in docs/outcome-parity.md: `analyze` takes only EN-4 items. The spelling and payload of the new member are FND-006 and FND-007. |
| FND-003 | accepted-no-change | The test doc now says the registry is ignored by design (FR-566 permits it) and that the package operation takes neither registry nor engines. The literal package-bytes clause stays uncompared, which is a stated gap. |
| FND-004 | fixed | Identities are compared with the case's file names, and check refusal/unsupported must carry at least one located source. |
| FND-005 | fixed | Amended FR-565 adopts `Result<CaseOutcome, CaseError>`. CaseError is Copy with a typed CaseStage, and the monitor document error is `Operation{MonitorDocument}`. The remaining mismatch with the amended variants is FND-009. |
| FND-006 | fixed | Round 2, branch `l5/p3-matrix` @ `(commit)`, scope `origin/l5/p2-render...HEAD`. `ItemDiagnostic` serializes `{code, cause, detail}` (qsl-replay/src/outcome.rs). |
| FND-007 | fixed | Round 2. `Payload::detail` writes every payload (`kind`, `backend`, `candidates`, `kind`/`candidate`/`modes`, `request_index`/`capability`/`family`), and a unit test covers RequiresBound, inapplicable, inconsistent and unknown-backend items (qsl-analyze/src/analyze.rs). |
| FND-008 | fixed | Round 2. `TerminalRecord::disposition_invalid_request` builds a `declined` record that writes both `cause` (`invalid-input`) and `diagnostic`. The `diagnostic` member is keyed on `disposition_detail`, which only the two disposition constructors set. |
| FND-009 | fixed | Round 2. `CaseError { stage: CaseStage }` is `Copy`, its stages are Read, Check, Package, Execute, Analyze, Monitor, Prove and Document, and `a_case_with_a_removed_input_fails_at_the_read_stage` tests the Read clause. |
| FND-010 | still-open | Round 2. `UnavailabilityCause::Disposition` is still a sentinel variant whose `as_str` returns `"disposition"`. `TerminalRecord::with_backend(.., Unsupported(Disposition))` still builds an `unsupported` record that has neither `cause` nor `diagnostic`. |
| FND-011 | fixed | Round 2. The spec now names `Supported(en-4)`. The stale `qsl.statistical` sentence in docs/outcome-parity.md is FND-018. |

## New findings (disposition pass 2)

Reviewed at `(commit)`, scoped to `git diff origin/l5/p2-render...origin/l5/p3-matrix`, against FR-564, FR-565, FR-567, FR-276, FR-281-AC-12 and FR-286 as they stand on this branch.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | `EngineItem` is `pub` in the private `matrix` module but lib.rs does not re-export it. It is the type of the public fields `AnalyzeInput::Engine::item` and `ProveInput::Provider::item`, so the driver cannot name it or match on its variants (claim name, `max_states`). That leaves it unable to build the CLI request for the 11 engine-backed analyze and prove cases FR-565 has it run. | qsl-conformance/src/lib.rs:16-21, qsl-conformance/src/matrix.rs:259-277 |
| FND-013 | medium | `MatrixCase` does not hold the operation's request parameters, as FR-565 requires ("the operation's request parameters and limits"). `run.rs` hard-codes the `AMPLE` accounting, the EN-4 item's method, seed, sampler, limits and `max_witnesses`, EN-5's `ExactProbLimits`, the clause `ClauseLimits`, the monitor limits and the package name `"conformance"`. The driver would have to copy them to reproduce a case, yet docs/outcome-parity.md says the case holds its limits. | qsl-conformance/src/run.rs:243, qsl-conformance/src/run.rs:321, qsl-conformance/src/run.rs:336-348, qsl-conformance/src/run.rs:424-442, qsl-conformance/src/run.rs:553-566 |
| FND-014 | medium | The analyze unsupported cell (`AnalyzeInput::Unadapted`) carries no input bytes. It runs `analyze_items` over a hand-built `AnalyzeItem::unadapted(Monitorability, Protocol)` and never calls `analyze` over a source, package and snapshot. FR-564 says "The cells' inputs are bytes, run through the real operation", and FR-565 says each case holds every input byte, so the driver's CLI half has nothing to write or run for this cell. | qsl-conformance/src/run.rs:518-527, qsl-conformance/src/cases.rs:523-530 |
| FND-015 | medium | The execute row runs `run_with_backends` (the FR-100 run). A pre-cancelled handle stops at parse (`RunRefusal::Cancelled`), `jit` refuses before any stage (`RunRefusal::Unsupported`), and `id(12)` refuses at `bind_arguments`. None of them produces the `CallFailure::Cancelled`, `CallFailure::Input` or `CallFailure::Unsupported(NotSupplied)` that FR-564-AC-3 names. `CaseOutcome` holds no typed library outcome, though FR-565 says it does, so `execute_row` can assert only the category and the diagnostic, and AC-3's typed causes go untested. | qsl-conformance/src/run.rs:287-334, qsl-replay/src/spine/call.rs:539-560, qsl-conformance/src/run.rs:59-64, qsl-conformance/tests/it/execute_row.rs:11-53 |
| FND-016 | medium | FR-567-AC-5: `qsl_version()` runs `cargo metadata --offline` on `CARGO_MANIFEST_DIR/../Cargo.toml` at run time, instead of using `env!("CARGO_PKG_VERSION")` of the quire-spec-language crate from the build. The report names whatever the checkout says when it runs, needs cargo and the source tree at run time, and ignores `output.status`, so stderr is lost. The AC-5 test recomputes the expected value with the same cargo-metadata call, which makes it tautological, and the 9.9.9 scratch-copy half has no test. Removing the build.rs is fine, since it shelled out to cargo metadata too and panicked at build time. The replacement should be the root crate's own `env!` constant. | xtask/src/qualification.rs:23-49, xtask/tests/qualify_parts.rs:62-95 |
| FND-017 | medium | `parity_part()` always returns `no-backend`. It does not compute this from the backends the workspace links, and `QualifyInputs` has no backend replacement, so FR-567-AC-1 (a linked delegating backend), AC-3 (`parity/jit`) and the reference-fault half of AC-4 have no code path. The FR-563 runner is a declared cross-lane gap, but the code does not say so. Its comment only says "The workspace links none", and once a backend links, the gate would keep reporting a passing `no-backend`. | xtask/src/qualify.rs:74-88, xtask/src/qualify.rs:36-46 |
| FND-018 | medium | docs/outcome-parity.md is stale against the code. It still describes `PendingEngine` cells, says "the execute verb is not held yet … not yet FR-565's 48 cells", says the pending cells wait on QSL-1819/QSL-1820, says "`analyze` itself takes only EN-4 items" (it now takes Claim, Clause and Exact work), says the case holds its request limits (FND-013), and says FR-564 names `qsl.statistical`. This is the driver team's guide to the crate. | docs/outcome-parity.md:9-24, docs/outcome-parity.md:57-59, docs/outcome-parity.md:71-73 |
| FND-019 | low | `PartReport.result` is a `&'static str` and `passed()` is `result != "failed"`, so any other spelling passes. It should be an enum. The `parts` doc says "in the order they ran", but `corpus` runs last and is inserted at index 0. The lifecycle-matrix part runs before `parse_args`, so a preflight refusal (exit 2) runs all 40 cases first. | xtask/src/qualify.rs:17-33, xtask/src/qualification.rs:400, xtask/src/qualification.rs:429-443, xtask/src/qualification.rs:630 |
| FND-020 | low | The FR-565-AC-3 test calls `std::env::set_current_dir` inside the shared, parallel `it` test binary. Concurrent tests run with the temporary cwd, and a panic between the two calls never restores it. | qsl-conformance/tests/it/matrix_shape.rs:194-209 |

## New dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-010 | fixed | `UnavailabilityCause::Disposition.as_str()` now returns the closed FR-272 spelling `unsupported-requested-capability` and is never written as a cause. `TerminalRecord::unsupported(.., Disposition)` and `with_backend` give the record the generic capability diagnostic, so no `unsupported` record has neither `cause` nor `diagnostic`; a unit test covers it. |
| FND-011 | fixed | The spec names `en-4`; docs/outcome-parity.md no longer says `qsl.statistical`. |
| FND-012 | fixed | `EngineItem` is re-exported from lib.rs, as is `RequestParameters`. |
| FND-013 | fixed | `MatrixCase` holds `RequestParameters`: package name, accounting, `max_window_values`, the EN-4 seed, witness bound and sampler, and the EN-1/EN-5 `max_states`. run.rs reads them from the case. EN-4's method and statistical limits and the EN-5 and clause limits other than `max_states` stay at their published defaults, which docs/outcome-parity.md states. |
| FND-014 | fixed | The analyze unsupported cell is an EN-1 clause item over the `test/counter-step` bytes negotiated to EN-4, run through `analyze`. `AnalyzeInput::Unadapted` is gone. |
| FND-015 | fixed | `CaseOutcome::execution()` holds the typed `qsl_eval::execution::execute` result over the checked package, and the execute row asserts `CallFailure::Input`, `Unsupported(NotSupplied)`, `Cancelled(Deadline)`, `Cancelled(Requested)`, the `WorkUnits` incomplete evaluation and `Fault`. The FR-286 document still comes from `run_with_backends`, the CLI's path, so its cancel is at parse. |
| FND-016 | fixed | The report names `quire_spec_language::VERSION`, a const of the root crate's `CARGO_PKG_VERSION` that xtask depends on. No cargo metadata at run time and no build.rs. The test asserts the report equals that const, differs from xtask's own version and matches the root manifest. The 9.9.9 scratch-copy half is not run (it needs a rebuild of the copy). |
| FND-017 | fixed | `parity_part` documents the gap: FR-563's runner arrives from the jit lane's branch and moves into `qsl-conformance` after this lands, the part reports `no-backend` only while no backend is linked, and AC-1's linked backend, AC-3 and AC-4's reference fault have no code path. |
| FND-018 | fixed | docs/outcome-parity.md describes 48 cells, the execute verb, the engine items, the request parameters and the known gaps. |
| FND-019 | fixed | `PartReport.result` is the `PartResult` enum, `passed()` is `result != Failed`, the parts list in report order with `corpus` first, and the matrix runs after every preflight refusal. |
| FND-020 | fixed | The FR-565-AC-3 test runs the cases in a child process of the test binary whose working directory is an empty directory and compares the serialized outcomes; no test changes its own working directory. |

## Reviewer dispositions (round 3)

The reviewer verified these against the fix commit on the branch. The "New dispositions (round 3)" section above was written by the coder. The rows below are the reviewer's, and where the two differ, these rows hold.

| FND | outcome | reason |
| --- | --- | --- |
| FND-010 | fixed | `as_str` now returns the closed spelling `unsupported-requested-capability`, and `with_backend` attaches the generic capability diagnostic to a `Disposition`-marked record. A unit test asserts the diagnostic is present and the cause is absent. The sentinel variant remains, but the two concrete defects are gone. |
| FND-012 | fixed | `EngineItem` and `RequestParameters` are re-exported from lib.rs. |
| FND-013 | fixed | `MatrixCase.parameters: RequestParameters` carries the package name, accounting, `max_window_values`, the EN-4 seed, sampler and witness bound, and `max_states`, and run.rs reads them from the case. The method that is still hard-coded is FND-023. |
| FND-014 | fixed | The cell is now byte-driven through `analyze`: the `test/counter-step` clause is negotiated to `en-4`, and FR-281 ("an authored clause settled `Supported` on any other backend settles `unsupported`") gives the kind. The input differs from the one FR-564's table names (FND-022). |
| FND-015 | fixed | `CaseOutcome::execution()` holds the typed `execute` result, and execute_row asserts `CallFailure::Input`, `Unsupported(NotSupplied)`, `Cancelled(Deadline/Requested)`, `WorkUnits` and `Fault`. The typed result and the document come from two separate calls (FND-021). |
| FND-016 | fixed | `quire_spec_language::VERSION = env!("CARGO_PKG_VERSION")` reaches xtask through a path dependency, with no run-time cargo and no build.rs, which matches FR-567-AC-5. How strong the test is, is gap-analysis FND-009. |
| FND-017 | deferred | FR-563's runner arrives from the jit lane. The gap is now stated in `parity_part`'s doc and in docs/outcome-parity.md "Known gaps", and the FR-567-AC-1/AC-4 tags are dropped. Deriving `no-backend` from the linked set waits for that runner. |
| FND-018 | fixed | docs/outcome-parity.md describes 48 cells, the execute verb, `RequestParameters`, the engine items and a "Known gaps" section. |
| FND-019 | fixed | `PartResult` enum, `corpus` first in report order, and the matrix part runs after preflight inside `execute_with_parts`. |
| FND-020 | fixed | The FR-565-AC-3 check runs in a child test process whose working directory is an empty temporary directory. The parent's working directory is never changed. |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-021 | low | The execute row's typed result and its document come from two separate calls. The typed result is from `execute` over a package checked through `Prepared`. The document is from `run_with_backends` over the bytes. So `CaseOutcome` pairs a typed outcome with a document of a different call; for a pre-cancelled handle, the typed result is `CallFailure::Cancelled` while the document is a parse-stage cancel. The two are cross-checked only through each cell's asserted category. `ExecuteResult` also implements `PartialEq` by comparing `format!("{:?}")` strings, and `Debug` is not an equality contract. | qsl-conformance/src/run.rs:340-418, qsl-conformance/src/run.rs:68-88 |
| FND-022 | low | The analyze unsupported cell's input is an authored clause negotiated to `en-4`, not the input FR-564's table names ("FR-281-AC-3's unsupported item alone": a kind no layer-A engine advertises). The outcome is spec-backed by FR-281's authored-clause rule. Either the FR-564 table or the cell should change so the two agree. | qsl-conformance/src/cases.rs:276-278, qsl-conformance/src/cases.rs:564-575 |
| FND-023 | low | `StatisticalMethod::Okamoto` and `StatisticalLimits::default()` are still hard-coded in `Subject::work` and are not in `RequestParameters`. The coder's disposition says docs/outcome-parity.md states these defaults, but it does not mention the method or the statistical limits, so the driver has to read run.rs to reproduce the EN-4 cells. | qsl-conformance/src/run.rs:500-510, docs/outcome-parity.md:16-23 |

## Coder dispositions (round 4)

| FND | outcome | reason |
| --- | --- | --- |
| FND-021 | fixed in part | `ExecuteResult` and `CaseOutcome` no longer implement equality: `Evaluation` and `Value` have none, and no caller compared them, so the `Debug`-string comparison is gone. The two calls stay separate: the document comes from `run_with_backends`, whose `convert_*` functions are crate-private to qsl-replay, and `quire_eval::Evaluation` has no `Clone`. docs/outcome-parity.md states this. |
| FND-022 | documented | docs/outcome-parity.md states that the cell is an EN-1 clause negotiated to `en-4`, spec-backed by FR-281, and why: `NegotiatedItem` and the `negotiate_*` functions that build FR-281-AC-3's item are not in the code this branch builds on. |
| FND-023 | documented | docs/outcome-parity.md states the EN-4 Okamoto method and `StatisticalLimits::default()`, and that other EN-1, EN-5 and clause limits are the published defaults. |
