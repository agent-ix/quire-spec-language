---
id: SR-12129
title: "Code review of quire-spec-language PR #957: property automata built in the automaton module"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7d09f41a8bf82b4bcad4302594f72656b2c44f19; PR #957: qsl-eval/src/automaton/{counters,dnf,error,formula,letter,mod,past,translate}.rs, qsl-eval/tests/it/{main,property_automata,temporal_trace}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #957

## Summary

Ticket: QSL-527. Code review with the rust-review lane
(agent-skills/rust-review/SKILL.md) folded in. Read-only, static review of
`git diff origin/main...HEAD`. No cargo or make run (disk constraint); the
lead's `make ci` receipt is the gate.

What was checked by hand:

- `formula.rs` lowering into negation normal form: both polarities per node,
  `implies` as `or`, `historically`/`triggered` as negated `once`/`since`,
  constant folding, and interning. Correct.
- `translate.rs` tableau: unbounded `eventually`/`until`/`always`/`release`
  expansion, the `exclude` determinizing step, and the acceptance set per
  unbounded eventuality core. Correct for lower bound 0.
- IV-3 counters: the deadline counter for `eventually`/`until` and the extent
  counter for `always`/`release`, including temporal operands. Merging into
  one counter is sound there too, because a claim at position `j` discharges
  every obligation opened at or before `j`. The descending-node counter order
  is also correct, since an operator only opens its operands' operators.
- `past.rs`: the stationary false history before origin (it matches FR-338's
  `historically[0,1]` example), the `since` reset when the guard breaks, and
  the lower-bound shift as an age-range set. Correct.
- Closed-window lower bounds (a set of delays per core): correct.
  `always[a,*]` keeps the earliest start: correct.
  `until[a,*]`/`release[a,*]` keep a set: correct.
  `eventually[a,*]` keeps only the latest start: **wrong** (FND-001).
- Test oracle: every battery compares the automaton with the layer-5
  `evaluate` on every word up to 3-6 positions: closed traces with the false
  extension, and lassos with every loop start. The comparison is an exact GBA
  lasso-acceptance check, and each battery asserts that both verdicts occur.
  The batteries are not vacuous. They miss FND-001 because no liveness case
  puts `always[a,*]` with `a >= 2` under an `eventually`.

Rust idioms: errors are `thiserror` with typed variants; no `unwrap`/index
panics in library code; counters use checked or saturating arithmetic; no
`unsafe`; no compat layer, shim or re-export.

The coder's open choices are judged against the spec in SR-12130. Short form:

- The `live` letter atom for bounded `on each` is not a defect. It matches
  FR-338's terminal false-extension rule.
- Refusing the timestamped profile is not a defect, because timed deadlines
  are FR-196/FR-204's separate relation.
- Refusing past-over-future, the missing `AutomatonStateKey` octets, and the
  eager build are spec gaps. They are recorded there.

## Verdict

Request changes: one high (FND-001). FND-002 to FND-004 are lows, recorded
only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `add_shift` keeps only the latest pending start of an `eventually[a,*]` core. When a new start arrives at least every `a-1` positions, the pending delay is reset before it reaches 0, so the core is never activated. The acceptance set looks only at `must`, so that run is accepting without ever meeting the eventuality. Hand trace: original `eventually always[2,*] holds(p)` is liveness, and its target is `always eventually[2,*] not p`. Every position re-adds delay 1, and `max(0,1)=1`, so the config is a self-loop on every letter. No state ever holds the core, so no acceptance set is added and the negation automaton accepts every word. EN-1 would refute the clause on a model where `p` always holds (a false refutation). The same happens to `always (p implies eventually always[2,*] q)`. Fix: keep the delays as a set, as the `_` arm does, or otherwise guarantee activation. Add a liveness battery case with `always[a,*]`, `a >= 2`, under `eventually`. | qsl-eval/src/automaton/translate.rs:760-782; qsl-eval/src/automaton/translate.rs:303-315; qsl-eval/tests/it/property_automata.rs:404-429 |
| FND-002 | low | Two safety-fragment classifiers implement one rule: `translate::is_safety` over the lowered arena and `temporal::plan::is_safety` over the checked formula. If they drift, the automaton kind and the evaluator's settlement will disagree. Use one source. | qsl-eval/src/automaton/translate.rs:175-192; qsl-eval/src/temporal/plan.rs:262-300 |
| FND-003 | low | No budget bounds the per-state expansion. `memories` takes the cross product of every past operator's cases, and `Dnf::negate` multiplies literal sets. Both are exponential in the number of past operators or literals before `max_automaton_states` counts a single state. | qsl-eval/src/automaton/translate.rs:452-471; qsl-eval/src/automaton/dnf.rs:54-59 |
| FND-004 | low | `a_bounded_monitor_is_deterministic_and_finite` claims "reading the same word always reaches the same state" but checks only that 8 empty letters reject and that the state count is at most 6. Also, `assert!(translation_of(body).is_ok())` repeats a translation the line before it already did. | qsl-eval/tests/it/property_automata.rs:505-521; qsl-eval/tests/it/property_automata.rs:392 |

## Dispositions

Round 2, reviewed at `f1a81ef3f4c256365e64d15bdcfb1eaf9dbdab32` (fix commit `58994f11e`; `git diff 7d09f41a8..origin/property-automata -- qsl-eval`, main merges excluded).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 58994f11e: `add_shift` keeps every delay except for `always[a,*]` (earliest). New liveness cases `eventually always[2,*] holds(p)`, `eventually always[3,*] holds(p)` and `always (holds(p) implies eventually always[2,*] holds(q))` fail on the old code: the old negation automaton accepts the all-`p` lasso, where the evaluator holds. |
| FND-002 | accepted-no-change | Low, recorded only under the one-reviewer, two-round cap. The two classifiers agree today. |
| FND-003 | accepted-no-change | Low, recorded only under the review cap. There is no wrong result, only cost. |
| FND-004 | fixed | 58994f11e: the test now reads one word twice and compares the states, checks a non-rejecting waiting state, and the redundant `translation_of(..).is_ok()` is removed. |
