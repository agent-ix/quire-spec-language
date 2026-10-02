---
id: SR-1192
title: "Spec review of quire-spec-language PR #588 commit ba70e5a5 (QSL-469): ADR-016 §9 G rows cite FR-300 to FR-303"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6ffdca6fe18c174773ef1b36e335bcc4cd7a33f2; PR #588 commit ba70e5a5 against main 2ece5712"
review_set: subset
---
# Spec review of quire-spec-language PR #588 commit ba70e5a5 (QSL-469)

## Summary

Ticket: QSL-469 (the SM4 item). Commit ba70e5a5 rewrites the acceptance and test cells of ADR-016 §9 rows G-2a, G-2b, G-2c, G-7 and G-8 to cite FR-300 to FR-303. Checked each cited AC and TC against the FR files and spec/tests.md: G-2a cites FR-300-AC-1 and AC-2 with TC-790, G-2b cites FR-300-AC-3 with TC-791, G-2c cites FR-301-AC-1 and AC-2 with TC-792, and G-8 cites FR-303-AC-1 to AC-4 with TC-796; all exist and the tests.md rows trace those ACs. G-5 and G-9 already cite FR-085, FR-084 and FR-086.

Examined:
- ADR-016 G-2a (examined)
- ADR-016 G-2b (examined)
- ADR-016 G-2c (examined)
- ADR-016 G-7 (examined)
- ADR-016 G-8 (examined)
- FR-302-AC-6 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | G-7 cites FR-302-AC-1 to AC-6 but its Test cell names only TC-793 to TC-795. FR-302-AC-6 (dispatch inside a precondition runs the most-specific body) is verified by TC-809, which the row leaves out. Write "TC-793 to TC-795, TC-809". | spec/decisions/ADR-016-state-model-finite-execution-mapping.md:400 |

## Verdict

One low finding: G-7's Test cell omits TC-809, which verifies FR-302-AC-6. Otherwise right. Mergeable once FND-001 is fixed.
