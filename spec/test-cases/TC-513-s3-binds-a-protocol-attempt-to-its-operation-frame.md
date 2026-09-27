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

Scope: FR-114-AC-1 to FR-114-AC-4.

## Test Procedure

Use the ConfigVersion package (TC-458) and the FR-104-AC-1 unit
(`ParentOrder`, `NoCycle`, `VersionUnchanged`), with the package's `probe`
operation of FR-104-AC-8 and a `pre ProbePre ... on
Config::ConfigVersion::probe`. Add a protocol whose `run` holds
`attempt Update by R on Config::ConfigVersion::attemptUpdate contracts
[VersionUnchanged] as (updated: Boolean) { updated };`.

1. Check and compile the unit; read the checked attempt, the package's
   anchor and frame nodes for `attemptUpdate`, and the requirement records.
2. Remove `VersionUnchanged` and use `contracts []`; check and compile again.
3. Check four variants, one defect each: `on Config::ConfigVersion::missing`;
   `contracts [Absent]`; `contracts [ParentOrder]`; `contracts [ProbePre]`.
4. Over a package where `Sub` specializes `ConfigVersion`, attempt on
   `Config::Sub::attemptUpdate` beside `VersionUnchanged`; compile.

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
- Steps 1, 2 and 4's emitted-node assertions wait on STD-111, as TC-462's do;
  the checked-graph assertions do not.
