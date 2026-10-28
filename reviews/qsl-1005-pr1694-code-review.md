---
id: SR-1694001
title: "code-review and rust-review of PR #1694 (QSL-1005b, timed refinement safety half on EN-6)"
type: SpecReview
analysis: code-review
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1694 git diff origin/qsl-1005a2-reduction-negotiation...5c7b8371d: qsl-analyze/src/zone_check/{refine.rs,search.rs}, qsl-semantics/src/check/{refinement.rs,mod.rs}, qsl-replay/src/spine/refinement.rs, tests timed_refinement.rs, timed_world.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: reviews
---
# SR-1694001: code-review and rust-review of PR #1694 (QSL-1005b)

## Summary

Ticket: QSL-1005. Head reviewed: 5c7b8371d, scoped to the diff against `origin/qsl-1005a2-reduction-negotiation`. The review was led by soundness.

**What is right.**
- **`check_step` runs only on discrete edges.** `Product::expand` judges each step of a node once (`judged` is keyed by step index) on FR-239's symbolic successors. A delay changes only the zone, and a node's `q` is the interned FR-142 position, keyed by the state key, the history key and the abstract candidates' keys.
- **The zone × discrete product is exact for the safety half.** Coverage needs equal `s` and `q`, and a failure reads only the discrete ends and the position, which the aLU abstraction preserves.
- **TV-1 is never missed.** `node_escape` explores the forward closure afresh (EZ-7), so a divergent pre-state is not dropped.
- **Concretisation.** The tree path is lifted and the failing step's stage appended (FR-241). The delays match in the PR's tests.
- **`TimedAbstract`.** It matches the abstract alias against the unit's `time` sources.

**Probes.**
- A clock read reached through a declaration, `inflight = near(self)` with `predicate near(c: M::Call) { c.x >= 1 ms }` (and the `function` form), is refused, `ill_typed`/`type-mismatch` at `c.x`. Clocks are not data in the kernel's type environment, so nothing that reads a clock is admitted. The name match only decides the cause and the locus (FND-002).
- **TV-1 into a dead trap: a false refutation (FND-001).**

**Gates.** All were run with locked-build at jobs 4, `--all-features`, debug info off, and QSPEC_DIR=qspec-e3d48d76.
- fmt: exit 0.
- qsl-analyze `timed_refinement` / `zone_`: 107/107.
- qsl-semantics `refinement`: 85/85.
- qsl-replay lib `refinement`: 43/43.
- root `refinement` / `family_outcome_layering`: 10/10.
- clippy with `-D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | **TV-1 is tested on the pre-state, not on the failing behaviour, so EN-6 refutes a half that holds on every admitted behaviour.** The edge refutes if `node_escape(node)` (the whole pre-state zone) diverges by any route, but the failing step may lead only into time-locked states. Probe: `send` (guard `x >= 1`, sets `inflight`, resets `x`) and `undo` (pre `inflight`; post `not inflight`, `late`, `x = 0`), `undo -> stutter`, with `time invariant when self.late { x <= 0 }`. After `undo` the state is dead and time-locked, so no time-divergent behaviour takes `undo`. EN-6 returns `Violated`: `send`@1, `undo`@0, `StutterChanged{position: 2}`. ADR-026 TV-1 / TS-4 say a violation refutes only on an admitted behaviour, and one reached only through time-locked states holds vacuously. FR-430's text ("its pre-state has a point with a time-divergent continuation") is weaker than TS-4, and the code follows that weaker text. The untimed `replay_refinement_trace` checks no time divergence, so nothing downstream catches it. Fix: require a time-divergent continuation from the failing step's post-state (the successor's state and zone), which implies the pre-state test. Tighten FR-430's sentence to match, as a spec gap. | qsl-analyze/src/zone_check/refine.rs:333-360; spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md (Timed refinement) |
| FND-002 | medium | **`clock_read` matches by name, not by place.** It refuses any `Field { field }` whose name equals the first path step of any clock place, whatever the receiver. For a clock inside a record (`Call.t.x`, path `[t, x]`), that refuses every `.t` access, including the data read `self.t.n`. It also refuses any field named like a clock field on another type, such as a step parameter's record. That is incompleteness, not unsoundness: as probed, a clock read it misses is refused as `ill_typed` instead. Fix: resolve the access to a place. The receiver must be the concrete object or a path from it, and the full field path must be a clock place's path, or a prefix of one that ends at a clock. | qsl-semantics/src/check/refinement.rs:1934-1969 |

## Verdict

**Not mergeable.** FND-001 is a false refutation by EN-6. FND-002 should be fixed in the same PR.

## Dispositions, round 1 (head ec07fbeab)


Round 1. Fix 5c7b8371d..ec07fbeab, built to QSL #1697 (STD-516).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | TV-1 is now judged at the failing step's post-state: `escape(successor.state, successor.zone)`. Details below. |
| FND-002 | fixed | `timed_model::first_clock_read` resolves a read by place path. The PR's `a_rows_clock_read_is_found_by_its_place_path` and `a_clock_read_through_a_predicate_or_function_is_refused_at_the_call` pass: `self.t.n` and `self.note.x` are admitted; `self.t.x`, `near(self.x, 3 ms)` and `pick(...)` are refused `expression-form`. My earlier predicate-over-`c.x` probe is still refused (`ill_typed` at `c.x`). |

**FND-001 in detail.**
- A failing initial state refutes only if it can diverge; otherwise it settles TD-6 `NoAdmittedBehaviour`.
- `Holds` requires an initial node that diverges.
- `Undetermined` stays `Undecided`.
- Concretisation ends at a point within `escape.to_quiescent`'s pre-path, or a checked least-delay point; otherwise it settles `LassoNotConcretized`.
- **Probe re-run:** my send@1 / undo@0 dead-trap model now **Holds** (2 symbolic states). Without the invariant it is Violated, with `StutterChanged{2}`.
- **Mutant:** reverting to `node_escape(node)`, the pre-state test, fails `a_failing_step_refutes_only_where_its_post_state_can_diverge` (AC-6).

**Verdict: mergeable.**

Gates at ec07fbeab (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=qspec-e3d48d76):
- fmt: exit 0.
- qsl-analyze `timed_refinement` / `zone_`: 113/113.
- qsl-semantics `refinement`: 85/85.
- qsl-replay `refinement`: 43/43; `model_check`: 6/6.
- root `refinement` / `family_outcome_layering`: 10/10.
- clippy with `-D warnings`: exit 0.

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: 'let escape = run.escape(&successor.state, &successor.zone, &scale, &lower, &upper)?; if !escape.diverges { continue; }'
- fnd: FND-002
  outcome: fixed
  after_excerpt: 'if let Some(span) = super::timed_model::first_clock_read(expression, self.concrete_side.model, self.declarations.models, self.clock_reading, *object) {'
```

+++
