---
id: SR-14034
title: 'Spec review of PR #1191: FR-147 node-row argument count and types (STD-249), EARS analysis'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@53cee90bd; git diff origin/main...53cee90bd (PR #1191, branch std-protocol-249, based on main, not stacked): spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md, spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md; ticket STD-249'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
---

## Summary

Ticket: STD-249. PR: quire-spec-language#1191, head 53cee90bd, based on main. Method: spec-review with the EARS sub-analysis, over the changed FR-147 bullet. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

The change removes build-log text ("Remaining work: STD-249 ... this build types each expression written") and leaves behaviour-only wording.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147 step-row statement | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:104-115 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The changed bullet holds one SHALL ("its expressions SHALL be typed against the node's binder record") followed by a colon and two refusal rules with no SHALL of their own, after the bullet's earlier SHALL for resolution. Three requirements share one bullet, and the two refusals read as illustrations of the typing rule. Fix: give the count refusal and the type refusal their own SHALL sentences. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:104-115 |

## Verdict

Mergeable for EARS apart from one low finding: the bullet packs three requirements under one SHALL.

## Dispositions

Re-checked at head ad0c475ce. Gates at head ad0c475ce: `make check-spec-validation` exit 0 with no warning on FR-147, FR-148 or TC-552; `tools/check-index-completeness.sh` exit 0; the diff touches nothing under `docs/` and no test-parsed vector table. The head contains origin/main. FR-147-AC-14 and TC-552 step 14 are used by no other open QSL PR. The PR body paraphrases no review finding.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed text at the new head: - The expressions of a node right side SHALL be typed against the node's binder record, whose fields per node kind are QSpec FR-432's. - An argument list whose length differs from the field count SHALL refuse `ill_typed`/`operator-ineligible` at the node, retaining `expected`, the field count, and `actual`, the written count. - An argument whose type is not its field's type SHALL refuse `ill_typed`/`type-mismatch` at the argument, retaining `expected`, the field's type, and `actual`, the argument's type. |
