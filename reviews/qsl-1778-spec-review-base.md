---
id: SR-15208992
title: "QSL-1778 spec-review-base"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; QSL-1778; native owner reports and FR-052"
review_set: subset
---

## Summary

Ticket QSL-1778; PR quire-spec-language#2448; frozen b80fec47d, owned delta fca596eac...HEAD. Dependency QSL-1777/#2446 remains Draco-owned and was not reviewed as this ticket. No source edits or builds performed. Historical /tmp/QSL-1778-tests.log reports nine tests passed; this is coder evidence, not an independent rerun. Final gate remains outstanding.

Direct base review of the FR-052 explanatory amendment and AC12/15-18 context: the checked subject is tied to an actual admitted occurrence, valuation authority and request ledger; sole temporal semantics owner remains the existing engine. No hash/digest/pin or cross-repository copied semantics added. No new relationship edges, verification-method allocations, architecture decisions or application UI. No applicable AssuranceProfile found; direct review used, no enforced workflow or human approval gate invoked.

## Examined Scope

```yaml
scope:
- id: FR-052-AC-12
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: Where the route accepts a correction relation, a valid correction names
    its exact direct predecessor and changed evaluation input; self, non-direct, cross-obligation,
    cross-occurrence (a predecessor of another declaration occurrence of the same
    checked declaration) or identity-mismatched predecessors refuse, and the predecessor
    result remains unchanged; on the native `Report` route the predecessor is named
    by FR-043 result identity. This criterion does not authorize bridge predecessor
    or supersession lineage, which Contract IR owns.
- id: FR-052-AC-15
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: An evaluation through `temporal::evaluate_v2` returns a `Report` whose
    subject (package identity and version, clause `CheckedNodeId`, occurrence key,
    source-map entry), observation and revision references, truth, settlement, support
    and typed outcome equal those of the evaluation; two occurrences of one declaration
    sharing a node return reports differing in the occurrence key.
- id: FR-052-AC-16
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: Every support entry of a report names a trace position ordinal and a valuation-evidence
    row key present in the admitted evidence; for an evaluation with three identical
    rows at one position and one row at another, the support relation lists the rows
    with that multiplicity, and no support entry exists for a position or row the
    evaluation did not admit.
- id: FR-052-AC-17
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: '`from_owner_bytes(to_owner_bytes(r), subject, observation)` returns a
    report equal to `r`, including support multiplicity and predecessor, and `to_owner_bytes`
    is byte-identical across two calls and two builds of the same evaluation.'
- id: FR-052-AC-18
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: 'Each single mutation of valid owner bytes is refused with the named `ReportRefusal`
    and returns no report: an unknown version or format tag (`UnsupportedVersion`),
    a prefix of the bytes (`Truncated`), one appended byte (`TrailingBytes`), an unknown
    outcome tag (`UnknownTag`), two members swapped (`OutOfOrder`), a different supplied
    subject, occurrence key or observation reference (`BindingMismatch` naming the
    dimension), and a support entry naming a row or position the report does not hold
    (`SupportNotInTrace`).'
```

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | implementation-bug-despite-evidence |

## Verdict

PASS within the stated scope.
