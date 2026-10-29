---
id: SR-1743001
title: "code review (with rust-review) of PR #1743 (QSL-580r): postcondition presence requirements"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1743 git diff origin/qsl-580q2-union-clock...309d37970 (4 files): qsl-semantics check/timed_model.rs, qsl-eval simulation/model/timed.rs, tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1743001: code review of PR #1743 (QSL-580r)

## Summary

Ticket: QSL-580. Head reviewed: 309d37970, on qsl-580q2-union-clock.

- **Reading.** `post_presence` takes the top-level conjuncts `present(p)` and `p != absent` (present) and `not present(p)` (absent). It records them in `ClockResets::requires()` and replaces the conjunct with `true` in the data clause. This matches FR-230 "Presence bit": `present(p)` is a Boolean data predicate, and a postcondition's predicates constrain the post-state. `p = absent` stays a drop.
- **Eval.** `bits_follow` now rejects a successor whose receiver presences differ from the requirements. The frame does not havoc a clock: FR-230 says "a clock in the frame that the operation does not reset keeps its value", and the bit follows that. So `present(self.v)` with no write steps only from the present state, which the PR's test checks.
- **Contradictions leave no successor** (probed, both conjunct orders):
  - `self.v = absent and present(self.v)` and `present(self.v) and self.v = absent` give no successor from either initial state.
  - `self.v = 0 ms and not present(self.v)` and `present(self.v) and not present(self.v)` give none either.
  - `self.v = 0 ms and present(self.v)` gives one present successor with reset 0. `self.v = absent and not present(self.v)` gives one absent successor.
  - This matches the treatment of an unsatisfiable data postcondition: the operation has no successor at that state.
- **Union case** (probed):
  - The `work` frame havocs `state`. With `present(self.state.Armed)` it steps only into `Armed`, with reset 0. With `not present(self.state.Armed)` it steps only into `Idle`.
- **Zone side.**
  - `bits_follow` runs inside `TimedSystem::guarded_steps`. The zone checker (`qsl-analyze` zone_check search, liveness and deadlock), the timed explorer, `is_urgent` and quiescence all expand through it.
  - So a requirement reaches the zone checker, not only eval. A step filtered out is not enabled, so it neither prevents a deadlock nor makes a state urgent. That is the right reading of an unsatisfiable postcondition.
- **Refusals kept.** A presence read under `or` in a postcondition, in a quantifier body, or on another object's place still refuses `unsupported_construct`/`not-yet-implemented`.
- **Rust.** `ClauseClocks::Resets(ResetCut)` replaces the tuple, and `std::mem::take` moves `leaves`/`removed` out. Clean.

Gates at 309d37970 (locked-build, jobs 4, `--all-features`, debug info off):
- qsl-semantics it `clock_places`: 30/30.
- qsl-eval it `timed_subject`: 62/62.
- qsl-analyze it `zone`: 107/107.
- clippy `-p qsl-semantics -p qsl-eval --all-targets -D warnings`: exit 0.
- fmt: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `not (self.v = absent)`, which means the same as `self.v != absent`, refuses `PresenceNotChecked`: `post_presence` reads `Not` only over `present(..)` | FR-230 Presence bit |

## Verdict

Mergeable. FND-001 is low.

## Dispositions

Round 1, head 3d67f219f (STD-542, QSpec #485, FR-415-AC-21).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `post_presence` reads `not (p = absent)` and `not (p != absent)` through `absent_test`; probe `pre { not (self.v != absent) }` with `post { self.v != absent }` refuses `clock-presence-unwritten` |

New this round:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | `unwritten_presence` counts only resets with no parameter: `bump(c: Clock)` whose range is `[v, x]` with `pre { not present(self.v) }` and `post { c = 0 ms and present(self.v) }` refuses `clock-presence-unwritten`, though binding `c` to `v` writes `v` and has a successor | FR-415-AC-21 |

## Dispositions (round 2)

Round 2, head 64cf0b329 (fix adec3561f).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-002 | fixed | A `Clock` parameter reset whose range holds the place counts as writing it: `bump(c: Clock)` over `[v, x]` with `pre { not present(self.v) }` and `post { c = 0 ms and present(self.v) }` checks; with `v` outside the frame (range `[x]`) it still refuses `clock-presence-unwritten`; eval decides per instance (c bound to v steps with v present at 0, c bound to x has no successor) |
