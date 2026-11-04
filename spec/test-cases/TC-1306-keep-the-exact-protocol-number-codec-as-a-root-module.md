---
id: TC-1306
title: "Keep the exact protocol number codec as a root module"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-511
    type: verifies
---
# TC-1306: Keep the exact protocol number codec as a root module

## Description

Verify that the FR-038 codec survives the deletion of `protocol_artifact` at
its new path with unchanged behavior. Scope: FR-511-AC-1 to FR-511-AC-4.

## Test Procedure

1. Compile a downstream crate that imports each of the seven names from
   `quire_spec_language::number` and compares `NUMERIC_PROFILE` with the
   literal `quire.protocol.numeric/1`; compile a second crate importing
   `protocol_artifact::number` and require a compile error.
2. Run the TC-117 vectors (zero, ±9007199254740991, ±9007199254740992,
   i64 extrema; 1/3, -1/2, 1/1, 0/1; the refused spellings -0, +1, 01, -01,
   1.0, 1e3, non-ASCII digits, out-of-range values, bare JSON numbers, floats,
   unreduced and nonpositive-denominator rationals; unknown, duplicate and
   missing members and tags) against the new path.
3. Search `qsl-*/src` and the checked-package v2 writer and reader for the
   module path and the profile string.
4. Read every `pub use` of `src/number.rs` and require that the set of re-exported items is exactly the seven names; a planted eighth item fails (TC-256 step 3 runs the same scan across the root crate).
5. Run `quire matrix` and list the tags of FR-038-AC-1 to AC-5.

Tag the test `#[trace("FR-511-AC-1", "FR-511-AC-2", "FR-511-AC-3", "FR-511-AC-4", "TC-1306")]`; the step 2 test also carries the FR-038-AC and TC-117 tags.

## Expected Results

- Step 1: the first crate compiles and the second does not.
- Step 2: every vector gives the expected string, kind or refusal TC-117 states.
- Step 3: no match.
- Step 4: the re-exported set is exactly the seven items.
- Step 5: each of the five criteria has a tagged test.
