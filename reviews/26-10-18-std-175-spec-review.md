---
id: SR-12890
title: "Base spec review of STD-175 zero-instance hyperproperty items (QSL PR #987)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@944699b543e42ba482bc3cd405281b27dcf10ba9; PR #987 diff against origin/main 49bcf9a: FR-125, FR-172, FR-182, TC-607"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: reviews
---
# SR-12890: Base spec review of STD-175 (QSL PR #987)

## Summary

Ticket: STD-175. Review set: base, integrity (SR-12891) and
failure-domain (SR-12892). The paired QSpec PR #281 is reviewed in
SR-12893 to SR-12895.

The PR applies the planner default: a hyper clause with zero instances is
`NoInstance`, V-6 `inconclusive`, basis `unsettled`, before any state is
explored. FR-182-AC-9 and TC-607 expect `RequiresBound` first, but the
FR-125 statement behind that order does not yet cover hyper clauses
(SR-12891 FND-004).
The new FR-182-AC-9 id is free on main and in open PRs #985 and #986. Neither
of those touches FR-125, FR-172, FR-182 or TC-607. The checklist pass found
the ID formats valid and the AC traced to TC-607. `quire validate` exits 0
on the four changed files.

## Verdict

No high findings. One low finding here. The medium findings are in SR-12891
and SR-12892.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-607 step 7 names only a domain, `behaviours (v: V::Vault)`, and no clause, while TC-607's fixtures are the outcomes of TC-601, TC-602, TC-604 and TC-598. A coder must guess which clause to instance and must run instancing, not settle a fixture outcome. Name the clause (for example ADR-023 §1 NonInterference) and the subject the step uses. | spec/test-cases/TC-607-hyper-verdicts-settle.md:27 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | FR-182-AC-9 and TC-607 step 7 cite "§8.2's `SavesPower`", but SavesPower is declared in ADR-023 §1 (line 143). §8.2 is `Opaque` over the single alias `V`. A reader who follows the citation finds a one-alias clause and loses the product case, which the AC's `(d: S::Device, e: N::Device)` names only in passing. Cite §1. | spec/functional/FR-182-settle-a-hyper-verdict.md:133; spec/test-cases/TC-607-hyper-verdicts-settle.md:27 |

## Dispositions

Round 1, reviewed at 470eeeb35 (merge of origin/main 7915615f1).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |

Round 2, reviewed at 6b569e4ada6b8615a75ed82fa1807d481fbe278a.
FR-182-AC-9 and TC-607 step 7 now cite ADR-023 §1 for SavesPower. Repeating
"ADR-023 §1's" after NonInterference reads correctly, because both clauses
are declared in §1.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | - |
