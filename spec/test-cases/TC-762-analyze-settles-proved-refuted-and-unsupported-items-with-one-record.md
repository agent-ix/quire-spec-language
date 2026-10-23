---
id: TC-762
title: "analyze settles proved, refuted and unsupported items with one record each"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: verifies
---
# TC-762: analyze settles proved, refuted and unsupported items with one record each

## Description

Verify `analyze` over QSL's layer-A engines: a `certified` proof, a replayed counterexample, and an item no engine accepts.

Scope: FR-281-AC-1 to FR-281-AC-3, FR-281-AC-8 to FR-281-AC-10.

## Test Procedure

1. Over a finite probabilistic model, run `analyze` with one EN-5 item claiming a reachability probability bound that holds.
2. Over a finite model whose invariant one transition breaks, run `analyze` with one EN-1 item claiming that invariant; replay the record's counterexample through `replay`.
3. Run `analyze` over `Engines::qsl()` with three items: one of a claim kind no layer-A engine advertises, settled `Unsupported` with `unsupported_projection`/`unsupported-requested-capability` naming that kind, then step 1's and step 2's items, each settled `Supported` on its engine.
4. Build `engines` with `Engines::with` holding test engines `qsl.test-a` and `qsl.test-b`, both advertising (`value-validity`, `bounded`), each counting its runs. Run one `value-validity` bounded item settled `Supported(qsl.test-b)`; settled `InvalidRequest` with `invalid_capability`/`ambiguous-backend` naming both; settled `Unsupported` with `unsupported_projection`/`unsupported-requested-capability`; settled `RequiresBound`; and settled `Supported(qsl.test-c)`.
5. Run step 3's request with a binding whose `universes` name a population the package does not declare.
6. Call `Engines::with` on step 4's `engines` with a second engine whose manifest's `BackendId` is `qsl.test-a`; then run an item settled `Supported(qsl.test-a)` on the set it was called on. Run a `temporal-satisfaction` item settled `Supported(qsl.test-a)`. Read step 3's first record's members.

Tag the tests `#[trace("TC-762", "<AC id>")]`.

## Expected Results

- Step 1: the item settles `proved`, category success, after the EN-5 checker accepts; the record carries the certificate.
- Step 2: the item settles `refuted` and carries the counterexample; `replay` reproduces it.
- Step 3: three records return in `request_index` order 0, 1, 2; the first is settled by its `unsupported` disposition with the diagnostic naming the claim kind.
- Step 4: `qsl.test-b` alone runs and the record names it; then records settled by `invalid-request` (`ambiguous-backend`, both named), `unsupported` and `requires-bound` (no diagnostic) dispositions with no run; then `invalid-request` with `invalid_capability`/`unknown-backend` naming `qsl.test-c` and no run. Neither test engine runs except in the first case.
- Step 5: `StageFailure::Refused(AnalyzeRefusal::Binding)` holding `invalid_runtime_input`/`wrong-role-mapping`, no record, no engine run.
- Step 6: `invalid_capability`/`duplicate-backend` naming `qsl.test-a`; the first `qsl.test-a` engine runs the `Supported(qsl.test-a)` item. The `temporal-satisfaction` item settles `invalid-request` with `invalid_capability`/`inconsistent-candidates` naming `qsl.test-a`, `temporal-satisfaction` and `bounded`, and no engine runs. Step 3's first record carries the `unsupported-requested-capability` diagnostic naming the claim kind and no `cause` member.
