---
id: SR-13079
title: "Scope-boundary review of PR #1023: QSL/QSpec ownership of the internal list"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@54b78f5f7; git diff origin/main...HEAD: ADR-020 RM-9/QS-11, FR-135, FR-147, FR-310"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
---

## Summary

Ticket: QSL-887. Checked that the PR hands the decisions STD-199 settled back
to QSpec and keeps only how S3 applies them. FR-135 now says QSpec FR-375
owns the feature names and the not-yet-built refusal. FR-147 says the list's
shape, spellings, resolution, refusals, node member, lock and replay are QSpec
FR-375's and FR-432's, and names `AbstractStepClass` and `abstract_internal`
as QSL's Rust-side names. The "QSL-proposed pending QSpec" text and the
Remaining work section are gone, and nothing in the changed text reads as a
build log. FR-310 builds QSpec's `abstract-subject` and leaves resolution to
S3. ADR-020 QS-11 cites FR-432. FR-147 restates QSpec's tables so that TC-552
can test them; the one drift that copy produced (the `"refinement": 7`
pointer) is recorded as SR-13075 FND-001.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147 The refinement's internal list | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 Inputs and Outputs | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-135 Refinement features the build does not check | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-310 abstract-subject bullet | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| ADR-020 RM-9, QS-11 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean on this lens. The ordering question the author raised is a QSpec
inconsistency: FR-432 orders by alias-including spellings and FR-375 orders
the node member alias-free. QSL follows FR-375, which owns the node, so it is
not a QSL defect (see SR-13075's QSpec note).
