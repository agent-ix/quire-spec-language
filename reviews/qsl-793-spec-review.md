---
id: SR-11748
title: "Spec review of quire-spec-language PR #906: effective view state machine states, transitions and conformance"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ffd1bdc98c6cb8ee38e12ce47b60e953705c33e5; spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md, spec/test-cases/TC-1049-view-exposes-state-machine-states-transitions-and-conformance.md"
review_set: subset
---
# Spec review of quire-spec-language PR #906

## Summary

Ticket: QSL-793. One pass of spec-review with the contradiction,
completeness, vector-soundness and EARS/AC-consistency sub-analyses over the
PR diff (`origin/main...HEAD`). Spec-only PR, so no code review. Only major
issues block; mediums and lows are recorded in the Verdict.

Oracles: QSL on main: FR-081, FR-082 (closed-graph reachability, bounded
walks), NFR-012 (`EffectiveView::effective_limits` precedent),
`qsl-semantics/src/model/normalize.rs` (`EffectiveView`, phase-3 edge
charging, state-machine checks), `qsl-semantics/src/model/index.rs`
(`conforms`, `conformance_walk`), `qsl-semantics/src/model/key.rs`
(`DeclarationKey` ordering), `qsl-semantics/tests/support/tc235_k2.rs` (K2),
`qsl-semantics/tests/it/tc235_state_machine.rs` (M19 runs 1-7). QSpec by id:
FR-208, TC-235 M19.

Examined:
- FR-081 Outputs addition and new section "The completed view exposes a state machine's effective states and transitions, and conformance" (examined)
- FR-081-AC-17, FR-081-AC-18, FR-081-AC-19 (examined)
- TC-1049 steps 1-6 and expected results (examined)
- FR-082 conformance and bounded-walk sections (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new ordering rule ("States are ordered by declaring machine's key ascending, then position; transitions likewise") contradicts FR-081-AC-17 and TC-1049 step 1's expected sequence. K2 keys are `ix://test/orders/<node>` compared as UTF-8 bytes (`DeclarationKey` derive(Ord), key.rs), so `ExpressLifecycle` sorts before `OrderLifecycle`: the rule yields states `Expedited`, `Draft`, `Placed`, `Cancelled` (K2 declares `OrderLifecycle` states `[Draft, Placed, Cancelled]`) and transitions `ExpressLifecycle [0]` then `OrderLifecycle [0]`-`[2]`. AC-17 and TC-1049 expect `OrderLifecycle [0]`-`[2]` "followed by" `ExpressLifecycle [0]`, and list states `Cancelled, Draft, Placed, Expedited` (neither key-then-position nor position order). A conforming implementation fails the TC row. Fix: either keep key-then-position and correct AC-17/TC-1049 to that sequence, or change the rule to ancestors first (and say how depth ties under multiple supertypes break) and list the states in position order. | spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md:205-206,298; spec/test-cases/TC-1049-view-exposes-state-machine-states-transitions-and-conformance.md:28 |
| FND-002 | medium | FR-081-AC-18's clause "two types with equal display names and distinct keys do not conform" (FR-082's display-name rule, restated in the new section) has no TC-1049 step: step 3 calls `conforms` only on four K2 pairs, none sharing a display name. TC-1049's Description claims to catch "an implementation ... that reports conformance by display name", but no row can fail such an implementation. Fix: add a step building two distinct-key types with the same display name (one with a `supertypes` edge to a third type the other lacks) and expect `false`. | spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md:299; spec/test-cases/TC-1049-view-exposes-state-machine-states-transitions-and-conformance.md:21,30 |

## Verdict

FAIL, two blocking findings (FND-001 high, FND-002 medium, the latter
blocking under the "an AC no row covers" rule).

Traced and correct:
- TC-1049 restates TC-235 M19 truthfully against the merged test: run 1
  effective state set `{Cancelled, Draft, Expedited, Placed}`, inherited
  `OrderLifecycle` with three transitions and `[0]` guarded `NotYetPlaced`,
  `ExpressLifecycle` one own transition with no guard (run 1) and guard
  `NotYetPlaced` (run 7), and `supertypes == [OrderLifecycle]`
  (`assert_express_effective_machine`). Step 6's repeated state matches run 2
  (`invalid_model_binding`, `DerivationConflict`).
- Choice 1 (typed `state_machine(T)` accessor, no `declarations()` entries):
  sound. The merged M19 test states states and transitions have no
  effective-view entry; adding them would change published identities.
- Choice 3 (`conforms` charges nothing, has no limit outcome): sound in
  effect. Normalization phase 3 charges every placed edge one
  `normalize.cycle-check` and counts it against a package-wide
  `ancestor_steps` total (`normalize.rs` around L2927-2943), so any single
  walk in a completed view follows at most already-admitted edges and can
  never exceed the limit.
- Choice 4 (Rust names in the spec): acceptable; NFR-012 and TC-434 already
  name `EffectiveView::effective_limits`.
- No roadmap language, no ticket ids in spec text.

Recorded, non-blocking:
- medium: FR-082's bounded-walk section says every conformance walk charges
  each edge before following it; the new "no charge" sentence should cite why
  (phase 3 admitted the package's whole edge set against `ancestor_steps`)
  or FR-082 should exempt reads from a completed view.
- medium: TC-1049 step 6 "a transition naming no effective state" expects
  `malformed-declaration`, which holds only when the reference names another
  machine's state; a reference naming nothing refuses `missing_declaration`
  (`normalize.rs` transition checks). Name the construction.
- low: Behavior requires each entry to carry its name, original key and
  emitted event keys; AC-17/TC-1049 assert none of them (K2 `[0]` emits
  `OrderPlaced`).
- low: AC-18's "byte-identical with and without calling either accessor" is
  near-vacuous for `&self` reads of an immutable view.
- low: `conforms` on a key that is not a declared type is unspecified
  (merged `conformance_walk` returns false).
- low: the new section quotes the QSpec FR-208 heading ("State machine");
  cite by id only.
