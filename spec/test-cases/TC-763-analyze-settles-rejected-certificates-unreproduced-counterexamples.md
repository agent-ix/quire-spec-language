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

Scope: FR-281-AC-4 to FR-281-AC-7.

## Test Procedure

1. Run TC-762 step 1 with a test hook that alters one member of the certificate before the checker runs.
2. Run `analyze` with a test engine that reports a proof and has no certificate checker.
3. Run `analyze` with a test engine that reports a counterexample whose replay evaluates the claim true, then with one whose counterexample names a parameter the function lacks.
4. Run TC-762 step 1 with the engine's state budget set to 1.
5. Over a finite model with one reachable state at which `x` is 0, run `analyze` with an EN-1 item claiming the invariant `100 / x > 0`, replay the record's counterexample, and map the outcome's category through FR-285.

Tag the tests `#[trace("TC-763", "<AC id>")]`.

## Expected Results

- Step 1: the item settles `inconclusive` with cause `CertificateRejected`.
- Step 2: the item settles `proved`, category success, with the basis `uncertified` naming the engine.
- Step 3: the first settles `inconclusive` with cause `replay_parity`; the second `inconclusive` with cause `replay_refused` and the refusal's catalog code.
- Step 4: the item settles `incomplete` with the state-budget cause.
- Step 5: the item settles `refuted`, category violation, with cause `UndefinedEvaluation` whose `where` names the state with `x` 0 and whose `cause` is `division-by-zero`; replay reproduces the undefined value; the record carries no `undefined` label; the exit code is 10.
