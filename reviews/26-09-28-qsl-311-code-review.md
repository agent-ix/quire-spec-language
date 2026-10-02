---
id: SR-770
title: "QSL-311 code review (with rust-review lane) of PR 511"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6307caf29145a05eedf251a428f6e40b0a8b069d; qsl-replay/src/spine/clause/tests.rs (new: config_version_domain_document_with_operations, config_version_probe_operation, config_version_step3_domain_document, config_version_step3_unit_and_packages, config_version_step3_model_digest_hex, config_version_step3_request, config_version_step3_snapshot, config_version_step3_object, config_version_step3_chain_objects, config_version_step3_loop_objects, config_version_probe_invocation_bytes, run_config_version_probe, evaluate_step3_case_a_with_meter, assert_step3_denial_is_incomplete, ten tc466_step3_* tests; read unchanged: no_cycle_completes_true_with_the_expected_reaches_charge_log, boolean_outcome, boolean_disposition, config_version_request_for); qsl-replay/src/spine/clause.rs (run_clause admission/evaluation, unchanged); quire-exact/src/accounting.rs (Meter, InjectedDenial, Incomplete, unchanged); qsl-semantics/tests/it/state_clauses.rs (existing probe_operation fixture, unchanged)"
review_set: subset
---
## Summary

Ticket: QSL-311. PR: quire-spec-language#511 at 6307caf2, base f8b89a9b.
Methods: code-review with the rust-review lane folded in. The diff is
test-only: +567/-33 in `qsl-replay/src/spine/clause/tests.rs`. The 33
deletions move `attemptUpdate`'s operation JSON out of the envelope into a
`Vec` so that `config_version_domain_document_with_operations` can append
`probe`. Called with `Vec::new()`, it yields the same document as before.

What I checked, and what I found:

1. **The `probe` fixture is minimal and matches the ticket.** It has one
   parameter `target: ConfigVersion`, no `returns` member, and an empty
   frame. It is appended through the one domain-document builder instead of
   a second envelope. The same operation already exists in
   `qsl-semantics/tests/it/state_clauses.rs:69-77` (TC-459 and TC-465), but
   that is an integration-test tree that `qsl-replay` cannot import. The
   qsl-replay copy follows this crate's own config-version fixture pattern
   (QSL-279/310/313) and means the same thing (no result, empty frame).
   Both pass real FR-106 intake.
2. **Chain and loop really exercise `reaches`.** (a) `a`->`c` needs two
   hops, and the 5-charge log proves both. (b) `a`->`a` over the chain is
   `false`, so `self == target` is not short-circuited to true. (c) `c` has
   no parent and reaches nothing. (d) The self-loop terminates `true`; an
   infinite loop would hang the test instead of passing. Each of (a)-(d)
   goes end to end through `run_clause` on an FR-106-admitted invocation,
   and `boolean_disposition` panics on anything other than
   `Evaluate(Completed(Boolean(_)))`.
3. **The denial meter already existed; the PR reuses it.** The ticket says
   no denial infrastructure existed. In fact `quire_exact::InjectedDenial`
   and `Meter::with_injected_denial` (`accounting.rs:450-458`, `562-568`,
   `604-621`) already existed, and they deny exactly the Nth admission at
   one `ChargePoint`. The PR adds no production code. I checked with a
   temporary probe test, reverted afterwards. For denial N (1..5),
   `Incomplete.charge_point` equals the denied point,
   `consumed == limit == N-1`, and exactly the first N-1 walk charges are
   admitted. A sixth denial (`GraphExpand` #3) never fires and gives
   `Completed(true)`. So every sub-case denies a different charge, and none
   passes vacuously: a denial that missed would give `Completed(true)`. But
   the tests assert none of those distinguishing fields (FND-001).
4. **The existing NoCycle charge-log test is unchanged.** The test at
   tests.rs:368-421 is not in the diff and passes in both feature
   configurations of my gate run. The new step-3(a) charge-log test builds
   its own `Meter` and documents, and shares no mutable state.
5. **Trace tags are correct.** Every step-3 test carries
   `#[trace("TC-466", "FR-107-AC-3")]`. TC-466 says to tag `FR-107-AC-n`,
   and FR-107-AC-3 (FR-107 line 117) is the row covering all of step 3,
   (a) through (e). The (a)-(e) letters in doc comments and test names match
   TC-466 lines 37-42 exactly.

Rust-review lane: test-only. The `expect`/`unwrap`/`panic!` calls are in
test code, there are no casts or `unsafe`, and no production code changed.

Gate, run fresh by me (not taken from the PR body): at 6307caf2, `make ci`
exited 0 with the worktree's own `CARGO_TARGET_DIR`. The log has 93
`test result: ok` lines and no FAILED or panic lines. All 10 new
`tc466_step3_*` tests and the NoCycle charge-log test ran and passed in both
the default-features and all-features passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The five step-3(e) tests share one assertion, `Incomplete(_)`, so nothing in them shows that the denial landed at the named charge or that the walk stopped there. The record carries what is needed: pin `incomplete.charge_point == point` and `limit_kind == WorkUnits`, `consumed == N-1`, and the admitted graph-charge prefix equal to the first N-1 entries of `[expand, edge, expand, edge, result-retain]`. These are the values I measured. A 6th control denying `GraphExpand` #3 should assert `Completed(true)`, which proves the walk makes exactly five charges. | qsl-replay/src/spine/clause/tests.rs:3257-3315 |
| FND-002 | low | `evaluate_step3_case_a_with_meter` repeats `run_config_version_probe`'s three-document build (pre, post, invocation) line for line. It also hand-mirrors `run_clause`'s `ClauseFacts` + `admit_observations` block (clause.rs:583-611), which can drift. A shared `probe_documents(objects, self, target)` helper removes the first copy. | qsl-replay/src/spine/clause/tests.rs:3006-3043; qsl-replay/src/spine/clause/tests.rs:3177-3252 |
| FND-003 | low | `config_version_step3_model_digest_hex` builds the whole unit text, then throws it away (`let _ = unit;`). It should call `package_input` on `config_version_step3_domain_document()` directly. | qsl-replay/src/spine/clause/tests.rs:2877-2884 |

## Verdict

Approve with findings. There are no high findings in the diff, and the gate
is green. Steps (a)-(d) discriminate, the charge log is exact, and the
denial seam works. FND-001 (medium) should be fixed in this PR so that the
five (e) cases each assert something different. FND-002 and FND-003 are
cleanups for the same round. The silent-acceptance defect found outside the
diff is recorded in SR-771 FND-001.
