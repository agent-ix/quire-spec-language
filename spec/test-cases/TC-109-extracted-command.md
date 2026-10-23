---
id: TC-109
title: "Run a selected Markdown clause through the spine clause runner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: verifies
---
## Description

Integration, P1, in `qsl-replay`'s tests built with
`cargo test -p qsl-replay --features quire-extraction`: extract licensed
generated Markdown through `qsl_source::extract` and run the extracted body
through `qsl_replay::spine::run_clause` over FR-108's fixtures. Scope:
FR-031-AC-1 to FR-031-AC-4.

## Test Procedure

1. Embed FR-108's unit in one `ix:native` fence of an LF document and of a
   CRLF document; extract each and run healthy-parent, violating-parent and
   missing-model.
2. Extract with a clause id the document does not hold; with an extraction
   Quire reports unavailable; with an original whose bytes differ from its
   verified digest; then run a body taken from an `ix:formal` fence.
3. Extract with an original byte limit one below the document's size and a
   line limit one below its line count; run healthy-parent with `work_units`
   0; repeat each with default limits.
4. Read step 1's healthy report's source; run it again with an expected
   `package_id` taken from compiling another unit.

Tag the tests `#[trace("TC-109", "FR-031-AC-n")]`, step n for AC-n.

## Expected Results

- Step 1: `success`, exit 0, with the `package_id` of compiling the body
  alone, for LF and CRLF alike; `violation`, exit 10; `compile`,
  `missing_import`/`missing-selection`, exit 20, no `package_id`.
- Step 2: each of the first three ends with FR-030's error, retaining the
  original identity, the selection and Quire's outcome, with no compile and
  no truth; the `ix:formal` body refuses `unknown_language`, exit 20.
- Step 3: each limited run reports incomplete; each default run succeeds.
- Step 4: the source names the body identity with the extracted body's
  digest and the original identity with its digest; the second run refuses
  `stale_dependency`, naming both identities.
