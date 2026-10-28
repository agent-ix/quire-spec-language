---
id: SR-1733001
title: "Code review of quire-spec-language PR #1733: a Clock payload in a union case (QSL-580q part 2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@aec5be25f; PR #1733 diff against qsl-580q-clock-places (c1387e815), 11 files, against FR-230 (Guards, Presence bit, Records/tuples/unions, AC-12, AC-41), EN-6, FR-360 item 9, FR-106"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1733

## Summary

Ticket: QSL-580 (580q part 2).

**Scope.** The diff is two commits on c1387e815, the #1721 head I passed: 11 files. Nothing outside the stack is in it.

**The soundness checklist I was given:**

- **The place is live only while the field's member is that case.**
  - `live` and `live_clocks` walk `PlaceStep::Union(case)` through `is_case` on the union member, and the place holds no bit.
  - Probes: an `Armed` start is live and an `Idle` start is not. With two clock cases (`Armed`, `Waiting`), only the held case is live.
- **The constructor reset selects the case; any other case drops it.**
  - `union_write` resets the written case to `c` and adds every other clock case to `drops`.
  - Probe, switching cases: from `Waiting`, `self.state = Armed(0 ms)` resets `Armed` to 0, and `Waiting` stops being live.
  - Probe, same case: from `Armed` at 1 ms, `Armed(2 ms)` resets the clock to 2.
  - Probe, frame-only write: a frame naming `state` with no postcondition gives two successors. In one `state` stays `Armed` with its clock kept; in the other it becomes `Idle` and the place is dropped.
  - A case entered by any route gets `steps`' newly-live 0 reset.
- **The state key distinguishes cases.** It goes through the union member. The PR test asserts that `Idle` and `Armed` keys differ.
- **The zone frees the dead case's dimension.** That is #1721's `insert` free, sound for the same reason: every place that becomes live is reset or carried.
- **`value()` without presence refuses at the read.** `value(self.state.Armed) >= 1 ms` with no presence refuses `UnprovedPresence` at 629..660.
- **Whole reads of a clock-bearing union refuse**, as FR-230 "Records, tuples and unions" requires. Probes: `self.state = M::Mode::Idle` or `= M::Mode::Armed` in a guard, and `self.state = pre(self.state)` in a postcondition, each refuse `clock-read-outside-constraint`.
- **A time invariant on the case.** `when { present(self.state.Armed) } { value(…) <= 3 ms }` bounds `Armed` only while it holds: 1 bound from `Armed`, 0 from `Idle`.
- **Two writes of the field in one postcondition: unsound.** See FND-001.

**The snapshot form.** The coder admits FR-360 item 9's `{"variant": {"type", "case"}}` into a union slot, for a case with no data payload. I judge this a reasonable interim reading, but it is a spec gap, not something the spec states:
- FR-360 item 9 covers a *variant slot*, and QSpec FR-208 defines a variant type as cases with no payload.
- FR-106's grammar has no union form.
- FR-230-AC-41 says a case's presence is read "from the snapshot's `member`".

So no spec text gives a union-typed slot's snapshot form, with or without a data payload. See FND-002, for STD-493.

**Gates** (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 48373530):
- qsl-semantics and qsl-eval: all binaries pass, including semantics it 1335, lib 804 and eval it 811.
- qsl-analyze and qsl-replay `zone`/`timed`/`refinement`/`clock`: 12 + 210 + 49 + 71, all pass.
- Workspace clippy `-D warnings`: exit 0.

The probes are in `probe_1733.diff`, not committed.

## Verdict

