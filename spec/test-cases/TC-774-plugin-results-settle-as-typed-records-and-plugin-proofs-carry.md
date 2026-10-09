---
id: TC-774
title: "Plugin results settle as typed records, and plugin proofs carry the trusted label"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: verifies
---
# TC-774: Plugin results settle as typed records, and plugin proofs carry the trusted label

## Description

Verify how a process provider's FR-331 result becomes a QSL terminal record.

Scope: FR-290-AC-1 to FR-290-AC-4.

## Test Procedure

1. Settle a process-provider FR-331 result of `proved` for a routed item; settle FR-281-AC-1's `analyze` record; settle FR-281-AC-4's record from the test engine with no certificate checker; build a Kani `proved` record as CG's C-09 map settles it.
2. Settle a process-provider counterexample replay reproduces, one whose assignment replay evaluates `true`, and one naming a parameter the function lacks.
3. Settle a process-provider result body that fails the FR-331 reader.
4. Render step 1's plugin record, its `uncertified` record and its Kani record as text, and serialize the plugin record as JSON.

Tag the tests `#[trace("TC-774", "<AC id>")]`.

## Expected Results

- Step 1: the plugin record is `proved` with the provider's `BackendId` and certification `trusted`; the `analyze` record's certification is `certified`, naming the EN-5 checker; the test engine's record is `proved` with certification `uncertified`, naming that engine; the Kani record is `proved` with certification `certified`.
- Step 2: `refuted`; `inconclusive` with cause `replay_parity`; `inconclusive` with cause `replay_refused`.
- Step 3: no record is built from the body, and the items settle as FR-291 states.
- Step 4: the plugin text shows `proved` with `trusted` beside it and the JSON holds certification `trusted`; the `uncertified` text shows `proved` with `uncertified` beside it; the Kani text shows `proved` with `certified` beside it.
