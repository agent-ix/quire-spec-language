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

Scope: FR-290-AC-1 to FR-290-AC-15.

## Test Procedure

1. Settle a process-provider FR-331 result of `proved` for a routed item; settle FR-281-AC-1's `analyze` record; settle FR-281-AC-4's record from the test engine with no certificate checker; build a Kani `proved` record as CG's C-09 map settles it.
2. Settle a process-provider counterexample replay reproduces, one whose assignment replay evaluates `true`, and one naming a parameter the function lacks.
3. Settle a process-provider result body that fails the FR-331 reader.
4. Render step 1's plugin record, its `uncertified` record and its Kani record as text, and serialize the plugin record as JSON.
5. Read each accepted and each refused `basis`, `certification` and inconclusive `cause` vector of FR-290-AC-5 to FR-290-AC-9, including AC-7's entry with a refused `certification` and a core-only basis read in the driver's order, and settle each accepted value through `TerminalRecord::from_plugin_proof` or `TerminalRecord::from_plugin_inconclusive`.
6. Read each `counterexamples` entry vector of FR-290-AC-10.
7. Build the outcome diagnostic and the terminal record of one refusal of each kind from steps 5 and 6.
8. Compile FR-290-AC-12's unit through `parse`, `select`, `check` and `package` and build a `PluginReplayContext` from its units and limits. Build the request for `p`'s function-contract item with `x = 0` and with `x = 5`, replay each and settle it; build the request for `q`'s `x + 1` application item with `x = 9`, replay it and settle it.
9. Encode FR-290-AC-13's two `process-provider` preimages and digest them.
10. Build a context with `s1.tokens` 200000, `library.definitions` 9000 and accounting `work_units` 5000 and read the request's `stage_limits` and `accounting_limits`; call `request` with each refused input of FR-290-AC-14 and settle each refusal.
11. Add FR-290-AC-15's `r` to step 8's unit; build, replay and settle the requests for `r`'s `10 mod x` application item with `x = 0`, `x = 3` and `x = 12`, and for `p`'s `x > 3` application item with `x = 0`. Compile FR-290-AC-15's operation-`body` unit and call `request` for its `+` item.

Tag the tests `#[trace("TC-774", "<AC id>")]`.

## Expected Results

- Step 1: the plugin record is `proved` with the provider's `BackendId` and certification `trusted`; the `analyze` record's certification is `certified`, naming the EN-5 checker; the test engine's record is `proved` with certification `uncertified`, naming that engine; the Kani record is `proved` with certification `certified`.
- Step 2: `refuted`; `inconclusive` with cause `replay_parity`; `inconclusive` with cause `replay_refused`.
- Step 3: no record is built from the body, and the items settle as FR-291 states.
- Step 4: the plugin text shows `proved` with `trusted` beside it and the JSON holds certification `trusted`; the `uncertified` text shows `proved` with `uncertified` beside it; the Kani text shows `proved` with `certified` beside it.
- Step 5: each accepted vector reads as the value FR-290-AC-5, AC-7 or AC-8 names and settles as stated; each refused vector returns the refusal and pointer FR-290-AC-6, AC-7 or AC-9 names.
- Step 6: the assignments and refusals of FR-290-AC-10.
- Step 7: each diagnostic carries the code and cause of FR-290's diagnostics table, the item index, the member or pointer and the received spelling; each record is `failed` naming the plugin.
- Step 8: the request's members are those FR-290-AC-12 names; `x = 0` settles `refuted` naming the plugin, `x = 5` settles `inconclusive` with `replay-parity`, and the `q` request selects `q` at the item's occurrence and settles `inconclusive` with `replay-parity`.
- Step 9: the encodings and digests FR-290-AC-13 gives; the step 8 request carries its item's identity.
- Step 10: `s1.tokens` 200000 and `library.definitions` 9000 with every other carried stage setting at its bound, and `work_units` 5000; the refusals of FR-290-AC-14 in its order, `NotReplayable` settling `inconclusive` and each other `failed` naming the plugin.
- Step 11: `x = 0` settles `refuted` naming the plugin, its request carrying the item's occurrence key and `process-provider` identity; `x = 3` and `p`'s application item settle `inconclusive` with `replay-parity`; `x = 12` settles `inconclusive` with `replay-refused` and `invalid_runtime_input`. The operation-`body` item is refused `NotReplayable` and settles `inconclusive` with `replay-refused` and `unknown_required_feature`.
