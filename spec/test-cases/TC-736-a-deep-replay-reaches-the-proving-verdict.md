---
id: TC-736
title: "A deep source and value replay to the proving run's verdict"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: verifies
---
# TC-736: A deep source and value replay to the proving run's verdict

## Description

Verify that replay recompiles a 100,000-deep source and decodes a
100,000-deep assignment under the request's limits.

Scope: FR-263-AC-1.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Compile a function whose body is a 100,000-term sum over a recursive list
   parameter under raised `s1.*` and `s3.*` settings, and record a
   counterexample whose value is a 100,000-long recursive list.
2. Replay it with those settings in the request's `stage_limits`.

Tag the tests `#[trace("TC-736", "FR-263-AC-1")]`.

## Expected Results

- Step 2: the replay reaches the proving run's verdict, and its recompiled
  `package_id` equals the request's.

## Status

Planned.
