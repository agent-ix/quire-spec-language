---
id: SR-2026111015000000000005
title: "scope-boundary review of QSL-1400 source-declared object types (PR 2674)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; PR 2674 origin/main...b029e1e03; spec/decisions/ADR-013 QC-30; spec/functional/FR-569"
review_set: subset
---

## Summary

Ticket: QSL-1400. The boundary between QSL and its contract owners (QSpec shared grammar, FR-322, FR-464, FR-465; IR's reader). A QSL ADR row is an adequate carrier for a change request, and FR-569 does state a dependency and an interim. The change request, however, is missing contract changes that FR-569 and TC-1407 rely on, and the interim covers only one of the refusals that occur.

## Verdict

FAIL on completeness of QC-30. The out-of-scope list, the `Node` / `M::Node` separation and the new reserved words are within the lead's decisions and are internally consistent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QC-30 omits three QSpec changes FR-569 relies on. (a) FR-464's `owner_model` and FR-465's role model must name a source-declared object type. (b) DeclarationOccurrenceRule must cover the `declaration` of a source clause function. (c) A `field` member must resolve on a node whose body is `aggregate{[]}`; item (2) frames this only as "not resolved against a domain document", which is not what IR checks today. QC-30's interim column states one refusal. QSL behaviour for the others is therefore left silent until QSpec acts. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1304; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:389-397 |

## Scope and evidence

Direct review. QSpec origin/main `proposals/quire-v1/shared-grammar.md:275` holds the quoted "cannot declare a domain object" sentence, so QC-30 item (1) is accurate. QSpec FR-322:658 requires `owner_model` per FR-464.

- ADR-013 QC-30 (examined).
- FR-569 Description, Out of scope and Dependencies (examined). The out-of-scope list names constructs that remain domain-package constructs and that the grammar does not admit. It names no unsupported alternative path (clean).
- FR-390 "Dependency on IR; behavior until it lands" (context_only).
