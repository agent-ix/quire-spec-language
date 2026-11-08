---
id: TC-1306
title: "Keep the exact protocol number codec in qsl-package"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-511
    type: verifies
---
# TC-1306: Keep the exact protocol number codec in qsl-package

## Description

Verify that the FR-038 codec survives the deletion of `protocol_artifact` at
its qsl-package path with unchanged behavior. Scope: FR-511-AC-2 to FR-511-AC-4
(FR-511-AC-1 is retired with the root re-export).

## Test Procedure

1. Run the TC-117 vectors (zero, ±9007199254740991, ±9007199254740992,
   i64 extrema; 1/3, -1/2, 1/1, 0/1; the refused spellings -0, +1, 01, -01,
   1.0, 1e3, non-ASCII digits, out-of-range values, bare JSON numbers, floats,
   unreduced and nonpositive-denominator rationals; unknown, duplicate and
   missing members and tags) against `qsl_package::number`.
2. Search `qsl-*/src` and the checked-package v2 writer and reader for the
   module path and the profile string.
3. Run `quire matrix` and list the tags of FR-038-AC-1 to AC-5.

Tag the test `#[trace("FR-511-AC-2", "FR-511-AC-3", "FR-511-AC-4", "TC-1306")]`; the step 1 tests also carry the FR-038-AC and TC-117 tags.

## Expected Results

- Step 1: every vector gives the expected string, kind or refusal TC-117 states.
- Step 2: no match.
- Step 3: each of the five criteria has a tagged test.
