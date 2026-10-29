---
id: TC-1240
title: "Establish the native-to-TL temporal correspondence"
type: TC
relationships:
  - target: ix://agent-ix/quire-contract-ir/FR-350
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: references
  - target: ix://agent-ix/quire-specification/FR-092
    type: references
  - target: ix://agent-ix/quire-specification/FR-095
    type: references
---
# TC-1240: Establish the native-to-TL temporal correspondence

## Description

These are the public Rust API controls for the Contract IR correspondence
requirement `quire-contract-ir:FR-350`, using the QSL `mapping_support_v2`
classification and the strict `/2` package.

Each case compiles a real native temporal declaration and emits a strict
`quire.compiled-protocol/2` package. It classifies the declaration through
`mapping_support_v2`, then projects it with `temporal::project_tl` against
an FR-043 trace built independently. The native and TL results are joined
with `temporal::join_tl`.

The TL formula, trace or history and request are admitted by the real
tl-syntax, tl-mltl and quire-mltl public readers, and are evaluated by those
owners. The compiler evaluates no TL artifact.

Expected truths come from the authored source and the profile definitions,
never from either evaluator's output.

The complete bounded-past operator set is defined by QSpec FR-092; strong
previous is `once[1,1]`.

Scope: Contract IR FR-350; QSL FR-045 supplies the classification and owner
inputs.

## Test Procedure

1. Emit and admit `always[0,1] holds(p)` under the event-position profile.
   Classify it under a complete surrounding execution and under an open one,
   and project each. Require the formula, trace and request to pass their
   owner readers. Over every member of QSpec FR-095-AC-1's exhaustive
   bounded-future formula and trace population, evaluate natively with
   `evaluate_v2` and through the TL owner, and join the two results
   (FR-350-AC-1).
2. Join the one-position `always[0,1] p` vector against a finite-window
   reading under `mltl.closed-trace/v1`. Project timestamped-event,
   mixed-direction and infinite-trace declarations, each classified
   `Unsupported`. Under each false-extension profile, project `once[0,2] p`,
   `historically[0,2] p`, `p since[0,2] q` and
   `p triggered[0,2] q` separately with an execution-origin boundary; also
   project strong previous as `once[1,1] p`. Require formula-v2, the past
   catalog and position-history owners to admit each projection. Repeat each
   `[0,2]` case with the anchor at the first retained position and a
   history-cutoff boundary that does not prove the required prior positions
   (FR-350-AC-1 and FR-350-AC-2).
3. Mutate each preserved axis of step 1's projection in turn: profile,
   source span, predicate binding, clock, capture, interval, closure, history
   and bridge identity. Then supply a numeric, a nullable and a pending leaf
   value. Then make a display-only source change. Compare each identity
   before and after each change (FR-350-AC-3).
4. Emit a fixed-sample declaration (`epoch` 0, `period` 1, `unit` `ms`).
   Project it with the exact trace, with a trace of `period` 2 and the same
   formula bytes, and with a trace missing one required sample
   (FR-350-AC-4).
5. Drive `join_tl` into each row of the join table. For each row, drop each
   preserved axis from one result. Join a corrected pair first with its prior
   join, then without it, then with a prior join whose predecessor differs
   (FR-350-AC-5).
6. Project with each of these: a missing valuation cell, a duplicate cell, a
   foreign cell, a TL artifact its owner reader rejects, and a noncanonical
   projection or join. Run every limit exactly at its value and one past it
   (FR-350-AC-6, FR-350-AC-8).
7. Encode FR-350-AC-3's correspondence record with `quire-canonical`,
   then encode it again with its interval changed to `[[0, 2]]`,
   with its model binding's package changed to 64 `a`s and with its bridge
   definition changed to 64 `b`s, and read
   the record back from reordered and from whitespace-padded bytes
   (FR-350-AC-7).
8. Join a refused TL result beside a failed native result, an unsupported
   native result beside an incomplete TL result, and two equal final values. Run each `TlLimits` limit
   at its default and one past it (FR-350-AC-5, FR-350-AC-8).

Tag the tests `#[trace("TC-1240", "FR-350-AC-n")]`.

## Expected Results

- Step 1: every owner reader admits the generated artifacts. Every join is
  `agreement` with the native truth. The open execution selects
  `mltl.online-prefix/v1` and the complete one selects `mltl.closed-trace/v1`.
- Step 2:
  - The finite-window claim is rejected.
  - Each `Unsupported` projection is `unsupported`, names the
    classification's dimensions and carries no formula.
  - Each bounded past operator under both false-extension profiles emits
    `tl-syntax.formula/v2` with a position history and the
    `mltl.origin-complete-history/v1` target when history reaches the execution
    origin; `once[1,1]` preserves strong-previous semantics.
  - Each `[0,2]` vector with an insufficient history cutoff yields
    `incomplete`, cause `missing-history`.
- Step 3:
  - Each axis mutation changes the correspondence reference and names that
    axis.
  - The three non-Boolean leaves never become propositions.
  - The display-only change leaves the semantic identities unchanged.
- Step 4:
  - The exact trace's projection retains epoch, period and unit.
  - The period-2 trace refuses.
  - The missing sample yields `incomplete`.
- Step 5:
  - Each row is reached.
  - No non-final decision carries a Boolean.
  - No result with a dropped axis gives `agreement`.
  - The corrected pair with its prior join is admitted.
  - The pair without a prior join, and the pair with the wrong predecessor,
    are both `conflict`.
- Step 6: each malformed or over-limit input gives one deterministic
  non-admitted decision with no partial artifact. Each limit reached exactly
  is admitted. No TL evaluator is invoked by the compiler.
- Step 7: 1429 bytes, and the interval change, the
  model binding change and the bridge definition change each give different
  bytes. Both
  noncanonical byte sequences refuse.
- Step 8: `refused` with causes `result-refused` then `result-failed`;
  `unsupported` with causes `result-unsupported` then `result-incomplete`;
  `agreement` / `equal-final` with an empty cause set. Each limit admits at its default,
  and one past it stops with `limit-reached` naming the limit, its value and
  its `temporal.correspondence.*` setting.
