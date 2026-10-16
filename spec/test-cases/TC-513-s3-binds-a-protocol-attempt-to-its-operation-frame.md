---
id: TC-513
title: "S3 binds a protocol attempt to its operation's one anchor and frame"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: verifies
---
# TC-513: S3 binds a protocol attempt to its operation's one anchor and frame

## Description

Verify that an `attempt` binds its operation's anchor and frame by identity,
adds no second frame node or frame record, and checks its `contracts` list.

Scope: FR-114-AC-1 to FR-114-AC-5.

## Test Procedure

Use the ConfigVersion package (TC-458) and the FR-104-AC-1 unit
(`ParentOrder`, `NoCycle`, `VersionUnchanged`), with the package's `probe`
operation of FR-104-AC-8 and a `pre ProbePre ... on
Config::ConfigVersion::probe`. Add a protocol whose `run` holds
`attempt Update by R on Config::ConfigVersion::attemptUpdate contracts
[VersionUnchanged] as (updated: Boolean) { true };`. Its role is `role R on
Config::ConfigVersion;`, and every other body in the protocol is a bare
Boolean literal, so that FR-113 Status's content check passes.

1. Check and compile the unit; read the checked attempt, the package's
   anchor and frame nodes for `attemptUpdate`, and the requirement records.
2. Remove `VersionUnchanged` and use `contracts []`; check and compile again.
3. Check four variants, one defect each: `on Config::ConfigVersion::missing`;
   `contracts [Absent]`; `contracts [ParentOrder]`; `contracts [ProbePre]`.
4. Over a package where `Sub` specializes `ConfigVersion`, attempt on
   `Config::Sub::attemptUpdate` beside `VersionUnchanged`; compile.

5. Add a process `Flow` declaring `go` with an empty frame (TC-460 row 13's
   package): check `attempt U by R on Config::Flow::go contracts []`, and
   again with `contracts [Absent]`.

Tag the tests `#[trace("TC-513", "FR-114-AC-n")]`.

## Expected Results

- Step 1: the checked attempt holds the identities of `attemptUpdate`'s
  anchor node, its frame node (`modifies` exactly `versionNumber`) and
  `VersionUnchanged`'s clause node. The package holds one anchor and one
  frame for `attemptUpdate`, and one frame `operation-contract` record.
- Step 2: still one anchor, one frame and one frame record.
- Step 3: `missing_declaration`/`missing-name` at `missing`; the same at the
  entry `Absent`; `wrong_snapshot`/`wrong-anchor` at `ParentOrder` and at
  `ProbePre`, each naming both anchors.
- Step 4: one anchor and one frame, context `ConfigVersion`.
- Step 5: `invalid_model_binding`/`malformed-declaration` at `Flow` both
  times.
- Steps 1, 2 and 4's emitted-node assertions wait on STD-111, as TC-462's do;
  the checked-graph assertions do not.
