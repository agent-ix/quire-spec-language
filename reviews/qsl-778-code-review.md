---
id: SR-12116
title: "code-review of quire-spec-language PR 950 (TC-830 step 4 full-size supplied Tree budgets)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c4e5206fb5f3c0f5df3fbe5081e7a67a592963ef; qsl-replay/src/execute/tests/supplied.rs"
review_set: subset
evaluated_revision: "c4e5206fb5f3c0f5df3fbe5081e7a67a592963ef"
---

## Summary

Ticket: QSL-778. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (tests only, +142 in
`qsl-replay/src/execute/tests/supplied.rs`). Read-only; no build was run. The
leader's gate on this head (check, clippy -D warnings, qsl-replay supplied
tests) was taken as reported.

- The admission oracle is tight. A-1 denying and A admitting pins the total at
  exactly A, so an implementation that adds or drops one event per level
  (89,995 or the old 69,997) fails. The sampled denials (first level and one
  more event, a middle level, the leaf) check the typed next event, the
  unspent step, the setting name, the configured bound, next charge 1 and the
  attempted count.
- `tree_admission_events` matches QSV `admit_occurrence` at the locked rev
  74bd0afc: LIFO worklist, union payloads scheduled last to first, one
  `same_type_walk` link for `Option<Tree>` (Composite/Composite equal on the
  first link), one scheduling for the present payload, an unbounded Integer
  with no numeric helper.
- `pre_call` asserts zero replay charges and no kernel `Incomplete`, which
  covers "zero evaluation consumption" for both denials.
- `WitnessValue` has an iterative `Drop`, so the 10,000-deep witness built per
  request does not recurse on drop. No `#[ignore]`, no `#[allow]`, no
  legacy comments. The rename to `weigh` is needed: `size` is an S1 keyword
  (`qsl-cst/src/token.rs`).
- Raising `replay.input_bytes` does not weaken the claim. FR-321-AC-5 and
  TC-830 step 4 run "with occurrence and other bounds permitting progress",
  and the default reader bound is FR-255's own row, tested elsewhere.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The occurrence-bound denial does not assert the setting or the successful spend. TC-830 step 4 Expected Results says a denial names "the phase, parameter, typed event, setting, configured bound, successful spend and denied amount". The 29,998 denial checks phase, parameter, event, kind, limit and next charge, but not `limit.field()` or `stop.context.consumed`/`limit.counter`. An implementation that reports the wrong setting name or a wrong spend for the occurrence stop passes. The admission-denial loop already checks these fields. | qsl-replay/src/execute/tests/supplied.rs:793-802 |

## Verdict

PASS, with one medium recorded. The admission oracle fails on a plausible
wrong implementation in both directions, the typed counters are asserted, and
the reader-bound raise matches what the TC permits. The occurrence denial
misses two of the fields the TC requires a denial to name (FND-001). Adding
two or three asserts fixes it.
