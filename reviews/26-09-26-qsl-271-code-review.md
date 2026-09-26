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
