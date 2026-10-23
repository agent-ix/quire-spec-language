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

Scope: FR-281-AC-1 to FR-281-AC-3, FR-281-AC-8, FR-281-AC-9.

## Test Procedure

1. Over a finite probabilistic model, run `analyze` with one EN-5 item claiming a reachability probability bound that holds.
2. Over a finite model whose invariant one transition breaks, run `analyze` with one EN-1 item claiming that invariant; replay the record's counterexample through `replay`.
3. Run `analyze` with three items: one of a claim kind no layer-A engine advertises, step 1's item and step 2's item.
4. Register test engines `qsl.test-a` and `qsl.test-b`, both advertising (`value-validity`, `bounded`), and run a `value-validity` bounded item naming no backend, naming `qsl.test-b`, and naming `qsl.test-c`; then with `qsl.test-a` advertising only (`value-validity`, `unbounded`), run the unnamed item.
5. Run step 3's request with a binding whose `universes` name a population the package does not declare.

Tag the tests `#[trace("TC-762", "<AC id>")]`.

## Expected Results

- Step 1: the item settles `proved`, category success, after the EN-5 checker accepts; the record carries the certificate.
- Step 2: the item settles `refuted` and carries the counterexample; `replay` reproduces it.
- Step 3: three records return in `request_index` order 0, 1, 2; the first is `unsupported` with a cause naming the claim kind.
- Step 4: `qsl.test-a` alone runs and the record names it; `qsl.test-b` alone runs; `unsupported` naming the kind and `qsl.test-c`, with no engine run; `qsl.test-b` runs.
- Step 5: `StageFailure::Refused(AnalyzeRefusal::Binding)` holding `invalid_runtime_input`/`wrong-role-mapping`, no record, no engine run.
