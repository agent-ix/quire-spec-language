---
id: TC-1089
title: "The replay facade associates the selected call's requirement items"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-1089: The replay facade associates the selected call's requirement items

## Description

Verify `run_with_requirements` returns, with the outcome of `run`, exactly the
requirement items whose owner is the selected function, in occurrence-key
order, and refuses as `run` does. Scope: FR-395-AC-1 to FR-395-AC-11, FR-062-AC-14 and FR-075-AC-17.

## Test Procedure

Test layer: an integration test in `qsl-replay/tests`, like
`admitted_context_facade.rs`. The facade calls under test reach only the
`qsl_replay` root. The independent oracle (the checked graph's records and
their owners, and the items `items_from_requirements` writes) is read through
`qsl_replay::spine` and the checked package inside the test, as
`admitted_context_facade.rs` reads the spine's graph; consumer code does not.

1. Call `sq` with `x = 3` and `inc` with `x = 3` over fixture K; read the
   members; write K's items with `items_from_requirements` over the
   package's records.
2. Call `id` and `two` with their arguments; call `sq` with `x = 0` and
   `x = 9`; call `sq` with a Boolean `x`.
3. Call `sq` with `x = 3` and `work_units = 0`.
4. Call `run` and `compile_package` over the same inputs for the calls of
   steps 1 to 3 and compare.
5. Call `nope`, the empty name, `inc.x` and `1inc`; call a source that does
   not parse; call with a cancelled handle.
6. Read each record's owner (the temporal unit below runs in the root crate's integration tests, which hold the ConfigVersion domain package in their test support, `crate::support::config_version_spine::spine::DOMAIN_PACKAGE`, not in `qsl-replay/tests`) in the checked graph for FR-062's RR-1 to RR-17,
   for QSpec TC-198 L07's `post Q` unit and for the unit `language "ix:native" edition "1-draft"; profile v = "quire.model.complete/v1"; profile t = "quire.temporal.infinite-trace/v1"; model Config = "example/config-version" version "1.0.0" digest "sha256-jcs:<digest of DOMAIN_PACKAGE>"; temporal Reaches using t over (c: Config::ConfigVersion) clock "model-steps" on origin { eventually holds(c.versionNumber = 2) }` (the temporal clause `Reaches` of TC-842) (a `temporal-satisfaction` record in `graph().requirements()`, as `tests/it/temporal_negotiation.rs` reads it); the state-graph claim `possible P` is not used.
7. Call `add` of FR-062 RR-3 with the arguments `x`, `y` and then `y`, `x`.
8. Fault arm: in a `qsl-replay` unit test, build a checked package whose
   record map is altered through the crate-internal test support (the same
   access the checked-graph tests use) to hold a `value-validity` record
   with no owner, and then one with an owner `ghost` no function bears, and
   run the association step.

## Expected Results

1. `sq` has three members, request indices 0, 2 and 3 (the `integer.mul`, then
   the ordinals 1 and 2 of the shared `x + 1` node); `inc` has one, index 1;
   no key is in both; every member key is in the package's records; the items
   at those indices carry the members' keys.
2. `id` and `two` have no members and complete; the `sq` calls with `x = 0` and `x = 9` have equal
   members; the Boolean argument refuses `WrongValueKind` at position 0 with
   no members.
3. `incomplete` on `work_units` with the same three members.
4. Equal `package_id` and outcome for each.
5. `MissingDeclaration` four times, `Compile`, `Cancelled`; no members.
6. One owner per `value-validity` record, the function's name; none for the
   `operation-contract` record and none for the `temporal-satisfaction`
   record.
7. Equal calls and equal members for the `add` call in both argument orders.
8. `Fault` with stage `spine-run` and invariant `requirement-owner-known`
   each time, with no partial list.
