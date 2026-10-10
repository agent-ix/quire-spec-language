---
id: TC-737
title: "Replay passes every stage limit through and names the setting it reached"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: verifies
---
# TC-737: Replay passes every stage limit through and names the setting it reached

## Description

Verify the request's `stage_limits` map, its decode refusals and the
replay byte bound.

Scope: FR-263-AC-2, FR-263-AC-3.

## Test Procedure

1. For each `s1.*` and `s3.*` setting, replay a request whose source
   reaches that limit at the bound its `stage_limits` entry gives; then raise
   the entry to fit.
2. Replay a request with no `stage_limits` entries.
3. Decode requests whose `stage_limits` holds `s9.nodes`, and one holding
   `replay.input_bytes`.
4. Set `replay.input_bytes` to `B` and replay a request of `B + 1` bytes;
   then raise it through the library replay entry and through
   FR-255's settings operation given `replay.input_bytes=<B + 1>`.

Tag the tests `#[trace("TC-737", "FR-263-AC-2")]`, `#[trace("TC-737", "FR-263-AC-3")]`.

## Expected Results

- Step 1: each refuses with the recompile stage-limit refusal naming the
  limit, bound, count and setting; each raised request recompiles.
- Step 2: it recompiles at the published defaults.
- Step 3: each refuses at decode naming the entry.
- Step 4: `BoundExceeded`, bound `B`, actual `B + 1`, setting
  `replay.input_bytes`; each raised run decodes.

