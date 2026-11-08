---
id: SR-2611082
title: "Failure-domain review for the L5 phase-0 batch"
type: SpecReview
analysis: failure-domain
scope: "Unstated failure modes, identity confusion, timing dependence and crate-graph hazards in FR-560 to FR-567, NFR-017, NFR-018 and ADR-035"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-563
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-566
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-017
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-035
    type: reviews
---

## Summary

The trust boundaries here are the parity reference, the driver's CLI half of
the matrix, process plugins and the clean-clone gate. The parity runner's
failure policy is strict and well stated, but its public input lets a caller
choose the oracle. The library-to-CLI byte-equality contract does not survive
the cells that depend on timing. The crate graph has a cycle and depends on a
crate that ADR-035 says may never be built. The clean-clone gate deletes the
evidence its own tests inspect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Cells that depend on timing cannot be byte-equal across the two halves. FR-565's Overlap requires the driver's `--format json` document to be byte-equal to `run_case`'s for every case, but the driver produces timeout and cancellation with its own deadline and interrupt handling. FR-286's `last_stage` records the stage the cancel stopped (it is `null` only before the first stage), so `check` and `package` timeout documents depend on timing. The `analyze` timeout cell uses a wall-clock `deadline_ms` "that expires before it settles", which is flaky on a fast host. Fix: define each timeout and cancellation cell as cancelled before the first charge (`last_stage` `null`) on both halves, use `deadline_ms: Some(0)` for `analyze`, or compare only category, exit and cause for these cells. | FR-565 Overlap, FR-564, FR-286, TC-1391 |
| FND-002 | medium | FR-565's cancellation plan "cancelled after a stated number of meter charges" needs a meter observer in `qsl-conformance`, which is a production library the driver depends on. The observer FR-276-AC-2 uses is a test hook, and no lifecycle operation exposes one. Fix: specify the public hook and its crate, or drop that plan and use pre-cancelled handles only. | FR-565, FR-276-AC-2 |
| FND-003 | medium | `run_parity` takes `Option<&JitCoverage>`, and ADR-029's amendment puts `qsl-conformance` above layer X. ADR-035 CG-3 allows that `qsl-jit` is never built, which would leave the runner's signature naming a type that does not exist. The program generator also lives in `qsl-conformance`, while `qsl-jit`'s own tests use it (FR-560-AC-2, FR-561-AC-3). That is a dev-dependency cycle (`qsl-jit` to `qsl-conformance` to `qsl-jit`), which builds `qsl-jit` twice with distinct types. Fix: move the coverage type into the FR-294 seam in `qsl-eval` (for example `BackendCoverage`), and the generator below `qsl-jit`. | FR-563, FR-560, ADR-029 CB-3 item 3, ADR-035 CG-3 |
| FND-004 | medium | `ParityInputs` lets the caller choose the reference backend ("the interpreter unless a test supplies another"). A caller, such as the driver gating `quire-aot`, can pass the backend under test as its own reference and the gate passes with no finding. That is the excuse channel FR-563's failure policy says does not exist. Fix: fix the reference to the interpreter in the public API, offer a substitute reference only behind a test-support feature for FR-563-AC-6, and extend FR-563-AC-8 to check this. | FR-563, FR-563-AC-8 |
| FND-005 | medium | FR-566 configuration 1 is ambiguous for `analyze` and `prove` cases. "No provider added to `Engines`" read as an empty `Engines` settles those cases `invalid-request`/`unknown-backend` (FR-281), which is not byte-equal to configuration 2. FR-281 also says plugin providers are not `Engine`s, except through the value-validity/unbounded provider arm, so configuration 3's "provider under that descriptor added to `Engines`" works only with a descriptor that advertises that pair. Fix: state configuration 1 as `Engines::qsl()` with an empty registry, and state that the test plugin's descriptor advertises (`value-validity`, `unbounded`) and is built through the provider arm. | FR-566, FR-281 |
| FND-006 | medium | NFR-017's gate removes every temporary directory at step 5, but the source-graph check (`cargo metadata`, `git ls-remote`) and the example-output checks are not gate steps. TC-1397 step 1 and TC-1398 inspect "its clone" after the gate has deleted it, and NFR-017-AC-3's "fails naming that source" needs a step the gate does not have. Fix: add the source check and the example-output checks as gate steps that run before cleanup, with the gate reporting each result, and have the TCs assert on that report. | NFR-017, NFR-017-AC-1, AC-2, AC-3, TC-1397, TC-1398 |
| FND-007 | medium | NFR-017-AC-3's "a scratch commit whose `Cargo.lock` names a `path` source outside the repository" cannot be built, because `Cargo.lock` records no source for path packages. The real leak routes are a `Cargo.toml` path dependency or `[patch]` outside the repository, and a `.cargo/config.toml` in a parent directory of the clone, which Cargo reads even with an empty `CARGO_HOME`. Fix: restate the case as a path dependency or patch outside the repository, add the parent-config route, and have the gate clone into a directory with no ancestor Cargo config. | NFR-017-AC-3, TC-1397 |
| FND-008 | low | The gate clones "the repository" and takes only `REV`, but TC-1397 step 2 and TC-1399 step 2 run it against scratch repositories. Fix: add a `REPO` parameter that defaults to the current checkout, or make the scratch commits on a branch of a local clone that the gate is pointed at. | NFR-017, NFR-018, TC-1397, TC-1399 |
| FND-009 | low | FR-563 makes a reference `Fault` a refusal (no verdict, exit 2), but an interpreter `Fault` on a generated unit is a real defect in the reference. Grouping it with usage refusals hides it. Fix: record `reference-fault` as a failing finding, exit 1, carrying the FR-295 report fields. | FR-563, FR-563-AC-6, FR-567-AC-4 |
| FND-010 | low | ADR-035 CG-1 item 4 requires "the same generated bytes", but JIT code with relocations applied embeds addresses that change from run to run, so the item can exclude both candidates. Fix: require equal code bytes before relocation, or require determinism of results only. | ADR-035 CG-1 item 4 |
| FND-011 | low | FR-562 does not define the result interval of `div`, `mod` and `rem`. That interval depends on the kernel's division convention, and the fast-path decision for every division depends on it. Fix: state the interval rule from `quire-exact`'s division semantics. | FR-562 |
| FND-012 | low | FR-564's `analyze` timeout cell depends on `analyze` reading a wall clock (FR-281 `deadline_ms`), while FR-276 says no QSL library operation reads a clock. This existing conflict now carries weight in a conformance case. Fix: resolve it in FR-276 or FR-281, or produce the cell with a `Cancel` cancelled with `Deadline`. | FR-564, FR-276, FR-281-AC-12 |

