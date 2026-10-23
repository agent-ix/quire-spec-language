---
id: TC-763
title: "analyze settles rejected certificates, unreproduced counterexamples and budgets without a false verdict"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: verifies
---
# TC-763: analyze settles rejected certificates, unreproduced counterexamples and budgets without a false verdict

## Description

Verify the adverse and uncertified settlements of `analyze`, using test engines registered in layer A under `cfg(test)`.

Scope: FR-281-AC-4 to FR-281-AC-7, FR-281-AC-11.

## Test Procedure

1. Run TC-762 step 1 with a test hook that alters one member of the certificate before the checker runs.
2. Run `analyze` with a test engine that reports a proof and has no certificate checker.
3. Run `analyze` with a test engine that reports a counterexample whose replay evaluates the claim true, then with one whose counterexample names a parameter the function lacks.
4. Run TC-762 step 1 with the engine's state budget set to 1.
5. Over a finite model with one reachable state at which `x` is 0, run `analyze` with an EN-1 item claiming the invariant `100 / x > 0`, replay the record's counterexample, and map the outcome's category through FR-285.
6. Run `analyze` with six items, each settled `Supported` on a test engine that refuses it: with `Admission` (a refused admission, then, as a seventh run, an incomplete one), `RequiresBound` naming a root `R`, `Symmetry`, `NotStatistical`, a refusal carrying `unsupported_construct`/`not-yet-implemented`, and a refusal carrying `invalid_runtime_input`/`wrong-value-kind`.
7. Over a model one of whose operations `modifies` a field of type `Integer`, run `analyze` over `Engines::qsl()` with one EN-1 item settled `Supported` on EN-1.

Tag the tests `#[trace("TC-763", "<AC id>")]`.

## Expected Results

- Step 1: the item settles `inconclusive` with cause `CertificateRejected`.
- Step 2: the item settles `proved`, category success, with the certification `uncertified` naming the engine.
- Step 3: the first settles `inconclusive` with cause `replay_parity`; the second `inconclusive` with cause `replay_refused` and the refusal's catalog code.
- Step 4: the item settles `incomplete` with the state-budget cause.
- Step 5: the item settles `refuted`, category violation, with cause `UndefinedEvaluation` whose `where` names the state with `x` 0 and whose `cause` is `division-by-zero`; replay reproduces the undefined value; the record carries no `undefined` label; the exit code is 10.
- Step 6: six records in `request_index` order, each naming its engine: `declined`, `invalid-input`, with the admission diagnostic (`incomplete-input` for the incomplete admission); `unsupported`, `unsupported_projection`/`unbounded-extent` naming `R`; `declined`, `invalid-input`, `invalid_runtime_input`/`invalid-value`; `invalid-request`, `invalid_capability`/`not-statistical`, no result; `unsupported` with `unsupported_construct`/`not-yet-implemented`; `failed` with `invalid_runtime_input`/`wrong-value-kind`.
- Step 7: one record, `unsupported` with `unsupported_projection`/`unbounded-extent` naming the field's root, naming EN-1; no state is explored.
