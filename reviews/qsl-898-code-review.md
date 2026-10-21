---
id: SR-12291
title: "Code review of quire-spec-language PR #1172 (QSL-898): guarded on-each activation in EN-1, terminal-stutter marker, AC-6 undefined-contract fixture"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@02898aab2cbbda7b837407f805deb76b6f9d9063; PR #1172 diff origin/main...HEAD: qsl-analyze/src/model_check/{ample,check,liveness}.rs, qsl-eval/src/model_subject.rs, qsl-replay/src/model_check/{counterexample,replay}.rs, qsl-replay/src/certificate/simulation.rs, qsl-analyze/tests/it/*"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
---
# Code review of quire-spec-language PR #1172

## Summary

Ticket: QSL-898. Ticket and PR text were read as data. The scope is the PR
diff only. This file includes the `rust-review` lane.

What was checked, with throwaway probes (deleted) at the reviewed sha:

- **Guarded activation, engine side (FR-126-AC-16).** `Run::read` reads
  `ModelLetters::activation` before the atoms at every position the monitor
  reads, sets the translation's `when` atom where it is true, and ends the
  run with `UndefinedEvaluation{locus: Activation}` where it is undefined.
  The translation (`translate.rs`, pre-existing) builds
  `always (not (when and live) or body)`, so the monitor activates exactly
  where the guard holds. Probes over Counter, compared with hand-computed
  semantics: `always[0,1] holds(v <= 2)` on each guarded `v = 2` / `v = 3` is
  violated with prefix 0..3, guarded `v = 1` or never holds; TP-1
  `always holds(v <= 2)` guarded `v = 2` / `v = 3` is violated at 3, never
  holds; a guard undefined only at value 3 ends the run at position 3 with
  locus `Activation`; an atom undefined at value 1 with a guard that is
  never true still ends the run (UE-1 reads every atom), which matches the
  evaluator's `read_letters`, so engine and evaluator agree.
- **POR footprint join.** Probe on the `reaches_two` world with letters whose
  atoms read nothing and whose `activation_reads` is `x.v`: with
  `guarded: false` no bump is visible; with `guarded: true` `bump(a)` is
  visible and `bump(b)`, `bump(c)` are not. The join works. A guarded run
  with `partial_order: true` returns the same violation as the unreduced run.
- **Closure certificate.** `certified = !guarded && ...`, so a guarded
  `Holds` carries an empty certificate. Probe: `check_closure` on that
  certificate returns `Err(Item(ClauseRoot(Undeclared C0)))` (S4 emits no
  `on each` clause, so the package has no root for it), which `Row::of_check`
  settles `failed`. Were the clause emitted, `reachable_invariant` ignores the
  activation and the empty certificate would be rejected at the initial
  state. Either way the proof never settles `proved`. Sound.
- **Terminal-stutter marker (FR-126-AC-17).** `stutter` is true when the bad
  node's parent edge is the stutter edge; `Lasso{entry: prefix.len(),
  terminal_stutter: true}` matches the liveness engine's stutter lasso and
  `TemporalCounterexample::len` counts it. Replay's `close_loop` accepts it
  only at a state with no successors, which is exactly the engine's
  `terminal` test (boundary states are skipped before stuttering). Probe:
  the TP-3 stutter claim guarded `v = 0` or `v = 3` gives the marker and
  length 4; the AC-17 test replays it to a reproduction.
- **Replay.** `ModelTraceRefusal::GuardedActivation` is removed and
  `ReplayedLetters::activation` reads the seam. `state_at` is a clean
  extraction. The path is unreachable end to end today (FND-001).
- **Rust lane.** No `unwrap`/`expect`/`panic!`/`unsafe`/lossy `as` in
  non-test code. The meter error mapping matches the atom path. The new trait
  methods have no default, so every implementor decides; all 14 implementors
  are test letters, which use the `unguarded!` macro (`unreachable!`, test
  only). `Atoms.guarded` is set `false` on the liveness path with a correct
  comment (both liveness entry points refuse a guarded clause first).
- **Tests run.** `cargo test -p qsl-analyze --test it` and
  `cargo test -p qsl-replay --lib`; results in the Verdict.

## Verdict

No major finding. The guarded activation is sound on every probe, the POR
join is correct, a guarded proof can never settle `proved`, and the stutter
marker matches the lasso shape and replays. Two low findings below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ReplayedLetters::activation` (the replay half of "replay reads the condition") has no test, and it cannot be reached end to end: replay recompiles the unit, S4 refuses every `on each` clause (`NotEmitted`), so `resolve_item` never resolves a guarded clause. The removal of `ModelTraceRefusal::GuardedActivation` is fine, but the PR body's "Replay reads it through the same seam" is unverified. Add a test once S4 emits `on each`, or note it in the ticket's remaining work. | qsl-replay/src/model_check/replay.rs:1244-1253 |
| FND-002 | low | The guarded tests spell `when (true)` in source while the letters compute `c.value = 1`, `>= 2`, `>= 3`, `= 7` or an undefined reading. The module header says each atom is read "by a predicate that computes the condition its source text spells", which the guard readings now contradict. Say in the header (as TC-521 step 9 does) that the guard reading is the test's own and the source condition is a placeholder until S3 types the `when` guard. | qsl-analyze/tests/it/model_check_safety.rs:5-10; qsl-analyze/tests/it/model_check_safety.rs:531-541 |
