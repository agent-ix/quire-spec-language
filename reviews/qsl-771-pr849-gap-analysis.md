---
id: SR-9134
title: "Gap analysis of quire-spec-language PR #849: population_v2_fixtures CLI"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cc7aeef5b7e30a065ebe94fd677c2a20693704a6; src/population_fixtures.rs, examples/population_v2_fixtures.rs, qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #849

## Summary

Ticket: QSL-771. Planless, diff-scoped. Plan completion: not assessed.
Oracle: the ticket's three acceptance criteria, FR-093-AC-19 and TC-416
step 11.

Examined:
- QSL-771 AC1 (shared emitter path, no second copy, one file per case):
  four case files plus the domain package document are written; the corpus
  still holds an independent `ModelPopulation` builder (examined)
- QSL-771 AC2 (written bytes admitted by the IR v2 reader and equal to the
  corpus's bytes): admission is asserted through `read_import_view`; byte
  equality is asserted only against the module's own `fixture()` (examined)
- QSL-771 AC3 (new output directory, non-zero exit with a message):
  `an_existing_output_path_is_refused_and_left_untouched` and
  `an_output_path_under_a_missing_parent_is_a_typed_io_refusal` cover the
  library; the example prints and exits 1 (examined)
- FR-093-AC-19 / TC-416 step 11 population cases N = 0, 1, 3, [2, 5]: still
  run under `make ci` via the `all-features-workspace` lane; the N = 3 corpus
  row still runs in the default lanes (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL-771 AC1/AC2: the written N = 3 file is not the corpus's bytes (region end 749 vs the corpus row's 66), and no test compares the written bytes with the corpus's emission; see SR-9133 FND-001 | src/population_fixtures.rs:172-180, qsl-package/src/emit/tests/admission_corpus.rs:211-226 |
| FND-002 | medium | TC-416 step 11 ("decode each checked node through IR's node kinds") lost its only test for N = 0, 1, [2, 5]: the moved test's `CheckedNodeKind` count is gone; see SR-9133 FND-002 | src/population_fixtures.rs:284-313 |

## Verdict

**FAIL: 2 blocking findings** (both also in SR-9133). AC3 is met. No
untraced production code: every new item carries `FR-093-AC-19`/`TC-416`
tests. No stub or tautology beyond the self-comparison in FND-001.
