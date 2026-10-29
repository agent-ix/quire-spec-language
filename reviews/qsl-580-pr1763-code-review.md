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

## Dispositions

Round 1, head 07cfab04e. The fix is commit 07cfab04e; the other commits after bf657815b are merges of main and of the base branches.

Checked against QSpec 161d231d: FR-415-AC-21, and FP-01..FP-14 through `the_clock_presence_frame_vectors_step_as_published`, which passes.

Gates (`--all-features`, debug info off):

| Suite | Result |
| --- | --- |
| qsl-eval it `timed_subject` | 71 |
| qsl-eval lib | pass |
| qsl-semantics it `clock_places timed` | 104 |
| qsl-semantics lib | 812 |
| qsl-package `clock_places` | 5 (1 ignored, IR-1083) |
| clippy (3 packages) and fmt | exit 0 |

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `slot_bits_follow` now takes only the writes (`written` plus the postcondition's resets) and leaves out the fresh-element zeros that `matched` enumerates. A slot held at post but not at pre and not written must be absent. With `!after` mutated to `true`, `an_option_element_keeps_its_bit_without_a_write` and `a_dropped_element_regrows_absent` both fail. |
| FND-002 | fixed | A written slot now requires `after`. With that branch mutated to `true`, `a_reset_over_option_elements_makes_each_present` fails. |
| FND-003 | fixed | In a precondition guard, a state invariant and a time invariant's `when`, the bodies `not present(e)`, `present(e) and value(e) >= 1 ms`, `present(e) or present(e)` and `x >= 1 ms and exists(..not present(e))` all refuse `unsupported_construct`/`not-yet-implemented`. The whole-body `present(e)`, its `not forall`/`not exists` forms and its `when` forms still check. Residual, low and non-blocking: in a postcondition, `exists(e in self.q : not present(e))` refuses `invalid_timed_model`/`clock-write`, and a requirement is no write (FR-415-AC-21). This fails closed, so it is a wrong cause only. |

## Round 2 (delta review)

Head 92831e891, reviewed as a delta from 07cfab04e. It carries QSL-1209 in two commits: 8136e9ff0 (the spec edit and the frame-vector test) and 92831e891 (a clippy type alias). There is no source change.

**Spec edit.** I compared the FR-230 "No implicit reset" paragraph, FR-230-AC-52 and TC-685 step 35 against QSpec 161d231d: FR-415 "No implicit reset on a presence change" and "The same rule for absence", FR-415-AC-21, TC-367 CK-31, and `clock-presence-frame-vectors.json`.
- **Faithful:**
  - A place becomes present only by a write.
  - An unwritten `Option` bit never changes.
  - A union may leave its clock case unwritten but never enter it.
  - A requirement is never a write.
  - `clock-presence-unwritten` applies when the precondition establishes the place absent.
  - The pairs FP-05/FP-06 and FP-13/FP-14 differ, and every other pair agrees.
- **Derived, not extra:**
  - An element a frame adds to a `Sequence<Option<Clock>>` unwritten is absent. This is QSpec's one rule applied to slots.
  - The `Clock`-parameter reset is FR-230's existing write.
- **Two low gaps:** FND-004 and FND-005 below.

**Validation.** `make check-spec-validation` exits 0 and gives no diagnostic on FR-230 or TC-685.

**FP vectors.** `the_clock_presence_frame_vectors_step_as_published` now runs all 14 vectors from the file, asserting `ran == 14`:
- FP-11 and FP-12 run through S3, with a precondition `not present(..)`. Each asserts its single refusal's `invalid_timed_model`/`clock-presence-unwritten` against the vector.
- CK-31's `pair.outcome` (`same`/`differs`) is asserted over the observed results: successors, or the refusal.

It passes with QSPEC_DIR set to 161d231d.

**Gates** (`--all-features`, debug info off):

| Suite | Result |
| --- | --- |
| qsl-eval it `timed_subject` | 71 |
| qsl-eval clippy `-D warnings` | exit 0 |
| fmt | exit 0 |

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | open | The residual of round 1 is not fixed: the delta changes no source. A postcondition `exists(e in self.q : not present(e))` still refuses `invalid_timed_model`/`clock-write`, and a requirement is no write (FR-415-AC-21). It fails closed, so it is low and non-blocking. |

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-230 "No implicit reset" refuses `clock-presence-unwritten` only "where the operation's precondition establishes the place absent". QSpec FR-415 says "the operation's precondition or an invariant", "at the `present(...)` requirement, naming the place". The invariant case and the naming of the place are dropped. | QSpec FR-415 "No implicit reset on a presence change", FR-415-AC-21 |
| FND-005 | low | "One that nothing writes leaves no successor from a state that contradicts it" overstates the rule for a union. QSpec gives `not present(self.st.Armed)` of an `Armed` predecessor the `Idle` successor (FP-14), and only the `Option` form has none (FP-13). The sentence should state the `Option`/union split as FR-415 "The same rule for absence" does. | QSpec FR-415 "The same rule for absence", TC-367 CK-31 (FP-13/FP-14) |

### Round 2 addendum: head f8b9481a0

Commit f8b9481a0 adds the fix for the FND-003 residual. Before reading resets, `resets` now refuses through `located.outside(query)` any quantifier that `Located::unread` marks. Its test case is the postcondition row in `another_quantified_presence_body_is_not_yet_implemented`.
- The postcondition `exists(e in self.q : not present(e))` now refuses `unsupported_construct`/`not-yet-implemented` at the quantifier, as it does in guards and invariants.
- With the early return removed as a mutant, the test fails with exactly the old `clock-write`.

Gates at f8b9481a0:

| Suite | Result |
| --- | --- |
| qsl-semantics it `clock_places timed` | 104 |
| qsl-semantics clippy `-D warnings` | exit 0 |
| fmt | exit 0 |

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | Its postcondition residual is fixed at f8b9481a0, and the mutant is killed. |

FND-004 and FND-005, the low spec-text gaps in FR-230, stay open. Neither blocks the merge.