## Dispositions

Fix round 1, in the same PR:

- FND-001, FND-002: fixed. Every timeout and cancellation cell cancels before the call, `analyze`'s uses `deadline_ms` 0, and the cancel-after-N-charges plan is removed; the driver Overlap produces those cells before the first stage.
- FND-003: fixed. The coverage type is `BackendCoverage` in `qsl-eval`'s `execution` module; `qsl-conformance` does not depend on `qsl-jit`.
- FND-004: fixed. The reference is always the interpreter; `run_parity_with_reference` exists only under `test-support`, covered by FR-563-AC-8.
- FND-005: fixed. Configuration 1 is an empty registry with `Engines::qsl()`; the plugin advertises only (`value-validity`, `unbounded`) and enters `Engines` through that arm.
- FND-006: fixed. The source check and the example-output checks are gate steps 4 and 5, before cleanup, and the gate writes a per-step report.
- FND-007: fixed. AC-3 uses a `Cargo.toml` path dependency; step 1 refuses an ancestor `.cargo/config.toml`.
- FND-008: fixed. The gate takes `REPO`.
- FND-009: fixed. An interpreter `Fault` is a `reference-fault` finding, exit 1.
- FND-010: fixed. CG-1 item 4 requires equal bytes before relocation.
- FND-011: fixed. FR-562 defines the `div`/`mod`/`rem` result interval.
- FND-012: the analyze timeout cell now uses `deadline_ms` 0. The conflict between FR-281's clock read and FR-276's no-clock rule is pre-existing and outside this PR's scope; it is reported to the planner.
