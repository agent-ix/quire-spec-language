---
id: SR-12322
title: "Code review (Rust lane) of QSL-655 process-provider terminal arms (PR #1208)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1ef4a3cbc6f981f2209d912e382c243893b00faa; qsl-replay/src/lib.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/proof_result/plugin.rs, qsl-replay/tests/terminal_record_facade.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: reviews
---
# SR-12322: Code review (Rust lane) of QSL-655 process-provider terminal arms (PR #1208)

## Summary

Ticket: QSL-655. PR quire-spec-language#1208 at head 1ef4a3cbc. Only the `origin/main...HEAD` diff was reviewed (4 code files). This is a code review with the rust-review lane folded in.

Commands run through the build lane: `cargo test -p qsl-replay --test terminal_record_facade` (13 pass), `make string-edge` (pass) and `make arch-lint-api-surface-qsl` (pass). No existing closed match gained a variant, so `make seam-probe` does not apply. No full gate was run.

Soundness, measured:

- **`Refuted` needs a reproduced violation.** The first arm requires `InputSettlement::ReproducedWithoutWitness` and `Category::Violation`. Every production Input-arm settle (`execute.rs` `settle`, `frame.rs`, `state_clause.rs`) passes `proved = Violation` and `category = replayed`. `Verdict` maps categories one to one. So `ReproducedWithoutWitness` means the replay completed a value whose category is `Violation`. `settle_range_violation` (FR-122) is the one other source, and it is a violation as well. A non-violating Input result, a Witness-arm result, an `Err`, and an Input result that reproduced but is not a violation never reach `Refuted`.
- **`ReplayParity` payload.** The payload is `arm.disagreement()`, which QSL computes in `DisagreementCause::of`. It is never read from the wire, which is what PL-7 requires. An `Inconclusive` Input arm always carries `Some` disagreement.
- **Fault and non-fault refusals.** Both go through `TerminalValue::from_replay_refusal`. A fault (`Fault`, or `Admission(Fault)`) settles `Failed`. Any other refusal settles `Inconclusive(ReplayRefused(code))`. Both are tested.
- **Plugin proof.** `from_plugin_proof` refuses `ProofBasis::Checks`. The only other place that builds `Checks` is `TerminalRecord::proved`, which is `Certified` and Kani-only. A plugin record therefore never carries `success_checks`.
- **Smuggling.** No other QSL-side reader turns a wire cause-kind string into an `InconclusiveCause`. The driver still builds plugin `inconclusive` records itself through the public `TerminalRecord::inconclusive`, so the refusal holds only if the driver calls `read_plugin_cause_kind` (FND-002).

Mutation probes. Each mutation was applied to `plugin.rs` on its own and the test file was run again; the source was restored afterwards and the tree is clean. Killed: `Refuted`→`Failed`; parity→`Failed`; Witness→`Refuted`; refusal→always `Failed`; `Checks` guard removed; `Trusted`→`Certified`; `kani-vacuous-proof` accepted; replay evidence dropped; `record()`→`tested`. Survived: dropping the `category() == Violation` guard, and changing the reproduced-but-not-violating arm (`None => Failed`) (FND-001).

Rust idioms: the enums are exhaustive, `Copy`, derive `thiserror`, and are re-exported at the root, as the PR body's changelog line states. The `#[string_edge]` marker is on the one string `match`. There is no panic surface, no `unwrap` in shipped code, and no new integer conversion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The soundness guard `arm.category() == Category::Violation` on the `Refuted` arm, and the `None => TerminalValue::Failed` arm (reproduced, not a violation), have no test. Mutating either survives the suite. The public `InputArmResult::settle(Success, Success, Success, Some(true), ..)` builds exactly that arm, so a driver-built or future-built result could settle `refuted` from a non-violation and no test would catch it. Fix: add one case to `plugin_counterexample_settles_by_its_replay` that settles a reproduced non-violation and asserts `Failed` with no replay result. | qsl-replay/src/proof_result/plugin.rs:83-99 |
| FND-002 | medium | `read_plugin_cause_kind` returns `Ok(())` for every non-QSL-only kind, and leaves the caller to build the `InconclusiveCause` through `TerminalRecord::inconclusive`. That constructor accepts `ReplayParity`, `ReplayRefused` and every other variant. So the PL-7 refusal works only if the driver remembers to call the guard first. It is not a typed reader whose output cannot carry a QSL-only cause. This is acceptable for the driver now. A reader that returns the typed cause, or refuses, would close it. | qsl-replay/src/proof_result/plugin.rs:57-68 |
| FND-003 | low | The three wire spellings are written out twice in `plugin.rs` (`QslOnlyCause::as_str` and the `match` in `read_plugin_cause_kind`). They also duplicate `InconclusiveCause::as_str` and the vacuous-proof cause's `as_str` in `proof_result.rs`. A rename in one place would silently change what is refused. Fix: match on `QslOnlyCause::as_str()` over the three variants, or test the two spellings against each other. | qsl-replay/src/proof_result/plugin.rs:30-36,62-66 |
| FND-004 | low | Merge order: open PR #1181 (QSL-951) migrates `#[qsl_attrs::string_edge]` to `ix_trace_rs::string_edge` across `qsl-replay`. This PR adds a new `qsl_attrs::string_edge` site. Whichever lands second must update `read_plugin_cause_kind`. | qsl-replay/src/proof_result/plugin.rs:60 |

## Verdict

Mergeable. There are no high findings. Every path to `Refuted` requires a QSL-side S6a replay that completed a `Violation` value. A plugin proof can never be `Certified` or carry `Checks`. The `ReplayParity` payload is derived on the QSL side. The fault and non-fault refusal split is correct. FND-001 is a small, cheap test addition and should ride in a fix round if one happens. FND-002 to FND-004 can follow.

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: tests reproduced_non_violation_settles_failed cover both arms; removing the category guard fails it | HEAD |
| FND-002 | fixed: TerminalRecord::from_plugin_inconclusive refuses ReplayParity/ReplayRefused by type; the kind reader refuses the three spellings, kani-vacuous-proof having no InconclusiveCause value | HEAD |
| FND-003 | fixed: one set of spelling constants shared by InconclusiveCause::as_str, ReportedInconclusiveCause::as_str and the reader | HEAD |
| FND-004 | accepted-no-change: whichever of #1181 and this PR lands second updates the marker | HEAD |
