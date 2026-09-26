---
id: SR-674
title: "Code and Rust review of spine run (FR-100)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c23b3128c6a16c60b2d30021ef87e9c8490b9109; qsl-replay/src/spine/call.rs; qsl-replay/src/spine.rs; qsl-foundation/src/wire_format.rs; src/command.rs; src/command/output.rs; src/command/output/types.rs; src/command/wire.rs; tests/fixtures/spine-run.native; tests/it/spine_run.rs; tests/it/compile_command.rs; tests/it/main.rs; tools/arch-lint/api_surface.rs; tools/arch-lint/main.rs; xtask/src/seam_probe.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
## Summary

Ticket: QSL-271. PR: quire-spec-language#468 at c23b3128. Code review with the
rust-review lane, scoped to `git diff origin/main...HEAD` (17 files).

The spine `run` entry, edition routing, the `spine-run-result/1` document and
the stage-`call` envelopes are sound in their main path. Checked by reading the
code and by running the built CLI on probe requests. `request_digest` is the
SHA-256 of the request bytes. `completed false` exits 0. `missing_declaration`
exits 20, `unsupported_construct` 21 and `incomplete` 22. The `details` keys
are `function`, `parameter` and `position`. Member refusals come before any
model is read, and function shape and lookup come before argument binding. The
`WireFormat::SpineRunResult` catalog entry and its catalog test are correct.
The `convert_outcome` seam-probe registration is correct. There is no
`unwrap`/`expect`/`panic!` outside tests; the one `unreachable!` sits under
`cfg(seam_probe_replay_downstream)`.

