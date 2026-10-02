---
id: TC-657
title: "Scheduler fairness is derived per thread and turned off by scheduling adversarial"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: verifies
---
# TC-657: Scheduler fairness is derived per thread and turned off by scheduling adversarial

## Description

Verify protocol fairness targets and classes, the derived scheduler constraints with their origin, the adversarial opt-out, and that safety verdicts do not change.

Scope: FR-212-AC-1 to FR-212-AC-4.

## Test Procedure

Use FR-212-AC-1's `Chatter` protocol.

1. Check `eventually holds(k.flag = 1)` under infinite-trace and read the
   fairness set.
2. Add `scheduling adversarial`; compare identities; check the same claim
   and read the lasso and fairness set.
3. Check constraints naming `s`, `Done` and `Missing`, and two `scheduling`
   members; read the root's and `left`'s classes over ADR-027 §7.
4. Check `always holds(k.flag <= 1)` with and without the member.

Tag the tests `#[trace("TC-657", "FR-212-AC-n")]`.

## Expected Results

- Step 1: `proved`; three `scheduler` constraints over `s`, `r` and the
  root.
- Step 2: different identities; `refuted` with stem `fork(Both)`,
  `send(Tx)` and loop `duplicate`, `lose`; empty fairness set.
- Step 3: the two targets resolve; `missing_declaration`/`missing-name`;
  `ambiguous_declaration`/`ambiguous-name`; the classes of
  FR-212-AC-3.
- Step 4: `proved` both times over equal state sets.
