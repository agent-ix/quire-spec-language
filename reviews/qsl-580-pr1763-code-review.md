---
id: SR-1763001
title: "code review (with rust-review) of PR #1763 (QSL-580v): per-slot Option<Clock> bits and PR-03 quantified presence"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1763 git diff origin/qsl-580t-general-presence...bf657815b (17 files)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1763001: code review of PR #1763 (QSL-580v)

## Summary

Head bf657815b, base qsl-580t-general-presence (#1754). Checked against FR-230-AC-41/42/46/49..51, STD-526 (presence places `v` and `q`, PR-03) and STD-542 FR-415-AC-21.

**What holds.** I probed these in eval over `q: Sequence<Option<Clock>>[0, 2]`, then reverted the probes.
- The bits are a hidden `Sequence<Boolean>[min, max]`, and its length is the count.
- The 7 initial states cover every combination of count and bit.
- The state keys are distinct, and each element writes its own option shell.
- `forall` is true on an empty `q`, and `exists` is false on it.
- A slot clock is live only while its bit is set.
- Under a frame havoc with no write, every element held before and after keeps its bit by slot index. Removal is from the end, like the clocks: "same path and slot keeps its value".
- No source form writes a collection value of `q` yet. `self.q[0]` is not parsed (`Postfix`), and `self.q = sequence[]` is ill-typed. So a carried element moving to a new slot cannot arise yet.
- Unbounded `q`, `Bag` and `OrderedSet` refuse `TimedNotChecked`. That is acceptable: they fail closed, and the bits of an unbounded sequence cannot be enumerated.
- The guard forms `not forall(..)` and `forall(..) or self.x >= 1 ms` are admitted.
- In a postcondition, `forall(e in self.q : present(e))` refuses `not-yet-implemented`.

**FND-001 (medium): an added element becomes present without a write.**
- Probe: `work` with `q` in its frame and no postcondition. `[] -> [true]`, `[false] -> [false, true]` and `[true] -> [true, true]` are all successors.
- In each, the new element is present with clock 0. That is an implicit reset.
- FR-415-AC-21: "a successor in which a place is present although it was absent in its predecessor exists only when the postcondition writes that place's clock value."
- An element the step adds unwritten should be absent. The PR's own test `an_option_element_keeps_its_bit_without_a_write` asserts "an added element takes either bit".

**FND-002 (medium): the reset `forall(e in self.q : e = 0 ms)` leaves an absent element either present or absent.**
- Probe: `[false] -> {[false], [true]}`, and `[false, false]` reaches every bit pattern.
- FR-230 says "a write that makes the place present ... sets the presence". So a reset slot is present afterwards.
- The cause is `slot_bits_follow`'s `before == after || (after && reset)`, which allows `after == before == false` for a reset slot. A reset slot should require `after`.

**FND-003 (low): other quantifier bodies refuse with the wrong cause.**
- A body other than exactly `present(e)` refuses `invalid_timed_model`/`clock-read-outside-constraint`. Examples: `exists(e in self.q : not present(e))` and `forall(e in self.q : present(e) and value(e) >= 1 ms)`.
- These read no clock outside a constraint. FR-415 "Reading" defines `forall(e in q: B)` at each live slot.
- They should check, or refuse `not-yet-implemented`.

**Rust.**
- `PlacePresences` is now a struct, and the `slot` flag is threaded through `presence_places`, `place_presence` and `slot_presence`. That is clean.
- `bit_path` gives one source for which paths hold a bit.

**Gates** at bf657815b (`--all-features`, debug info off, QSPEC_DIR = a `git archive` of QSpec origin/main 161d231d):

| Suite | Result |
| --- | --- |
| qsl-eval `timed_subject` | 67 passed. The frame-vectors test failed only because the archive vanished mid-run; it passes 1/1 on re-run. |
| qsl-semantics it `clock_places timed` | 102/102 |
| qsl-semantics lib | 806 passed. The 5 `node_key` QSpec-vector tests failed for the same vanished archive and pass 32/32 on re-run. |
| qsl-package `clock_places` | 5/5 on re-run |
| qsl-eval lib | 140/140 |
| clippy (3 packages) and fmt | exit 0 |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An element a frame havoc adds unwritten may be present (clock 0): `[] -> [true]`, `[false] -> [false, true]`; FR-415-AC-21 allows a newly present place only with a write | FR-415-AC-21, FR-230 Presence bit |
| FND-002 | medium | `forall(e in self.q : e = 0 ms)` leaves an absent element present or absent (`[false] -> {[false], [true]}`); a write makes the place present, so a reset slot must be present | FR-230 Presence bit, FR-415 Reset by selection |
| FND-003 | low | A quantifier body other than `present(e)` (`not present(e)`, `present(e) and value(e) >= 1 ms`) refuses `clock-read-outside-constraint`; it reads no clock outside a constraint | FR-415 Reading, FR-230-AC-51 |

## Verdict

Changes requested (FND-001, FND-002).
