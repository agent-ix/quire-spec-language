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

Verify the adverse and uncertified settlements of `analyze`, using test engines registered in layer A under `cfg(test)` and EN-1 under reduction.

Scope: FR-281-AC-4 to FR-281-AC-7, FR-281-AC-11.

## Test Procedure

1. Run TC-762 step 1 with a test hook that alters one member of the certificate before the checker runs.
2. Run `analyze` over EN-1 with the TC-762 step 1 item and partial-order reduction selected, then unreduced.
3. Run `analyze` with a test engine that reports a counterexample whose replay evaluates the claim true, then with one whose counterexample names a parameter the function lacks.
4. Run TC-762 step 1 with the engine's state budget set to 1.
5. Over a finite model with one reachable state at which `x` is 0, run `analyze` with an EN-1 item claiming the invariant `100 / x > 0`, replay the record's counterexample, and map the outcome's category through FR-285.
6. Run `analyze` with six items, each settled `Supported` on a test engine that refuses it: with a refused `Admission`, `RequiresBound` from an engine advertising (`value-validity`, `bounded`), `Symmetry`, `NotStatistical`, a refusal carrying `unsupported_construct`/`not-yet-implemented`, and a refusal carrying `invalid_runtime_input`/`wrong-value-kind`. Then run a seventh request of one item whose test engine reports an incomplete `Admission`.
7. Over a model one of whose operations `modifies` a field of type `Integer`, run `analyze` over `Engines::qsl()` with one EN-1 item settled `Supported` on EN-1.

Tag the tests `#[trace("TC-763", "<AC id>")]`.

## Expected Results

- Step 1: the item settles `inconclusive` with cause `CertificateRejected`.
- Step 2: the reduced item settles `proved`, category success, proof basis `reduced`, with the certification `uncertified` naming EN-1; the unreduced item settles `proved`, `certified`.
- Step 3: the first settles `inconclusive` with cause `replay_parity`; the second `inconclusive` with cause `replay_refused` and the refusal's catalog code.
- Step 4: the item settles `incomplete` with the state-budget cause.
- Step 5: the item settles `refuted`, category violation, with cause `UndefinedEvaluation` whose `where` names the state with `x` 0 and whose `cause` is `division-by-zero`; replay reproduces the undefined value; the record carries no `undefined` label; the exit code is 10.
- Step 6: six records in `request_index` order, each naming its engine: `declined`, `invalid-input`, with the admission diagnostic; `unsupported`, `unsupported_projection`/`unbounded-extent` whose payload names `value-validity`, the engine's `BackendId` and `bounded`; `declined`, `invalid-input`, `invalid_runtime_input`/`invalid-value`; `invalid-request`, `invalid_capability`/`not-statistical`, no result; `unsupported` with `unsupported_construct`/`not-yet-implemented`; `failed` with `invalid_runtime_input`/`wrong-value-kind`. The seventh request returns one `declined`, `incomplete-input` record with the admission diagnostic.
- Step 7: one record naming EN-1, `unsupported` with `unsupported_projection`/`unbounded-extent` whose payload names `temporal-satisfaction`, EN-1's `BackendId` and `bounded`; no state is explored.
