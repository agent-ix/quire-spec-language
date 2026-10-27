---
id: SR-1662001
title: "code-review and rust-review of PR #1662 (QSL-580l, collection-valued writes and Carried/Fresh provenance)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1662 git diff origin/qsl-580k-quantified-clocks...b802eb302 (own commit fa84abba2): qsl-semantics/src/check/timed_model.rs, qsl-semantics/src/check/timed_model/collection_write.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/tests/it/clock_places.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
---
# SR-1662001: code-review and rust-review of PR #1662 (QSL-580l, collection-valued writes and Carried/Fresh provenance)

## Summary

Ticket: QSL-580. The head is b802eb302; its own diff is unchanged from 8ee37baa2 (commit fa84abba2). I checked soundness first, with probes in the review worktree (qsl-semantics S3 and qsl-eval `TimedSystem::steps`), and reverted them.

Answers to the soundness questions:

- **Do carried elements keep their clocks across filter, flatten and reorder?** Only as a record. `Written` describes the carrying, but nothing applies it. Eval never executes a changing write. Probe: from `ts = [n2, n1]` after a delay of 3, `send` with `self.ts = filter(e in self.ts: e.n < 2)` has **0 steps**. So do `self.ts = sequence[M::Timer {...}]` and the filter combined with a selection reset. `self.ts = self.ts` and a plain selection reset still give 16 steps. `TimedSystem::steps` returns `Ok(vec![])`, not a refusal (FND-001).
- **Does a fresh element reset?** Yes in the record: `Fresh` holds the literal's constants, and the constants are cut from the data write. A literal that leaves out a clock component refuses `clock-write` (STD-493 item 22). As above, nothing executes it.
- **Is Set/Bag slot identity now element identity?** Not in code. SR-1655001 FND-004 is answered only by describing the provenance. No code maps clocks by element, because writes never execute; the slots stay canonical positions (FND-005).
- **Do eval moves match S3?** No. S3 accepts and records the write; eval drops the operation's step (FND-001).
- **Can a write lose or duplicate a clock place?** In the record, yes:
  - A `Set` literal's duplicate test is syntactic. `set[T{x: 0 ms, n: 1}, T{x: 5 ms, n: 1 + 0}]` records two `Fresh` occurrences for what the set holds as one element (FND-003).
  - Two writes of one field in one postcondition are both recorded (FND-004).

