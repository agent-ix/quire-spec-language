---
id: TC-1089
title: "The replay facade associates the selected call's requirement items"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: verifies
---
# TC-1089: The replay facade associates the selected call's requirement items

## Description

Verify `run_with_requirements` returns, with the outcome of `run`, exactly the
requirement items whose owner is the selected function, in occurrence-key
order, and refuses as `run` does. Scope: FR-395-AC-1 to FR-395-AC-10.

## Test Procedure

From an integration test that reaches only the `qsl_replay` root, over
fixture K of FR-395:

1. Call `sq` with `x = 3` and `inc` with `x = 3`; read the members, and write
   K's items with `items_from_requirements` over the package's records.
2. Call `id` and `two` with their arguments; call `sq` with `x = 0` and
   `x = 9`; call `sq` with a Boolean argument for `x`.
3. Call `sq` with `x = 3` and `work_units = 0`.
4. Call `run` and `compile_package` over the same inputs for each call of
   steps 1 and 3 and compare.
5. Call `nope`, the empty name, `inc.x` and `1inc`; call a source that does
   not parse; call with a cancelled handle.
6. Check FR-062's RR-1 to RR-17 and the `post Q using V on M::A::remove` unit of TC-198 L07 (FR-104) at the
   checked graph for each record's owner.

## Expected Results

1. `sq` has three members, ascending by key; `inc` has one; no key in both;
   every member key is in the package records; each member's request index
   is the position of its key in the four-record order, and the item written
   at that index has that key.
2. `id` and `two` have no members and complete; the two `sq` calls have
   equal members; the Boolean argument refuses `WrongValueKind` with no
   members.
3. `incomplete` on `work_units` with the same three members.
4. Equal `package_id` and outcome for each.
5. `MissingDeclaration` four times, `Compile`, `Cancelled`; no members.
6. One owner per `value-validity` record, equal to its function; none for the
   `operation-contract` record.
