---
id: SR-010
title: "base review of Rust fixture verification"
type: SpecReview
analysis: base
scope: "FR-012, NFR-005, IT-004 and associated lineage/producer-gate amendments"
review_set: all
evaluated_revision: "11a9128"
---

## Summary

Manual CI addendum: specification revision `1649ef7` was reviewed before
workflow changes. The owner selected local execution during stabilization. NFR-002 defines the event allowlist and IT-004-SC-05 makes inspection observable. No review choice or authorization is inferred from a timeout or absent run.
The bounded review remains PASS. Both changed artifacts validate with the six
previously recorded installed-registry diagnostics; no clean-registry claim is made.

The bounded Rust audit specification is reviewed under the owner-selected full
analysis set. Dispositions preserve original fixture evidence and explicitly
withhold fresh unapproved producer qualification.

## Verdict

**PASS** for the specified audit/refusal slice. This is the performed review
result, not an invented owner acceptance of other contracts or publication.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No open blocking finding in the bounded Rust-audit contract after the dispositions below. Existing compiler findings and producer-language approval remain outside this acceptance slice. | FR-012; NFR-005; IT-004 |

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

## Checklist and dispositions

FR-012 specifies one observable audit result with mode-specific acceptance
cases. Inputs, errors, language boundary, byte/record limits and claim limits
are explicit. It traces to US-004 and StR-001; NFR-005 constrains it and is
referenced back by it. IT-004 supplies real command/file boundaries.

The initial remediation design raised three issues, all resolved before this
reviewed specification revision: a Rust wrapper must not approve Node execution
(model-producer now refuses); moving syntax checks into Rust must not create a
second parser (the existing parser library is reused); malformed fields and
optimized builds must not disable assertions (fallible checks and typed errors
replace Python assertions). FR-012 AC-6/9/10 also specify panic and input limits.

The existing NFR-002 optional-tool ambiguity is narrowed: fresh producer use
stays unavailable, with IT-001 recording its owner gate. The only native runtime
dependency is the reviewed Rust package. Audit dependency grants are recorded
before adoption. No separate crate or producer implementation is justified.

The [test matrix](../../tests.md) and [Plan-001](../../../plan/Plan-001-rust-fixture-audits/plan.md)
record the scoped implementation/test work. They do not claim all old FRs are
covered. Readiness is satisfied for Rust audit implementation and its refusal
mode; owner approval is still required before enabling fresh external production.

## Reviewed implementation clarification

Specification revision `085dd09` clarifies the fixed role-profile layout and
one-byte growth sentinel before the affected mode implementation. The fixed parent profile read preserves the selected historical packet layout; one sentinel byte explicitly bounds detection of growth. No additional mode or executable language is introduced.
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
