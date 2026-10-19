---
id: SR-13080
title: "Dependency review of PR #1023: which FR-147 criteria need a built feature"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language@54b78f5f7; git diff origin/main...HEAD: FR-147 criteria, TC-552"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-552
    type: reviews
---

## Summary

Ticket: QSL-887. STD-182 and STD-199 have merged in QSpec, so the PR removes
the "Waits on STD-199 and STD-182" markers from FR-147-AC-1, AC-3 and AC-4,
the Remaining work section, and TC-552's run-now/waits split. The external
dependency is gone. The internal one is not: the build still omits
`protocol-abstract-side` and `refinement-internal-list` (FR-135's support
set; open code PR #970), and that enablement is what decides which criteria
can pass.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147-AC-1 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-3 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-4 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-7 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-9 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| TC-552 | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| QSpec FR-375-AC-14 | quire-specification spec/functional/analysis/FR-375-declare-model-refinement.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | With the waits markers gone and no support-set condition in their place, FR-147-AC-1 ("`CasTwice` checks with … `abstract_internal = [Twice::finish]`") and FR-147-AC-7 ("Given a non-empty resolved list for `CasTwice` … refuses exactly twice") give opposite outcomes for the same fixture, and only AC-7 says which build it applies to. AC-3, AC-4 and AC-9 (node identity of a checked `CasTwice`) also need both features built. TC-552 does not say which steps run against which support set, so in the current build steps 1, 3, 4 and 9 cannot pass. QSpec FR-375-AC-14 handles this with "A producer that has built a feature is checked by that feature's own criteria". Add the same condition: AC-1, AC-3, AC-4, AC-8 and AC-9 apply to a build whose support set holds `protocol-abstract-side` and `refinement-internal-list`; AC-7 and AC-11 to one that omits them; TC-552 runs each step against the build's support set. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:248; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:17,24-46 |

## Verdict

Mergeable after FND-001. The removal of the STD-199 and STD-182 waits is right;
the support-set condition is behaviour and belongs in the criteria.
