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

Scope: FR-283-AC-1, FR-283-AC-2, FR-283-AC-5, FR-283-AC-7, FR-283-AC-9, FR-283-AC-10, FR-283-AC-11, FR-283-AC-14, FR-283-AC-15.

Unless a step names another shape, each trace is a `Document` trace: a `quire.monitor-trace/1` document whose positions are snapshot `DocumentRef`s in the request's `snapshots` provision; a finite trace has `prefix` only, and a lasso has `prefix` and `loop`. Step 9 uses a `Samples` trace and step 10 a `Typed` `Timestamped` trace. Every request's `sources` holds the unit's source.

## Test Procedure

1. Over a three-position finite trace whose `p` is true at positions 0 and 1 and false at position 2, monitor `always p` with activation `on origin`.
2. Monitor `always p` over a three-position trace whose `p` is true at every position.
3. Over a lasso of a two-position prefix and a two-position loop whose `q` holds at loop position 3, monitor `always eventually q`.
4. Repeat step 3 with `q` false at every loop position.
5. Over a three-position finite trace whose `x` is 0 at position 1, monitor `always (10 / x > 0)` and map the outcome's category through FR-285.
6. Monitor step 1's clause and step 5's clause in one request over step 5's trace.
7. Over FR-327-AC-1's closed `Counter` trace as a three-position finite document, monitor `eventually[0,2] holds(c.value = 2)` and `eventually[0,1] holds(c.value = 2)` under event-position false-extension, `on origin`.
8. Read step 1's `counterexample` and the `decisive` beside it. Over a three-position finite trace whose position-0 and position-1 snapshots hold `items` 1, 2, 3 and whose position-2 snapshot holds `items` 3, 0, 5, monitor `always holds(forall i in self.items : i > 0)`, and evaluate its `holds` atom with FR-107's evaluator over the position-2 observation.
9. Over FR-327-AC-7's non-model subject `c: Counter` as a `Samples` trace with `level` samples 0, 1, 2 and no `over`, monitor `always[0,2] (count over past[0, 9] min_count 2 >= 1)` `on origin` and `always[0,0] (count where holds(c.level > 9) over past[0, 0] min_count 0 >= 1)` `on origin`; then select the first clause with an `over` present.
10. Over FR-501-AC-4's trace (stamps 0, 2, 5 ms, `W` 5, `Complete { through: 5 }`, scope `Closed`, `p` false at stamps 0 and 5 and absent at stamp 2) as a `Typed` `Timestamped` trace with an agreeing `binding`, monitor `eventually[0,5] holds(p)` under the finite-window profile; repeat with `W` 3; repeat with a `binding` asserting another clock name; monitor the clause over step 1's `Document` trace.
11. This is TC-770 step 10 run from the monitor side: render each record of steps 1, 2, 5, 9 and 10 through `OutcomeItem::from_clause_run`, build the document from them, and compare it with `OutcomeDocument::from_clause_runs` over the same clause runs.

Tag the tests `#[trace("TC-765", "<AC id>")]`.

## Expected Results

- Step 1: one `MonitorRecord` naming `always p`, `Violated` at position 0, category violation, whose `counterexample` has `prefix` the three positions' state keys, an empty `loop` and `trace_position` 0.
- Step 2: pending, category pending, exit 0.
- Step 3: `tested`, category success.
- Step 4: a violation, category violation.
- Step 5: a violation at position 1, category violation, cause `UndefinedEvaluation` with `division-by-zero`; no `undefined` label; exit 10.
- Step 6: two records, in request order.
- Step 7: `Tested`, category success, exit 0; then `Violated` at position 0.
- Step 8: step 1's record has a `counterexample` with `trace_position` 0, a `decisive` beside it with `position` 2, no quantifier or index, element `false` and the root value path, and no `witness`; the `forall` clause reports `Violated` at position 0 with a `counterexample` of `trace_position` 0 and a `decisive` of `position` 2 naming the `forall` occurrence key, element 0, index 1 and the value path to `items`, equal to FR-107's values, and no `witness`.
- Step 9: `InsufficientData` at position 0, `count` 1, `min_count` 2, category undefined, label `undefined` with reason `insufficient-data`, exit 10; `Violated` at position 0 with no `counterexample` and a `decisive` naming the aggregate occurrence at position 0; with `over` present, `MonitorRefusal::ClauseRefused` with `invalid_runtime_input`/`invalid-value` and no record.
- Step 10: `Indeterminate` at position 0 with reason `MissingValuation { position: 1, atom: p }`, category indeterminate, exit 11; with `W` 3, `Pending`, exit 0; with the other clock name, `MonitorRefusal::ClauseRefused` with `invalid_runtime_input`/`invalid-value` and `mismatched_dimension` `Clock`, no record; over the `Document` trace, `ClauseRefused` with `invalid_runtime_input`/`invalid-value`.
- Step 11: the two documents are equal byte for byte.
