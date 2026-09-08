---
id: SR-011
title: "failure-domain review of Rust fixture verification"
type: SpecReview
analysis: failure-domain
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
| FND-001 | low | No open failure-policy gap in the audit slice: every invalid/incomplete mode has an explicit error/claim boundary. | FR-012 AC-1 through AC-11 |

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

## Boundary checks

| Boundary | Policy / edge cases |
| --- | --- |
| Fixture file intake | Bounded read; explicit root; canonicalized contained path; reject absolute/traversal/symlink escape; immutable local trees are a precondition. |
| JSON and required fields | Duplicate decoded keys, malformed UTF-8/JSON/scalars and wrong types return errors; no unchecked Value-null equality or panic-based assertion is an oracle. |
| Identity registry | Preserve the historical authority/identity/revision tuple; exercise stale digest and recomputed-digest key conflicts independently. |
| Source/role correspondence | Exact bytes, scalar coordinates, profile/features and role membership are checked; stored output never claims a fresh run. |
| Native syntax | Reuse actual parse API and selected Limits; never evaluate the fixture's authored logical expectations. |
| External producer | Strict producer-language-unapproved refusal; no child process or language fallback. |
| Work topology | File/aggregate/record/depth limits bound trees and loops; no graph evaluator or concurrent shared state is introduced. |

Input mutations are in-memory or isolated test copies; the audit is read-only.
Timeout/process-lifecycle complexity disappears from the disabled external mode
and direct parser call. No fault is swallowed into a partial success summary.

## Reviewed implementation clarification

Specification revision `085dd09` clarifies the fixed role-profile layout and
one-byte growth sentinel before the affected mode implementation. Manifest-controlled locators remain confined to the fixture root. Only the fixed profile.md layout read selects the parent directory, using the same aggregate audit budget. The sentinel detects a violated immutable-input precondition without unbounded allocation.
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
