---
id: SR-014
title: "evidence review of Rust fixture verification"
type: SpecReview
analysis: evidence
scope: "FR-012, NFR-005, IT-004 and associated lineage/producer-gate amendments"
review_set: all
evaluated_revision: "11a9128"
---

## Summary

The bounded Rust audit specification is reviewed under the owner-selected full
analysis set. Dispositions preserve original fixture evidence and explicitly
withhold fresh unapproved producer qualification.

## Verdict

**PASS** for the specified audit/refusal slice. This is the performed review
result, not an invented owner acceptance of other contracts or publication.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | NFR-005-M-1: advisor recommends performance-benchmarking only for quantified-threshold; reviewer disposition retains inspection for the executable/CI inventory. | NFR-005-M-1; data/advice.json |
| FND-002 | low | NFR-005-M-2: advisor recommends performance-benchmarking only for quantified-threshold; reviewer disposition retains integration-testing of the real Rust executable with external runtimes unavailable. | NFR-005-M-2; data/advice.json |
| FND-003 | low | NFR-005-M-3: advisor recommends performance-benchmarking only for quantified-threshold; reviewer disposition retains negative-abuse-testing of the producer refusal. | NFR-005-M-3; data/advice.json |

## Scope and provenance

This follow-up reviews FR-012/NFR-005/IT-004 and the associated NFR-002,
IT-001, US-004, StR-001 and master lineage amendments at `11a9128`. It does not
reopen or accept the remaining native evaluator/shared-interface scope.
The owner's existing selection is base plus all seven Quoin analyses; the
optional gap-analysis semantic comparison remains declined. Agent A performed
the analyses sequentially under the assignment's no-extra-agents rule.

The installed Quoin 0.20.0 specify/spec-review skills and catalog packs were
used. Quoin 0.23.1, Quire CLI 0.31.0 / engine 0.46.0 and the same manifests as
the [baseline review](../base.md) apply. The new authoring pack resolved org
agent-ix from this Git remote. The original 35 scoped documents, including
baseline reviews, were grammar-clean after this specification change. The six
known installed registry errors remain external and are not an error-free
validation signoff. No source, test or CI implementation changed before this
review.

The user's Rust-remediation direction governs owned verification logic. The
[LC01 campaign audit](https://github.com/agent-ix/quire-spec-language/issues/2#issuecomment-5579283030)
also requires a separate disposition for the existing TypeSpec/Node producer.
FR-012 explicitly refuses that mode; this review does not approve its language.
The historical producer result remains pinned evidence, not a fresh execution.

## Deterministic advice and reviewer judgment

The actual `quoin advise --repo <language-root> --json` and `quire coverage
--scope <language-root> --json` runs are retained under data/. The eleven new
FR-012 obligations match Test-class recommendations with no inconclusive or
uncatalogued row. All three NFR-005 rows report explicit-method mismatches,
solely from quantified-threshold -> performance-benchmarking. The authored
catalog methods remain in the requirement; these reviewer dispositions do not
claim human acceptance or evidence discharge. The owner explicitly requested
judgment beyond the incomplete advisor.

Tests use independently authored malformed JSON/path controls, real fixture
files, byte mutations, recomputed-digest collisions and actual parser outcomes.
The matrix includes generated malformed/path/budget cases where useful, not
only a re-execution of the original Python self-test. Follow-up fuzzing and
mutation testing can assess the strict JSON/identity checks; neither was already
run by this review. There is no concurrent shared state requiring Loom.

Inspection discharges the source/CI-language inventory, and actual Rust command
tests discharge the runtime outcomes. No source tag is treated as a human
inspection record. No duplicate test implementation in another language is
introduced to validate the Rust replacement.

## Reviewed implementation clarification

Specification revision `085dd09` clarifies the fixed role-profile layout and
one-byte growth sentinel before the affected mode implementation. AC and metric statements/methods are unchanged, so the recorded deterministic obligation advice still applies. TC-003 checks the fixed selected profile and TC-008 checks accepted/over-budget behavior; no new benchmark obligation is inferred.
The scoped review result remains PASS; no additional blocking finding was
identified. Existing owner/external qualification gates remain unchanged.

## Canonical trace marker clarification

Reviewed NFR-005 clarification at `86c47c7`: the installed module declares
`rust-trace-attribute` as canonical, while doc-comment/name tags are legacy.
New audit tests use the existing shared ix-trace-rs macro at
2ce4ebf47f726b9d76388220545cd0abda8a5cfb, retaining AGPL-3.0-or-later.
The macro checks argument shape; actual Quire binding remains a separate gate.
This resolves the older default skill convention without inventing a grammar
or a marker crate. No obligation/method identity changed, and the scoped
review result remains PASS.
