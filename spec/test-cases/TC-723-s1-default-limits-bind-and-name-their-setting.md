---
id: TC-723
title: "S1 parses to its default limits and names the setting it reached"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: verifies
---
# TC-723: S1 parses to its default limits and names the setting it reached

## Description

Verify that S1's defaults admit realistic deep chains, that one element
past a default names the S1 setting to raise, and that S1's limits hold
bytes, tokens, nodes and work only.

Scope: FR-256-AC-2, FR-256-AC-3.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Build each chain of TC-722 step 1 to the longest length the default S1
   limits admit, and parse it; then build each one element longer and parse
   it; then parse the longer one again with the setting its outcome names
   raised through FR-255's settings operation.
2. Parse brackets nested 10,000 deep around `1` at the default limits.
   Then, with `s1.nodes` at 200000 and every other S1 limit at its default,
   parse brackets nested to the depth the default token limit admits, and
   then one pair deeper.
3. Set each `qsl_cst::Limits` field through its builder method.

Tag the tests `#[trace("TC-723", "FR-256-AC-2")]`, `#[trace("TC-723", "FR-256-AC-3")]`.

## Expected Results

- Step 1: each longest chain parses; each longer one stops with
  `stage_limit_exceeded` naming `s1.tokens`, `s1.nodes` or `s1.work_units`,
  its configured value and the count reached; each parses once that setting
  is raised.
- Step 2: the 10,000-deep input parses; the token-limited nesting parses,
  and one pair deeper stops with `stage_limit_exceeded` naming `s1.tokens`,
  bound 100000 and the count reached.
- Step 3: the fields are source bytes, tokens, nodes and parser work, and
  no S1 outcome in this case names a depth.

## Status

Planned.
