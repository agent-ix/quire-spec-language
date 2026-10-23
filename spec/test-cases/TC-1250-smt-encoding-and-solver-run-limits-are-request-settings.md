---
id: TC-1250
title: "The SMT encoding limits and solver run limits are request settings that settle V-7 and are recorded for the certificate check"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: verifies
---
# TC-1250: The SMT encoding limits and solver run limits are request settings that settle V-7 and are recorded for the certificate check

## Description

Verify that FR-315's six encoding limits and two solver run limits are
request settings with published defaults and one setting name each, that a
`too-large` refusal and a solver stop under a run limit each name the limit,
its value and its setting and settle V-7 `LimitReached`, that FR-182's stop
order places the run limits at rule 3, and that FR-314's checker rebuilds a
query under the encoding limits the certificate request records.

Scope: FR-315-AC-40 to FR-315-AC-42, FR-182-AC-14, FR-314-AC-7 and
FR-255-AC-14.

Step 2 reads CG's SMT map against injected backend outcomes: each outcome
is handed to the map in place of a solver run, so the vectors check the
settlement, not the solver.

## Test Procedure

1. Over FR-315-AC-1's `Counter` under event-position false-extension,
   encode `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` under
   the default `SmtEncodingLimits`, under `max_constants` 8, under
   `max_constants` 7 and under `max_script_bytes` 1. Encode FR-315-AC-23's
   TP-3 item under `max_monitor_states` 1, FR-315-AC-21's
   `Unrolling{depth: 1048576}` under the defaults, and `NonInterference`'s
   `InductionBase{depth: 1}` for instance `v` under `encoding.max_constants`
   7, with `HyperProductIdentity::of` for it under the defaults and under 7.
   Repeat each call.
2. Hand CG's SMT map, for `always holds(c.value <= 3)` over `Counter`: step
   1's `max_constants` 7 refusal as the item's next query; an
   `InductionBase{depth: 1}` stop at `max_solver_steps` 1; one at
   `max_solver_time` 1; the step-limit stop with the cancellation handle
   tripped; the step-limit stop with the backend's own wall clock expired;
   and a solver `unknown` with no limit reached. Compute the request
   identity of two requests that differ only in `max_solver_time`, and of
   two that differ only in `max_solver_steps`.
3. Call `settle_hyper_smt` for `NonInterference` at `v` with the
   FR-182-AC-14 outcomes: the two solver-limit `LimitStopped` outcomes, a
   `LimitStopped` naming `SmtMaxConstants`, and step 1's hyper `too-large`
   refusal as `EncodingRefused`.
4. Check FR-314-AC-1's bounded certificate with certificate requests that
   record `max_constants` 8, `max_constants` 7 and the defaults.
5. Call the settings operation with `smt_encoding.max_constants=8`, with
   `smt.max_solver_steps=1000` and `smt.max_solver_time=5`, with no operand,
   with `smt.max_solver_time=0` and with `smt_encoding.max_monitor_states=0`;
   replay with the `stage_limits` entry `smt_encoding.max_constants` at 8.
6. Build FR-315-AC-42's `ModelCheckRequest`s with no SMT setting, with
   `smt_encoding.max_constants` 7 and 8, and with `smt.max_solver_time` 5
   and 10; compute their request identities and cache keys; settle an SMT
   proof of the first and read its `CertificateRequest.smt_encoding`.

Tag the tests `#[trace("TC-1250", "<criterion id>")]`.

## Expected Results

- Step 1: the defaults and `max_constants` 8 give byte-equal scripts;
  `max_constants` 7 refuses `too-large` naming `constants`, 8, 7 and
  `smt_encoding.max_constants`; `max_script_bytes` 1 refuses naming
  `script-bytes`, limit 1 and `smt_encoding.max_script_bytes`; the monitor
  item names `monitor-states`, a value above 1, limit 1 and
  `smt_encoding.max_monitor_states`; the depth-1048576 request names
  `smt_encoding.max_constants`; the hyper query names `constants`, 8, 7 and
  `smt_encoding.max_constants`, and the two identities are equal; repeated
  calls are equal (FR-315-AC-40).
- Step 2: `Incomplete(LimitReached{SmtMaxConstants, 7,
  smt_encoding.max_constants})`, not `unsupported`, with the encoding limits
  stated on the record; `LimitReached{max_solver_steps, 1,
  smt.max_solver_steps}`; `LimitReached{max_solver_time, 1,
  smt.max_solver_time}`; `Cancelled{source}`; `TimedOut`;
  `InductionNotClosed{depth: 1}`. The `max_solver_time` pair has equal
  identities and the `max_solver_steps` pair different ones (FR-315-AC-41).
- Step 3: `LimitReached{max_solver_steps, 1000, smt.max_solver_steps}` and
  `LimitReached{max_solver_time, 5, smt.max_solver_time}`, neither
  `InductionNotClosed`, `TimedOut` nor `LimitNotApplicable`; the
  `SmtMaxConstants` stop writes no record and refuses `LimitNotApplicable`;
  the `too-large` outcome `LimitReached{max_constants, 7,
  smt_encoding.max_constants}` (FR-182-AC-14).
- Step 4: `Verified` for `max_constants` 8 and for the defaults; for 7,
  `QueryMismatch` at `Query { part: Unrolling }`, the rebuild refused
  `too-large` naming `smt_encoding.max_constants`, settled `inconclusive`,
  `CertificateRejected` (FR-314-AC-7).
- Step 5: the limits with the named members set and the others at their
  defaults; both types at their defaults with no operand; usage refusals
  naming `smt.max_solver_time=0` and `smt_encoding.max_monitor_states=0`;
  the replay's encoding under 8 (FR-255-AC-14).
- Step 6: `Default` limits on the plain request; different identities and
  keys for the two `max_constants` requests and no reuse between them;
  equal ones for the two `max_solver_time` requests; the certificate
  request's `smt_encoding` equal to the run's (FR-315-AC-42).
