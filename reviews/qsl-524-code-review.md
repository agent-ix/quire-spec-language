---
id: SR-12123
title: "Code review of quire-spec-language PR #953: qsl-eval automaton and fairness kernels"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b4d2e0400c3c706869fa42c7b8a6bd5e1087582e; PR #953: qsl-eval/src/automaton/*, qsl-eval/src/fairness/*, qsl-eval/src/lib.rs, qsl-eval/tests/it/automaton_kernels.rs, qsl-eval/tests/it/fairness_kernels.rs, qsl-eval/tests/it/support/automata.rs"
review_set: subset
---
# Code review of quire-spec-language PR #953

## Summary

Ticket: QSL-524. Code review with the rust-review lane folded in. Read-only
review of `git diff origin/main...HEAD`; no build or test run (the lane's full
`make ci` passed at this head).

Each kernel was checked by hand against its spec rule:

- `DeadlineCounter`, `ExtentCounter`, `EventuallyWindows`, `AlwaysOffsets`
  against ADR-018 IV-3's next-position expansion. Correct, including bound 0,
  open-and-hold at one position, window containment, adjacent-range merging and
  `u64::MAX` bounds (saturating arithmetic, no overflow).
- `PastAgeCounter` (`b + 2` values) and `ShiftCounter` (`a + 1` values) against
  IV-3 as amended in this PR. Correct for `once[0,b]`.
- `Monitor::bad_prefix` against SM-6: greatest-fixpoint trim to states with an
  infinite run (a satisfiable guard to a live target), then subset construction
  with the empty subset as the one rejecting state. Correct.
- `Numbering` against IV-6: checked arithmetic, refuses before inserting, keeps
  already numbered keys reachable at the limit.
- `components`: iterative Tarjan. Low links, on-stack test, and the
  parent-before-root-check order are correct. Results are sorted by least
  vertex, as amended FR-126/FR-130 require. `has_edge` counts self-loops.
- `analyze`/`classify` against FR-130's filter and ADR-019 FS-2 to FS-4: (a)
  then (b) reject, (c) refines by removing every state where a failing strong
  constraint is enabled. Checking weak before refining is sound, since a weak or
  acceptance failure carries into every sub-SCC.
- `check_loop` against FA-3 and SF-4. Correct; the stutter step belongs to no
  constraint.

No panics on edge inputs: indexing in `Walk` is guarded by `require_vertex`,
`Atom::mask` by the `< 64` check, and every counter uses saturating or checked
arithmetic. There is no compatibility layer, shim or re-export. The tests
assert typed fields (`Outcome` variants, `StateId`s, error variants, window
sets), never message text. Of the counter tests, four are exhaustive over every
trace of up to six positions against an independent oracle.

No `#[trace]` tags were added, and that is right. Every FR-126 and FR-130 AC is
an engine-level `Violated`/`Holds`/certificate outcome, and no kernel test fully
checks one.

## Verdict

Approve. Not blocking: one medium (complexity against FS-5) and two lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Strong-fairness refinement does not meet ADR-019 FS-5's `O(n + m')` per depth. Each `components` call allocates `Walk` vectors sized to the whole graph (`graph.vertex_count()`), and `classify` calls `analyze` once per failing component, so a depth with many small refined SCCs costs `O(#components × n)`. At FR-126's 10,000,000-state default that is quadratic. `classify` also scans every member and edge once per constraint, which is `O(k × (|S| + m_S))` per component and not `O(m')` with `k` the `each`-expanded identity count. Results stay correct. Fix: size the walk to `within`, using a local index map or one shared walk reset per depth, and compute per-constraint enabled/taken in one pass over members and edges. | qsl-eval/src/fairness/scc.rs:126; qsl-eval/src/fairness/refine.rs:317-349 |
| FND-002 | low | The `PastAgeCounter` doc says it serves `since[0,b]` and `triggered[0,b]`. `p since[0,b] q` also has to drop the age when `p` is false and `q` is not, and `step` takes only the `q` input, so a caller must know to rebuild the counter with `new`. Either take a `left_holds` input or narrow the doc to `once`/`historically`. | qsl-eval/src/automaton/counters.rs:164-210 |
| FND-003 | low | The test name `counters_report_the_value_counts_of_ir_5` cites a rule that does not exist. The rule is IV-5. | qsl-eval/tests/it/automaton_kernels.rs:514 |
