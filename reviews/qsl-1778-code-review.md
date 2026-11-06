---
id: SR-15208990
title: "QSL-1778 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1778; native owner reports and FR-052"
review_set: subset
---

## Summary

Ticket QSL-1778; PR quire-spec-language#2448; frozen b80fec47d, owned delta fca596eac...HEAD. Dependency QSL-1777/#2446 remains Draco-owned and was not reviewed as this ticket. No source edits or builds performed. Historical /tmp/QSL-1778-tests.log reports nine tests passed; this is coder evidence, not an independent rerun. Final gate remains outstanding.

Rust lane folded into code-review. Traced owner framing, immutable report accessors, native atom valuation projection, support multiplicity, result identity parsing and predecessor admission. No unsafe or CI workflow changes; no new vendored temporal evaluator: the adapter invokes the existing evidence engine.

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
| FND-001 | medium | The strict report reader accepts a fabricated result identity containing only tag 17 with a zero-length payload; this invented input identity also bypasses its unchanged-input predecessor check. | qsl-eval/src/temporal/evidence/identity.rs:177-215 | implementation-bug-despite-evidence |
| FND-002 | low | All nine new native owner tests omit a TC id from their trace attributes, contrary to the required paired TC and criterion trace form. | tests/it/native_owner_report.rs:118-752 | implementation-bug-despite-evidence |

## Verdict

FAIL within the stated scope.

## Failure scenario and repair

FND-001: Replace field 10 of otherwise valid report bytes by `[17, 0, 0, 0, 0, 0, 0, 0, 0]`, adjusting the field length. The parser accepts it because its only completeness condition is `previous == 17`. Original reports have no further identity check; corrections compare this invented input identity with the predecessor and treat it as changed. Require complete valid identity members and check encoded evaluation input against the supplied subject/request/observation authority before granting correction authority. Add malformed identity and invented changed-input controls; reusing the owner identity encoder avoids a second definition.

FND-002: Add a suitable TC artifact covering native AC15-18 and native AC12 controls, or correctly extend an owning TC, then use paired TC/FR attributes. Existing TC-144 explicitly excludes `src/temporal`, so simply attaching TC-144 without updating its scope would misrepresent the test binding.

## Dispositions

Round 1, reviewed head `9718b750a`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Complete canonical members and evidence-owner request/observation identity equality; malformed and foreign canonical identities refuse. |
| FND-002 | fixed | All nine native owner tests now pair TC-144 with criteria; owning procedure includes the native API. |
| FND-003 | still-open | Two newly added tests still omit TC-144; add the owning TC id. |

## Disposition evidence (round 1)

Disposition round 1 at 9718b750a in the original QSL-1778 worktree only. Static inspection of b80fec47d..9718b750a, no reviewer build or final gate. Canonical parsing now requires all 17 members; payload authority is checked by the existing evidence-owner identity_of encoder shared with evaluation. native_request is shared by evaluation and reading, binding observation/revision and admitted rows without executing temporal positions. The foreign canonical identity control transplants a revised observation identity onto unchanged original bytes and original observation, and must refuse UnknownTag. The coder mutation log /tmp/QSL-1778-fix-canonical-mutation.log shows this control failing at its intended assertion when comparison is disabled; restored log shows nine tests passing. All nine tests now have TC-144 plus criterion tags; TC-144 explicitly includes native owner report procedures 10-13.

Original findings and original verdict are unchanged historical records. Current disposition: both original code-review findings fixed; no new defect in the fix diff. The separate later SR-20261105101067 gap review on Linear is outside this artifact set and remains for the lead to reconcile before merge. The original computed-matrix limitation remains.

## Additional round 1 head reconciliation

Final adopted head `5ed40a131` adds peer fixes after `9718b750a`. Both original findings remain fixed. Two newly added tests omit TC ids; this is a new finding in SR-15208990. The peer review SR-20261105101066/67 is now included in this disposition pass, not superseded. No extra worktree, clone or build was created.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | low | Two tests added by the peer fix omit TC ids from their trace attributes, reintroducing the paired TC/criterion metadata violation. | tests/it/native_owner_report.rs:877,904 | implementation-bug-despite-evidence |

## Round 1 new-finding disposition

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | still-open | Add TC-144 to reader_refuses_a_result_identity_of_another_clause_or_occurrence and subject_and_evaluation_refuse_foreign_occurrence_revision_and_anchor. |
