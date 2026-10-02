---
id: TC-704
title: "Task automata lower to timed subjects for deadline checking, and the stopwatch class is unsupported"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-249
    type: verifies
---
# TC-704: Task automata lower to timed subjects for deadline checking, and the stopwatch class is unsupported

## Description

Verify the task-automaton lowering for the decidable class and the `StopwatchRequired` disposition.

Scope: FR-249-AC-1 to FR-249-AC-2.

## Test Procedure

Fixtures: the `sample`/`command` model over `Ctl`'s first two tasks and its variants of FR-249-AC-1 and AC-2.

1. Check the model, then the variant with `a`'s WCET 4, and replay its counterexample.
2. Check the interval-WCET, completion-feedback, preemptive variant, then the same with a single WCET.

Tag the tests `#[trace("TC-704", "FR-249-AC-n")]`.

## Expected Results

- Step 1: `Holds`; `Violated` with a counterexample that replays.
- Step 2: `Unsupported(StopwatchRequired)`; lowered and checked.
