---
id: SR-1830002
title: "gap analysis of PR #1830 (QSL-518, partial): FR-345 generated layering corpus"
type: SpecReview
analysis: gap-analysis
date: 2026-10-29
scope: "agent-ix/quire-spec-language; PR #1830 at ebd5adea8 against FR-345-AC-4, 6, 7, 11, 13 and the FR-345 amendment, compared with FR-340's #1815 precedent"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1830002: gap analysis of PR #1830 (QSL-518, partial)

## Summary

These are covered over the generated corpus (TC-868):
- AC-4 and AC-7: every edge, plus a seeded regression per edge and a parent admitting its distinguishing unit.
- AC-6, for the five untimed layers.
- AC-11: E6, and the missing temporal entries.
- AC-13: the `expected_run` mismatches, and the CLI seam with a stub executable passed directly.

AC-9, AC-10, AC-12 and the temporal witness half are declared remaining (QSL-1200/1201, FR-230) and are not claimed.

The amendment follows FR-340's generated-corpus wording (#1815): the corpus is generated into a temporary directory, digests are computed at run time, nothing is committed, and the directory is removed at exit.

Gap:
- **FND-001.** FR-340's precedent backs its generation clauses with acceptance criteria:
  - AC-6: generated into a temporary directory;
  - AC-8: an edited model file regenerates with the new digest;
  - AC-9: a root holding only the model file.

  FR-345's new "Where it runs" clauses, "SHALL read no other file and commit nothing" and "compute each model's digest from its bytes when it runs", have no AC. The test that exercises them, `the_corpus_is_generated_from_the_model_file_alone`, is traced to TC-868 alone. It also compares only `witness-state-core/unit.native`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-345's generation clauses have no AC, unlike FR-340-AC-8/9. Add an AC (a root holding only the model file generates the same corpus, and an edited model gives the new digest), and tag `the_corpus_is_generated_from_the_model_file_alone` with it. | FR-345 "Where it runs", FR-340-AC-8, FR-340-AC-9 |

## Verdict

Mergeable. The finding is low.