A further issue is in what is recorded at all. A postcondition reads unprimed `self.ts` in the **post**-state (FR-104), but `Written::Field` records the right-hand `self.ts` as carried from the **pre**-state. The correct spelling `filter(e in pre(self.ts): ...)` refuses `clock-write` (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A changing collection write is accepted at S3 but silently disables the operation in exploration: `TimedSystem::steps` returns no successor and no refusal. A checker over the model therefore treats `send` as never enabled, which can give a wrong `Holds` (a violation reachable only after `send`) or a false deadlock or time-lock. This is filed as STD-493 item 21 for record literals, and this PR widens it to every collection write. The timed subject should refuse an operation it cannot execute (a `TimedSubjectRefusal`, or `Undecided`) rather than drop its step. | timed_model.rs 1499-1523; qsl-eval simulation/model/timed.rs |
| FND-002 | medium | `self.ts = filter(e in self.ts: e.n > 1)`: per FR-104, the right-hand `self.ts` in a postcondition reads `post`, so the conjunct constrains the post-state to a fixpoint of the filter. `Written::Field` records it as carried from the pre-state. `pre(self.ts)`, which does read the pre-state, is refused `clock-write` (probe: `self.ts = pre(self.ts)` and `filter(e in pre(self.ts): ...)`). FR-230-AC-20 spells the conjunct without `pre`, so FR-230 and FR-104 conflict. File it on STD-493, and accept `pre(self.f)` as the carried source. | collection_write.rs 235-250; FR-104 "reads"; FR-230-AC-20 |
| FND-003 | medium | `equal_data` compares data components syntactically, and only leaf nodes. `set[T{x: 0 ms, n: 1}, T{x: 5 ms, n: 1 + 0}]` records two `Fresh` occurrences, while the set holds one element, so the record duplicates a clock place. Equality by data part is in general state-dependent (`n: self.k`). Either defer the "first occurrence" rule to the executor, or refuse a set literal whose components are not all literals. | collection_write.rs 183-200, 255-286 |
| FND-004 | low | Two writes of the same field in one postcondition (`self.ts = filter(...) and self.ts = self.ts`) are both recorded with no refusal. So is a write with a selection reset of the same field, and it is unspecified whether the reset selects pre- or post-write slots. Refuse a second write, and define the order of a write against a reset. | timed_model.rs 1499-1523 |
| FND-005 | low | `Written::Element` does not name its binder, so under nested traversals (`map(e in A: map(d in B: e))`) the record cannot say which element is carried. | collection_write.rs 159-164 |

## Verdict

Changes requested: FND-001, FND-002 and FND-003. FND-002 also needs a spec ruling.

## Coverage

- I read the whole own diff.
- S3 probes:
  - `pre(self.ts)` in both forms;
  - a set with a non-literal duplicate;
  - two writes of one field;
  - a write with a selection reset;
  - `map` to the binder;
  - a three-element set with a literal duplicate (correctly dropped).
- Eval probes: three changing writes (0 steps each), and an identity write and a reset (16 steps each, clocks correct).
- All probes were reverted with `git checkout --`. The `clock_places` tests pass.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The zone checker, the zone certificate checker and timed closure exploration all refuse a model that writes a clock collection, by name (`CarriedClocks`). That is an explicit, acceptable deferral. But the follow-up (mapping carried places in a DBM) is only listed in the PR body, and no ticket exists. File one, so that the refusal has an owner. | zone_check/search.rs 376; zone_certificate.rs 674; timed/explore.rs 83 |

## Dispositions

Round 1. The head is c3110bf0a, and its fix commit is c3110bf0a on top of 859dca6c3. Checked in a fresh worktree and target under locked-build at jobs 4:
- qsl-eval `timed` tests: 44 passed, plus my own reverted probe.
- qsl-semantics `clock_places`: 13/13 passed.
- clippy `-p qsl-semantics -p qsl-eval -p qsl-replay -p qsl-analyze --all-targets --all-features -D warnings`: exit 0.

I re-ran the round-0 eval probes as a two-step run: `send` resets slot 1's `x` at t = 3, then `reply` writes at t = 4. All probes were reverted afterwards.

| Write | Result |
| --- | --- |
| `filter(e in pre(self.ts): e.n > 1)` over a Sequence, a Set and a Bag | 16 steps each; slot 0 becomes `x = 1`, `y = 4` (slot 1's clocks moved) and slot 1 is gone |
| `map(e in pre(self.ts): e)`, `filter(... e.n > 0)`, `pre(self.ts)` | clocks unchanged per slot |
| Fresh `sequence[M::Timer{x: 2 ms, y: 5 ms, n: 3}]` | `x = 2`, `y = 5` |
| Set `[T{x: 0, n: 1}, T{x: 5, n: 1 + 0}]` | one slot, `x = 0` (the first occurrence) |
| Bare `filter(e in self.ts: ...)` and bare `self.ts = self.ts` | the expansion stops `unsupported_construct`/`collection-write-not-computed`. No successor is silently dropped. |

At S3, a second write of one field, and a write plus a selection reset in either order, refuse `conflicting-clock-write`. `map(e in pre(self.ts): e)` records `Element("e")`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Frame exploration computes each write from the pre-state and confirms that the value is a function of it (computed again over the written state). Each post slot's provenance is recorded by value. The timed subject applies `GuardedStep::carries` before resets, and gives fresh slots their literal constants. A write that cannot be computed stops the expansion by name. Zone analyses refuse `CarriedClocks` (see FND-006). |
| FND-002 | fixed | `pre(self.f)` is the carried source (`Written::Field { pre: true }`). A bare `self.f` is recorded with `pre: false` and refused at execution. Filed as STD-493 item 28 (bab3d3c9, read back). |
| FND-003 | fixed | The checker records every occurrence. Duplicate occurrences are resolved at execution by value: for a Set or OrderedSet, the k-th equal slot takes the k-th equal occurrence, so the first occurrence's clocks win. The `1 + 0` probe gives one slot at `x = 0`. |
| FND-004 | fixed | A second write of one field, or a write plus a reset of one of its clocks, refuses `conflicting-clock-write`. |
| FND-005 | fixed | `Written::Element(binder)`. |