The deliberate kernel-refusal deferral is isolated in
`qsl-replay/src/spine/call.rs` (`convert_outcome`'s `Outcome::Refused(_)` arm)
and documented in the module header. FND-003 is about its exit status, which
the note leaves out.

Defects: the AC-8 arch-lint check misses the common forms of a `qsl_eval`
leak. The `program.clauses` key is required, although FR-100 says a `1-draft`
program has none. Internal faults exit 20 instead of FR-100's 30. The
quire-extraction route skips the edition read. One TC-450 step 4 case never
reaches the branch it names.

## Verdict

**Changes requested.** One high and five medium findings; the remainder are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-100-AC-8 check matches only the literal identifier `qsl_eval` in top-level items. A `use qsl_eval::...;` import followed by the bare type name in a `pub` item passes, and so does a `pub fn` inside an `impl` block (`Item::Impl` is never classified). Both were confirmed on a probe tree. AC-8's own case, a `qsl_eval` type in `spine::run`'s signature, therefore passes when written through an import. | tools/arch-lint/api_surface.rs:1224-1231; tools/arch-lint/api_surface.rs:1325-1337 |
| FND-002 | medium | `wire::Program.clauses` is a required key. A `1-draft` request that follows FR-100 Inputs (`program` carries no `clauses`) is refused with `invalid-request` "missing field `clauses`", confirmed on the CLI. The FR-100 refusal row targets a request that *carries* `clauses`, but `run_complete` refuses only non-empty `clauses`, so `"clauses": []` is admitted. Both directions differ from the spec. | src/command/wire.rs:167-168; src/command.rs:897 |
| FND-003 | medium | `RunRefusal::Fault` maps to `Code::RuntimeInvariant`, whose `exit_code()` is 20. FR-100 says `runtime_invariant` (internal failure) exits 30. The fallback of every kernel refusal (including `CheckedInvariant`, specified at 30), `CallFailure::Fault`, and the two invariant guards therefore exit 20, "invalid/refused input". The deferred note in the module header does not state this exit status. | qsl-replay/src/spine/call.rs:168-172; qsl-replay/src/spine/call.rs:201; qsl-replay/src/spine/call.rs:414-421 |
| FND-004 | medium | Under `quire-extraction`, a request with `program.extraction` goes to `run_native` before the program source's edition is read. A `1-draft` source with `extraction` and no `call` therefore runs natively instead of refusing `invalid-request`, as FR-100 Behavior and AC-3 require. `CompleteRunSelection::Extraction` in `run_complete` can never be reached. The code comment says extraction has no `program.source` to read an edition from, but `extraction::select` uses `&program.source` and `compile` reads it as `original`. | src/command.rs:779-786; src/command.rs:903 |
| FND-005 | medium | In the TC-450 step 4 test, the base request always carries a `native-rule-model/1` model. The "no call" case is therefore refused by the `NativeModel` check, which runs before `NoCall` (CLI message: "selects a native rule-model source"). The `NoCall` branch is never exercised; deleting it leaves the test green. No case isolates the native model alone, and there is no `extraction` case. | tests/it/spine_run.rs:143-199; src/command.rs:907-918 |
| FND-006 | medium | Nothing tests the root crate's outcome renderer and exit mapping for `CallOutcome::Undefined` (exit 20) or a family `CallOutcome::Refused` (`code.exit_code()`, 21 for `unsupported_construct`). TC-452 step 4 only asserts `convert_outcome` in `qsl-replay`; no test asserts those JSON spellings or exit statuses. | src/command/output.rs:322-370; qsl-replay/src/spine/call.rs:703-799 |
| FND-007 | medium | TC-450 step 6, a `1-draft` run with `libraries` plus `models` exiting 0 and completed, has no `run` test. The `run_complete` library/model intake path (`package_input`, `DependencyInput::new`, `spine_run_failure` locating a library region) is never run end to end (coder-reported gap 1). | tests/it/spine_run.rs; src/command.rs:920-950 |
| FND-008 | low | The CLI tests assert selected fields, not exact documents. `details["position"] == 0` passes even if `details` has extra keys. TC-450 step 1 does not pin the member set of the document or of `source`. TC-451 step 1 covers only `lt` at the CLI (`flag(1)`/`id(4)` are unit-only). TC-450 step 2 asserts only `format` and exit 0, and relies on the existing standalone tests. | tests/it/spine_run.rs:74-102; tests/it/spine_run.rs:303; tests/it/spine_run.rs:505-521 |
| FND-009 | low | `work_units: null` is admitted as the default 1,000,000, confirmed on the CLI. FR-100 says `work_units`, when present, is a JSON integer. | src/command/wire.rs:114 |
| FND-010 | low | Binding and conversion run together per position. With a missing later argument and a non-0/1 `Boolean` earlier one, `WrongValueKind{0}` is reported instead of `UnboundParameter`. FR-100 orders the binding refusals before value conversion. | qsl-replay/src/spine/call.rs:320-332 |
| FND-011 | low | A `0-draft` run with `package` selected reads the program source twice: once for the edition, and again in `selected_package`. Its bytes are charged twice against `TOTAL_BYTES`, so a request near the budget can be refused when it would have been admitted before. | src/command.rs:836-846; src/command/compilation.rs:120-128 |
| FND-012 | low | Nits. `#[allow(clippy::too_many_arguments)]` on a 7-argument fn does nothing (the lint fires above 7). `default_accounting`'s doc names the default 1,000,000, but that value is a bare literal in the root crate (`unwrap_or(1_000_000)`), not a named constant beside the doc. The fixture uses `corner(p: Point): Point` where TC-451 names `origin(): Point`. | qsl-replay/src/spine/call.rs:37-39; qsl-replay/src/spine/call.rs:209; src/command.rs:963; tests/fixtures/spine-run.native:12 |

## Rust review

- Panic surface: clean. `unwrap`/`panic!` appear in tests only.
- Wildcards: `convert_outcome` is `#[deny(clippy::wildcard_enum_match_arm)]`, and `argument_value` lists every `ValueType`.
- Integer conversion at the wire boundary: `value: i64` and `work_units: Option<u64>` refuse out-of-range, fractional, string and boolean JSON at decode. Exception: `null` (FND-009).
- Layering: `qsl_replay::spine`'s re-exports (`default_accounting, run, Call, CallArgument, CallOutcome, CallValue, RunRefusal`) name no `qsl_eval` type today. `qsl_eval` appears only in private items and fn bodies. The check that guards this is weak (FND-001).
- Error types: `thiserror` with typed variants. The stage and code come from `RunRefusal::stage`/`code`, not from Display text.

## Gate

`make ci` at c23b3128, re-run by the reviewer (`CARGO_BUILD_JOBS=4`, one target dir in the worktree): exit 0. FND-001 was confirmed on a probe copy of the tree. A `pub` struct field typed through `use qsl_eval::value::QualifiedName;`, plus a `pub fn` in `impl RunRefusal` returning `qsl_eval::value::QualifiedName`, gives `FR-100-AC-8 ...: PASS`. Adding `pub use qsl_eval::value::Evaluation;` to `lib.rs` gives FAIL as intended.

## Dispositions

Every SR-674 (this review) and SR-675 (gap analysis) finding, with the commit
that fixed it. SR-675's own findings mostly restate an SR-674 finding under a
different angle; where they do, the SR-675 row names the SR-674 finding it
tracks rather than repeating a separate fix.

| Review | ID | Severity | Fixed in |
| --- | --- | --- | --- |
| SR-674 | FND-001 | high | 630728ba (use-alias/impl-block resolution added to the AC-8 check) |
| SR-674 | FND-002 | medium | 3f49b580 |
| SR-674 | FND-003 | medium | 630728ba (`CheckedInvariant`/`CallFailure::Fault`/locus faults now exit 30 directly, never through `Code::exit_code`) |
| SR-674 | FND-004 | medium | 3f49b580 |
| SR-674 | FND-005 | medium | 8eda2b73 (this fix round: isolated native-model/no-call/extraction fixtures, TC-450 step 4) |
| SR-674 | FND-006 | medium | 630728ba (kernel-side rows, `convert_outcome`); 8eda2b73 (remaining half: root-crate renderer/exit-mapping unit tests for `Undefined`, TC-452 step 4) |
| SR-674 | FND-007 | low | 630728ba (TC-450 step 6 CLI test for `libraries` plus `models`) |
| SR-674 | FND-008 | low | 630728ba (whole-document assertions, `flag`/`id` CLI coverage, stronger TC-450 step 2 oracle) |
| SR-674 | FND-009 | low | 3f49b580 |
| SR-674 | FND-010 | low | 630728ba (`bind_arguments` binds every parameter before converting any value) |
| SR-674 | FND-011 | low | 630728ba (`source` threaded through so `selected_package` does not re-read it) |
| SR-674 | FND-012 | low | 630728ba (the `#[allow]` and named-constant nits); 7ce0d104 (the fixture's `corner`-not-`origin` note) |
| SR-675 | FND-001 | medium | tracks SR-674 FND-005 (8eda2b73) and FND-007 (630728ba) |
| SR-675 | FND-002 | medium | tracks SR-674 FND-006 (8eda2b73) |
| SR-675 | FND-003 | medium | tracks SR-674 FND-002 (3f49b580) |
| SR-675 | FND-004 | low | 630728ba |
| SR-675 | FND-005 | low | tracks SR-674 FND-003 (630728ba) |
| SR-675 | FND-006 | low | 630728ba |

## Reviewer dispositions (efa42552)

I verified these myself at efa42552, which is based on a643665a (#469). That
included a fresh `make ci`, the FND-001 arch-lint probes and mutants in the
worktree, all reverted afterwards. I did not rely on the table above. SHAs are
the commits named by the table above, confirmed against the code on the branch.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 630728ba. Probes re-run with `arch-lint api-surface`: a `pub` field typed through `use qsl_eval::value::QualifiedName;` now FAILs (`struct Call`), and so does a `pub fn` in `impl RunRefusal` (`impl fn probe`). The clean tree passes. See FND-015 for a residual bypass. |
| FND-002 | fixed | 3f49b580. `clauses: Option<Vec<Binding>>`; a 1-draft request with no `clauses` runs (CLI, exit 0), and `"clauses": []` refuses (`clauses (empty array)` case). |
| FND-003 | fixed | 630728ba. `RunCause::exit_code` returns 30 for `SpineRun(Fault)`. The stage is `call`, and `details` is `{stage, invariant}`. The exit status is untested (FND-013). |
| FND-004 | fixed | 630728ba. The edition is read before extraction routing, a 1-draft request with `extraction` hits `CompleteRunSelection::Extraction`, and `tc_450_step_4_extraction_refuses_alone` passes under all-features. |
| FND-005 | fixed | 8eda2b73. `no_call_refuses_alone`, `native_model_refuses_alone` and `extraction_refuses_alone` pin the message. A mutant that turns NoCall into Clauses is killed by `tc_450_step_4_no_call_refuses_alone`. |
| FND-006 | deferred | partly fixed: 8eda2b73 covers `Undefined`, and a mutant 20→21 is killed. The refused and fault rows of the root renderer are still untested; this is carried as FND-013. |
| FND-007 | fixed | 630728ba. `tc_450_step_6_libraries_and_models_both_present_runs` (CLI and unit). |
| FND-008 | fixed | 630728ba. The TC-450 step 1 whole-document assertion, `flag`/`id` at the CLI, and a stronger step 2. |
| FND-009 | fixed | 3f49b580. `"work_units": null` gives `invalid-request` "invalid type: null, expected u64", exit 20 (CLI probe). No test pins it. |
| FND-010 | fixed | 630728ba. `bind_arguments` binds every parameter before converting any, pinned by `fnd_010_unbound_parameter_is_reported_before_an_earlier_wrong_kind`. |
| FND-011 | fixed | 630728ba. `selected_package` takes the already-read `source`. See FND-014 for the extraction path. |
| FND-012 | fixed | 630728ba and 7ce0d104. The allow is removed, `DEFAULT_WORK_UNITS` is added, and TC-451 Status records that `origin` is reserved. |

## Re-review findings (efa42552)

These are new or carried findings on the fix round, including the kernel-row
code (`CallRefusal::{Record, Family, Kernel}`, locus resolution, the
internal-failure path). They were checked against the amended FR-100 (#469)
and FR-096. I recomputed the fixture-`F` values independently: 229 bytes,
`sha256:5f2742391e3eaef04bc5dd7141fd639b1913dc821d14bb2f2ca618ad8598ca26`,
and the literal `5` at byte 225..226, line 3, column 54..55. The spec and the
unit test agree with these.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | The root crate's rendering of refused outcomes and internal faults has no test, and TC-452's ✅ overclaims. Four mutants all left `cargo test --lib --test it` green (43 lib + 905 it): `refusal_exit_code` → always 20, so AncestorSteps exits 20 instead of 22; `RunCause::exit_code`'s Fault→30 arm disabled, so faults exit 20; `SpineOrigin` spelled `type_declaration`; the fault `details.stage` rendered from `invariant()`. TC-452 step 4 names "the root crate's renderer and exit mapping" and its `CallFailure::Fault` case, but `call/tests.rs` covers neither: `convert_call_failure` has no test. | src/command/output.rs:361-364; src/command/output.rs:391-425; src/command.rs:366-373; src/command/output.rs:277-280; src/command/output/types.rs:234; qsl-replay/src/spine/call/tests.rs:323-588 |
| FND-014 | low | A `0-draft` extraction request now reads `program.source` twice: once in `run_bytes` for the edition, and again in `extraction::Selected::compile`. It is charged twice against `TOTAL_BYTES`, so the extraction path has the same double-charge FND-011 fixed for `package`. | src/command.rs:805; src/command/extraction.rs:274 |
| FND-015 | low | The AC-8 alias scan collects `use` bindings only. A private `type Q = qsl_eval::value::QualifiedName;` used as `pub fn probe(&self) -> Option<Q>` in `impl RunRefusal` still passes (probe: `FR-100-AC-8 ...: PASS`). Only glob imports are documented as a limitation. | tools/arch-lint/api_surface.rs:1276-1311 |
| FND-016 | low | `run` rebuilds `supplied_sources`, re-reading the program and every library, after every call, including completed ones that never render a locus. A read failure there turns a completed call into a `locus-source-supplied` fault. It should be built lazily, only for a record that carries a locus. | qsl-replay/src/spine/call.rs:281-282 |
| FND-017 | low | Behaviour outside FR-100 has changed: `compile` (FR-027) now refuses a 1-draft request carrying `"clauses": []`, and the old compile test that admitted it was flipped. That matches FR-027-AC-7 read strictly, but FR-027's text ("selects no clause bindings") was not amended to say the key must be absent. | src/command.rs:551; tests/it/compile_command.rs:534-548 |
| FND-018 | low | Nits. The doc comment on `tc_450_step_4_extraction_refuses_alone` still calls FND-004 an "already-reported ordering defect", but it is fixed. The coder's Dispositions table labels FND-007 low; it was raised medium. No test pins `"work_units": null` (FND-009). | tests/it/spine_run.rs:315-322; reviews/26-09-26-qsl-271-code-review.md:73-99 |

Checked clean at efa42552:

- The internal-failure path: stage `call`, `runtime_invariant`, `details` `{stage, invariant}` with `S6a`/`checked-program-invariant` and `spine-run`/`locus-source-supplied`, and exit 30 via the special case, not `Code::exit_code`.
- The three refused rows follow the member table.
- Kernel no-record is fixed at exit 20.
- Record and family exits use `Code::from_code`, or 20.
- `location` uses the four `origin` kinds in kebab case.
- `locus` is `{source_digest, span}`, resolved by reference equality over the program and library `Source`s.
- `spine`'s public surface (`CallLocus`, `CallRefusal`, `DEFAULT_WORK_UNITS` and the rest) names no `qsl_eval` type.
- No `unwrap`/`expect`/`panic!` outside tests; the one `unreachable!` is under `cfg(seam_probe_replay_downstream)`.

Gate: `make ci` at efa42552 exit 0 (`make-ci-r1.log`).

**Re-review verdict: not mergeable yet.** One medium (FND-013) remains, and it is a tests-only fix.
