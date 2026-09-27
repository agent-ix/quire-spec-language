---
id: SR-711
title: "Spec review of the FR-064, TC-162, ADR-012 and tests.md edits for the string-edge gate"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@9686fc5f872dc1e3fc7317f12fc7de44bec0707c; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/test-cases/TC-162-string-edge-scan-coverage.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/tests.md; spec/spec.md; docs/family-migration-recipe.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-162
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
## Summary

Ticket: QSL-145 (cross-reference QSL-268). PR: quire-spec-language#485 at
9686fc5f. Spec review (integrity and consistency) of the Status, deferral
and test-case edits this PR makes.

These edits are clean:

- spec/spec.md: the FR-064 row.
- spec/tests.md: TC-162 set to passed.
- docs/family-migration-recipe.md.
- ADR-012: the QSL-150 and QSL-155 deferral rows.

## Verdict

**Changes requested.** TC-162's Expected Results contradict its new
Status. FR-064's "Implemented" and ADR-012's "done" overstate what landed
against their own decision text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-162's Expected Results were not updated and now contradict the new Status paragraph and the tests. Step 6 still says "none of the four real sites still present in the tree is rejected today". Step 7 still says "no real `Makefile` target invokes `xtask string-edge` yet (QSL-145's own gap)". Step 7's procedure still reads "Confirm whether any real `Makefile` target invokes". | spec/test-cases/TC-162-string-edge-scan-coverage.md:86-94; spec/test-cases/TC-162-string-edge-scan-coverage.md:109-116 |
| FND-002 | medium | FR-064 Status says "Implemented and gated ... All six ACs are backed". Its own Behavior still requires detecting a comparison against "another `&str`/`String` value", which the tool does not do (SR-710 FND-002). AC-5's "each of the five" sites is covered for 3 of the 4 in the tree (SR-710 FND-001). The Status should either state these as open, with an owner, or the Behavior text should be amended to the literal-only scope. As written, the Status claims more than the code does. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:55-58; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:121-160 |
| FND-003 | medium | ADR-012 §9's site table still records decisions this PR did not implement: "typed canonicalization enum decoded at state-input intake" and "typed adapter-kind enum decoded at intake" (both marked instead, SR-709 FND-001), and "a typed clock-role variant ... the prefix is parsed once, at lexing" (wrapped in a runtime helper, SR-709 FND-002). Yet the deferral row marks QSL-145 "done". Either implement those decisions or amend the Decision column to the disposition actually taken, with an owning ticket for the remaining conversion. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:855-858; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1087 |
| FND-004 | low | The FR-064 Status and TC-162 Status add a filename rule (`tests.rs`, `*_tests.rs`) to the scan's exclusions. FR-064 Inputs and AC-3 were not amended and still scope out only `tests/` directories and `#[cfg(test)]` modules. Spec and implementation disagree. The preferred fix is to drop the rule (SR-709 FND-006) rather than amend the Inputs. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:29-30; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:131-134 |
