---
id: SR-20261104190830
title: "Gap analysis of PR #2311: QSL-1641 identity file (FR-531-AC-8, AC-12, AC-13)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2db5b5c3a trial-merged with origin/main 3800efb02; spec/functional/FR-531, FR-517; tests/it/bare_package.rs, tests/it/checked_handoff.rs; ticket QSL-1641"
review_set: subset
---

## Summary

Ticket: QSL-1641. PR: quire-spec-language#2311, head 2db5b5c3a. Method: quoin gap-analysis by hand.

Criteria due in this PR: FR-531-AC-8 (file name, format, determinism), AC-12 (the closed refusals), AC-13 (value encodings, joins, forged joining row), and the temporal use of the supply (record-typed capture on the bare route). The slot-call halves of FR-531-AC-1, AC-3, AC-5, AC-7 and the omitted-supply `MissingIdentityWitness` case belong to part B.

Examined, clean: AC-12 asserts `Malformed(File)`, `Malformed(Unknown)`, `Malformed(Repeated)`, `Format`, and `Unjoined` for an absent node and for a form that does not match its identity, each with the typed member and index (the earlier `is_err()` is gone); AC-13 asserts a non-`declared` unit is `Malformed(ExactIdentity)`, an absent unit node and a mismatched `declared_unit` kind are `Unjoined`, and a forged row that passes the joins is accepted. The supply test shows a record-typed capture on the bare route equals the in-process type.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-8 and AC-13 are checked by `read` then `to_bytes` equalling the file, which pins read and write symmetry but not the literal spelling of an `EffectiveId` chain, a `MemberId` owner or a field row, and the file is not compared with `of_rows` over the source graph's rows; the test does not assert that R's file holds an object or field row. | tests/it/bare_package.rs:253-290 |

## Verdict

Mergeable once A is merged; the single finding is low.
