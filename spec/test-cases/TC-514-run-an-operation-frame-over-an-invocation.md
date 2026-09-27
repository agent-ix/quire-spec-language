---
id: TC-514
title: "The spine run entry checks an invocation against its operation frame"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: verifies
---
# TC-514: The spine run entry checks an invocation against its operation frame

## Description

Verify `run_clause`'s `Frame` selection: a frame-respecting invocation
succeeds, a change outside the frame is a violation with its frame witness,
and stale or
missing inputs refuse before evaluation.

Scope: FR-115-AC-1 to FR-115-AC-5.

## Test Procedure

Build `ClauseRunRequest`s from FR-108's ConfigVersion unit, package and
invocation fixtures, in memory, each with selection `Frame { operation:
Config::ConfigVersion::attemptUpdate, invocation: ... }` unless stated.

1. changed-version.
2. forbidden-parent-change; a post snapshot that adds object `c2` to
   `config_history`; an invocation declaring `created: [child]`.
3. changed-version with an expected `package_id` from another unit; with its
   invocation bytes edited after the digest was taken; with another
   `revision` label; with another `model` digest; with `operation` `probe`.
4. `operation: Config::ConfigVersion::missing`; a package operation that no
   clause or attempt names; changed-version with its pre snapshot removed
   from the provision.
5. changed-version twice.

Tag the tests `#[trace("TC-514", "FR-115-AC-n")]`.

## Expected Results

- Step 1: `evaluate`, `success`, `truth: true`, exit 0; provenance holds the
  frame node's identity and the three document identities and digests.
- Step 2: `evaluate`, `violation`, `truth: false`, exit 10, with a frame
  witness naming `child`, `parent` and the frame's `modifies`, cause
  `frame_violation`/`unauthorized-change`; `violation` with a witness naming
  the creation of `c2`; `evaluate`, `refusal`,
  `population_delta_mismatch`/`delta-disagreement`, exit 20.
- Step 3: `compile`, `stale_dependency`, naming both; `admit`,
  `stale_dependency`/`byte-digest-mismatch`; `admit`, `stale_dependency`/
  `revision-mismatch`; `admit`, `invalid_model_binding`/
  `wrong-model-selection`; `admit`, `wrong_snapshot`/`wrong-invocation`.
  None reaches `evaluate`.
- Step 4: `select`, `missing_declaration`/`missing-name`, twice; `admit`,
  `incomplete`, `unavailable_observation`.
- Step 5: equal reports, including usage.
