---
id: SR-637
title: "QSL-266 EARS conformance review of per-operation requirement records"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@64ee12700cd66bb17767a8e9090cbca114308364; spec/functional/FR-057-admit-shared-capability-kinds.md; spec/functional/FR-062-implement-checked-family-contract.md; spec/functional/FR-075-compute-candidates-from-registered-backends.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
---

## Summary

Ticket: QSL-266. This review covers the new and edited requirement
statements in FR-057, FR-062 and FR-075. The ADR prose in ADR-012 and
ADR-014 is out of this lens's scope.

The engine check (`quire validate --summary`) reports no `[ears:*]` or
`[quality:*]` warning in any changed file. The repo total is 884/887
documents grammar-clean, and all four findings it reports are in files
this diff does not touch.

Six new or edited SHALL statements were read for semantic conformance:
- FR-062 part 4;
- FR-062 "`check` SHALL record";
- FR-075's builder SHALLs (three);
- FR-057's claim-form SHALL.

Each has a named subject and a concrete response. The defects are small
and wording-only:
- one unwanted-condition rule is written as prose, not as `If … then … SHALL`;
- the builder SHALL uses the vague verb `provide`;
- three sentences state what is absent rather than what is.

The substantive contradiction in FR-057's claim-form SHALL is recorded
under integrity (SR-638 FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The one failure rule of the new section is written as prose with no SHALL: "No requirement record is dropped: a scalar operation application occurrence that `check` cannot key is `KeyFault::UnkeyableRequirements`, and the package does not check." It is an unwanted condition. Use `If check cannot key a scalar operation application occurrence, then check SHALL fail with KeyFault::UnkeyableRequirements and SHALL produce no checked package.` | spec/functional/FR-062-implement-checked-family-contract.md:228-230 |
| FND-002 | low | "`route` SHALL provide a request builder (design name …)" uses the vague verb `provide`. The engine's lexicon does not catch it here because the sentences that follow are concrete. Fold it into the next SHALL: "`route`'s request builder SHALL take … and SHALL return one requested item per record …". | spec/functional/FR-075-compute-candidates-from-registered-backends.md:116-122 |
| FND-003 | low | Three sentences state what is absent instead of what is: "with no caller-supplied item list" / "The caller supplies no item list" (FR-075:122, ADR-012:660), "never by omission" (FR-075:134), and FR-062's closed list "Every other application (…) carries no claim of its own" (FR-062:192-195). The FR-062 list is acceptable as a closed definition. The two FR-075 phrases add nothing that "one requested item per record" does not already state, so drop them. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:122, 132-135 |
