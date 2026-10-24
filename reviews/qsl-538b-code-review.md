---
id: SR-12207
title: "Code review (Rust lane) of QSL-538 E8d remainder: settle_state_graph, FR-169 settlement map and package_id binding (PR #1058)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@344aec0b80462523e685c49f270076ae02a31992; qsl-replay/src/model_check/graph_settle.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/src/model_check/state_graph.rs, qsl-replay/src/model_check.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/outcome.rs, qsl-replay/src/lib.rs, qsl-analyze/src/model_check/liveness.rs, qsl-analyze/tests/it/state_graph_settle.rs, qsl-analyze/tests/it/state_graph_certificate.rs, qsl-analyze/tests/it/graph_world.rs, qsl-analyze/tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
---
# SR-12207: Code review (Rust lane) of QSL-538 E8d remainder: settle_state_graph, FR-169 settlement map and package_id binding (PR #1058)

## Summary

Ticket: QSL-538 (E8d), the remainder after #1046. PR quire-spec-language#1058 at head 344aec0b8. The scope is the diff `origin/main...HEAD` only: 1 commit, 12 files. This is code-review with the rust-review lane folded in. The reviewer ran no builds and no gates; the lead runs them.

What was checked, and holds:

- **Soundness of `proved`.** `Witnessed` reaches `Row::proved` only inside `Certified::row`. That is, only after `Row::bound` finds that the subject's emitted `package_id` equals the run's, and `check_state_graph` returns `Accepted`. The witness replay runs inside the `accepted` closure, so a rejected or stopped certificate is never replayed. The replay must also come back `Reproduced` against the run's `package_id`. `Holds` takes the same bound-then-check path with no replay. No other arm can produce `Proved`.
- **Exhaustiveness.** The outer match over `StateGraphOutcome` has no wildcard. `NoDecision{Completed}` follows FR-169's precedence: `UndecidedSuccessor` first, then horizon-only with a horizon (`BoundReached{depth: max_depth}`). Any other completed case, which is an engine invariant break, gives `Failed`.
- **The FR-127 refactor keeps behaviour.** `settle_model_check` now builds its rows from the shared `Row` builders. Its arms keep their old rows: `Holds` binding and check, `BoundReached` depth, `Undecided`, `Stopped` (`ResourceExhausted` plus a limit becomes `LimitReached`), and `Violated` (stale, reproduced with counterexample, parity, fault becomes `failed`, other refusals become `ReplayRefused`). `certificate_package` became `emitted_package_id` over a `&CheckedPackage`, with the same emission and cancel handling.
- **The wire spelling.** `WitnessSource` serializes to `{"type":"sampled","seed":"<decimal>","trace_index":"<decimal>"}` / `{"type":"explored"}`, matching QSpec FR-393. `ProofBasis::Witness` is renamed `witnessed`. The new `InconclusiveCause::WellDefinednessUnchecked` has its `as_str`, `measured_bytes` and category arms.
- **Rust lane.** The production code has no `unwrap`, `expect`, panic, `unsafe`, integer cast or lock. `replay` is `FnOnce`, and every arm calls it at most once. `Row` and its builders are `pub(super)`, which keeps them inside `model_check`. `BOUND_POSITIONS` is de-duplicated into `check.rs`, where it was already defined. The test helpers made `pub(crate)` are used only by the sibling `it` module.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The reported item cause for `WellDefinednessUnchecked` keeps only the kind and drops the witness sources. QSpec FR-391-AC-13 says the `well-definedness-unchecked` result carries the witnesses. The defect is latent, because `settle_state_graph` has no production caller yet, but the reporting path will lose the sources once one is wired. | qsl-replay/src/outcome.rs:297-299 |
| FND-002 | low | The `WitnessedUnchecked` arm sets `record.end` and copies the run's limit into `limit_reached`, whatever the replay row is. A stale, parity, refused or failed replay therefore gives a `ReplayRefused`/`ReplayParity`/`Failed` record that carries a WDU-only `end` and a run limit. The doc on `end` says only a `WellDefinednessUnchecked` record states it. | qsl-replay/src/model_check/graph_settle.rs:310-330 |
| FND-003 | low | `Witnessed` settles `Proved{Witness{sources}}` from `paths` without checking that there is one path per initial state. A `Witnessed` outcome with an accepted certificate and an empty `paths` would settle `Proved{Witness{sources: []}}`. FR-168 guarantees one witness per initial state, so this is an engine invariant, but the map is the last place that could refuse it. | qsl-replay/src/model_check/graph_settle.rs:297-306 |
| FND-004 | medium | A package mismatch settles `ReplayRefused(stale_dependency)` and names neither identity. FR-282 says a recompiled `package_id` that differs is rejected "with a cause naming both identities". This PR reuses FR-127's existing `Holds` binding, so the gap predates the PR; it now also covers state-graph certificates. The map is still sound: a mismatch never settles `proved`. | qsl-replay/src/model_check/settle.rs:791-811; spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md:62-63 |

## Verdict

**Mergeable. There is no major finding.** The map cannot settle `proved` without a package match, an accepted core check and, for `Witnessed`, a reproduced replay against the run's package. Its tests show this for rejected, stopped and stale certificates. The FR-127 refactor keeps every row it had. FND-001 and FND-004 are medium spec-conformance gaps worth fixing in this PR or routing. FND-002 and FND-003 are low.

## Dispositions

Round 1, at 3aa1addf20576f12c5c08adecd96a5c7d3e474c7. Each outcome was checked against the fix commit's code, not against the coordinator's claims.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3aa1addf2. `ItemCause` gains `sources`, which the `WellDefinednessUnchecked` arm fills. `well_definedness_unchecked_wire_keeps_its_sources` pins the wire. |
| FND-002 | fixed | 3aa1addf2. `end` and `limit_reached` are now set inside the reproduced closure only. The stale-replay test asserts `(end, limit_reached) == (None, None)` for `WitnessedUnchecked`. |
| FND-003 | fixed | 3aa1addf2. `one_per_initial(paths, subject.simulation.initial.len())` guards `Witnessed` and `WitnessedUnchecked` before any check or replay, and settles `failed`. The engine builds one `ModelSimState` per initial document and one witness per `graph.initial` index, so the counts agree on real runs. `a_witnessed_outcome_missing_a_witness_never_settles_proved` pins the guard. |
| FND-004 | deferred | Pre-existing. The `stale_dependency` cause is FR-127's existing binding, now shared by both maps. Naming both identities, as FR-282 asks, is a spec-level change to the cause for FR-127 and FR-282 together, outside this PR. It is sound as is: a mismatch never settles `proved`. |
