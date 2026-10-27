---
id: SR-1676001
title: "code-review and rust-review of PR #1676 (QSL-580m, sequence types, clock/unit aliases, max_clocks, unbounded quantifier families)"
type: SpecReview
analysis: code-review
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1676 git diff 1ed0d0bee..27a6266a4 (one commit): qsl-eval/src/simulation/model/timed.rs, qsl-semantics/src/check/{timed_model.rs,timed_model/quantified.rs,timed_model/reading.rs,assemble.rs,lowering/model.rs}, qsl-semantics/src/model/{intake.rs,domain_package.rs,index.rs}, tests timed_subject.rs and clock_places.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# SR-1676001: code-review and rust-review of PR #1676 (QSL-580m)

## Summary

Ticket: QSL-580. Head reviewed: 27a6266a4. The review is scoped to the one commit `1ed0d0bee..27a6266a4` and was led by soundness.

**Family instantiation is correct, including the empty collection.** `TimedSystem::instantiated` is a conjunction over held slots of a disjunction of template parts when `each` is set, and one disjunction over all slots otherwise. The flag is `forall == positive`. Probed over `ts: Sequence<Timer>` with no maximum:

| Guard | Over `[]` | Over `[n=2, n=1]` |
| --- | --- | --- |
| `forall(e in self.ts: e.x <= 1 ms)` | one empty part (true) | — |
| `exists(...)` | no part (false) | — |
| `not forall(...)` | false | the parts `x0 > 1` and `x1 > 1` |
| `not exists(...)` | true | one part, `x0 > 1` and `x1 > 1` |
| `not exists(e: e.n = 2 and e.x <= 1 ms)` | — | correct, if redundant, parts |
| `forall ... or exists ...` | — | correct |
| `self.t.x <= 1 ms and forall ...` | — | correct |

A quantifier over an unbounded collection nested inside another (`forall(a in ts: ... exists(b in ts: ...))`) is refused at check with `clock-read-outside-constraint`. So the inner families that `instantiated` would ignore are never built.

**Field paths pick the right places.**
- `collection` follows the path through `typing.component`.
- `element_type` follows the record composites.
- `element_clock` matches a prefix of fields, then a slot.
- `site_collection` requires a slot just after the argument path.
- The PR's `self.h.ts` tests pass, and I traced the code.

**MaxClocks can truncate silently.** This is FND-001.

**Aliases and sequences.** `unaliased` detects cycles. An alias accepts only `Clock` or a unit as its target, and an alias with constraints is refused `malformed-declaration`.

**Gates.** All were run with locked-build at jobs 4, `--all-features`, and QSPEC_DIR at bd73a062.
- fmt: exit 0.
- qsl-eval `timed_subject`: 28/28.
- qsl-semantics `clock_places`: 17/17.
- qsl-semantics lib `intake`: 109/109.
- qsl-analyze `zone_`: 88/88.
- clippy on qsl-eval, qsl-semantics, qsl-analyze and qsl-replay with `-D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | **MaxClocks truncates silently when no slot fits.** When the bounded places leave room for fewer than one element's clocks, `listed` is 0. `slot_vectors` then gives the unbounded collection no places at all. But `beyond_listing` only walks `self.places`, so it never inspects that collection and never stops. Probe: `TimedSystem::with_max_clocks(.., 3)` or `4` over `ts = [Timer, Timer]`. The bounded places are `t.x`, `t.y` and `x`, and each element holds 2 clocks, so the live clocks number 7. Result: with guard `true` the state **steps normally**, its four element clocks silently absent from the zone and the digest. With a guard `forall(e in self.ts: ...)` the state stops `runtime_invariant`/`established-invariant-broken` instead of `MaxClocks`. With `max_clocks` 5 the stop is `MaxClocks` as intended. The default 1024 reaches this case in any model with at least 1023 bounded places and one unbounded collection. Fix: walk the clock fields of each object, not the listed places, or stop at any element when `listed == 0`. Add the `max_clocks = 3` and `4` cases to `an_unbounded_collection_is_bounded_by_max_clocks`. | qsl-eval/src/simulation/model/timed.rs:490-509,564-628 |
| FND-002 | medium | **A family's alternatives grow exponentially, with no budget.** In `instantiated`, an `each` family multiplies the alternatives by each slot's options. A body with two convex parts per slot gives `2^n` parts at `n` held elements, and `n` may reach `max_clocks / clocks-per-element`. For example, `forall(e in self.ts: e.n = 2 implies e.x >= 1 ms)` gives 2 options at each slot whose `n` is not 2. Bounded quantifiers are checked against `PartBudget` at check time; families are expanded at evaluation time with no limit. So a state with about 30 such elements exhausts memory or time instead of stopping. Fix: charge an evaluation-time budget and stop `Expansion` with a limit cause. | qsl-eval/src/simulation/model/timed.rs:1205-1256 |
| FND-003 | low | **The listing loop can fail to terminate.** `while places(listed + 1) <= room` never ends when every open entry has weight 0: an owner with no objects, or a declared maximum of 0 on another level of the path. `listed` then grows without bound. I could not build such a model through the test API, because an empty universe is refused at admission. Cap `listed` at `max_clocks`. | qsl-eval/src/simulation/model/timed.rs:490-501 |
| FND-004 | low | **`FAMILY_SLOT` is a sentinel value.** `ElementBinding::FAMILY_SLOT = u64::MAX` marks a template slot inside the public `slot: u64`, and `placed_at` and `at_slot` compare against it. An enum such as `Slot::Held(u64)` / `Slot::Family` would let the type system keep a template binding from reaching a reader that expects a real slot. | qsl-semantics/src/check/timed_model.rs:290-300; qsl-eval/src/simulation/model/timed.rs:1260-1290,1908-1915 |

## Verdict

**Not mergeable until FND-001 is fixed.** FND-001 is a silent truncation, exactly what the brief asked to rule out. FND-002 should be fixed in the same PR. FND-003 and FND-004 can ride along.
