---
id: SR-1313
title: "Code review of quire-spec-language PR #638: quire-outcome/1 serializer (QSL-592)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5dd6bacc001141ba412f385c0562ae8ee5e6f64d; PR #638 diff against c8f0c2818: qsl-replay/src/outcome.rs, qsl-replay/src/lib.rs, qsl-foundation/src/diagnostic/stage.rs, qsl-replay/tests/it/{main,outcome_facade}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: reviews
---
# Code review of quire-spec-language PR #638

## Summary

Ticket: QSL-592 (LC4). PR: quire-spec-language#638. Rust lane (`rust-review`) folded in.

The PR adds `qsl_replay::outcome`: a typed `OutcomeDocument` (format, operation,
last_stage, category, items, diagnostics, artifacts), constructors `from_check`
and `from_call`, item builders over `TerminalRecord`, and `to_bytes` through
`serde_json`. All of it is re-exported from the `qsl_replay` root. It also adds
`Staged::value()`.

Checked and clean:
- Root re-export: the facade test reaches every type through `qsl_replay::` alone,
  so the driver lane (IR-608) can use it without its own serializer.
- Byte stability: the document is plain structs and `Vec`s serialized by
  `serde_json`. There are no maps and no floats, so field order and bytes are
  deterministic. The AC-3 twice-equal check is what the AC asks for.
- `OutcomeStage: From<SpineStage>` is total, with no `_` arm.
- `StageFailure` arms: Refused takes its cause's category, Limit and Cancelled
  are incomplete, and Fault takes `fault.category()`. This matches ADR-029 CB-4.
- No `unwrap` or panic outside tests. `to_bytes` returns a typed error.
- FR-284: the serializer is the core's own JSON serialization of its outcome
  types. The make ci FR-284 direction and ambient-input checks pass.
- Merge with current main (9f2db743e): main changes the line next to the new
  `pub use outcome::…` in qsl-replay/src/lib.rs. A three-way merge-tree shows no
  conflict.
- Tests: the supplied make ci log (exit 0) shows the four `outcome::tests::*`
  tests and `outcome_facade::a_driver_writes_a_prove_document_through_the_facade` passing. A focused rerun through locked-build.sh was queued behind the
  machine-wide build lock.

## Verdict

Request changes: 2 high, 4 medium and 2 low findings. The document shape and the
re-export are right. But `from_check` runs E4 inside the serializer. A vacuous
proof is written as `proved`, which QSpec FR-331-AC-8 forbids. And the cause
spellings are a new hand-written table that disagrees with QSpec FR-331.

Answers to the brief's questions:
1. E4 inside the serializer is wrong (FND-002). FR-286 says only that the check
   document's `artifacts` hold "the package's `package_id`". ADR-029 OP-1 gives
   `check` the output `Staged<CheckedPackage>`, which carries no identity, and
   mints `package_id` in `package` (E4, `EmittedUnit`). The caller should supply
   the identity: add `from_package` over `package`'s result, and make the check
   document not emit. The spec side is SR-1315 FND-002.
2. CLI: see SR-1315 FND-004 and FND-005. The QSL CLI is not changed in this PR.
3. Builders for AC-3 and AC-4: see SR-1314 FND-001.
4. The undefined reason: see SR-1315 FND-001. It is a spec gap.
5. `compile_cause` is not a second spelling table, because each arm delegates to
   that cause's own `cause()`. But it is a second per-variant dispatch, apart from
   `CompileRefusal::code()`, `stage()` and `region()` (FND-007). The terminal-record
   cause spellings in `from_terminal` *are* a second table (FND-003). `last_stage`
   null and the always-present `items` are fine as code. The spec should state
   both (SR-1315 FND-006).
