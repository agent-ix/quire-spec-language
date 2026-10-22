---
id: TC-1173
title: "The replay facade carries the call's plugin replay context from the same run"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-404
    type: verifies
---
# TC-1173: The replay facade carries the call's plugin replay context from the same run

## Description

Verify that `run_with_replay_context` returns, with the outcome and members
of `run_with_requirements`, a replay context built from the same compile and
limits, that the context builds requests only for the call's own items and
only with the run's `package_id`, and that a consumer obtains it through the
`qsl_replay` root alone. Scope: FR-404-AC-1 to FR-404-AC-9.

## Test Procedure

Test layer: integration tests in `qsl-replay/tests`, beside
`call_requirements_facade.rs` and `call_requirements_root_only.rs` (TC-1089).
The facade calls under test reach only the `qsl_replay` root. The independent
oracle (a `PluginReplayContext` built by `PluginReplayContext::new` from a
separate `parse`, `select`, `check` and `package` of the same inputs, and the
items `items_from_requirements` writes) is read through `qsl_replay::spine`
inside the test, as `call_requirements_facade.rs` reads the spine's graph;
consumer code does not. Fixtures R and R2 and the plugin `acme` are those of
FR-404.

1. Run `q` with `x = 3` over R under `CallerLimits::default()`; run
   `run_with_requirements` and `compile_package` over the same inputs with the
   default `spine` limits and compare.
2. Build the oracle context over R. Call the run's `request` and the oracle's
   `request` for `q`'s member with `x = 9`; compare; `replay` the run's
   request under `CallerLimits::default().replay` and settle it.
3. Run `r` with `x = 0` over R; build, replay and settle the request for its
   member with `x = 0`. Run `q` with `x = 3` under call accounting
   `work_units = 0` and build the request for its member with `x = 9`.
4. Call step 1's context's `request` with `r`'s member key, `p`'s
   `declaration` occurrence and the key of node `11` repeated 32 times at
   `{"ordinal":0,"role":"declaration"}`; settle each refusal. Call the
   oracle's `request` with `p`'s `declaration` occurrence and `x = 0`.
5. Run `q` with `x = 3` over R and over R2 and build each run's request for
   its own `q` member; run `s` with `x = 3` over R2 and call R's `q` context's
   `request` with its member key.
6. Run `q` with `x = 3` over R under the FR-404-AC-6 `CallerLimits` and read
   the request's `stage_limits` and `accounting_limits`; run it under
   `s1.tokens` 1.
7. Call step 1's context's `request` with a non-member key and two proof
   bounds at one `DomainKey`, and with `q`'s member and two proof bounds at
   one `DomainKey`.
8. Run `nope`, `q` with a Boolean `x`, a source that does not parse, and a
   cancelled handle; run `run_with_requirements` over the same inputs.
9. In a root-only test (like `call_requirements_root_only.rs`), run step 1,
   build step 2's request and settle a step 4 refusal naming only
   `qsl_replay` root paths; run FR-060's T12-A rule over that test. Add a
   `compile_fail` doctest that builds a `CallReplayContext` by struct literal
   and one that calls a constructor.

Tag the tests `#[trace("TC-1173", "<AC id>")]`.

## Expected Results

1. Outcome `completed` `4`, one member; the context's `package_id` equals the
   result's; `package_id`, outcome and members equal `run_with_requirements`';
   the `package_id` equals `compile_package`'s.
2. The two requests are equal member by member; `package_id` is the run's,
   `selected_function` is `q`, `occurrence_key` is the member's; `replay`
   completes `10`; the item settles `inconclusive` with cause `replay-parity`,
   naming `acme`.
3. `r` returns `undefined`, one member and a context; its request names `r`
   and the `process-provider` identity of the member's node and occurrence;
   it settles `refuted`, naming `acme`. The `work_units = 0` call is
   `incomplete` on `work_units`, and its request equals step 2's.
4. `UnknownOccurrence` (`missing_declaration`/`missing-name`) with the key
   retained, three times, each settling `failed` naming `acme`; the oracle
   builds a request for `p`'s `declaration` occurrence.
5. Different `package_id`s; each request carries its own run's `package_id`
   and source digest; `UnknownOccurrence` for `s`'s key.
6. `stage_limits` holds `s1.tokens` 200000 and `library.definitions` 9000;
   `accounting_limits` has `work_units` 5000; `s1.tokens` 1 refuses `Compile`
   with no context.
7. `UnknownOccurrence`; then `DuplicateBoundKey`
   (`invalid_runtime_input`/`duplicate-member`).
8. `MissingDeclaration`, `WrongValueKind` at position 0, `Compile`,
   `Cancelled`, each equal to `run_with_requirements`' refusal, with no context
   and no members.
9. The root-only test compiles and passes; T12-A reports no call site; both
   doctests fail to compile.
