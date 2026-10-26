---
id: TC-774
title: "Plugin results settle as typed records, and plugin proofs carry the trusted label"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: verifies
---
# TC-774: Plugin results settle as typed records, and plugin proofs carry the trusted label

## Description

Verify how a process provider's FR-331 result becomes a QSL terminal record.

Scope: FR-290-AC-1 to FR-290-AC-18.

## Test Procedure

1. Settle a process-provider FR-331 result of `proved` for a routed item; settle FR-281-AC-1's `analyze` record; settle FR-281-AC-4's record from EN-1 under partial-order reduction, which no core certificate checker covers; build a Kani `proved` record as CG's C-09 map settles it.
2. Settle a process-provider counterexample replay reproduces, one whose assignment replay evaluates `true`, and one naming a parameter the function lacks.
3. Settle a process-provider result body that fails the FR-331 reader.
4. Render step 1's plugin record, its `uncertified` record and its Kani record as text, and serialize the plugin record as JSON.
5. Read each accepted and each refused `basis`, `certification` and inconclusive `cause` vector of FR-290-AC-5 to FR-290-AC-9, including AC-7's entry with a refused `certification` and a core-only basis read in the driver's order. Settle non-hybrid proof bases through `TerminalRecord::from_plugin_proof`; settle bounded-solver bases for hybrid claims through `TerminalRecord::from_hybrid_plugin_proof`.
6. Read each `counterexamples` entry vector of FR-290-AC-10.
7. Build the outcome diagnostic and the terminal record of one refusal of each kind from steps 5 and 6.
8. Compile FR-290-AC-12's unit through `parse`, `select`, `check` and `package` and build a `PluginReplayContext` from its units and limits. Build the request for `p`'s function-contract item with `x = 0` and with `x = 5`, replay each and settle it; build the request for `q`'s `x + 1` application item with `x = 9`, replay it and settle it.
9. Encode FR-290-AC-13's two `process-provider` preimages and digest them.
10. Build a context with `s1.tokens` 200000, `library.definitions` 9000 and accounting `work_units` 5000 and read the request's `stage_limits` and `accounting_limits`; call `request` with each refused input of FR-290-AC-14 and settle each refusal.
11. Add FR-290-AC-15's `r` to step 8's unit; build, replay and settle the requests for `r`'s `10 mod x` application item with `x = 0`, `x = 3` and `x = 12`, and for `p`'s `x > 3` application item with `x = 0`. Compile FR-290-AC-15's operation-`body` unit and call `request` for its `+` item.
12. Settle FR-314-AC-1's certified SMT `proved` result and the same result with no certificate, each with Z3 verdict `Proved`, then `Refuted`, then `Inconclusive`, then with no cross-check and a `cross-check-absent{tool}` warning; settle a reproduced counterexample with Z3 verdict `Proved`, then `Refuted`; settle a cvc5 `inconclusive` (`SolverInconclusive`) with Z3 `Proved`, `Refuted`, then `Inconclusive`; serialize each record (FR-286) and read it back.
13. Settle FR-314-AC-1's accepted certificate, the same proof with no certificate, with one step's rule replaced by `hole`, and FR-314-AC-2's depth-2 query certificate.

14. Read the `bounded-solver`, `inductive-solver` and `insufficient-horizon` vectors of FR-290-AC-18; settle a bounded-solver basis with sufficient coverage and one with insufficient coverage through `TerminalRecord::from_hybrid_plugin_proof`, and confirm the generic `from_plugin_proof` entry refuses a bounded-solver basis.

Tag the tests `#[trace("TC-774", "<AC id>")]`.

## Expected Results

- Step 1: the plugin record is `proved` with the provider's `BackendId` and certification `trusted`; the `analyze` record's certification is `certified`, naming the EN-5 checker; EN-1's record under partial-order reduction is `proved` with certification `uncertified`, naming EN-1; the Kani record is `proved` with certification `certified`.
- Step 2: `refuted`; `inconclusive` with cause `replay_parity`; `inconclusive` with cause `replay_refused`.
- Step 3: no record is built from the body, and the items settle as FR-291 states.
- Step 4: the plugin text shows `proved` with `trusted` beside it and the JSON holds certification `trusted`; the `uncertified` text shows `proved` with `uncertified` beside it; the Kani text shows `proved` with `certified` beside it.
- Step 5: each accepted vector reads as the value FR-290-AC-5, AC-7 or AC-8 names and settles as stated; bounded-solver is not settled through `from_plugin_proof`; each refused vector returns the refusal and pointer FR-290-AC-6, AC-7 or AC-9 names.
- Step 6: the assignments and refusals of FR-290-AC-10.
- Step 7: each diagnostic carries the code and cause of FR-290's diagnostics table, the item index and the received spelling, every `invalid_result` diagnostic its refusal's pointer as `document_pointer`, and `CertificationNotTrusted`'s `expected` `trusted` and `actual` the received value; each record is `failed` naming the plugin.
- Step 8: the request's members are those FR-290-AC-12 names; `x = 0` settles `refuted` naming the plugin, `x = 5` settles `inconclusive` with `replay-parity`, and the `q` request selects `q` at the item's occurrence, its replay settles `inconclusive` with cause `Verdicts` (`violation`/`success`) and evaluated value `true`, and the item settles `inconclusive` with `replay-parity`.
- Step 9: the encodings and digests FR-290-AC-13 gives; the step 8 request carries its item's identity.
- Step 10: `s1.tokens` 200000 and `library.definitions` 9000 with every other carried stage setting at its bound, and `work_units` 5000; the refusals of FR-290-AC-14 in its order, `NotReplayable` settling `inconclusive` and each other `failed` naming the plugin.
- Step 11: `x = 0` settles `refuted` naming the plugin, its request carrying the item's occurrence key and `process-provider` identity; `x = 3` and `p`'s application item settle `inconclusive` with `replay-parity`; `x = 12` settles `inconclusive` with `replay-refused` and `invalid_runtime_input`. The operation-`body` item is refused `NotReplayable` and settles `inconclusive` with `replay-refused` and `unknown_required_feature`.
- Step 12: certified: `proved`, `certified` with `cross_check` verdict `Proved`, then `Refuted`, then `Inconclusive`; uncertified: `proved`, `uncertified` with verdict `Proved`, then `inconclusive` `CrossCheckDisagreement { primary: Proved, cross_check: Refuted }`; with the warning, cvc5's settlement with no `cross_check` and the warning kept; the counterexample: `inconclusive` `CrossCheckDisagreement { primary: Refuted, cross_check: Proved }`, then `refuted` with no `cross_check`; the cvc5 `inconclusive`: `CrossCheckDisagreement { primary: Inconclusive, cross_check }` twice, then `SolverInconclusive` with no `cross_check`; each record reads back equal.
- Step 13: `proved`, `certified`; `proved`, `uncertified` twice; `inconclusive`, `CertificateRejected { QueryMismatch, Unrolling }`; each `proved` record has one certification and the SMT `BackendId`.
- Step 14: sufficient bounded-solver coverage settles `proved`, `trusted`; insufficient coverage settles `inconclusive`, `InsufficientHorizon`; `inductive-solver` settles `proved`, `trusted`; the generic `from_plugin_proof` entry refuses bounded-solver.