6. Bytes are stable. The oracles are weak: see FND-005 and FND-008.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `OutcomeItem::from_terminal` writes a vacuous proof (`Proved { success_checks: 0 }`) as `result: "proved"` with cause `kani_vacuous_proof` and category inconclusive. QSpec FR-331-AC-8 says such a run "records the item's result as `inconclusive` with a typed vacuity cause, never as `proved`". `TerminalValue::category` already maps it to inconclusive, so the label and category disagree inside one item. Fix: label `Inconclusive` with the vacuity cause, and add a test for `success_checks: 0`. | qsl-replay/src/outcome.rs:207-209 |
| FND-002 | high | `from_check` calls `qsl_package::emit_checked` (E4) to get a `package_id`, because `CheckedUnit` carries none. This runs a lifecycle stage inside a serializer. It ignores the caller's `Cancel` (`emit_checked` builds `Cancel::new()`). It charges no work meter and takes no `PackageLimits`, which breaks ADR-029 LC-3/LC-4 and FR-275 for that stage. It re-runs emission and RFC 8785 encoding on every call. It duplicates `emit_unit`'s omitted-node rule. And it turns an `Ok` check into a refusal document at S4, a refusal the `check` operation never returned, so the document is no longer "the serialized library outcome" (CB-4). Fix: keep the serializer pure. Add `from_package(&Result<Staged<EmittedUnit>, FrontEndFailure>)` (operation `package`, S4, artifact `package_id` from `EmittedUnit::package().package_id()`). Then either `from_check(Ok)` writes success at S4 with no artifact (with the SR-1315 FND-002 spec change), or it takes the caller's `&EmittedUnit`. | qsl-replay/src/outcome.rs:530-533, 577-597; qsl-replay/src/spine/lifecycle.rs:555-575, 659-671 |
| FND-003 | medium | The terminal-record cause spellings are hand-written string literals in `from_terminal` (`"refused"`, `"invalid_input"`, `"solver_absent"`, `"timed_out"`, …). That is a second spelling table beside the cause enums in `proof_result.rs`. They mix three conventions in one document: a PascalCase tag `UndefinedEvaluation`, snake `replay_parity` / `replay_refused` / `kani_vacuous_proof`, and snake values. QSpec FR-331 fixes kebab spellings (`undefined-evaluation{where, cause}`, `replay-parity`, `cancelled{source}`). `ReplayParity(_)` also drops its `DisagreementCause`, which proof_result.rs says "travels with the cause". Fix: give `ProofRefusalCause`, `UnavailabilityCause`, `IncompleteCause` and `InconclusiveCause` one `as_str` each in proof_result.rs, spelled as FR-331 spells them. Read those here, and keep the parity reason. | qsl-replay/src/outcome.rs:143-189, 213-254 |
| FND-004 | medium | `from_call` writes a refused call's diagnostic with `locus: None`. But `CallRefusal::Record` carries `locus` and `location` and `CallRefusal::Family` carries `location`. FR-286 says each diagnostic carries its `Locus`, and `OutcomeLocus::Occurrence` already converts a `Location`. Fix: map `location` (or the record's locus) into the diagnostic, and assert it in a test. | qsl-replay/src/outcome.rs:609-618; qsl-replay/src/spine/call.rs:108-133 |
| FND-005 | medium | The FR-286-AC-2 test checks the diagnostic's cause against `compile_cause(refusal)`, the same private function that produced it. If `compile_cause` picked the wrong cause, the test would still pass. Only a `None` would fail it. The commit title says "literal cause check", but the cause is not checked against a literal. Fix: assert the literal catalog cause the `inv` refusal carries. | qsl-replay/src/outcome.rs:722-725 |
| FND-006 | medium | The document `category` is whatever the caller passes. Nothing ties it to the items, so a driver must fold the CB-4 "most severe" rule itself, which is the logic this PR exists to keep out of the driver. The facade test passes `InternalFailure` by hand. Nothing stops a document whose items are all `success` from saying `violation`. Fix: an items constructor (for example `OutcomeDocument::with_settled_items`) that computes the whole-outcome category with one fold over `Category` in FR-301 order. | qsl-replay/src/outcome.rs:480-497; qsl-replay/tests/it/outcome_facade.rs:26-31 |
| FND-007 | low | `compile_cause` is a per-variant dispatch over `CompileRefusal` that lives in outcome.rs, apart from `CompileRefusal::code()`, `stage()` and `region()` in spine.rs. A new `CompileRefusal` variant has to be added in two files. Fix: move it to `CompileRefusal::cause()` in spine.rs. | qsl-replay/src/outcome.rs:394-412; qsl-replay/src/spine.rs:165-245 |
| FND-008 | low | No test compares a whole document. Each test pokes chosen members, so an added or renamed member (for example a new top-level key, or the `cause: null` every plain item carries) passes unnoticed. The FR-286 Overlap says the driver checks "no member added or removed". The AC-1 `package_id` oracle is also computed by the same `emit_checked` call the code makes. Fix: one `json!` literal equality per constructor (check success, check refusal, execute undefined, one item of each label). | qsl-replay/src/outcome.rs:676-818 |

