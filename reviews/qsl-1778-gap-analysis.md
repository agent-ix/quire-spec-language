---
id: SR-15208991
title: "QSL-1778 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1778; native owner reports and FR-052"
review_set: subset
---

## Summary

Ticket QSL-1778; PR quire-spec-language#2448; frozen b80fec47d, owned delta fca596eac...HEAD. Dependency QSL-1777/#2446 remains Draco-owned and was not reviewed as this ticket. No source edits or builds performed. Historical /tmp/QSL-1778-tests.log reports nine tests passed; this is coder evidence, not an independent rerun. Final gate remains outstanding.

Plan completion: not assessed

Manual requirement/test/source mapping for the owned PR slice examined AC12 and AC15-18. All have executable FR-tagged behavioral tests, including opposite/missing rows, duplicate multiplicity, cross-occurrence predecessors, two-build determinism and malformed framing. Identity member omission/payload fabrication has no negative control and is reported in code-review FND-001; paired TC metadata defect is code-review FND-002. No additional independent gap finding. Computed matrix was attempted but cancelled without output; engine-wide matrix coverage is unavailable. This is a scoped manual gap review, not a repository-wide matrix PASS.

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

CONDITIONAL within the stated scope.

## Dispositions

Round 1, reviewed head `9718b750a`.

| FND | outcome | reason |
| --- | --- | --- |

No defect findings in this artifact required disposition; placeholder is not a finding.

## Disposition evidence (round 1)

Disposition round 1 at 9718b750a in the original QSL-1778 worktree only. Static inspection of b80fec47d..9718b750a, no reviewer build or final gate. Canonical parsing now requires all 17 members; payload authority is checked by the existing evidence-owner identity_of encoder shared with evaluation. native_request is shared by evaluation and reading, binding observation/revision and admitted rows without executing temporal positions. The foreign canonical identity control transplants a revised observation identity onto unchanged original bytes and original observation, and must refuse UnknownTag. The coder mutation log /tmp/QSL-1778-fix-canonical-mutation.log shows this control failing at its intended assertion when comparison is disabled; restored log shows nine tests passing. All nine tests now have TC-144 plus criterion tags; TC-144 explicitly includes native owner report procedures 10-13.

Original findings and original verdict are unchanged historical records. Current disposition: both original code-review findings fixed; no new defect in the fix diff. The separate later SR-20261105101067 gap review on Linear is outside this artifact set and remains for the lead to reconcile before merge. The original computed-matrix limitation remains.

## Additional round 1 head reconciliation

Final adopted head `5ed40a131` adds peer fixes after `9718b750a`. Both original findings remain fixed. Two newly added tests omit TC ids; this is a new finding in SR-15208990. The peer review SR-20261105101066/67 is now included in this disposition pass, not superseded. No extra worktree, clone or build was created.
