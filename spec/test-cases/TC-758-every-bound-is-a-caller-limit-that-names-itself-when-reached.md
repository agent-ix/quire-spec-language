---
id: TC-758
title: "Every bound is a caller limit that names itself when reached"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: verifies
---
# TC-758: Every bound is a caller limit that names itself when reached

## Description

Verify that each limit a lifecycle operation enforces comes from the caller's limits value, and that reaching it names the limit, its value and the field that raises it.

Scope: FR-277-AC-1, FR-277-AC-2, FR-277-AC-3.

## Test Procedure

1. For each field of the limits types of `parse`, `format`, `select`, `check`, `package`, `execute`, `analyze`, `monitor`, `replay`, `inspect` and `render`, run the operation over its TC-755 step 6 input with default limits and read the counter reached for that field from the outcome's accounting.
2. Rerun with that field set to one below the counter.
3. Rerun with that field set to the counter.
4. Generate a source whose body is a 100,000-term sum and `check` it with `s3.nodes` raised to its node count, on a thread with the platform's default stack.
5. `check` the same source with `s3.nodes` one below its node count.
6. `execute` a function with `work_units` one below what it needs.

Tag the tests `#[trace("TC-758", "<AC id>")]`.

## Expected Results

- Step 1: each run succeeds.
- Step 2: each run returns `LimitExceeded` whose limit kind, configured value and limits-field name are that field's; `execute` returns the outcome `Incomplete` naming the counter; `analyze` settles each open item `incomplete` with that limit as its cause.
- Step 3: each run succeeds.
- Step 4: `check` succeeds, and the thread does not overflow its stack.
- Step 5: `check` returns `LimitExceeded` naming `s3.nodes`; no outcome names a depth.
- Step 6: the outcome `Incomplete` carries the limit kind, the configured value, the counter at the failed charge and the `work_units` field name.
