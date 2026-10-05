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
