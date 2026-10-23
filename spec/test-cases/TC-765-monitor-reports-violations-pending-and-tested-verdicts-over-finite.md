---
id: TC-765
title: "monitor reports violations, pending and tested verdicts over finite traces and lassos"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: verifies
---
# TC-765: monitor reports violations, pending and tested verdicts over finite traces and lassos

## Description

Verify offline `monitor` verdicts under ADR-014 A-4.

Scope: FR-283-AC-1, FR-283-AC-2, FR-283-AC-5, FR-283-AC-7, FR-283-AC-9.

Each trace is a `quire.monitor-trace/1` document whose positions are snapshot `DocumentRef`s in the request's `snapshots` provision: a finite trace has `prefix` only, and a lasso has `prefix` and `loop`.

## Test Procedure

1. Over a three-position finite trace whose `p` is false at position 2, monitor `always p`.
2. Monitor `always p` over a three-position trace whose `p` is true at every position.
3. Over a lasso of a two-position prefix and a two-position loop whose `q` holds at loop position 3, monitor `always eventually q`.
4. Repeat step 3 with `q` false at every loop position.
5. Over a three-position finite trace whose `x` is 0 at position 1, monitor `always (10 / x > 0)` and map the outcome's category through FR-285.
6. Monitor step 1's clause and step 5's clause in one request over step 5's trace.
7. Over FR-327-AC-1's closed `Counter` trace as a three-position finite document, monitor `eventually[0,2] holds(c.value = 2)` and `eventually[0,1] holds(c.value = 2)` under event-position false-extension, `on origin`.

Tag the tests `#[trace("TC-765", "<AC id>")]`.

## Expected Results

- Step 1: one `MonitorRecord` naming `always p`, `Violated` at position 2, category violation, whose `counterexample` has `prefix` the three positions' digests, an empty `loop` and `trace_position` 2.
- Step 2: pending, category inconclusive, exit 0.
- Step 3: `tested`, category success.
- Step 4: a violation, category violation.
- Step 5: a violation at position 1, category violation, cause `UndefinedEvaluation` with `division-by-zero`; no `undefined` label; exit 10.
- Step 6: two records, in request order.
- Step 7: `Tested`, category success, exit 0; then `Violated` at position 0.
