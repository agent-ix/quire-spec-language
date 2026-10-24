---
id: SR-9054
title: "QSL-726 code review of PR #819"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e11f3fc5269d895d4cf8a97ca9f54082684ae538; merge-base aa268c49c1f79f667ccee7f53be1bd0ed4551693; base impl/business-model-meanings (now includes #817, no file overlap); qsl-semantics/src/check/dispatch_link.rs; qsl-semantics/src/check/refusal.rs; qsl-replay/tests/dispatch_link_compile.rs; consumers: qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-semantics/src/check/mod.rs"
review_set: subset
---
## Summary

Ticket: QSL-726, AC-4 (QSL-INPUT-AC-4). This is a code review with its Rust lane folded in. It was run read-only over the PR diff at the reviewed sha. The PR changes `DispatchLinkCause.subtypes: Vec` to `subtype: Option<DispatchSubtypeFailure>`. `link_cause` becomes `link_causes`, which returns one cause for each failing subtype of an ambiguous family and one cause for any other failure. `PendingFamily.refusal` becomes `refusals: Vec`, and `PendingLinks::report` emits every record of a family at the first call that needed it.

Checked clean:

- Consumers were checked by signature and pattern match. `DispatchLinkCause` is built only in `dispatch_link.rs`. It is read in `refusal.rs` (`catalog`, `code`, `cause`) and `spine.rs` (`fields()` reads `denial` only), and through `pub use` in `check/mod.rs`. Tests read `.subtype`. No `.subtypes` reader or `link_cause` caller is left. No serialization path reads the subtype payload.
- Each record's catalog pair comes from its own subtype's cause (`ambiguous_dispatch` with `multiple-undominated` or `no-applicable`). That matches QSpec FR-272's `ambiguous_dispatch` row, whose record names one closed subtype. The old first-subtype fallback, which labelled a mixed family by its first cause, is gone.
- Subtype order within a family is the bridge's order, ascending effective identity (`model/dispatch.rs` 319-322). That matches the QSpec TC-196 D01 and D03 order.
- No `unwrap`, `expect` or `panic!` is added in production code. Every non-ambiguous arm still yields exactly one cause.
- No ticket ids appear in code or spec text.
- The base moved by #817 only, which touches no file in this PR, so there is no conflict.
- Gate log `qsl726f-gate.log` starts with the worktree path. fmt, all-features clippy, the sem/eval/replay/package/forms tests (qsl-replay dispatch_link_compile 13 passed), root, xtask (182), and spec validation all exit 0. The xtask `error:` lines are intentional failures in compile-fail fixtures.

## Verdict

**PASS** with one low finding. The shape change follows QSpec FR-272 and is applied consistently. The aggregation in `report` is correct for the cases tested. The cross-family order question is a spec-level finding, recorded in SR-9056.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `link_causes` maps `DispatchLinkOutcome::Ambiguous(failed)` with an empty `failed` to an empty `Vec`. The previous code had a fallback for this case. `PendingLinks::report` then marks that family reported, emits nothing, and drops the call's own `MissingName` refusal. The unit can then refuse with no record, or with one record fewer. Today the bridge builds `Ambiguous` only for a non-empty list (`model/dispatch.rs` 461-467), so this cannot happen yet, but the type does not enforce it. Fix: map an empty list to `vec![invariant(operation)]`, or have the bridge carry a non-empty type. | qsl-semantics/src/check/dispatch_link.rs:546-568; qsl-semantics/src/check/dispatch_link.rs:89-96 |
