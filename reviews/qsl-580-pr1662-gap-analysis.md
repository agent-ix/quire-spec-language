---
id: SR-1662002
title: "gap-analysis of PR #1662 (QSL-580l) against FR-230-AC-20 and QSpec FR-181-AC-25's record part"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1662 git diff origin/qsl-580k-quantified-clocks...b802eb302, against FR-230 Collection-valued writes, FR-230-AC-20, QSpec FR-181-AC-25 (record part) and SR-1655001 FND-004"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# SR-1662002: gap-analysis of PR #1662 (QSL-580l) against FR-230-AC-20 and QSpec FR-181-AC-25's record part

## Summary

Plan completion: not assessed.

FR-230-AC-20 coverage, all at S3:
- **Covered:** filter carried; fresh sequence literal; set first-occurrence; and the `clock-write` refusals for a clock-reading component, a `let`, the frame and an invariant.
- **Not covered:** `flatten(...)`, typed `AmbiguousLiteral`, and the `self.ts[0].x` component, which waits on STD-493 item 24.

QSpec FR-181-AC-25's record part (execution of carried and fresh clocks) is **not implemented**. Writes never execute (STD-493 item 21), and eval drops the step instead (SR-1662001 FND-001). SR-1655001 FND-004 (Set/Bag slot identity) was deferred to this PR. Here it is answered only by description: no code maps clocks by element.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSpec FR-181-AC-25's record part ("a step that filters out the element at slot 0 moves the clock of the element at slot 1 to slot 0 with its value"; a fresh front insert takes the literal's constant), is in the PR's scope but has no implementation and no test. The only reason given is STD-493 item 21. Either implement the executor for bounded record collections, or mark the AC part deferred on a ticket. Do not report it as covered. | QSpec FR-181-AC-25; STD-493 item 21 |
| FND-002 | medium | SR-1655001 FND-004 is not resolved. Slots of `Set`/`Bag`/`OrderedSet` remain canonical positions in eval. The PR body's resolution is a design note, not code. Keep FND-004 open, tied to whichever ticket executes collection writes. | SR-1655001 FND-004 |
| FND-003 | low | AC-20's `flatten(sequence[sequence[..], self.ts])` is not reachable through typing (`AmbiguousLiteral`). This is a typer gap, not filed on STD-493 or ticketed. | FR-230-AC-20 |
| FND-004 | low | The FR-104 post-read conflict (SR-1662001 FND-002) is a spec gap: FR-230-AC-20's `self.ts = filter(e in self.ts: ...)` reads `post` under FR-104. File it on STD-493. | FR-230-AC-20; FR-104 |

## Verdict

Changes requested on FND-001 and FND-002: record them as deferred on a ticket, or implement them. They are not covered.

## Coverage

- I re-read FR-230 "Collection-valued writes", FR-230-AC-20, FR-104 on `pre`/`post` reads, and STD-493 items 21 and 22.
- The PR body claims were treated as data and re-measured with the probes in SR-1662001.