**Changes requested.** One high soundness defect (FND-001), the same class as #1721's FND-001, now through the union write path. FND-002 is a spec gap for STD-493, and FND-003 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A postcondition that writes a clock-bearing union field twice is not refused. `union_write` keeps no record of places already written, unlike #1721's `option_writes`. So `self.state = M::Mode::Armed(0 ms) and self.state = M::Mode::Armed(1 ms)` checks, and from `Idle` it gives one successor with resets `[(Armed, 0), (Armed, 1)]` and the clock at 1. That postcondition has no satisfying post-state (the case's clock cannot be both 0 and 1), so this is a wrong successor: an invented behaviour for safety, and a wrong enabled step for deadlock freedom. A case switch plus a reset (`Armed(0 ms) and Idle`) checks too. It has no successor only because the data conjuncts contradict, where #1721 refuses a reset plus a drop of one place with `conflicting-clock-write`. Track the places each union write resets or drops (sharing #1721's set), refuse any second write of the field `conflicting-clock-write` at the second write, and add a test. | qsl-semantics/src/check/timed_model.rs:2087-2100 (`resets`), 2229-2290 (`union_write`); FR-230 "Presence bit", "Records, tuples and unions" |
| FND-002 | medium | Spec gap, for STD-493: no spec text gives the snapshot (FR-106) form of a value in a union slot. FR-360 item 9's `variant` form is defined for a payload-free variant type (QSpec FR-208), and FR-106's grammar has no union or payload form. Yet FR-230-AC-41 reads a union case's presence "from the snapshot's `member`". The PR admits `{"variant": {"type", "case"}}` into a union slot for a case with no data payload. That is reasonable, but it is an unstated extension, and a case with a data payload has no admissible form at all. Ask the spec owner for the union snapshot form (with and without data payload, and a `Clock`-payload case), and cite the ruling in `admit_union_case`. | qsl-semantics/src/model/observation/document.rs:1765 (`admit_union_case`); QSL spec FR-106, FR-360 item 9; FR-230-AC-41 |
| FND-003 | low | The interim `clock_place` argument writes a union step as `{"type":"union","case":…}`, an encoding that no spec gives (the place-id form waits on the selector ruling). It is internal to transition text today. Make sure the selector ruling covers the union step, so the interim text is not frozen into a format. | qsl-eval/src/simulation/model/timed.rs (`place_step_enc`); FR-230 "Checked package" (`clock_place` path) |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The workspace clippy gate fails at 742127c84: `cargo clippy --workspace --all-targets --all-features -- -D warnings` stops with "function `union_clock_document` is never used" (qsl-eval/tests/it/timed_subject.rs:4743). The round-1 refactor routes every caller through `union_clock_document_with`, and the doc links that still name the old function are not uses. The tests pass, but the gate is red. Delete the wrapper and repoint its doc links, or call it. | qsl-eval/tests/it/timed_subject.rs:4743-4796 |

## Dispositions

Round 1, fix commit 742127c84.

Gates (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 48373530):
- qsl-semantics and qsl-eval tests: all pass (semantics it 1336, eval it 812).
- qsl-analyze and qsl-replay `zone`/`timed`/`refinement`/`clock`: 12 + 210 + 49 + 71, all pass.
- Workspace clippy `-D warnings`: **fails** (FND-004).

I re-ran the probe on the head; it is in `probe_1733_r1.diff`, not committed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Each union write now enters every clock case it resets or drops into the `option_writes` set it shares with `Option<Clock>` places. A second write touching any of them refuses `conflicting-clock-write` at the second conjunct. Re-run probes: `Armed(0 ms) and Armed(1 ms)`, `Armed(0 ms) and Idle`, `Idle and Armed(0 ms)`, and (with two clock cases) `Armed(0 ms) and Waiting(0 ms)` each refuse `ConflictingClockWrite` at the second write's span. A single write still behaves as reviewed: a switch, a rewrite of the same case, and frame-only havoc. |
| FND-002 | fixed | Filed on STD-493 (comment 747158c5, item 1): the union snapshot form, with and without a data payload, and for a `Clock` case. `admit_union_case` cites it and states that a case with a data payload is not admitted. The spec gap stays open on STD-493. |
| FND-003 | fixed | STD-526 (QSpec 7d8d2090) rules the `clock_place` path as ordered `field(name)`, `tuple(index)`, `union(case)` and `slot(…)` entries, so the union step has a spec encoding (`union(case)`). The eval-side argument text `{"type":"union","case":…}` is internal, and matches that step's name and member. When `clock_place` emission lands, derive the argument from the node path. |