## New findings (disposition pass 1)

Reviewed at 9e42b2c56888c7432a6cec58136b2a6c4e6dd578, after the rebase onto #636 (merge base bcca43356). A three-way merge with current main (a9cfe1113) shows no conflict.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | `from_replay` writes `last_stage` `null` for every `ReplayRefusal`, including non-fault refusals such as a decode refusal, an identity mismatch or a stale dependency. This PR's own FR-286 text says `null` only "when no stage is known: a cancelled outcome that stopped before its first stage, or an internal-failure (fault) outcome", and its Behavior adds "for a cancelled outcome that reached no stage and for a fault outcome". The replay refusal's diagnostic also writes `cause` `null`. The only replay-refusal test uses a fault, so it cannot catch this. Fix: give a non-fault refusal its stage (S8 / E9 admission), keep `null` for `ReplayRefusal::Fault`, write the refusal's catalog cause where it has one, and test one non-fault refusal. Or widen FR-286's `null` rule to cover replay refusals. | qsl-replay/src/outcome.rs:676-692 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 17619203d (a vacuous proof is `inconclusive` with `kani-vacuous-proof`; test `a_vacuous_proof_is_inconclusive_never_proved`) |
| FND-002 | fixed | 17619203d (`from_check` is pure and writes no artifact; `from_package` carries `package_id` from `EmittedUnit`; the tests use the real `package` operation) |
| FND-003 | fixed | 17619203d (`as_str` on `ProofRefusalCause`, `UnavailabilityCause`, `IncompleteCause`, `InconclusiveCause`, `ReportedInconclusiveCause` and `DisagreementCause`, in kebab spelling; the parity reason is kept in `parity`) |
| FND-004 | fixed | 2bbadf34e (`from_call_refusal` writes the record's resolved locus as `located`; test `a_refused_call_keeps_its_locus`) |
| FND-005 | fixed | 2937875b2 (AC-2 asserts the whole document with the literal cause `ambiguous-literal`) |
| FND-006 | fixed | 17619203d (`OutcomeDocument::settled` folds item categories by `Category::most_severe`, using `trace_exit_code` for `monitor`; test `the_document_category_is_the_most_severe_items`) |
| FND-007 | fixed | 17619203d (`CompileRefusal::cause()` in spine.rs; outcome.rs calls it) |
| FND-008 | fixed | 17619203d (`json!` whole-document equality per constructor, plus an exact-bytes literal for AC-3; the `package_id` comes from the `package` operation's output) |
| FND-009 | fixed | 3f0ba08fa (`replay_stage`: a recompile refusal gives its spine stage, every other refusal S8, and `null` only for `Fault` and `Admission(Fault)`; the diagnostic carries `ReplayRefusal::cause()`; tested with `DependencyIdentityMismatch` giving S8, refusal, stale_dependency, content-mismatch. The causes it still misses are FND-010.) |

## New findings (disposition pass 2)

Reviewed at 3f0ba08fa412bad509cac9a814f5b01b855be91d, rebased onto main 4403f2f0e. `git range-diff` shows commits 1 to 11 have the same content as round 1's 17619203d..9e42b2c56 under new SHAs (17619203d is 58af45b2d, 2bbadf34e is 29e98e1e1, 2937875b2 is d6650cfdc). Round 1's rows keep the SHAs reviewed then.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | `ReplayRefusal::cause()` returns `None` for seven variants whose own `Display` spells a catalog cause. `PackageIdMismatch` is `stale_dependency/content-mismatch` (ADR-015 D-4 says the same). `UnknownFunction`, `UnknownOperation` and `UnknownClause` are `missing_declaration/missing-name`. `FrameIdentity` and `ClauseIdentity` are `stale_dependency/revision-mismatch`. `WrongObservation` is `wrong_snapshot/wrong-observation`. `Input(refusal)` writes `refusal.cause()` in its message. So the replay document writes `cause: null` on these paths, against FR-286's "each with its typed cause", and the cause now lives in two places, the `#[error]` strings and `cause()`, which already disagree. `DependencyIdentityMismatch` gets `content-mismatch` but the same-coded `PackageIdMismatch` does not. Fix: return those causes from `cause()` (`Input` from `refusal.cause()`), and test one of them, for example `PackageIdMismatch`. | qsl-replay/src/execute.rs:271-302; qsl-replay/src/execute.rs:132, 140, 175, 189, 200, 204, 214, 218 |
